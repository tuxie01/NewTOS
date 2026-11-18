//! A62 (stage 1) — **the AES process descriptor is keyed by the GEMDOS slot.**
//!
//! Spec: `docs/api/aes/application.md` § "AES process identity and pid",
//! NewTOS contract **(a) Identity**: a client's descriptor (queue, `AP_*`
//! flags, name, wait-key, resource list, window ownership) is keyed by its
//! **GEMDOS process slot**, plus one reserved index for the kernel-context
//! desktop — table size `MAX_PROCESSES + 1`. Because every slot has its own
//! descriptor by construction, **`appl_init` never returns `-1`** for "table
//! full" (EmuTOS `ap_init`, `aes/gemaplib.c:54-63`, three unconditional
//! statements).
//!
//! Stage 2 (A56) makes the **reported value** EmuTOS's — contract **(b)**
//! of the same spec section: `ap_id`, `appl_find`, `WF_OWNER` and `msg[1]`
//! are **0** for every member of the application chain (the kernel-context
//! desktop, the launched program, every `Pexec` child at any depth — one
//! `rlr`, `aes/geminit.c:850-877`, `gemshlib.c:572`, `gemaplib.c:54-63`);
//! `msg[1]` is **1** only as the Screen Manager's sender stamp on the
//! control-manager messages (`gemctrl.c:81-368`); `appl_write`/`appl_read`
//! target 0 is the currently-scheduled client. The four tests here are, in
//! order:
//!
//! * **(a)** sixteen nested processes, each calling `appl_init` — none gets
//!   `-1` (the measured defect: depth 9 → `-1`,
//!   `measurements/measurement-2026-09-20-a62/serial/nest-serial.log`);
//! * **(b)** nine programs one after another in the same slot — `appl_find`
//!   answers for the current occupant only (EmuTOS `p_nameit` overwrites the
//!   one `p_name` in place, `aes/gemshlib.c:633`);
//! * **(c)** a slot's release resets its descriptor — a program that dies
//!   with `AP_OPEN` set and a message unread leaves nothing for the slot's
//!   next tenant (the `handle_pexec` side finding of the A62 report, §8);
//! * **(d)** the reported values, contract (b): 0 for the chain, 1 as the
//!   Screen Manager's stamp, target 0 = the caller.
//!
//! The process table, `RUN`, the AES descriptor table and the window table are
//! process-wide statics; the tests serialise on one lock and release every
//! slot they claim.
//!
//! Harness shape (backend + host + parameter block) follows
//! `aes_app_return_descriptor.rs` (silent sink, no pixels); the process
//! fixture follows `aes_rsrc_per_process.rs`.

#![allow(clippy::too_many_arguments)]

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, Once};

use tos_api::bdos::process::{self, Abi, MAX_PROCESSES, PROCESS_TABLE};
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
    fn set_active_palette(&self, _palette: &[u32; 256]) {}
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

// ── the host ──────────────────────────────────────────────────────────────

/// One synthetic mouse packet, as the VDI's `mouse_int` receives it
/// (`aes_window_drag.rs`'s shape).
#[derive(Copy, Clone, Debug)]
struct Packet {
    dx: i16,
    dy: i16,
    /// VDI convention: bit 0 = left.
    buttons: u8,
}

/// A host with no pixels and a scripted pointer: `pump_input` feeds the next
/// packet, if any, so a trap that waits for the button to come up
/// (`intercept`'s `wait_release`) can be given the release it waits for.
struct SilentHost {
    raster: SinkBackend,
    clock: AtomicU64,
    script: Mutex<VecDeque<Packet>>,
}

impl SilentHost {
    fn push(&self, p: Packet) {
        self.script.lock().unwrap().push_back(p);
    }
}

impl Host for SilentHost {
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
        let n = self.clock.fetch_add(1, Ordering::Relaxed);
        let next = self.script.lock().unwrap().pop_front();
        if let Some(p) = next {
            tos_api::vdi::feed_relative(p.dx, p.dy, p.buttons, &self.raster);
        } else {
            assert!(n < 1_000_000, "a trap is spinning on an exhausted pointer script");
        }
    }
}

unsafe impl Sync for SilentHost {}
unsafe impl Send for SilentHost {}

static HOST_ONCE: Once = Once::new();
static HOST: Mutex<Option<&'static SilentHost>> = Mutex::new(None);

