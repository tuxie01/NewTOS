//! A75 — **`appl_read` blocks on an empty queue; `appl_write` on a full queue
//! blocks until a read makes room (A116).**
//!
//! Spec: `docs/api/aes/application.md` § "Blocking behaviour (A75,
//! 2026-09-20)". EmuTOS's `ap_rdwr` (`aes/gemaplib.c:69-89`) hands a read to
//! `aqueue` (`aes/gemqueue.c:85-125`), whose readiness for a read is
//! `p_qindex > 0` (`:98-99`) — **the queue is non-empty**, not "`length`
//! bytes are buffered". A ready read is serviced at once (`doq` + `azombie(e,
//! 1)`, `:104-107`) and returns 1; an empty queue parks the caller on `p_qdq`
//! (`evinsert`, `:120-124`) and in `dsptch()` (`mwait`, `aes/gemasync.c:117-127`)
//! until a **write into the same queue** services one entry of `p_qdq`
//! (`:108-118`). There is no `0` return for "empty".
//!
//! NewTOS's cooperative substrate for that block is the one `evnt_mesag`
//! already uses (`tos-api/src/aes/event.rs`): `sched::sleep(wait_mesg(idx))`,
//! which — with no runnable peer — pumps host input (`Host::pump_input`), runs
//! the idle wake-pass and returns for a re-check; the poster
//! (`pd::post_words`, reached by `appl_write`) wakes that key. The host here
//! plays the "other process" of EmuTOS's model: on the **first** pump of a
//! block it records the sleeping slot's wake key and performs one
//! `appl_write` into the reader's queue, exactly as a peer scheduled by
//! `dsptch()` would, and the resumed `appl_read` must deliver that message.
//!
//! The write side (A116, 2026-10-22; the A75 return-0 deviation is gone): the
//! capacity is EmuTOS's arithmetic (`QUEUE_SIZE = 128` bytes,
//! `aes/struct.h:56` = eight 16-byte messages; the 9th does not fit,
//! `gemqueue.c:96-97`), and the 9th write **parks** on `p_qnq`
//! (`gemqueue.c:120-124`) until a read of that queue services it
//! (`:108-118`). Here the writer writes to its own queue and the host's first
//! pump plays the other process that reads it; the cross-process cases are in
//! `aes_a116_appl_write_blocks.rs`.
//!
//! Harness shape (silent sink backend, process fixture, parameter block)
//! follows `aes_pd_by_slot.rs`.

#![allow(clippy::too_many_arguments)]

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, Once};

use tos_api::bdos::process::{self, sched, Abi, MAX_PROCESSES, PROCESS_TABLE};
use tos_api::host::Host;
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};
use tos_types::BasePage;

const SCREEN_W: u16 = 640;
const SCREEN_H: u16 = 480;

// ── a raster backend that accepts every draw and keeps no pixels ───────────

struct SinkBackend;

impl RasterBackend for SinkBackend {
    fn width(&self) -> u16 {
        SCREEN_W
    }
    fn height(&self) -> u16 {
        SCREEN_H
    }
    fn num_colors(&self) -> u16 {
        256
    }
    fn planes(&self) -> u16 {
        16
    }
    fn set_color(&self, _index: u16, _r: u16, _g: u16, _b: u16) {}
    fn query_color(&self, _index: u16) -> (u16, u16, u16) {
        (0, 0, 0)
    }
    fn set_active_palette(&self, _palette: &[u16; 256]) {}
    fn pel_to_device16(&self, pel: Pel) -> u16 {
        pel as u16
    }
    fn device_to_rgb565(&self, argb: u32) -> u16 {
        argb as u16
    }
    fn rgb565_to_device(&self, px: u16) -> u32 {
        px as u32
    }
    fn read_pixel(&self, _x: i16, _y: i16) -> Pel {
        0
    }
    fn fill_rect(&self, _x: i16, _y: i16, _w: i16, _h: i16, _pel: Pel, _clip: ClipRect) {}
    fn abline(
        &self,
        _x0: i16,
        _y0: i16,
        _x1: i16,
        _y1: i16,
        _pel: Pel,
        _mode: WriteMode,
        _clip: ClipRect,
        mask: u16,
        _last: bool,
    ) -> u16 {
        mask
    }
    fn fill_pattern(&self, _x: i16, _y: i16, _w: i16, _h: i16, _f: PatternFill, _c: ClipRect) {}
    fn draw_glyph_sized(
        &self,
        _x: i16,
        _y: i16,
        _glyph: &[u8],
        _cell_w: i16,
        _cell_h: i16,
        _fg: Pel,
        _mode: WriteMode,
        _clip: ClipRect,
    ) {
    }
    fn blit_mono(&self, _d: Rect, _s: MonoSrc, _fg: Pel, _bg: Pel, _m: WriteMode, _c: ClipRect) {}
    fn copy_rect(&self, _src: Rect, _dx: i16, _dy: i16, _op: u8, _clip: ClipRect) {}
    fn get_device_pixel(&self, _x: i16, _y: i16) -> u32 {
        0
    }
    fn put_device_pixel(&self, _x: i16, _y: i16, _argb: u32) {}
    fn flush_rect(&self, _x0: i16, _y0: i16, _x1: i16, _y1: i16) {}
}

