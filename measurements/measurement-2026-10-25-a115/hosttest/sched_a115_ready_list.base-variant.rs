//! A115 — **the dispatcher runs EmuTOS's ready list (`rlr`/`drl`), not slot
//! order.**
//!
//! Spec: `docs/api/aes/event.md` "Wake conditions and dispatch", point 3
//! ("Order = round robin over `rlr`"). EmuTOS `disp()` (`aes/gemdisp.c:221-253`)
//! takes the running process off the head of `rlr`; a ready one goes to the
//! **end** (`disp_act` → `insert_process`, `:74-79`, `aes/gempd.c:133-145`), a
//! waiting one to `nrl` (`mwait_act`, `:82-93`); then the fork ring runs
//! (`forker`, `tchange` → `azombie` → `signal`) and `schedule()` appends `drl`
//! to `rlr` (`:181-205`), and the **head** of `rlr` is resumed. `drl` is a
//! push-front stack: `signal` (`aes/gemasync.c:56-57`) and `pstart`
//! (`aes/gempd.c:125-128`) push at the head, and `schedule` drains it from the
//! head — so processes made ready in one pass join `rlr` in reverse order,
//! behind the caller.
//!
//! Pinned here, at the AES trap boundary:
//!
//! 1. sender and receiver: a process woken by an `appl_write` runs behind the
//!    writer — the write completes **without** a switch when the writer is
//!    otherwise alone, and the writer's next AES call hands over;
//! 2. two processes woken in one pass (two expired timers) run in reverse
//!    wake order;
//! 3. desk accessories started with `pstart` run in reverse start order, and
//!    the EL1 kernel client (EmuTOS pid 0) is one more entry of the round
//!    robin.
//!
//! Harness: the `aes_a116_appl_write_blocks.rs` fixture (GEMDOS slots the host
//! "runs", re-issued parameter blocks for an S2 restart, `int_out[0]`
//! sentinel), plus a clock the test sets.

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