fn install_host() {
    HOST_ONCE.call_once(|| {
        let host: &'static SilentHost = Box::leak(Box::new(SilentHost {
            raster: SinkBackend,
            clock: AtomicU64::new(0),
            script: Mutex::new(VecDeque::new()),
        }));
        let href: &'static &'static dyn Host = Box::leak(Box::new(host as &'static dyn Host));
        tos_api::host::install_host(href);
        *HOST.lock().unwrap() = Some(host);
    });
}

fn host() -> &'static SilentHost {
    HOST.lock().unwrap().expect("host installed by lock()")
}

/// One lock over every process-wide static the tests touch.
static LOCK: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    install_host();
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
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
        let sw = unsafe { tos_api::aes::dispatch(self.block.as_ptr() as u64, ABI) };
        // The call must complete. Since A112 a completed call may come back
        // with the `completed` Switch of the dispatch every AES call ends
        // with (another slot of this test is on the ready list); the host has
        // no trap handler to apply it, and the call's results are already
        // copied back. A mid-call yield (S2, not `completed`) is a failure.
        assert!(sw.is_none_or(|s| s.completed));
        self.intout[0]
    }
}

// AES opcodes used here.
const APPL_INIT: i16 = 10;
const APPL_READ: i16 = 11;
const APPL_WRITE: i16 = 12;
const APPL_FIND: i16 = 13;
const APPL_EXIT: i16 = 19;
const WIND_CREATE: i16 = 100;
const WIND_OPEN: i16 = 101;
const WIND_CLOSE: i16 = 102;
const WIND_DELETE: i16 = 103;
const WIND_GET: i16 = 104;
const WIND_SET: i16 = 105;

// Window kinds (`aesdefs.h`).
const NAME: i16 = 0x0001;
const CLOSER: i16 = 0x0002;
const MOVER: i16 = 0x0008;
/// `HOTCLOSE` (`aesdefs.h:531`, PC-GEM): the closer fires `WM_CLOSED` on the
/// press, without `gr_watchbox`.
const HOTCLOSE: i16 = 0x1000;

/// `wind_get`/`wind_set` fields (`aesdefs.h`).
const WF_NEWDESK: i16 = 14;
const WF_OWNER: i16 = 20;

/// `WM_REDRAW` (`aesdefs.h:228`).
const WM_REDRAW: i16 = 20;

/// The desktop window handle.
const DESKWH: i16 = 0;

/// The pid every member of the application chain reports (`rlr->p_pid` of
/// `D.g_int[0]`, `aes/geminit.c:851-877`) — `ap_id`, `appl_find`, `WF_OWNER`,
/// and `msg[1]` of a message the application itself sends (`WM_REDRAW`).
const AP_CHAIN: i16 = 0;
/// The Screen Manager's pid (`iprocess("SCRENMGR", ctlmgr)`,
/// `aes/geminit.c:888`) — `msg[1]` of every control-manager message, and the
/// one `appl_write` target that is not a NewTOS process.
const SCRENMGR: i16 = 1;

/// Control-manager message ids (`aesdefs.h`) the stamp is checked on.
const WM_TOPPED: i16 = 21;
const WM_CLOSED: i16 = 22;
const WM_UNTOPPED: i16 = 30;

/// Somewhere on the 640×480 screen.
const WR: (i16, i16, i16, i16) = (40, 40, 300, 200);

// ── AES calls ─────────────────────────────────────────────────────────────

/// `appl_init()` — returns the caller's AES id (`ap_id`).
fn appl_init(pb: &mut Aespb) -> i16 {
    pb.intin.fill(0);
    pb.call(APPL_INIT, 0, 1, 0)
}

/// `appl_exit()`.
fn appl_exit(pb: &mut Aespb) -> i16 {
    pb.intin.fill(0);
    pb.call(APPL_EXIT, 0, 1, 0)
}

/// `appl_find(name)` — the 8-char name, space-padded by the caller.
fn appl_find(pb: &mut Aespb, name: &[u8; 8]) -> i16 {
    pb.intin.fill(0);
    pb.addrin[0] = name.as_ptr() as u64;
    let r = pb.call(APPL_FIND, 0, 1, 1);
    pb.addrin[0] = 0;
    r
}

/// `appl_read(pid, 16, buf)` — pop of one message; 1 when one was delivered,
/// 0 when the id has no descriptor. **Blocks on an empty FIFO** (A75, EmuTOS
/// `aqueue`, `aes/gemqueue.c:98-124`), so callers that may find the queue
/// empty check [`pending`] first — the kernel's `evnt_mesag` pattern.
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

