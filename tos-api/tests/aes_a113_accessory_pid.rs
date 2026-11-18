//! A113 — **desk accessories report their own pid, 2, 3, … in load order.**
//!
//! Spec: `docs/api/aes/application.md` § "Pid assignment, accessories, and a
//! failed load" and NewTOS contract **(b)** (rewritten 2026-10-17, A113);
//! `window-graphics.md` `WF_OWNER`; `event.md` `msg[1]`. EmuTOS: `totpds =
//! num_accs + 2` (`aes/geminit.c:819`), pid 0 = the application chain, 1 =
//! SCRENMGR, accessories 2… in successful-load order (`geminit.c:236-257`,
//! `gempd.c:60-70`); `appl_find`/`appl_write` resolve through `fpdnm`
//! (`gempd.c:38-57`, `gemaplib.c:95-107`, `gemsuper.c:101-105`); `msg[1]` =
//! the sender's `rlr->p_pid` (`gemwmlib.c:851-862`); `WF_OWNER` =
//! `w_owner->p_pid` (`gemwmlib.c:1580`).
//!
//! The measured defect (`measurements/measurement-2026-10-17-a113-a114/
//! serial/idprobe1.log`): every accessory reported ap_id 0, `appl_find` of an
//! accessory answered 0, `appl_write(2|3)` returned 0 and delivered nothing,
//! and an accessory's own `WM_REDRAW` carried `msg[1] = 0`.
//!
//! Pinned here, at the AES trap boundary:
//!
//! 1. accessory pids 2/3 (`appl_init` return and `global[2]`), the chain 0,
//!    `appl_find` from any caller;
//! 2. program → accessory `appl_write(2|3)` is delivered to that accessory,
//!    and the accessory's `appl_write(0, …)` reply reaches the program (the
//!    chain's innermost live member), not the accessory itself;
//! 3. pid 0 from an accessory resolves to the innermost member of a `Pexec`
//!    chain, and to the kernel context when no chain process is alive;
//! 4. `WF_OWNER` of an accessory window and `msg[1]` of its `WM_REDRAW` are
//!    the accessory's pid;
//! 5. a pid no accessory holds (`>= totpds`) is refused with 0 (the spec's
//!    NewTOS safety choice; EmuTOS is undefined there);
//! 6. a forgotten accessory does not renumber the others.
//!
//! Harness copied from `aes_pd_by_slot.rs` (silent raster, process slots).

#![allow(clippy::too_many_arguments)]
#![allow(dead_code)]

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, Once};

use tos_api::bdos::process::{self, Abi, State, PROCESS_TABLE};
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


// ── process slots ─────────────────────────────────────────────────────────

/// A claimed GEMDOS process slot with a basepage page of its own; released on
/// drop through the kernel's slot release (`pterm::release_slot`).
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

    /// A program the kernel launcher started (`launch.rs`: named, then
    /// `State::Running`) — a live member of the application chain.
    fn program(path: &[u8]) -> Self {
        let p = Proc::new();
        tos_api::aes::register_process(p.slot, path);
        PROCESS_TABLE[p.slot].set_state(State::Running);
        p
    }

    /// A `Pexec` child of `parent`, started (`handle_pexec`: `parent_slot`,
    /// `State::Running`).
    fn child_of(parent: &Proc) -> Self {
        let c = Proc::new();
        PROCESS_TABLE[c.slot]
            .parent_slot
            .store(parent.slot as u32, Ordering::Release);
        PROCESS_TABLE[c.slot].set_state(State::Running);
        c
    }

    /// A desk accessory the loader started (`accs.rs` `load_one_acc`: named,
    /// `State::Loaded`, `acc::register_process`).
    fn accessory(path: &[u8]) -> Self {
        let p = Proc::new();
        tos_api::aes::register_process(p.slot, path);
        tos_api::aes::acc::register_process(p.slot);
        p
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
        // SAFETY: single-threaded under `LOCK`.
        unsafe { process::pterm::release_slot(self.slot) };
        // SAFETY: `page` came from `Box::into_raw` and the slot no longer names it.
        unsafe { drop(Box::from_raw(self.page)) };
    }
}

/// The kernel context: no running process (`RUN` null).
fn kernel_context() {
    process::set_current(std::ptr::null_mut());
}

/// `appl_init` plus `global[2]` (`ap_id`, `gemsuper.c:94`).
fn init_ids(pb: &mut Aespb) -> (i16, i16) {
    let ret = appl_init(pb);
    (ret, pb.global[2])
}

const ACCX: &[u8; 8] = b"ACCX    ";
const ACCY: &[u8; 8] = b"ACCY    ";

// ── 1. pids 2/3 for the accessories, 0 for the chain ──────────────────────

