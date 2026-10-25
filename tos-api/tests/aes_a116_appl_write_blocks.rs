//! A116 — **`appl_write` on a full queue blocks the writer until a read of
//! that queue makes room.**
//!
//! Spec: `docs/api/aes/application.md` § "Blocking behaviour (A75,
//! 2026-09-20)" (write half revised 2026-10-22, A116). EmuTOS `aqueue`
//! (`aes/gemqueue.c:85-125`): write readiness is `qpb_cnt <= QUEUE_SIZE -
//! p_qindex` (`:96-97`, `QUEUE_SIZE = 128`, `aes/struct.h:56`); a write that
//! does not fit is linked onto the target's `p_qnq` (`evinsert`, `:120-124`,
//! which links at the **head**, `aes/gemasync.c:81-93`) and the writer sleeps
//! in `mwait` (`gemasync.c:117-127`); a later successful read of the same
//! queue services the head of `p_qnq` with `doq` + `azombie(e, 1)`
//! (`:108-118`), so the parked `appl_write` returns 1. `ap_rdwr`'s 16-byte
//! fast path (`aes/gemaplib.c:77-82`) reads without `aqueue` and services
//! nobody. EmuTOS makes no exception for a self-targeted write.
//!
//! Pinned here, at the AES trap boundary:
//!
//! 1. a program fills an accessory's queue (8 standard messages); the 9th
//!    write parks the program, and the accessory's `evnt_mesag` (or
//!    `evnt_multi` `MU_MESAG`, or `appl_read`) releases it with 1, its message
//!    queued behind the other seven;
//! 2. a parked writer re-executed without a read in between stays parked;
//! 3. several writers parked on one queue are served one per read, the most
//!    recently parked first;
//! 4. a writer that ends while parked leaves no message behind;
//! 5. a target that ends releases its parked writers with 1 (NewTOS-only);
//! 6. a self-write on the writer's own full queue parks until another process
//!    reads that queue;
//! 7. readiness counts bytes: a 128-byte message parks on a queue holding 16
//!    bytes, and a fast-path read does not release it.
//!
//! Harness: GEMDOS process slots the host "runs" (`RUN` + `sched::set_running`)
//! and re-issues the parameter block of a call that yielded (the S2
//! restartable block), as `aes_a112_wake_conditions.rs` does; accessories are
//! registered as `aes_a113_accessory_pid.rs` does. Whether a call
//! **completed** is read from `int_out[0]`, pre-set to a sentinel no call
//! returns.

#![allow(clippy::too_many_arguments)]

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, Once};

use tos_api::bdos::process::{self, sched, Abi, State, MAX_PROCESSES, PROCESS_TABLE};
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

// ── the host: a clock, one millisecond per input pump ──────────────────────

/// A host with no pixels and no keys. Every `pump_input` (an idle pass of a
/// block with nobody else ready, and the drain every completed AES call ends
/// with) advances the clock; a runaway loop trips the bound — which is what a
/// writer parked with nobody to read would do.
struct ClockHost {
    raster: SinkBackend,
    clock: AtomicU64,
    pumps: AtomicU64,
}

impl Host for ClockHost {
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
        self.clock.fetch_add(1, Ordering::Relaxed);
        let n = self.pumps.fetch_add(1, Ordering::Relaxed);
        assert!(n < 100_000, "a trap is spinning");
    }
}

unsafe impl Sync for ClockHost {}
unsafe impl Send for ClockHost {}

static HOST_ONCE: Once = Once::new();
static HOST: Mutex<Option<&'static ClockHost>> = Mutex::new(None);

fn install_host() {
    HOST_ONCE.call_once(|| {
        let host: &'static ClockHost = Box::leak(Box::new(ClockHost {
            raster: SinkBackend,
            clock: AtomicU64::new(0),
            pumps: AtomicU64::new(0),
        }));
        let href: &'static &'static dyn Host = Box::leak(Box::new(host as &'static dyn Host));
        tos_api::host::install_host(href);
        *HOST.lock().unwrap() = Some(host);
    });
}