/// `appl_write(pid, 16, msg)` — 1 when the message was queued, 0 when not.
fn appl_write(pb: &mut Aespb, pid: i16, msg: &[i16; 8]) -> i16 {
    pb.intin.fill(0);
    pb.intin[0] = pid;
    pb.intin[1] = 16;
    pb.addrin[0] = msg.as_ptr() as u64;
    let r = pb.call(APPL_WRITE, 2, 1, 1);
    pb.addrin[0] = 0;
    r
}

/// `wind_create(kind, full)` → handle.
fn wind_create(pb: &mut Aespb) -> i16 {
    pb.intin.fill(0);
    pb.intin[0] = NAME | CLOSER | MOVER;
    pb.intin[1] = 0;
    pb.intin[2] = 0;
    pb.intin[3] = SCREEN_W as i16;
    pb.intin[4] = SCREEN_H as i16;
    let h = pb.call(WIND_CREATE, 5, 1, 0);
    assert!(h > 0, "wind_create must give a handle, got {h}");
    h
}

/// `wind_open(handle, r)` — posts one `WM_REDRAW` to the window's owner
/// (`gemwmlib.c:1420-1436,1004-1048,887-907`).
fn wind_open(pb: &mut Aespb, h: i16, r: (i16, i16, i16, i16)) {
    pb.intin.fill(0);
    pb.intin[0] = h;
    pb.intin[1] = r.0;
    pb.intin[2] = r.1;
    pb.intin[3] = r.2;
    pb.intin[4] = r.3;
    assert_eq!(pb.call(WIND_OPEN, 5, 1, 0), 1, "wind_open failed");
}

fn wind_close(pb: &mut Aespb, h: i16) {
    pb.intin.fill(0);
    pb.intin[0] = h;
    pb.call(WIND_CLOSE, 1, 1, 0);
}

fn wind_delete(pb: &mut Aespb, h: i16) {
    pb.intin.fill(0);
    pb.intin[0] = h;
    pb.call(WIND_DELETE, 1, 1, 0);
}

/// `wind_get(handle, WF_OWNER)` → `int_out[1]`, the owner's reported id.
fn wf_owner(pb: &mut Aespb, h: i16) -> i16 {
    pb.intin.fill(0);
    pb.intin[0] = h;
    pb.intin[1] = WF_OWNER;
    assert_eq!(pb.call(WIND_GET, 2, 5, 0), 1, "wind_get(WF_OWNER) failed");
    pb.intout[1]
}

/// `wind_set(0, WF_NEWDESK, NULL, 0)` — re-owns the desktop window to the
/// caller (`gemwmlib.c:1684-1687`).
fn wind_set_newdesk(pb: &mut Aespb) {
    pb.intin.fill(0);
    pb.intin[0] = DESKWH;
    pb.intin[1] = WF_NEWDESK;
    pb.addrin[0] = 0;
    pb.call(WIND_SET, 6, 1, 1);
}

/// How many messages are waiting for `pid` (`tos-api/src/aes/window.rs` test
/// hook — reads the FIFO depth without popping).
fn pending(pid: i16) -> usize {
    tos_api::aes::window::test_msg_count(pid)
}

/// Read everything queued for `pid` (depth checked first: `appl_read` blocks
/// on an empty FIFO).
fn drain(pb: &mut Aespb, pid: i16) -> usize {
    let mut n = 0;
    while pending(pid) > 0 {
        assert_eq!(appl_read(pb, pid).0, 1, "a queued message must be delivered");
        n += 1;
        assert!(n < 64, "runaway queue");
    }
    n
}

// ── a process slot the host can "run" ─────────────────────────────────────

/// A claimed GEMDOS process slot with a basepage page of its own. Its
/// **reported** AES id is 0 whatever the slot — it is a member of the
/// application chain (contract (b)). Released on drop through the kernel's
/// own slot release (`pterm::release_slot`), so a failing test leaves neither
/// the slot nor its descriptor behind for the next test.
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

    /// The value `appl_init` reports for this slot: 0, the application
    /// chain's pid (`rlr->p_pid`, `aes/gemaplib.c:54-63`), at every depth.
    fn reported(&self) -> i16 {
        AP_CHAIN
    }

    fn bp(&self) -> *mut BasePage {
        self.page.cast::<BasePage>()
    }

    /// Make this the running process: every AES call now carries its identity.
    fn run(&self) {
        process::set_current(self.bp());
    }
}