#[test]
fn accessories_report_pids_in_load_order_and_appl_find_answers_them() {
    let _g = lock();
    let mut pb = Aespb::new();
    let x = Proc::accessory(b"C:\\ACCX.NAC");
    let y = Proc::accessory(b"C:\\ACCY.NAC");
    let main = Proc::program(b"C:\\MAIN.NAP");

    x.run();
    assert_eq!(init_ids(&mut pb), (2, 2), "the first accessory reports 2 (geminit.c:819, gempd.c:60-70)");
    y.run();
    assert_eq!(init_ids(&mut pb), (3, 3), "the second accessory reports 3");
    main.run();
    assert_eq!(init_ids(&mut pb), (0, 0), "the program is the chain: 0");

    for (who, p) in [("ACCX", &x), ("ACCY", &y), ("MAIN", &main)] {
        p.run();
        assert_eq!(appl_find(&mut pb, ACCX), 2, "appl_find(ACCX) from {who}");
        assert_eq!(appl_find(&mut pb, ACCY), 3, "appl_find(ACCY) from {who}");
        assert_eq!(appl_find(&mut pb, b"MAIN    "), 0, "appl_find(MAIN) from {who}");
        assert_eq!(appl_find(&mut pb, b"SCRENMGR"), SCRENMGR, "appl_find(SCRENMGR) from {who}");
    }
    kernel_context();
    assert_eq!(appl_find(&mut pb, ACCY), 3, "appl_find(ACCY) from the kernel context");
    drop(main);
    drop(y);
    drop(x);
}

// ── 2. program → accessory and back ───────────────────────────────────────

#[test]
fn a_program_reaches_an_accessory_and_the_reply_to_0_reaches_the_program() {
    let _g = lock();
    let mut pb = Aespb::new();
    let x = Proc::accessory(b"C:\\ACCX.NAC");
    let y = Proc::accessory(b"C:\\ACCY.NAC");
    let main = Proc::program(b"C:\\MAIN.NAP");

    main.run();
    let to_x = [0x4000, 0, 0, 2, 0, 0, 0, 0];
    let to_y = [0x4000, 0, 0, 3, 1, 0, 0, 0];
    assert_eq!(appl_write(&mut pb, 2, &to_x), 1, "appl_write(2) is accepted");
    assert_eq!(appl_write(&mut pb, 3, &to_y), 1, "appl_write(3) is accepted");
    assert_eq!(pending(0), 0, "nothing landed in the program's own queue");

    for (p, me, sent) in [(&x, 2i16, to_x), (&y, 3i16, to_y)] {
        p.run();
        assert_eq!(pending(me), 1, "accessory {me} has the program's message");
        let (got, body) = appl_read(&mut pb, me);
        assert_eq!((got, body), (1, sent), "accessory {me} reads the program's message");
        // Its reply, as the probe sends it: to msg[1] = 0, stamped with its id.
        let reply = [0x4001, me, 0, me, body[4], 0, 0, 0];
        assert_eq!(appl_write(&mut pb, 0, &reply), 1, "accessory {me}: appl_write(0)");
        assert_eq!(pending(me), 0, "accessory {me}: the reply to 0 did not stay with the accessory");
    }

    main.run();
    let (got, r1) = appl_read(&mut pb, 0);
    assert_eq!((got, r1[0], r1[1]), (1, 0x4001, 2), "the program gets ACCX's reply, msg[1] = 2");
    let (got, r2) = appl_read(&mut pb, 0);
    assert_eq!((got, r2[0], r2[1]), (1, 0x4001, 3), "the program gets ACCY's reply, msg[1] = 3");
    assert_eq!(pending(0), 0);
    drop(main);
    drop(y);
    drop(x);
}

// ── 3. pid 0 from an accessory: the innermost chain member ────────────────

#[test]
fn pid_0_from_an_accessory_is_the_innermost_live_chain_member() {
    let _g = lock();
    let mut pb = Aespb::new();
    let x = Proc::accessory(b"C:\\ACCX.NAC");

    // No chain process alive: 0 is the kernel context (D.g_int[0] exists from
    // gem_main on, geminit.c:851-877).
    x.run();
    let m = [0x4100, 2, 0, 0, 0, 0, 0, 0];
    assert_eq!(appl_write(&mut pb, 0, &m), 1);
    assert_eq!(pending(2), 0, "not into the accessory's own queue");
    kernel_context();
    assert_eq!(appl_read(&mut pb, 0), (1, m), "the kernel context reads it as pid 0");

    // A program and its Pexec child: the child is innermost.
    let parent = Proc::program(b"C:\\PARENT.NAP");
    let child = Proc::child_of(&parent);
    x.run();
    let m = [0x4101, 2, 0, 0, 0, 0, 0, 0];
    assert_eq!(appl_write(&mut pb, 0, &m), 1);
    // The parent is suspended in Pexec; its own queue stays empty (asked as
    // the parent: pid 0 from a chain member is the caller).
    parent.run();
    assert_eq!(pending(0), 0, "not into the suspended Pexec parent's queue");
    child.run();
    assert_eq!(appl_read(&mut pb, 0), (1, m), "the Pexec child — innermost — has it");
    assert_eq!(pending(0), 0, "nothing else for the child");
    drop(child);
    // The child is gone: the parent is innermost again.
    x.run();
    let m = [0x4102, 2, 0, 0, 0, 0, 0, 0];
    assert_eq!(appl_write(&mut pb, 0, &m), 1);
    parent.run();
    assert_eq!(appl_read(&mut pb, 0), (1, m), "after the child's end the parent has it");
    drop(parent);
    drop(x);
}