fn host() -> &'static ClockHost {
    HOST.lock().unwrap().expect("host installed by lock()")
}

/// One lock over every process-wide static the tests touch.
static LOCK: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    install_host();
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    host().pumps.store(0, Ordering::Relaxed);
    tos_api::aes::window::wm_new();
    g
}

// ── AES parameter block ───────────────────────────────────────────────────

const ABI: Abi = Abi::Lp64;

/// `int_out[0]` before a call: no AES call returns it, so finding it there
/// afterwards means the call did not complete (no copy-back happened).
const NOT_COMPLETED: i16 = -0x5A5A;

struct Aespb {
    contrl: Box<[i16; 8]>,
    global: Box<[i16; 16]>,
    intin: Box<[i16; 16]>,
    intout: Box<[i16; 16]>,
    addrin: Box<[u64; 4]>,
    addrout: Box<[u64; 4]>,
    block: Box<[u64; 6]>,
}

/// What one trap did: the `Switch` the dispatcher returned (its target slot,
/// and whether it is the closing dispatch of a completed call), and
/// `RET_CODE` when the call completed.
struct Outcome {
    switch_to: Option<usize>,
    ret: Option<i16>,
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

    /// Issue (or re-issue) the call in `contrl` as the running slot.
    fn trap(&mut self) -> Outcome {
        self.intout.fill(0);
        self.intout[0] = NOT_COMPLETED;
        // SAFETY: all six arrays are live, LP64, and cover the declared lengths.
        let sw = unsafe { tos_api::aes::dispatch(self.block.as_ptr() as u64, ABI) };
        let ret = (self.intout[0] != NOT_COMPLETED).then_some(self.intout[0]);
        if let Some(s) = &sw {
            assert_eq!(
                s.completed,
                ret.is_some(),
                "a completed call yields only as its closing dispatch; a mid-call yield completes nothing"
            );
        }
        Outcome { switch_to: sw.map(|s| s.slot), ret }
    }

    fn call(&mut self, opcode: i16, in_len: i16, out_len: i16, ain_len: i16) -> Outcome {
        self.contrl[0] = opcode;
        self.contrl[1] = in_len;
        self.contrl[2] = out_len;
        self.contrl[3] = ain_len;
        self.contrl[4] = 0;
        self.trap()
    }
}

const APPL_INIT: i16 = 10;
const APPL_READ: i16 = 11;
const APPL_WRITE: i16 = 12;
const EVNT_MESAG: i16 = 23;
const EVNT_MULTI: i16 = 25;

const MU_MESAG: i16 = 0x0010;

/// The application chain's pid (`D.g_int[0]`, `aes/geminit.c:851-877`).
const AP_CHAIN: i16 = 0;
/// The first accessory's pid (`D.g_acc[0]`, `aes/geminit.c:819,851-888`).
const ACC0: i16 = 2;
/// The second accessory's pid.
const ACC1: i16 = 3;

/// Standard messages that fit the 128-byte queue (`aes/struct.h:56`).
const QUEUE_MSGS: i16 = 8;

/// A standard 16-byte message tagged `n` from sender `from`.
fn msg(from: i16, n: i16) -> [i16; 8] {
    [0x4A00 + n, from, 0, n, 0, 0, 0, 0]
}

fn appl_init(pb: &mut Aespb) -> i16 {
    pb.intin.fill(0);
    pb.call(APPL_INIT, 0, 1, 0).ret.expect("appl_init completes")
}

/// `appl_write(pid, len_bytes, buf)`.
fn appl_write_len(pb: &mut Aespb, pid: i16, buf: &[i16], len_bytes: i16) -> Outcome {
    pb.intin.fill(0);
    pb.intin[0] = pid;
    pb.intin[1] = len_bytes;
    pb.addrin.fill(0);
    pb.addrin[0] = buf.as_ptr() as u64;
    pb.call(APPL_WRITE, 2, 1, 1)
}