impl Drop for Proc {
    fn drop(&mut self) {
        if process::current() == self.bp() {
            process::set_current(std::ptr::null_mut());
        }
        // The slot may already have been released by a `Pterm` in the test;
        // releasing a free slot only re-zeroes it.
        // SAFETY: single-threaded under `LOCK`; no frame snapshot is consumed
        // on the host.
        unsafe { process::pterm::release_slot(self.slot) };
        // SAFETY: `page` came from `Box::into_raw` above and is not aliased
        // once the slot no longer names it.
        unsafe { drop(Box::from_raw(self.page)) };
    }
}

/// The kernel boot context: no running process (`RUN` null) — the built-in
/// desktop's situation.
fn desktop_context() {
    process::set_current(std::ptr::null_mut());
}

/// `Pterm` of the running process — the real `terminate` path followed by the
/// handler's `release_slot`, as `aes_rsrc_per_process.rs` drives it. On the
/// host the pool sweep finds no pool and the parent frame is a marker; what is
/// under test is what the release does to the slot's AES descriptor.
fn pterm(p: &Proc) {
    PROCESS_TABLE[p.slot].parent_frame.elr().store(1, Ordering::Release);
    // SAFETY: single-threaded under `LOCK`; `RUN` is `p`'s basepage.
    let resume = unsafe { process::pterm::terminate(0) }.expect("Pterm of the running process");
    assert_eq!(resume.parent_frame_slot, p.slot);
    // SAFETY: the frame snapshot is not needed on the host.
    unsafe { process::pterm::release_slot(p.slot) };
}

/// The launcher's pre-run step (`kernel/src/launch.rs`,
/// `register_process(child.slot, path)` — EmuTOS `p_nameit` **and**
/// `rlr->p_flags = 0`, `aes/gemshlib.c:633-636`).
fn launcher_register(slot: usize, path: &[u8]) {
    tos_api::aes::register_process(slot, path);
}

/// The launcher's post-run step (`kernel/src/launch.rs`,
/// `close_client_if_open(child.slot)` — EmuTOS `aes/gemshlib.c:650-657`),
/// addressed by the slot: the identity the descriptor is keyed by.
fn launcher_cleanup(slot: usize) {
    tos_api::aes::close_client_if_open(slot);
}

// ── (a) nesting: one descriptor per process, appl_init never -1 ───────────

/// The measured defect: `NESTTST.NAP` `Pexec`ing itself 16 deep got
/// `appl_init` = 1…8 for depths 1–8 and **−1 from depth 9** (GEMDOS slot 8),
/// because the old descriptor table had 8 entries against 16 process slots.
/// EmuTOS's `ap_init` cannot fail (`aes/gemaplib.c:54-63`); with one
/// descriptor per slot by construction, neither can NewTOS's.
///
/// Every depth reports the chain's pid, 0 (contract (b)): the sixteen nested
/// processes are sixteen identities and one reported value.
#[test]
fn every_nested_process_gets_a_descriptor_and_appl_init_never_returns_minus_one() {
    let _g = lock();
    let mut pb = Aespb::new();
    desktop_context();
    assert_eq!(appl_init(&mut pb), AP_CHAIN, "the desktop is a chain member: ap_id 0");

    let chain: Vec<Proc> = (0..MAX_PROCESSES).map(|_| Proc::new()).collect();
    for (depth, p) in chain.iter().enumerate() {
        p.run();
        let id = appl_init(&mut pb);
        assert_ne!(
            id, -1,
            "depth {}: appl_init returned -1 — EmuTOS's ap_init cannot fail \
             (aes/gemaplib.c:54-63); a descriptor per GEMDOS slot leaves no \
             'table full' to report",
            depth + 1
        );
        assert_eq!(id, p.reported(), "depth {}: the reported id is the chain's, 0", depth + 1);
        // …and the process is not deaf: it can address itself.
        let probe = [0x4711, id, 0, depth as i16, 0, 0, 0, 0];
        assert_eq!(appl_write(&mut pb, id, &probe), 1, "depth {}: appl_write(self)", depth + 1);
        let (got, body) = appl_read(&mut pb, id);
        assert_eq!((got, body), (1, probe), "depth {}: appl_read(self)", depth + 1);
    }

    // Unwind innermost-first, as the Pexec chain does.
    for p in chain.iter().rev() {
        p.run();
        assert_eq!(appl_exit(&mut pb), 1);
        pterm(p);
    }
    desktop_context();
    assert_eq!(appl_init(&mut pb), AP_CHAIN, "the desktop is still there after the chain");
    assert_eq!(appl_exit(&mut pb), 1);
}

// ── (b) nine names in one slot: appl_find answers for the occupant ────────