// ── 4. WF_OWNER and msg[1] of an accessory's WM_REDRAW ───────────────────

#[test]
fn an_accessory_window_reports_the_accessory_as_owner_and_sender() {
    let _g = lock();
    let mut pb = Aespb::new();
    let _x = Proc::accessory(b"C:\\ACCX.NAC");
    let y = Proc::accessory(b"C:\\ACCY.NAC");
    let main = Proc::program(b"C:\\MAIN.NAP");

    y.run();
    let h = wind_create(&mut pb);
    assert_eq!(wf_owner(&mut pb, h), 3, "WF_OWNER of ACCY's window, asked by ACCY (gemwmlib.c:1580)");
    wind_open(&mut pb, h, WR);
    let (got, body) = appl_read(&mut pb, 3);
    assert_eq!(
        (got, body[0], body[1], body[3]),
        (1, WM_REDRAW, 3, h),
        "ACCY's own WM_REDRAW: msg[1] = 3, the caller's pid (w_redraw inside wind_open, gemwmlib.c:904)"
    );
    main.run();
    assert_eq!(wf_owner(&mut pb, h), 3, "WF_OWNER of ACCY's window, asked by the program");
    let mw = wind_create(&mut pb);
    assert_eq!(wf_owner(&mut pb, mw), 0, "WF_OWNER of the program's window: 0");
    wind_delete(&mut pb, mw);
    y.run();
    wind_close(&mut pb, h);
    wind_delete(&mut pb, h);
    drain(&mut pb, 3);
    main.run();
    drain(&mut pb, 0);
}

// ── 5. pids no accessory holds ────────────────────────────────────────────

#[test]
fn a_pid_beyond_the_loaded_accessories_is_refused() {
    let _g = lock();
    let mut pb = Aespb::new();
    let _x = Proc::accessory(b"C:\\ACCX.NAC");
    let _y = Proc::accessory(b"C:\\ACCY.NAC");
    let main = Proc::program(b"C:\\MAIN.NAP");
    main.run();
    let m = [0x4200, 0, 0, 0, 0, 0, 0, 0];
    assert_eq!(appl_write(&mut pb, 4, &m), 0, "pid 4 >= totpds = 4: no descriptor, 0");
    assert_eq!(appl_write(&mut pb, -1, &m), 0, "a negative pid: 0");
    assert_eq!(appl_write(&mut pb, 17, &m), 0, "pid 17: 0");
    assert_eq!(pending(0), 0);
    assert_eq!(pending(2), 0);
    assert_eq!(pending(3), 0);
    assert_eq!(appl_read(&mut pb, 4).0, 0, "appl_read(4): no descriptor, 0");
}

// ── 6. a forgotten accessory does not renumber the others ─────────────────

#[test]
fn a_forgotten_accessory_does_not_shift_the_others() {
    let _g = lock();
    let mut pb = Aespb::new();
    let x = Proc::accessory(b"C:\\ACCX.NAC");
    let y = Proc::accessory(b"C:\\ACCY.NAC");
    let z = Proc::accessory(b"C:\\ACCZ.NAC");
    let main = Proc::program(b"C:\\MAIN.NAP");
    z.run();
    assert_eq!(appl_init(&mut pb), 4, "the third accessory reports 4");

    // ACCY ends (an accessory that Pterms: its slot is released).
    drop(y);
    z.run();
    assert_eq!(appl_init(&mut pb), 4, "ACCZ still reports 4 after ACCY is gone");
    x.run();
    assert_eq!(appl_init(&mut pb), 2, "ACCX still reports 2");
    main.run();
    assert_eq!(appl_find(&mut pb, b"ACCZ    "), 4, "appl_find(ACCZ) still 4");
    assert_eq!(appl_find(&mut pb, ACCY), -1, "the ended accessory's name answers nobody");
    let m = [0x4300, 0, 0, 0, 0, 0, 0, 0];
    assert_eq!(appl_write(&mut pb, 3, &m), 0, "pid 3 names no live accessory any more");
    assert_eq!(appl_write(&mut pb, 4, &m), 1, "pid 4 still reaches ACCZ");
    assert_eq!(pending(4), 1);
    z.run();
    drain(&mut pb, 4);
    drop(main);
    drop(z);
    drop(x);
}