/// `appl_write(pid, 16, msg)`.
fn appl_write(pb: &mut Aespb, pid: i16, m: &[i16; 8]) -> Outcome {
    appl_write_len(pb, pid, m, 16)
}

/// `appl_read(pid, 16, buf)`.
fn appl_read(pb: &mut Aespb, pid: i16, buf: &mut [i16; 8]) -> Outcome {
    pb.intin.fill(0);
    pb.intin[0] = pid;
    pb.intin[1] = 16;
    pb.addrin.fill(0);
    pb.addrin[0] = buf.as_mut_ptr() as u64;
    pb.call(APPL_READ, 2, 1, 1)
}

/// `evnt_mesag(buf)`.
fn evnt_mesag(pb: &mut Aespb, buf: &mut [i16; 8]) -> Outcome {
    pb.intin.fill(0);
    pb.addrin.fill(0);
    pb.addrin[0] = buf.as_mut_ptr() as u64;
    pb.call(EVNT_MESAG, 0, 1, 1)
}

/// `evnt_multi(MU_MESAG, …, buf)`.
fn evnt_multi_mesag(pb: &mut Aespb, buf: &mut [i16; 8]) -> Outcome {
    pb.intin.fill(0);
    pb.intin[0] = MU_MESAG;
    pb.addrin.fill(0);
    pb.addrin[0] = buf.as_mut_ptr() as u64;
    pb.call(EVNT_MULTI, 16, 7, 1)
}

/// Queue depth of the process reporting `pid`, without popping.
fn pending(pid: i16) -> usize {
    tos_api::aes::window::test_msg_count(pid)
}

/// Read one queued message of `pid` (as the running process) — must be there.
fn read_one(pb: &mut Aespb, pid: i16) -> [i16; 8] {
    assert!(pending(pid) > 0, "read_one({pid}): nothing queued (appl_read would block)");
    let mut buf = [0i16; 8];
    let o = appl_read(pb, pid, &mut buf);
    assert_eq!(o.ret, Some(1), "a queued message is delivered at once");
    buf
}

/// Fill `pid`'s queue with the eight standard messages `msg(from, 1..=8)`.
fn fill(pb: &mut Aespb, pid: i16, from: i16) {
    for n in 1..=QUEUE_MSGS {
        let o = appl_write(pb, pid, &msg(from, n));
        assert_eq!(o.ret, Some(1), "message {n} of {QUEUE_MSGS} fits (gemqueue.c:96-97)");
    }
    assert_eq!(pending(pid), QUEUE_MSGS as usize);
}

// ── process slots the host can "run" ───────────────────────────────────────

/// A claimed GEMDOS process slot with a basepage page of its own; released on
/// drop through the kernel's slot release (`pterm::release_slot`).
struct Proc {
    slot: usize,
    page: *mut [u8; 4096],
    gone: bool,
}

impl Proc {
    fn new() -> Self {
        let slot = process::allocate_slot().expect("a free process slot");
        let page = Box::into_raw(Box::new([0u8; 4096]));
        PROCESS_TABLE[slot].basepage_kva.store(page.cast::<BasePage>(), Ordering::Release);
        Proc { slot, page, gone: false }
    }

    /// A program the kernel launcher started — a live member of the chain.
    fn program(path: &[u8]) -> Self {
        let p = Proc::new();
        tos_api::aes::register_process(p.slot, path);
        PROCESS_TABLE[p.slot].set_state(State::Running);
        p
    }

    /// A desk accessory the loader started (`acc::register_process`).
    fn accessory(path: &[u8]) -> Self {
        let p = Proc::new();
        tos_api::aes::register_process(p.slot, path);
        tos_api::aes::acc::register_process(p.slot);
        p
    }

    fn bp(&self) -> *mut BasePage {
        self.page.cast::<BasePage>()
    }