/// `SEQ1.NAP` … `SEQ9.NAP` one after another all land in GEMDOS slot 0 and
/// rename **one** descriptor (measured: `appl_find(previous) = -1` every
/// time, A62 report §1a) — EmuTOS's `p_nameit` overwrites the one `p_name` in
/// place (`aes/gemshlib.c:633`, `aes/gempd.c:77-86`). One name per slot.
#[test]
fn nine_programs_in_one_slot_rename_one_descriptor() {
    let _g = lock();
    let mut pb = Aespb::new();
    let mut previous: Option<[u8; 8]> = None;
    for n in 1..=9u8 {
        let p = Proc::new();
        assert_eq!(p.slot, 0, "each program takes the first free slot");
        let path = format!("C:\\SEQ{n}.NAP");
        let mut name = *b"        ";
        name[..4].copy_from_slice(b"SEQ ");
        name[3] = b'0' + n;
        launcher_register(p.slot, path.as_bytes());
        p.run();
        assert_eq!(appl_init(&mut pb), AP_CHAIN, "SEQ{n}: appl_init reports the chain's 0");
        assert_eq!(appl_find(&mut pb, &name), AP_CHAIN, "SEQ{n}: appl_find(self) = 0");
        if let Some(prev) = previous {
            assert_eq!(
                appl_find(&mut pb, &prev),
                -1,
                "SEQ{n}: the previous occupant's name must be gone — one name per slot"
            );
        }
        assert_eq!(appl_exit(&mut pb), 1);
        pterm(&p);
        previous = Some(name);
    }
    desktop_context();
}

// ── (c) the slot's release resets its descriptor ──────────────────────────

/// A program that called `appl_init`, has a message unread, and dies without
/// `appl_exit` (a crashed GEM app, or a `Pexec` child that never bothered).
/// The A62 report (§8) found that `handle_pexec` runs neither `register` nor
/// `close_client_if_open`, so the slot's next tenant inherited `AP_OPEN` and
/// the queue. With the descriptor keyed by the slot, the slot's release is the
/// one place the process model knows the tenant is gone — and it does what
/// `sh_ldapp` does at its boundary (`aes/gemshlib.c:636,650-657`): flags
/// cleared, queue drained.
#[test]
fn releasing_a_slot_resets_its_descriptor_and_queue() {
    let _g = lock();
    let mut pb = Aespb::new();

    let slot = {
        let dying = Proc::new();
        dying.run();
        let id = appl_init(&mut pb);
        assert_eq!(id, dying.reported());
        let stale = [0x0BAD, id, 0, 0, 0, 0, 0, 0];
        assert_eq!(appl_write(&mut pb, id, &stale), 1);
        // No appl_exit: AP_OPEN stays set, the message stays queued.
        pterm(&dying);
        dying.slot
    };

    // The next tenant of the same slot.
    let tenant = Proc::new();
    assert_eq!(tenant.slot, slot, "the freed slot is the first free one again");
    tenant.run();
    let id = appl_init(&mut pb);
    assert_eq!(id, tenant.reported());
    // The queue must be empty — checked by depth, since `appl_read` blocks on
    // an empty FIFO (A75); the front message, if any, names the leak.
    let leaked = tos_api::aes::window::test_pending_msg(id);
    assert_eq!(
        (pending(id), leaked.map(|m| m[0]).unwrap_or(0)),
        (0, 0),
        "the new tenant inherits the dead tenant's message (type {:#x}) — the \
         slot's release did not drain the descriptor's queue",
        leaked.map(|m| m[0]).unwrap_or(0)
    );

    // The flag reset, seen through its one consumer (A28's argument): the
    // launcher's "appl_init without appl_exit" cleanup must judge the program
    // that just ran. A console program (never `appl_init`) in this slot, with a
    // message posted to it meanwhile, must keep that message — a stale AP_OPEN
    // would have drained it.
    assert_eq!(appl_exit(&mut pb), 1);
    pterm(&tenant);
    drop(tenant);

    let console = Proc::new();
    assert_eq!(console.slot, slot);
    launcher_register(console.slot, b"C:\\HELLO.NCP");
    console.run();
    let posted = [0x4711, console.reported(), 0, 0, 0, 0, 0, 0];
    assert_eq!(appl_write(&mut pb, console.reported(), &posted), 1);
    launcher_cleanup(console.slot);
    let (got, body) = appl_read(&mut pb, console.reported());
    assert_eq!(
        (got, body),
        (1, posted),
        "close_client_if_open fired for a program that never called appl_init \
         — a stale AP_OPEN survived the slot's release"
    );
    drop(console);
    desktop_context();
}