// ── the host: the "other process" that writes while the reader is parked ──

/// A host with no pixels whose `pump_input` — the call `sched::sleep` makes on
/// every pass of a block with no runnable peer, and the call `dsptch()` makes
/// at the end of every completed AES call — plays EmuTOS's peer process: on
/// its **first** pump it records the running slot's wake key and, if a message
/// is armed, writes it into the caller's own queue through the AES dispatcher
/// (`appl_write(0, 16, msg)`). A nested dispatch ends in its own `dsptch()`
/// pump, which the re-entrancy guard turns into a no-op.
struct PostingHost {
    raster: SinkBackend,
    clock: AtomicU64,
    /// Number of `pump_input` calls (blocking passes + the trailing `dsptch`).
    pumps: AtomicU64,
    /// `wait_cond` of the running slot at the first pump — `0` when the caller
    /// reached the pump without sleeping.
    key_seen: AtomicU64,
    /// The one message to write at the first pump, if armed.
    to_post: Mutex<Option<[i16; 8]>>,
    /// Read one message of the chain's queue at the first pump, if armed —
    /// the message delivered.
    to_read: Mutex<Option<Option<[i16; 8]>>>,
    /// Re-entrancy guard for the nested dispatch's trailing pump.
    in_pump: AtomicBool,
}

impl PostingHost {
    fn reset(&self) {
        self.pumps.store(0, Ordering::Relaxed);
        self.key_seen.store(0, Ordering::Relaxed);
        *self.to_post.lock().unwrap() = None;
        *self.to_read.lock().unwrap() = None;
        self.in_pump.store(false, Ordering::Relaxed);
    }

    fn arm(&self, msg: [i16; 8]) {
        *self.to_post.lock().unwrap() = Some(msg);
    }
}

impl Host for PostingHost {
    fn console_write_byte(&self, _byte: u8) {}
    fn console_try_read_key(&self) -> Option<u16> {
        None
    }
    fn raster(&self) -> Option<&dyn RasterBackend> {
        Some(&self.raster)
    }
    fn monotonic_millis(&self) -> u64 {
        self.clock.load(Ordering::Relaxed)
    }
    fn pump_input(&self) {
        if self.in_pump.swap(true, Ordering::AcqRel) {
            return; // the nested appl_write's own dsptch() pump
        }
        self.clock.fetch_add(1, Ordering::Relaxed);
        let n = self.pumps.fetch_add(1, Ordering::Relaxed);
        assert!(n < 100_000, "a trap is spinning on an empty queue with nothing left to post");
        if n == 0 {
            let cur = sched::current();
            if cur < MAX_PROCESSES {
                self.key_seen
                    .store(PROCESS_TABLE[cur].wait_cond.load(Ordering::Acquire), Ordering::Relaxed);
            }
            let armed = self.to_post.lock().unwrap().take();
            if let Some(msg) = armed {
                let mut pb = Aespb::new();
                assert_eq!(appl_write(&mut pb, AP_CHAIN, &msg), 1, "the peer's write must land");
            }
            let mut read = self.to_read.lock().unwrap();
            if let Some(slot) = read.as_mut() {
                let mut pb = Aespb::new();
                let (got, body) = appl_read(&mut pb, AP_CHAIN);
                assert_eq!(got, 1, "the peer's read of a full queue delivers");
                *slot = Some(body);
            }
        }
        self.in_pump.store(false, Ordering::Release);
    }
}