    /// Make this the running process for GEMDOS and the scheduler.
    fn run(&self) {
        process::set_current(self.bp());
        sched::set_running(self.slot);
    }

    /// Put this slot on the ready list as a process that has run before.
    fn ready(&self) {
        PROCESS_TABLE[self.slot].has_ctx.store(true, Ordering::Release);
        sched::ready(self.slot);
    }

    fn is_ready(&self) -> bool {
        PROCESS_TABLE[self.slot].runnable.load(Ordering::Acquire)
    }

    /// The process ends: its slot is released (every way a process ends goes
    /// through `pterm::release_slot`).
    fn end(&mut self) {
        if process::current() == self.bp() {
            process::set_current(std::ptr::null_mut());
        }
        sched::set_running(MAX_PROCESSES);
        sched::unready(self.slot);
        PROCESS_TABLE[self.slot].wait_cond.store(0, Ordering::Release);
        // SAFETY: single-threaded under `LOCK`; no frame snapshot is consumed
        // on the host.
        unsafe { process::pterm::release_slot(self.slot) };
        self.gone = true;
    }
}

impl Drop for Proc {
    fn drop(&mut self) {
        if !self.gone {
            self.end();
        }
        // SAFETY: `page` came from `Box::into_raw` above and is not aliased
        // once the slot no longer names it.
        unsafe { drop(Box::from_raw(self.page)) };
    }
}

// ── 1. a program fills an accessory's queue; the accessory's read releases ─

/// The program's 9th standard message does not fit the accessory's queue
/// (`p_qindex == 128`, `gemqueue.c:96-97`): the write is parked and the
/// program gives up the CPU instead of returning. The accessory's
/// `evnt_mesag` (`ev_mesag` → `ap_rdwr(MU_MESAG, rlr, 16, …)`, a slow-path
/// read of a full queue) delivers message 1 and services the parked write
/// (`gemqueue.c:108-118`): message 9 now sits behind 2–8, and the program's
/// re-executed `appl_write` returns 1.
#[test]
fn a_write_to_a_full_accessory_queue_parks_until_the_accessorys_evnt_mesag() {
    let _g = lock();
    let x = Proc::accessory(b"C:\\ACCX.NAC");
    let m = Proc::program(b"C:\\MAIN.NAP");
    let (mut px, mut pm) = (Aespb::new(), Aespb::new());
    x.run();
    assert_eq!(appl_init(&mut px), ACC0);
    m.run();
    assert_eq!(appl_init(&mut pm), AP_CHAIN);

    fill(&mut pm, ACC0, AP_CHAIN);
    x.ready();
    let w9 = appl_write(&mut pm, ACC0, &msg(AP_CHAIN, 9));
    assert_eq!(
        w9.ret, None,
        "the 9th message does not fit: EmuTOS parks the writer on p_qnq \
         (gemqueue.c:120-124) — it neither returns 0 nor drops the message"
    );
    assert_eq!(w9.switch_to, Some(x.slot), "…and gives the CPU to the ready accessory");
    assert!(!m.is_ready(), "…the writer is WAITING");
    assert_eq!(pending(ACC0), QUEUE_MSGS as usize, "nothing was queued yet");

    x.run();
    let mut got = [0i16; 8];
    let r = evnt_mesag(&mut px, &mut got);
    assert_eq!((r.ret, got), (Some(1), msg(AP_CHAIN, 1)), "the accessory reads message 1");
    assert!(m.is_ready(), "the read serviced the parked write and woke the writer (azombie)");
    assert_eq!(pending(ACC0), QUEUE_MSGS as usize, "message 9 took the freed room");

    m.run();
    let resumed = pm.trap();
    assert_eq!(resumed.ret, Some(1), "the re-executed appl_write completes with 1");

    x.run();
    for n in 2..=9 {
        assert_eq!(read_one(&mut px, ACC0), msg(AP_CHAIN, n), "message {n} in order");
    }
    assert_eq!(pending(ACC0), 0);
}