// ── (d) the reported values, contract (b) ─────────────────────────────────

/// `wind_set(handle, WF_TOP)` — the owner's answer to `WM_TOPPED`.
fn wind_set_top(pb: &mut Aespb, h: i16) {
    pb.intin.fill(0);
    pb.intin[0] = h;
    pb.intin[1] = 10; // WF_TOP
    pb.call(WIND_SET, 6, 1, 1);
}

/// `wind_create(kind, full)` with an explicit `kind` → handle.
fn wind_create_kind(pb: &mut Aespb, kind: i16) -> i16 {
    pb.intin.fill(0);
    pb.intin[0] = kind;
    pb.intin[1] = 0;
    pb.intin[2] = 0;
    pb.intin[3] = SCREEN_W as i16;
    pb.intin[4] = SCREEN_H as i16;
    let h = pb.call(WIND_CREATE, 5, 1, 0);
    assert!(h > 0, "wind_create must give a handle, got {h}");
    h
}

/// One pointer packet, fed the way the mouse interrupt feeds it: the VDI
/// position moves, `user_but` updates the AES's button state.
fn feed(dx: i16, dy: i16, buttons: u8) {
    tos_api::vdi::feed_relative(dx, dy, buttons, &host().raster);
}

/// Put the pointer at `(x, y)`, buttons up. (The first packet initialises
/// the VDI pointer; the position is read after it.)
fn pointer_at(x: i16, y: i16) {
    feed(0, 0, 0);
    let (cx, cy) = tos_api::vdi::mouse_position();
    feed(x - cx, y - cy, 0);
    assert_eq!(tos_api::vdi::mouse_position(), (x, y), "pointer_at");
}

/// Press the left button where the pointer is and script its release for
/// the trap that waits for it, then run the window-frame interception
/// (`evnt_multi`'s Control-Manager hook, `window::intercept`).
fn press_and_intercept() {
    feed(0, 0, 1);
    for _ in 0..4 {
        host().push(Packet { dx: 0, dy: 0, buttons: 0 });
    }
    tos_api::aes::window::intercept();
    // Anything the trap did not consume: the button is up either way.
    host().script.lock().unwrap().clear();
    feed(0, 0, 0);
}