unsafe impl Sync for PostingHost {}
unsafe impl Send for PostingHost {}

static HOST_ONCE: Once = Once::new();
static HOST: Mutex<Option<&'static PostingHost>> = Mutex::new(None);

fn install_host() {
    HOST_ONCE.call_once(|| {
        let host: &'static PostingHost = Box::leak(Box::new(PostingHost {
            raster: SinkBackend,
            clock: AtomicU64::new(0),
            pumps: AtomicU64::new(0),
            key_seen: AtomicU64::new(0),
            to_post: Mutex::new(None),
            to_read: Mutex::new(None),
            in_pump: AtomicBool::new(false),
        }));
        let href: &'static &'static dyn Host = Box::leak(Box::new(host as &'static dyn Host));
        tos_api::host::install_host(href);
        *HOST.lock().unwrap() = Some(host);
    });
}

fn host() -> &'static PostingHost {
    HOST.lock().unwrap().expect("host installed by lock()")
}

/// One lock over every process-wide static the tests touch.
static LOCK: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    install_host();
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    host().reset();
    g
}

// ── AES parameter block ───────────────────────────────────────────────────

const ABI: Abi = Abi::Lp64;

struct Aespb {
    contrl: Box<[i16; 8]>,
    global: Box<[i16; 16]>,
    intin: Box<[i16; 16]>,
    intout: Box<[i16; 16]>,
    addrin: Box<[u64; 4]>,
    addrout: Box<[u64; 4]>,
    block: Box<[u64; 6]>,
}

impl Aespb {
    fn new() -> Aespb {
        let mut pb = Aespb {
            contrl: Box::new([0; 8]),
            global: Box::new([0; 16]),
            intin: Box::new([0; 16]),
            intout: Box::new([0; 16]),
            addrin: Box::new([0; 4]),
            addrout: Box::new([0; 4]),
            block: Box::new([0; 6]),
        };
        pb.block[0] = pb.contrl.as_ptr() as u64;
        pb.block[1] = pb.global.as_ptr() as u64;
        pb.block[2] = pb.intin.as_ptr() as u64;
        pb.block[3] = pb.intout.as_ptr() as u64;
        pb.block[4] = pb.addrin.as_ptr() as u64;
        pb.block[5] = pb.addrout.as_ptr() as u64;
        pb
    }

    fn call(&mut self, opcode: i16, in_len: i16, out_len: i16, ain_len: i16) -> i16 {
        self.contrl[0] = opcode;
        self.contrl[1] = in_len;
        self.contrl[2] = out_len;
        self.contrl[3] = ain_len;
        self.contrl[4] = 0;
        self.intout.fill(0);
        // SAFETY: all six arrays are live, LP64, and cover the declared lengths.
        assert!(
            unsafe { tos_api::aes::dispatch(self.block.as_ptr() as u64, ABI) }.is_none(),
            "no runnable peer exists here, so no AES call may yield a Switch"
        );
        self.intout[0]
    }
}

// AES opcodes used here.
const APPL_INIT: i16 = 10;
const APPL_READ: i16 = 11;
const APPL_WRITE: i16 = 12;
const APPL_EXIT: i16 = 19;

/// The pid every member of the application chain reports (`rlr->p_pid` of
/// `D.g_int[0]`, `aes/geminit.c:851-877`); `appl_write`/`appl_read` target 0
/// is the currently-scheduled client — the caller itself.
const AP_CHAIN: i16 = 0;

/// EmuTOS's queue capacity in standard messages: `QUEUE_SIZE / 16`
/// (`aes/struct.h:56`, `aes/gemqueue.c:96-97`).
const QUEUE_MSGS: usize = 8;

// ── AES calls ─────────────────────────────────────────────────────────────

fn appl_init(pb: &mut Aespb) -> i16 {
    pb.intin.fill(0);
    pb.call(APPL_INIT, 0, 1, 0)
}

fn appl_exit(pb: &mut Aespb) -> i16 {
    pb.intin.fill(0);
    pb.call(APPL_EXIT, 0, 1, 0)
}