/// Same with `evnt_multi(MU_MESAG)`: its quick check reads through `ev_mesag`
/// (`gemevlib.c:222-228`), the same `ap_rdwr` read.
#[test]
fn evnt_multi_mesag_releases_a_parked_writer_too() {
    let _g = lock();
    let x = Proc::accessory(b"C:\\ACCX.NAC");
    let m = Proc::program(b"C:\\MAIN.NAP");
    let (mut px, mut pm) = (Aespb::new(), Aespb::new());
    x.run();
    appl_init(&mut px);
    m.run();
    appl_init(&mut pm);

    fill(&mut pm, ACC0, AP_CHAIN);
    x.ready();
    assert_eq!(appl_write(&mut pm, ACC0, &msg(AP_CHAIN, 9)).ret, None, "parked");

    x.run();
    let mut got = [0i16; 8];
    let r = evnt_multi_mesag(&mut px, &mut got);
    assert_eq!((r.ret, got), (Some(MU_MESAG), msg(AP_CHAIN, 1)));
    assert!(m.is_ready(), "the MU_MESAG read serviced the parked write");
    m.run();
    assert_eq!(pm.trap().ret, Some(1));
    x.run();
    for n in 2..=9 {
        assert_eq!(read_one(&mut px, ACC0), msg(AP_CHAIN, n));
    }
}

// ── 2. re-executed without a read: still parked; appl_read releases ───────

/// The S2 restart of a parked write that nobody has serviced sleeps again —
/// it neither completes nor queues a second copy. `appl_read` of the target's
/// queue is a read like `evnt_mesag` (`ap_rdwr` → `aqueue`) and releases it.
#[test]
fn a_parked_writer_stays_parked_until_appl_read_of_its_target() {
    let _g = lock();
    let x = Proc::accessory(b"C:\\ACCX.NAC");
    let m = Proc::program(b"C:\\MAIN.NAP");
    let (mut px, mut pm) = (Aespb::new(), Aespb::new());
    x.run();
    appl_init(&mut px);
    m.run();
    appl_init(&mut pm);

    fill(&mut pm, ACC0, AP_CHAIN);
    x.ready();
    assert_eq!(appl_write(&mut pm, ACC0, &msg(AP_CHAIN, 9)).ret, None, "parked");

    // Resumed spuriously (no read happened): the restart parks again.
    x.ready();
    m.run();
    let again = pm.trap();
    assert_eq!(again.ret, None, "no read has made room: the write is still parked");
    assert_eq!(again.switch_to, Some(x.slot));
    assert!(!m.is_ready());
    assert_eq!(pending(ACC0), QUEUE_MSGS as usize, "and nothing was queued twice");

    x.run();
    assert_eq!(read_one(&mut px, ACC0), msg(AP_CHAIN, 1));
    assert!(m.is_ready(), "appl_read serviced the parked write");
    m.run();
    assert_eq!(pm.trap().ret, Some(1));
    x.run();
    for n in 2..=9 {
        assert_eq!(read_one(&mut px, ACC0), msg(AP_CHAIN, n));
    }
}

// ── 3. several parked writers: one per read, newest first ─────────────────