fn set_clock(ms: u64) {
    host().clock.store(ms, Ordering::Relaxed);
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
const APPL_WRITE: i16 = 12;
const EVNT_MESAG: i16 = 23;
const EVNT_TIMER: i16 = 24;

/// The application chain's pid (`D.g_int[0]`, `aes/geminit.c:851-877`).
const AP_CHAIN: i16 = 0;
/// The first accessory's pid (`D.g_acc[0]`, `aes/geminit.c:819,851-888`).
const ACC0: i16 = 2;

/// A standard 16-byte message tagged `n` from sender `from`.
fn msg(from: i16, n: i16) -> [i16; 8] {
    [0x4A00 + n, from, 0, n, 0, 0, 0, 0]
}

fn appl_init(pb: &mut Aespb) -> i16 {
    pb.intin.fill(0);
    pb.call(APPL_INIT, 0, 1, 0).ret.expect("appl_init completes")
}

/// `appl_write(pid, 16, msg)`.
fn appl_write(pb: &mut Aespb, pid: i16, m: &[i16; 8]) -> Outcome {
    pb.intin.fill(0);
    pb.intin[0] = pid;
    pb.intin[1] = 16;
    pb.addrin.fill(0);
    pb.addrin[0] = m.as_ptr() as u64;
    pb.call(APPL_WRITE, 2, 1, 1)
}

/// `evnt_mesag(buf)`.
fn evnt_mesag(pb: &mut Aespb, buf: &mut [i16; 8]) -> Outcome {
    pb.intin.fill(0);
    pb.addrin.fill(0);
    pb.addrin[0] = buf.as_mut_ptr() as u64;
    pb.call(EVNT_MESAG, 0, 1, 1)
}

/// `evnt_timer(ms)`.
fn evnt_timer(pb: &mut Aespb, ms: u32) -> Outcome {
    pb.intin.fill(0);
    pb.intin[0] = ms as u16 as i16;
    pb.intin[1] = (ms >> 16) as u16 as i16;
    pb.call(EVNT_TIMER, 2, 1, 0)
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
        PROCESS_TABLE[self.slot].runnable.store(true, Ordering::Release);
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
        PROCESS_TABLE[self.slot].runnable.store(false, Ordering::Release);
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

// ── 1. sender and receiver ─────────────────────────────────────────────────

/// The accessory waits in `evnt_mesag`; the program, alone on `rlr`, writes it
/// a message. `aqueue` services the parked reader and `signal` puts it on
/// `drl` (`gemqueue.c:104-118`, `gemasync.c:46-59`); `super()`'s closing
/// `dsptch()` appends the writer to `rlr` first and then drains `drl` behind
/// it (`gemdisp.c:231-243`): `rlr = [writer, reader]`, the writer is resumed at
/// once. Only its next AES call puts it behind the reader.
#[test]
fn a_process_woken_by_appl_write_runs_behind_the_writer() {
    let _g = lock();
    set_clock(10_000);
    let x = Proc::accessory(b"C:\\ACCX.NAC");
    let m = Proc::program(b"C:\\MAIN.NAP");
    let (mut px, mut pm) = (Aespb::new(), Aespb::new());
    x.run();
    assert_eq!(appl_init(&mut px), ACC0);
    m.run();
    assert_eq!(appl_init(&mut pm), AP_CHAIN);

    // The accessory blocks in its message wait and the CPU goes to the program.
    x.run();
    let mut got = [0i16; 8];
    let wait = evnt_mesag(&mut px, &mut got);
    assert_eq!(wait.ret, None, "nothing is queued: the wait blocks");
    assert_eq!(wait.switch_to, Some(m.slot), "…and the program is the only one ready");

    m.run();
    let w1 = appl_write(&mut pm, ACC0, &msg(AP_CHAIN, 1));
    let x_woken = x.is_ready();
    let w2 = appl_write(&mut pm, ACC0, &msg(AP_CHAIN, 2));

    x.run();
    let resumed = px.trap();

    assert_eq!(w1.ret, Some(1), "the write completes");
    assert!(x_woken, "…and makes the waiting accessory ready (signal, gemasync.c:46-59)");
    assert_eq!(
        w1.switch_to, None,
        "the reader joins rlr BEHIND the writer (disp_act before schedule, gemdisp.c:231-243): \
         the writer, otherwise alone, is resumed at once"
    );
    assert_eq!(w2.ret, Some(1));
    assert_eq!(
        w2.switch_to,
        Some(x.slot),
        "the writer's next AES call goes to the end of rlr and the reader, now its head, runs"
    );
    assert_eq!((resumed.ret, got), (Some(1), msg(AP_CHAIN, 1)), "the reader gets message 1");
}

// ── 2. two processes woken in one pass ─────────────────────────────────────

/// Two accessories wait in `evnt_timer`, A with the earlier deadline. Both
/// deadlines pass before the program's next AES call; `tchange` completes the
/// delay list in deadline order (`gemflag.c:32-68`, `dlr` sorted by
/// `adelay`, `geminput.c:520-540`), so `signal` pushes A, then C onto `drl`,
/// and `schedule` appends them as `[C, A]` (`gemdisp.c:193-197`). The program
/// then blocks: C runs first, and C's closing dispatch hands over to A.
/// Slot order would give A first — the slots are allocated A < C < program.
#[test]
fn two_processes_woken_in_one_pass_run_in_reverse_wake_order() {
    let _g = lock();
    set_clock(20_000);
    let a = Proc::accessory(b"C:\\ACCA.NAC");
    let c = Proc::accessory(b"C:\\ACCC.NAC");
    let b = Proc::program(b"C:\\MAIN.NAP");
    assert!(a.slot < c.slot && c.slot < b.slot, "fixture: slot order A < C < program");
    let (mut pa, mut pc, mut pb) = (Aespb::new(), Aespb::new(), Aespb::new());
    b.ready();

    a.run();
    appl_init(&mut pa);
    let wa = evnt_timer(&mut pa, 100);
    assert_eq!(wa.ret, None, "A's 100-ms wait blocks");
    c.run();
    appl_init(&mut pc);
    let wc = evnt_timer(&mut pc, 200);
    assert_eq!(wc.ret, None, "C's 200-ms wait blocks");

    b.run();
    set_clock(20_500);
    appl_init(&mut pb);
    let (a_woken, c_woken) = (a.is_ready(), c.is_ready());
    let mut got = [0i16; 8];
    let blocked = evnt_mesag(&mut pb, &mut got);

    c.run();
    let rc = pc.trap();
    a.run();
    let ra = pa.trap();

    assert!(a_woken && c_woken, "both deadlines passed: both are ready");
    assert_eq!(blocked.ret, None, "the program's evnt_mesag blocks");
    assert_eq!(
        blocked.switch_to,
        Some(c.slot),
        "drl is a push-front stack (gemasync.c:56-57): the later-woken C is ahead of A"
    );
    assert_eq!(rc.ret, Some(1), "C's evnt_timer completes");
    assert_eq!(rc.switch_to, Some(a.slot), "…and its closing dispatch hands over to A");
    assert_eq!(ra.ret, Some(1), "A's evnt_timer completes");
}

// ── 3. pstart order and the kernel client ──────────────────────────────────

/// `pstart` puts each new process on top of `drl` (`gempd.c:125-128`), so two
/// accessories started X then Y join `rlr` as `[Y, X]` behind the dispatcher's
/// caller. The caller here is the EL1 kernel client (the boot path's
/// accessory barrier, EmuTOS pid 0, `geminit.c:736-737`): its first dispatch
/// appends it before the drain, so it stays at the head and is resumed at once
/// (EmuTOS's `dsptch(); /* off we go */` ahead of `wait_for_accs`); the next
/// one runs Y, Y's dispatch runs X, X's the kernel client again.
#[test]
fn accessories_run_in_reverse_start_order_and_the_kernel_client_takes_its_turn() {
    let _g = lock();
    set_clock(30_000);
    let x = Proc::accessory(b"C:\\ACCX.NAC");
    let y = Proc::accessory(b"C:\\ACCY.NAC");

    process::set_current(std::ptr::null_mut());
    sched::set_running(MAX_PROCESSES);
    sched::set_kernel_parked(false);
    let first = sched::pick_peer_for_kernel();
    let second = sched::pick_peer_for_kernel();
    // The trap handler parks the kernel frame and switches.
    sched::set_kernel_parked(true);
    y.run();
    let after_y = sched::dsptch();
    x.run();
    let after_x = sched::dsptch();
    sched::set_kernel_parked(false);

    assert!(
        first.is_none(),
        "the kernel client's first dispatch keeps it at the head of rlr (appended before drl is drained)"
    );
    assert_eq!(
        second.map(|s| s.slot),
        Some(y.slot),
        "pstart pushes onto drl: the later-started Y runs first"
    );
    match after_y {
        sched::Dispatch::Yield(sw) => assert_eq!(sw.slot, x.slot, "Y's dispatch runs X"),
        sched::Dispatch::Idle => panic!("X is ready"),
    }
    match after_x {
        sched::Dispatch::Yield(sw) => assert!(
            sw.el1_internal,
            "X's dispatch resumes the kernel client, the next entry of rlr"
        ),
        sched::Dispatch::Idle => panic!("the kernel client is parked on rlr"),
    }
}