/// `appl_read(pid, 16, buf)` → (return, the 8 WORDs delivered).
fn appl_read(pb: &mut Aespb, pid: i16) -> (i16, [i16; 8]) {
    let mut buf = [0i16; 8];
    pb.intin.fill(0);
    pb.intin[0] = pid;
    pb.intin[1] = 16;
    pb.addrin[0] = buf.as_mut_ptr() as u64;
    let r = pb.call(APPL_READ, 2, 1, 1);
    pb.addrin[0] = 0;
    (r, buf)
}

/// `appl_write(pid, 16, msg)` → return (1 = queued).
fn appl_write(pb: &mut Aespb, pid: i16, msg: &[i16; 8]) -> i16 {
    pb.intin.fill(0);
    pb.intin[0] = pid;
    pb.intin[1] = 16;
    pb.addrin[0] = msg.as_ptr() as u64;
    let r = pb.call(APPL_WRITE, 2, 1, 1);
    pb.addrin[0] = 0;
    r
}

/// Queue depth of the process reporting `pid`, without popping.
fn pending(pid: i16) -> usize {
    tos_api::aes::window::test_msg_count(pid)
}

// ── a process slot the host can "run" ─────────────────────────────────────

/// A claimed GEMDOS process slot with a basepage page of its own, made the
/// running process for GEMDOS (`RUN`) **and** for the scheduler
/// (`sched::set_running`) so that `sched::sleep` does the WAITING bookkeeping
/// a real EL0 caller gets — that is where the wake key becomes observable.
/// Released on drop through the kernel's own slot release.
struct Proc {
    slot: usize,
    page: *mut [u8; 4096],
}

impl Proc {
    fn new() -> Self {
        let slot = process::allocate_slot().expect("a free process slot");
        let page = Box::into_raw(Box::new([0u8; 4096]));
        PROCESS_TABLE[slot]
            .basepage_kva
            .store(page.cast::<BasePage>(), Ordering::Release);
        Proc { slot, page }
    }

    fn bp(&self) -> *mut BasePage {
        self.page.cast::<BasePage>()
    }

    /// Make this the running process for GEMDOS and the scheduler.
    fn run(&self) {
        process::set_current(self.bp());
        sched::set_running(self.slot);
    }
}

impl Drop for Proc {
    fn drop(&mut self) {
        if process::current() == self.bp() {
            process::set_current(std::ptr::null_mut());
        }
        // Leave the scheduler with no running slot and this slot out of the
        // run-set, so a later test's sleep finds no stale peer.
        sched::set_running(MAX_PROCESSES);
        sched::unready(self.slot);
        // SAFETY: single-threaded under `LOCK`; no frame snapshot is consumed
        // on the host.
        unsafe { process::pterm::release_slot(self.slot) };
        // SAFETY: `page` came from `Box::into_raw` above and is not aliased
        // once the slot no longer names it.
        unsafe { drop(Box::from_raw(self.page)) };
    }
}

// ── (a) a queued message is delivered at once ────────────────────────────

/// `aqueue`'s read readiness is `p_qindex > 0` (`gemqueue.c:98-99`): a
/// non-empty queue is serviced immediately (`doq` + `azombie(e, 1)`) and the
/// call returns 1 with the message — no sleep, so no wake key is ever set.
#[test]
fn appl_read_on_a_non_empty_queue_returns_1_with_the_message_without_sleeping() {
    let _g = lock();
    let p = Proc::new();
    p.run();
    let mut pb = Aespb::new();
    assert_eq!(appl_init(&mut pb), AP_CHAIN);

    let probe = [0x4711, AP_CHAIN, 0, 1, 2, 3, 4, 5];
    assert_eq!(appl_write(&mut pb, AP_CHAIN, &probe), 1, "setup: queue one message");
    assert_eq!(pending(AP_CHAIN), 1);
    host().reset();

    let (got, body) = appl_read(&mut pb, AP_CHAIN);
    assert_eq!((got, body), (1, probe), "appl_read of a queued message");
    assert_eq!(pending(AP_CHAIN), 0, "the message was dequeued");
    assert_eq!(
        host().key_seen.load(Ordering::Relaxed),
        0,
        "a ready read must not sleep: no wake key was set when the trailing dsptch pumped"
    );

    assert_eq!(appl_exit(&mut pb), 1);
    drop(p);
}

// ── (b) an empty queue blocks until a write lands ────────────────────────