/// Two writers park on one full queue. `evinsert` links each new `EVB` at the
/// head of `p_qnq` (`aes/gemasync.c:81-93`) and `aqueue` services the head
/// (`gemqueue.c:108-112`): the first read releases the writer that parked
/// **last**, the second read the other one — one writer per read.
#[test]
fn several_parked_writers_are_served_one_per_read_most_recent_first() {
    let _g = lock();
    let x = Proc::accessory(b"C:\\ACCX.NAC");
    let y = Proc::accessory(b"C:\\ACCY.NAC");
    let m = Proc::program(b"C:\\MAIN.NAP");
    let (mut px, mut py, mut pm) = (Aespb::new(), Aespb::new(), Aespb::new());
    x.run();
    assert_eq!(appl_init(&mut px), ACC0);
    y.run();
    assert_eq!(appl_init(&mut py), ACC1);
    m.run();
    appl_init(&mut pm);

    fill(&mut pm, ACC0, AP_CHAIN);
    x.ready();
    assert_eq!(appl_write(&mut pm, ACC0, &msg(AP_CHAIN, 9)).ret, None, "the program parks first");
    y.run();
    assert_eq!(appl_write(&mut py, ACC0, &msg(ACC1, 9)).ret, None, "the second accessory parks after it");
    assert!(!m.is_ready() && !y.is_ready());

    x.run();
    assert_eq!(read_one(&mut px, ACC0), msg(AP_CHAIN, 1));
    assert!(y.is_ready(), "the first read serves the most recently parked writer (p_qnq head)");
    assert!(!m.is_ready(), "…and only that one");
    assert_eq!(read_one(&mut px, ACC0), msg(AP_CHAIN, 2));
    assert!(m.is_ready(), "the second read serves the next one");

    y.run();
    assert_eq!(py.trap().ret, Some(1));
    m.run();
    assert_eq!(pm.trap().ret, Some(1));
    x.run();
    for n in 3..=8 {
        assert_eq!(read_one(&mut px, ACC0), msg(AP_CHAIN, n));
    }
    assert_eq!(read_one(&mut px, ACC0), msg(ACC1, 9), "the accessory's message came in first");
    assert_eq!(read_one(&mut px, ACC0), msg(AP_CHAIN, 9));
    assert_eq!(pending(ACC0), 0);
}

// ── 4. a writer that ends while parked ────────────────────────────────────

/// A writer whose process ends while its write is parked takes the write with
/// it: the reads that follow deliver the eight queued messages and nothing of
/// the dead writer's, and the next tenant of its slot writes normally.
#[test]
fn a_writer_that_ends_while_parked_leaves_nothing_behind() {
    let _g = lock();
    let x = Proc::accessory(b"C:\\ACCX.NAC");
    let mut m = Proc::program(b"C:\\MAIN.NAP");
    let (mut px, mut pm) = (Aespb::new(), Aespb::new());
    x.run();
    appl_init(&mut px);
    m.run();
    appl_init(&mut pm);

    fill(&mut pm, ACC0, AP_CHAIN);
    x.ready();
    assert_eq!(appl_write(&mut pm, ACC0, &msg(AP_CHAIN, 9)).ret, None, "parked");
    let slot = m.slot;
    m.end();

    x.run();
    for n in 1..=8 {
        assert_eq!(read_one(&mut px, ACC0), msg(AP_CHAIN, n));
    }
    assert_eq!(pending(ACC0), 0, "the dead writer's message never arrives");

    let next = Proc::program(b"C:\\NEXT.NAP");
    assert_eq!(next.slot, slot, "setup: the slot is reused");
    let mut pn = Aespb::new();
    next.run();
    appl_init(&mut pn);
    let w = appl_write(&mut pn, ACC0, &msg(AP_CHAIN, 0x10));
    assert_eq!(w.ret, Some(1), "the slot's next tenant starts with no write in flight");
    assert_eq!(pending(ACC0), 1);
    x.run();
    assert_eq!(read_one(&mut px, ACC0), msg(AP_CHAIN, 0x10));
}

// ── 5. a target that ends while writers are parked on it ──────────────────

/// NewTOS-only: EmuTOS's descriptors are never released (accessories never
/// end, `D.g_int[0]` outlives every program), so a parked writer there always
/// has a reader left. A NewTOS identity can end; the writers parked on its
/// queue complete with 1, their messages discarded with the queue.
#[test]
fn a_target_that_ends_releases_its_parked_writers_with_1() {
    let _g = lock();
    let mut x = Proc::accessory(b"C:\\ACCX.NAC");
    let m = Proc::program(b"C:\\MAIN.NAP");
    let (mut px, mut pm) = (Aespb::new(), Aespb::new());
    x.run();
    appl_init(&mut px);
    m.run();
    appl_init(&mut pm);

    fill(&mut pm, ACC0, AP_CHAIN);
    x.ready();
    assert_eq!(appl_write(&mut pm, ACC0, &msg(AP_CHAIN, 9)).ret, None, "parked");
    x.end();
    assert!(m.is_ready(), "the target's release wakes the parked writer");
    m.run();
    assert_eq!(pm.trap().ret, Some(1), "…which completes with 1");
}