/// Stage 2's contract, spec § "AES process identity and pid" (b): the
/// numbers a program reads back are EmuTOS's, computed from the identity —
///
/// * `ap_id`, `appl_find`, `WF_OWNER`: **0** for the kernel-context desktop,
///   the program in slot 0 and its `Pexec` child in slot 1 alike (one `rlr`
///   for the whole chain, `aes/geminit.c:850-877`; `w_owner->p_pid`,
///   `gemwmlib.c:1580`);
/// * `msg[1]` of the `WM_REDRAW` a `wind_open` posts: **0** — sent from
///   inside the application's own trap (`w_redraw`, `gemwmlib.c:904`, via
///   `draw_change` from `wm_open`, `:1426`);
/// * `msg[1]` of the control-manager messages — here `WM_TOPPED`,
///   `WM_UNTOPPED` and the `HOTCLOSE` `WM_CLOSED`, the three posters a
///   sink backend can be made to fire — **1**, the Screen Manager's stamp
///   (`ct_msgup`/`perform_untop`, `gemctrl.c:151,178,210-213`; `WM_MOVED`
///   and `MN_SELECTED` are the same posters and are covered with a scripted
///   pointer in `aes_window_drag.rs` / `aes_menu_click_ownership.rs`);
/// * `appl_write(0)`/`appl_read(0)`: the **caller** — the currently-scheduled
///   client (`fpdnm(NULL, 0)` = `D.g_int[0]`, `gemsuper.c:102-105`);
///   `appl_write(1)` is accepted (queued for the Screen Manager,
///   `ctlmgr`'s `ev_multi(MU_MESAG)`, `gemctrl.c:448-481`) and reaches no
///   application; `appl_write(2)` finds no descriptor (no accessories).
#[test]
fn reported_values_follow_emutos() {
    let _g = lock();
    let mut pb = Aespb::new();

    // The desktop: ap_id 0, and its window's owner reports 0.
    desktop_context();
    assert_eq!(appl_init(&mut pb), AP_CHAIN, "desktop ap_id (gemsuper.c:94, rlr->p_pid)");
    assert_eq!(wf_owner(&mut pb, DESKWH), 0, "the desktop window has no owner yet");
    let dw = wind_create(&mut pb);
    assert_eq!(wf_owner(&mut pb, dw), AP_CHAIN, "WF_OWNER of a desktop-created window");
    wind_open(&mut pb, dw, WR);
    let (got, body) = appl_read(&mut pb, AP_CHAIN);
    assert_eq!(
        (got, body[0], body[1], body[3]),
        (1, WM_REDRAW, AP_CHAIN, dw),
        "desktop WM_REDRAW: msg[1] is the application's own pid (w_redraw inside wind_open)"
    );
    drain(&mut pb, AP_CHAIN);
    wind_set_newdesk(&mut pb);
    assert_eq!(wf_owner(&mut pb, DESKWH), AP_CHAIN, "WF_NEWDESK re-owns the desktop window: still 0");
    wind_close(&mut pb, dw);
    wind_delete(&mut pb, dw);
    drain(&mut pb, AP_CHAIN);

    // A launched program in slot 0 and its Pexec child in slot 1.
    let parent = Proc::new();
    let child = Proc::new();
    assert_eq!((parent.slot, child.slot), (0, 1));
    launcher_register(parent.slot, b"C:\\TERADESK\\DESKTOP.NAP");

    parent.run();
    assert_eq!(appl_init(&mut pb), AP_CHAIN, "slot 0 reports 0 — the chain's pid");
    assert_eq!(appl_find(&mut pb, b"DESKTOP "), AP_CHAIN, "appl_find by the launch name → 0");
    assert_eq!(appl_find(&mut pb, b"NOTHERE "), -1, "appl_find of an unknown name → -1");
    assert_eq!(
        appl_find(&mut pb, b"SCRENMGR"),
        SCRENMGR,
        "appl_find(\"SCRENMGR\") → 1 (iprocess(\"SCRENMGR\", ctlmgr), geminit.c:888)"
    );
    let pw = wind_create(&mut pb);
    assert_eq!(wf_owner(&mut pb, pw), AP_CHAIN, "WF_OWNER, slot 0");
    wind_open(&mut pb, pw, WR);
    let (got, body) = appl_read(&mut pb, AP_CHAIN);
    assert_eq!((got, body[0], body[1], body[3]), (1, WM_REDRAW, AP_CHAIN, pw), "slot 0 WM_REDRAW msg[1]");
    // A message the parent sends to 0 reaches the parent — the caller.
    let own = [0x4711, AP_CHAIN, 0, 7, 0, 0, 0, 0];
    assert_eq!(appl_write(&mut pb, AP_CHAIN, &own), 1);
    assert_eq!(appl_read(&mut pb, AP_CHAIN), (1, own), "appl_write(0) from slot 0 reaches slot 0");

    child.run();
    assert_eq!(appl_init(&mut pb), AP_CHAIN, "slot 1 (the Pexec child) reports 0 too");
    assert_eq!(appl_find(&mut pb, b"DESKTOP "), AP_CHAIN, "the parent's name still answers 0");
    let cw = wind_create(&mut pb);
    assert_eq!(wf_owner(&mut pb, cw), AP_CHAIN, "WF_OWNER, slot 1");
    wind_open(&mut pb, cw, WR);
    let (got, body) = appl_read(&mut pb, AP_CHAIN);
    assert_eq!((got, body[0], body[1], body[3]), (1, WM_REDRAW, AP_CHAIN, cw), "slot 1 WM_REDRAW msg[1]");
    // The parent's window reports 0 as well, and a message to 0 from the
    // child lands in the child's own queue — the caller — not the parent's.
    assert_eq!(wf_owner(&mut pb, pw), AP_CHAIN);
    let to_self = [0x4712, AP_CHAIN, 0, 0, 0, 0, 0, 0];
    assert_eq!(appl_write(&mut pb, AP_CHAIN, &to_self), 1);
    assert_eq!(appl_read(&mut pb, AP_CHAIN), (1, to_self), "appl_write(0) from the child reaches the child");
    // Target 1 is the Screen Manager: the write succeeds (its queue exists in
    // EmuTOS, gemsuper.c:102-105 + geminit.c:888) but no application reads it.
    let to_ctl = [0x4713, AP_CHAIN, 0, 0, 0, 0, 0, 0];
    assert_eq!(appl_write(&mut pb, SCRENMGR, &to_ctl), 1, "appl_write(1): accepted by the Screen Manager");
    assert_eq!(pending(AP_CHAIN), 0, "…and not delivered to the caller");
    assert_eq!(appl_read(&mut pb, SCRENMGR).0, 0, "appl_read(1): nothing an application can read");
    // Targets beyond the live table: no descriptor (totpds = 2 without
    // accessories, geminit.c:819).
    assert_eq!(appl_write(&mut pb, 2, &to_ctl), 0, "no accessory reports 2");
    assert_eq!(appl_write(&mut pb, 17, &to_ctl), 0, "no process reports 17");

    // The control-manager stamps, on the child's window. A press on the
    // frame of a non-top window: `WM_UNTOPPED` to the old top's owner,
    // `WM_TOPPED` to the clicked window's owner (`hctl_window`,
    // gemctrl.c:170-178) — both stamped 1 by `ap_sendmsg` under `ctl_pd`.
    let cw2 = wind_create(&mut pb);
    wind_open(&mut pb, cw2, (360, 240, 200, 150)); // now on top, cw below
    drain(&mut pb, AP_CHAIN);
    pointer_at(WR.0 + 60, WR.1 + 6); // cw's title bar, not covered by cw2
    press_and_intercept();
    let (got, untopped) = appl_read(&mut pb, AP_CHAIN);
    assert_eq!(
        (got, untopped[0], untopped[1], untopped[3]),
        (1, WM_UNTOPPED, SCRENMGR, cw2),
        "WM_UNTOPPED to the old top: msg[1] = 1 (perform_untop, gemctrl.c:151)"
    );
    let (got, topped) = appl_read(&mut pb, AP_CHAIN);
    assert_eq!(
        (got, topped[0], topped[1], topped[3]),
        (1, WM_TOPPED, SCRENMGR, cw),
        "WM_TOPPED to the clicked window: msg[1] = 1 (ct_msgup, gemctrl.c:178)"
    );
    // The owner answers with WF_TOP; the WM_REDRAW that may follow is its own.
    wind_set_top(&mut pb, cw);
    drain(&mut pb, AP_CHAIN);
    // HOTCLOSE: a press on the closer posts WM_CLOSED at once
    // (gemctrl.c:208-214), stamped 1.
    let hot = wind_create_kind(&mut pb, NAME | CLOSER | MOVER | HOTCLOSE);
    wind_open(&mut pb, hot, (100, 300, 200, 100));
    drain(&mut pb, AP_CHAIN);
    pointer_at(100 + 4, 300 + 4); // the closer, top-left of the title row
    press_and_intercept();
    let (got, closed) = appl_read(&mut pb, AP_CHAIN);
    assert_eq!(
        (got, closed[0], closed[1], closed[3]),
        (1, WM_CLOSED, SCRENMGR, hot),
        "HOTCLOSE WM_CLOSED: msg[1] = 1 (ct_msgup's HOTCLOSE exit, gemctrl.c:86-96,210-213)"
    );
    wind_close(&mut pb, hot);
    wind_delete(&mut pb, hot);
    wind_close(&mut pb, cw2);
    wind_delete(&mut pb, cw2);
    wind_close(&mut pb, cw);
    wind_delete(&mut pb, cw);
    drain(&mut pb, AP_CHAIN);
    assert_eq!(appl_exit(&mut pb), 1);
    pterm(&child);

    parent.run();
    // What the parent finds queued: the WM_REDRAWs the child's window closes
    // fanned out to `pw` (posted from inside the child's `wind_close` trap,
    // hence stamped 0 too) — never the message the child sent to 0. (Depth
    // checked first: `appl_read` blocks on an empty FIFO, A75.)
    while pending(AP_CHAIN) > 0 {
        let (got, body) = appl_read(&mut pb, AP_CHAIN);
        assert_eq!(got, 1, "a queued message must be delivered");
        assert_ne!(body[0], 0x4712, "the child's appl_write(0) reached the parent's queue");
        assert_eq!(
            (body[0], body[1], body[3]),
            (WM_REDRAW, AP_CHAIN, pw),
            "the parent's queue holds only WM_REDRAWs for its own window, stamped 0"
        );
    }
    wind_close(&mut pb, pw);
    wind_delete(&mut pb, pw);
    drain(&mut pb, AP_CHAIN);
    assert_eq!(appl_exit(&mut pb), 1);
    pterm(&parent);
    drop(child);
    drop(parent);

    // Back in the desktop: still 0, still addressable as the caller (A29).
    desktop_context();
    assert_eq!(appl_init(&mut pb), AP_CHAIN, "desktop ap_id after the programs");
    let probe = [0x4714, AP_CHAIN, 0, 0, 0, 0, 0, 0];
    assert_eq!(appl_write(&mut pb, AP_CHAIN, &probe), 1);
    assert_eq!(appl_read(&mut pb, AP_CHAIN), (1, probe));
    assert_eq!(appl_exit(&mut pb), 1);
}