/// An empty queue parks the reader (`evinsert` on `p_qdq`, `gemqueue.c:120-124`;
/// `mwait` → `dsptch()`, `gemasync.c:117-127`) until a write into the same
/// queue services it (`:108-118`); the call then returns 1 with **that**
/// message. It never returns 0 for "empty".
///
/// Observed through the host: the first `pump_input` of the block sees the
/// running slot WAITING on `sched::wait_mesg(slot)` — the key `pd::post_words`
/// wakes — and performs the peer's `appl_write`; the resumed `appl_read`
/// delivers it.
#[test]
fn appl_read_on_an_empty_queue_blocks_and_delivers_the_message_written_meanwhile() {
    let _g = lock();
    let p = Proc::new();
    p.run();
    let mut pb = Aespb::new();
    assert_eq!(appl_init(&mut pb), AP_CHAIN);
    assert_eq!(pending(AP_CHAIN), 0, "setup: the queue starts empty");

    let probe = [0x4712, AP_CHAIN, 0, 9, 8, 7, 6, 5];
    host().reset();
    host().arm(probe);

    let (got, body) = appl_read(&mut pb, AP_CHAIN);
    assert_eq!(
        got, 1,
        "appl_read on an empty queue returned {got} instead of blocking until the \
         peer's write landed (EmuTOS aqueue, aes/gemqueue.c:98-124: no 0 for empty)"
    );
    assert_eq!(body, probe, "the message written during the block is the one delivered");
    assert_eq!(pending(AP_CHAIN), 0, "…and it was dequeued");
    assert_eq!(
        host().key_seen.load(Ordering::Relaxed),
        sched::wait_mesg(p.slot),
        "the reader must sleep on WAIT_MESG(its own identity) — the key the poster wakes"
    );
    assert!(
        host().pumps.load(Ordering::Relaxed) >= 2,
        "at least one blocking pass plus the trailing dsptch pump"
    );

    assert_eq!(appl_exit(&mut pb), 1);
    drop(p);
}

// ── (c) the write side: capacity as EmuTOS, full → the writer parks ─────

/// `QUEUE_SIZE = 128` bytes holds eight 16-byte messages (`aes/struct.h:56`);
/// the 9th does not fit (`qpb_cnt > QUEUE_SIZE - p_qindex`, `gemqueue.c:96-97`)
/// and the writer is parked on `p_qnq` (`:120-124`) — sleeping on
/// `sched::wait_qnq(its identity)` — until a read of that queue services it
/// (`:108-118`). The host's first pump plays the other process that reads the
/// queue: it gets message 1, the parked message 9 takes the freed room, and
/// the write completes with 1. Messages 2–9 then read back in order.
#[test]
fn appl_write_on_a_full_queue_parks_until_a_read_of_that_queue() {
    let _g = lock();
    let p = Proc::new();
    p.run();
    let mut pb = Aespb::new();
    assert_eq!(appl_init(&mut pb), AP_CHAIN);
    assert_eq!(pending(AP_CHAIN), 0, "setup: the queue starts empty");

    let msg = |n: i16| [0x4800 + n, AP_CHAIN, 0, n, 0, 0, 0, 0];
    for n in 1..=QUEUE_MSGS as i16 {
        assert_eq!(appl_write(&mut pb, AP_CHAIN, &msg(n)), 1, "message {n} of {QUEUE_MSGS} fits");
    }
    assert_eq!(pending(AP_CHAIN), QUEUE_MSGS);
    host().reset();
    *host().to_read.lock().unwrap() = Some(None);
    assert_eq!(
        appl_write(&mut pb, AP_CHAIN, &msg(9)),
        1,
        "the 9th write completes with 1 once the read has made room"
    );
    assert_eq!(
        host().key_seen.load(Ordering::Relaxed),
        sched::wait_qnq(p.slot),
        "the writer must have slept on WAIT_QNQ(its own identity) — the key the read wakes"
    );
    assert_eq!(*host().to_read.lock().unwrap(), Some(Some(msg(1))), "the reader got message 1");
    assert_eq!(pending(AP_CHAIN), QUEUE_MSGS, "message 9 took the freed room");

    for n in 2..=9 {
        assert_eq!(appl_read(&mut pb, AP_CHAIN), (1, msg(n)), "message {n} reads back in order");
    }
    assert_eq!(pending(AP_CHAIN), 0);

    assert_eq!(appl_exit(&mut pb), 1);
    drop(p);
}