// ── 6. a self-write on the writer's own full queue ────────────────────────

/// EmuTOS has no self-check (`fpdnm(NULL, id)`, edge case 7): a write to the
/// caller's own full queue parks like any other, and only a read of that
/// queue by another process releases it — here an accessory's
/// `appl_read(0, …)`, which reaches the chain's queue (A113).
#[test]
fn a_self_write_on_a_full_own_queue_parks_until_another_process_reads_it() {
    let _g = lock();
    let x = Proc::accessory(b"C:\\ACCX.NAC");
    let m = Proc::program(b"C:\\MAIN.NAP");
    let (mut px, mut pm) = (Aespb::new(), Aespb::new());
    x.run();
    appl_init(&mut px);
    m.run();
    appl_init(&mut pm);

    fill(&mut pm, AP_CHAIN, AP_CHAIN);
    x.ready();
    let w9 = appl_write(&mut pm, AP_CHAIN, &msg(AP_CHAIN, 9));
    assert_eq!(w9.ret, None, "a self-write on a full own queue parks (gemqueue.c:120-124)");
    assert_eq!(w9.switch_to, Some(x.slot));

    x.run();
    assert_eq!(read_one(&mut px, AP_CHAIN), msg(AP_CHAIN, 1), "the accessory reads the chain's queue");
    assert!(m.is_ready(), "…which services the parked self-write");
    m.run();
    assert_eq!(pm.trap().ret, Some(1));
    for n in 2..=9 {
        assert_eq!(read_one(&mut pm, AP_CHAIN), msg(AP_CHAIN, n));
    }
}

// ── 7. readiness counts bytes; the fast path services nobody ──────────────

/// Readiness is `qpb_cnt <= QUEUE_SIZE - p_qindex` in **bytes**
/// (`gemqueue.c:96-97`): a 128-byte message parks on a queue that holds one
/// 16-byte message. The accessory's `evnt_mesag` of that one message is
/// `ap_rdwr`'s fast path (`p_qindex == length == 16`, `gemaplib.c:77-82`),
/// which reads without `aqueue` — the parked writer is not serviced, though
/// the queue is now empty.
#[test]
fn a_128_byte_message_parks_on_a_queue_holding_16_bytes_and_the_fast_path_read_leaves_it() {
    let _g = lock();
    let x = Proc::accessory(b"C:\\ACCX.NAC");
    let mut m = Proc::program(b"C:\\MAIN.NAP");
    let (mut px, mut pm) = (Aespb::new(), Aespb::new());
    x.run();
    appl_init(&mut px);
    m.run();
    appl_init(&mut pm);

    assert_eq!(appl_write(&mut pm, ACC0, &msg(AP_CHAIN, 1)).ret, Some(1));
    x.ready();
    let long: [i16; 64] = core::array::from_fn(|i| 0x5100 + i as i16);
    let w = appl_write_len(&mut pm, ACC0, &long, 128);
    assert_eq!(w.ret, None, "128 > 128 - 16: the 128-byte message does not fit and parks");

    x.run();
    let mut got = [0i16; 8];
    assert_eq!(evnt_mesag(&mut px, &mut got).ret, Some(1));
    assert_eq!(got, msg(AP_CHAIN, 1));
    assert_eq!(pending(ACC0), 0, "the queue is empty now");
    assert!(
        !m.is_ready(),
        "the 16-from-16 read is ap_rdwr's fast path: it services no parked writer"
    );
    m.end();
}
