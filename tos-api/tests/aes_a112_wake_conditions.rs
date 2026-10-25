//! A112 — **which event wakes a blocked AES call, independent of which process
//! is running, and every AES call gives up the CPU.**
//!
//! Spec: `docs/api/aes/event.md` "Wake conditions and dispatch — contract
//! (A112)" and its edge cases; `docs/api/aes/application.md` `appl_yield`.
//! EmuTOS arms one `EVB` per requested event (`ev_multi` → `iasync`,
//! `aes/gemevlib.c:235-259`) and sleeps on their union (`mwait`,
//! `aes/gemasync.c:117-127`); each completes through `signal()`
//! (`gemasync.c:35-61`) from its own source — the timer through `tchange`
//! (`aes/gemflag.c:32-68`) on any dispatch, a message through `aqueue`
//! (`aes/gemqueue.c:104-118`) by whichever process writes it. `super()` ends
//! every AES call in `dsptch()` (`aes/gemsuper.c:454-466`), round robin. The
//! immediate conditions are tested once, at arm time (`gemevlib.c:172-290`).
//! `evnt_timer(0)` waits one tick (`geminput.c:500-501`).
//!
//! F4 (a resumed `evnt_multi` must not repeat its arm-time tests) has no case
//! here: it is not observable through the AES at this layer. The arm-time
//! tests differ from the parked ones only when the caller does not own the
//! mouse at resume, and NewTOS's parked rectangle/button tests ask
//! `input::own_mouse()` — "an application, not the Control Manager, owns the
//! mouse" — while the inline `w_setactive` of the same pass hands the mouse
//! back to the application at once. Covered by review of
//! `aes/event.rs` (`EV_ARMED`).
//!
//! Harness: two GEMDOS process slots, A and B, with the host playing the trap
//! handler — the test makes a slot "run" (`RUN` + `sched::set_running`) and
//! re-issues the very parameter block of a call that yielded mid-way (the S2
//! restartable block re-executes the `svc`). Whether a call **completed** is
//! read from `int_out[0]`: it is pre-set to a sentinel no call returns, and
//! only the copy-back of a completed call overwrites it.
//!
//! Shape (silent backend, `Proc` fixture, parameter block) follows
//! `aes_appl_read_blocks.rs`.

#![allow(clippy::too_many_arguments)]

use std::sync::atomic::{AtomicU64, Ordering};
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

// ── the host: a clock the test sets, one millisecond per input pump ────────

/// A host with no pixels and no keys. The clock is set by the test; every
/// `pump_input` (an idle pass of a block with nobody else ready, and the drain
/// every completed AES call ends with) advances it by one millisecond, so a
/// lone wait reaches its deadline and a runaway loop trips the bound.
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

fn clock() -> u64 {
    host().clock.load(Ordering::Relaxed)
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

/// What one trap did: the `Switch` the dispatcher returned (its target slot),
/// and `RET_CODE` when the call completed.
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
        Outcome {
            switch_to: sw.map(|s| s.slot),
            ret: (self.intout[0] != NOT_COMPLETED).then_some(self.intout[0]),
        }
    }

    fn call(&mut self, opcode: i16, in_len: i16, out_len: i16, ain_len: i16) -> Outcome {
        self.contrl[0] = opcode;
        self.contrl[1] = in_len;
        self.contrl[2] = out_len;
        self.contrl[3] = ain_len;
        self.contrl[4] = 0;
        self.trap()
    }

    /// A call that must complete; its `RET_CODE`.
    fn done(&mut self, opcode: i16, in_len: i16, out_len: i16, ain_len: i16) -> i16 {
        let o = self.call(opcode, in_len, out_len, ain_len);
        o.ret.unwrap_or_else(|| panic!("AES opcode {opcode} did not complete"))
    }
}

const APPL_INIT: i16 = 10;
const APPL_EXIT: i16 = 19;
const APPL_YIELD: i16 = 17;
const EVNT_TIMER: i16 = 24;
const EVNT_MULTI: i16 = 25;
const GRAF_MKSTATE: i16 = 79;
const WIND_CREATE: i16 = 100;
const WIND_OPEN: i16 = 101;
const WIND_CLOSE: i16 = 102;
const WIND_DELETE: i16 = 103;

const MU_MESAG: i16 = 0x0010;
const MU_TIMER: i16 = 0x0020;
const WM_REDRAW: i16 = 20;

/// `gl_ticktime`, ms per AES tick (`geminit.c:704`).
const TICK: u64 = 20;

fn appl_init(pb: &mut Aespb) {
    pb.intin.fill(0);
    pb.done(APPL_INIT, 0, 1, 0);
}

fn appl_exit(pb: &mut Aespb) {
    pb.intin.fill(0);
    let _ = pb.call(APPL_EXIT, 0, 1, 0);
}

fn graf_mkstate(pb: &mut Aespb) -> Outcome {
    pb.intin.fill(0);
    pb.call(GRAF_MKSTATE, 0, 5, 0)
}

/// Set up `evnt_multi(flags, timer ms, m1 = rect)` with `msg` as the buffer
/// and issue it.
fn evnt_multi(pb: &mut Aespb, flags: i16, ms: u32, m1: (i16, i16, i16, i16), msg: &mut [i16; 8]) -> Outcome {
    pb.intin.fill(0);
    pb.intin[0] = flags;
    pb.intin[4] = 0; // m1: enter
    pb.intin[5] = m1.0;
    pb.intin[6] = m1.1;
    pb.intin[7] = m1.2;
    pb.intin[8] = m1.3;
    pb.intin[14] = ms as u16 as i16;
    pb.intin[15] = (ms >> 16) as u16 as i16;
    pb.addrin.fill(0);
    pb.addrin[0] = msg.as_mut_ptr() as u64;
    pb.call(EVNT_MULTI, 16, 7, 1)
}

fn evnt_timer(pb: &mut Aespb, ms: u32) -> Outcome {
    pb.intin.fill(0);
    pb.intin[0] = ms as u16 as i16;
    pb.intin[1] = (ms >> 16) as u16 as i16;
    pb.call(EVNT_TIMER, 2, 1, 0)
}

fn open_window(pb: &mut Aespb, r: (i16, i16, i16, i16)) -> i16 {
    pb.intin.fill(0);
    pb.intin[0] = 0x000B; // NAME | CLOSER | MOVER
    pb.intin[3] = SCREEN_W as i16;
    pb.intin[4] = SCREEN_H as i16;
    let h = pb.done(WIND_CREATE, 5, 1, 0);
    assert!(h > 0, "wind_create: {h}");
    pb.intin.fill(0);
    pb.intin[0] = h;
    pb.intin[1] = r.0;
    pb.intin[2] = r.1;
    pb.intin[3] = r.2;
    pb.intin[4] = r.3;
    pb.done(WIND_OPEN, 5, 1, 0);
    h
}

fn close_window(pb: &mut Aespb, h: i16) {
    pb.intin.fill(0);
    pb.intin[0] = h;
    pb.done(WIND_CLOSE, 1, 1, 0);
    pb.intin[0] = h;
    pb.done(WIND_DELETE, 1, 1, 0);
}

// ── process slots the host can "run" ───────────────────────────────────────

/// A claimed GEMDOS process slot with a basepage page of its own (the
/// `aes_appl_read_blocks.rs` fixture). Released on drop.
struct Proc {
    slot: usize,
    page: *mut [u8; 4096],
}

impl Proc {
    fn new() -> Self {
        let slot = process::allocate_slot().expect("a free process slot");
        let page = Box::into_raw(Box::new([0u8; 4096]));
        PROCESS_TABLE[slot].basepage_kva.store(page.cast::<BasePage>(), Ordering::Release);
        Proc { slot, page }
    }

    fn bp(&self) -> *mut BasePage {
        self.page.cast::<BasePage>()
    }

    /// Make this the running process for GEMDOS and the scheduler — what the
    /// trap handler's `apply_switch` leaves behind.
    fn run(&self) {
        process::set_current(self.bp());
        sched::set_running(self.slot);
    }

    /// Put this slot on the ready list as a process that has run before (a
    /// peer waiting for the CPU, not a fresh entry).
    fn ready(&self) {
        PROCESS_TABLE[self.slot].has_ctx.store(true, Ordering::Release);
        sched::ready(self.slot);
    }

    fn is_ready(&self) -> bool {
        PROCESS_TABLE[self.slot].runnable.load(Ordering::Acquire)
    }
}

impl Drop for Proc {
    fn drop(&mut self) {
        if process::current() == self.bp() {
            process::set_current(std::ptr::null_mut());
        }
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

/// A rectangle away from the menu row nobody's pointer is in.
const NOWHERE: (i16, i16, i16, i16) = (0, 0, 0, 0);

// ── F1: the timer of a parked evnt_multi fires while another process runs ──

/// A process blocked in `evnt_multi(MU_TIMER)` is woken by its own deadline
/// while a different process runs and calls the AES — `tchange` completes the
/// `EVB` on any dispatch, whoever is running (`gemflag.c:32-68`, spec point
/// 2/4). The process that called the AES when the deadline passed keeps the
/// CPU for that call (the woken one joins the ready list behind it,
/// `gemdisp.c:231-244`) and hands it over at its next call; the resumed wait
/// returns `MU_TIMER`.
#[test]
fn f1_the_timer_of_a_parked_evnt_multi_fires_while_another_process_runs() {
    let _g = lock();
    set_clock(10_000);
    let a = Proc::new();
    let b = Proc::new();
    let mut pa = Aespb::new();
    let mut pbb = Aespb::new();
    let mut msg = [0i16; 8];

    b.ready();
    a.run();
    appl_init(&mut pa);
    let wait = evnt_multi(&mut pa, MU_TIMER, 100, NOWHERE, &mut msg);
    assert_eq!(wait.ret, None, "the 100-ms wait cannot complete at once");
    assert_eq!(wait.switch_to, Some(b.slot), "…it gives the CPU to the ready peer");
    assert!(!a.is_ready(), "…and A is parked");

    // B runs and calls the AES after A's deadline.
    b.run();
    set_clock(10_200);
    let first = graf_mkstate(&mut pbb);
    let a_woken = a.is_ready();
    let second = graf_mkstate(&mut pbb);

    // A is resumed: the svc re-executes the very same evnt_multi.
    a.run();
    let resumed = pa.trap();
    appl_exit(&mut pa);

    assert!(first.ret.is_some(), "graf_mkstate completes");
    assert!(
        a_woken,
        "A's MU_TIMER deadline passed while B called the AES: A must be READY \
         (EmuTOS tchange on B's dispatch, gemflag.c:32-68), not parked until input"
    );
    assert_eq!(
        second.switch_to,
        Some(a.slot),
        "B's next completed AES call must hand the CPU to the woken A \
         (super() -> dsptch(), gemsuper.c:454-466)"
    );
    assert!(second.ret.is_some(), "…after completing (the hand-over is the closing dispatch)");
    assert_eq!(resumed.ret, Some(MU_TIMER), "the resumed evnt_multi reports MU_TIMER");
}

// ── F2: a message from another process wakes a parked evnt_multi(MU_MESAG) ─

/// A process blocked in `evnt_multi(MU_MESAG)` is woken by a message another
/// process causes to be written into its pipe — here the `WM_REDRAW` the AES
/// posts to A when B closes a window covering A's (`aqueue` services the
/// parked reader, `gemqueue.c:104-118`); `evnt_mesag` already behaved so.
#[test]
fn f2_a_message_from_another_process_wakes_a_parked_evnt_multi_mesag() {
    let _g = lock();
    set_clock(20_000);
    let a = Proc::new();
    let b = Proc::new();
    let mut pa = Aespb::new();
    let mut pbb = Aespb::new();
    let mut msg = [0i16; 8];

    a.run();
    appl_init(&mut pa);
    let wa = open_window(&mut pa, (40, 60, 200, 120));
    tos_api::aes::window::test_clear_msg(0); // A's own WM_REDRAW from wind_open
    b.ready();
    let wait = evnt_multi(&mut pa, MU_MESAG, 0, NOWHERE, &mut msg);
    assert_eq!(wait.ret, None, "nothing is queued: the wait blocks");
    assert_eq!(wait.switch_to, Some(b.slot));
    assert!(!a.is_ready());

    b.run();
    appl_init(&mut pbb);
    let cover = open_window(&mut pbb, (0, 20, 640, 460));
    close_window(&mut pbb, cover);
    let a_woken = a.is_ready();

    a.run();
    let resumed = if a_woken { Some(pa.trap()) } else { None };
    close_window(&mut pa, wa);
    appl_exit(&mut pa);
    b.run();
    appl_exit(&mut pbb);

    assert!(
        a_woken,
        "the WM_REDRAW B's window close posted into A's pipe must make A READY \
         (gemqueue.c:104-118, signal gemasync.c:35-61), not wait for input"
    );
    let resumed = resumed.unwrap();
    assert_eq!(resumed.ret, Some(MU_MESAG));
    assert_eq!((msg[0], msg[3]), (WM_REDRAW, wa), "the message is A's WM_REDRAW");
}

// ── F3: every completed AES call gives the CPU to a ready peer ─────────────

/// `super()` falls through to `dsptch()` after every AES call
/// (`gemsuper.c:454-466`): a call that completed without blocking still lets a
/// ready peer run first. Alone on the ready list the caller simply continues.
#[test]
fn f3_a_completed_aes_call_gives_the_cpu_to_a_ready_peer() {
    let _g = lock();
    set_clock(30_000);
    let a = Proc::new();
    let b = Proc::new();
    let mut pa = Aespb::new();

    a.run();
    let alone = graf_mkstate(&mut pa);
    b.ready();
    let shared = graf_mkstate(&mut pa);
    drop(b);

    assert!(alone.ret.is_some() && alone.switch_to.is_none(), "alone: complete and continue");
    assert!(shared.ret.is_some(), "with a ready peer the call still completes");
    assert!(
        shared.switch_to.is_some() && shared.switch_to != Some(a.slot),
        "…and then gives the CPU to the peer (round robin), got {:?}",
        shared.switch_to
    );
    assert!(a.is_ready(), "the caller stays on the ready list (disp_act, gemdisp.c:74-79)");
}

// ── 0-ms / sub-tick timers ─────────────────────────────────────────────────

/// `evnt_timer(0)` is not a poll: `adelay` raises 0 ticks to 1
/// (`geminput.c:500-501`), so it returns at the next tick boundary. A sub-tick
/// `evnt_multi` count is armed the same way (`gemevlib.c:259`). Only
/// `evnt_multi` with `tmcount == 0` exactly is a poll (`gemevlib.c:216-220`);
/// counts of a tick or more keep their length.
#[test]
fn evnt_timer_0_and_sub_tick_counts_wait_for_the_next_tick() {
    let _g = lock();
    let a = Proc::new();
    let mut pa = Aespb::new();
    let mut msg = [0i16; 8];
    a.run();
    appl_init(&mut pa);

    set_clock(40_005);
    let t0 = evnt_timer(&mut pa, 0);
    let after_t0 = clock();

    set_clock(40_105);
    let poll = evnt_multi(&mut pa, MU_TIMER, 0, NOWHERE, &mut msg);
    let after_poll = clock();

    set_clock(40_205);
    let sub = evnt_multi(&mut pa, MU_TIMER, 5, NOWHERE, &mut msg);
    let after_sub = clock();

    set_clock(40_305);
    let long = evnt_multi(&mut pa, MU_TIMER, 30, NOWHERE, &mut msg);
    let after_long = clock();
    appl_exit(&mut pa);

    assert_eq!(t0.ret, Some(1));
    assert!(
        after_t0 >= 40_000 + TICK,
        "evnt_timer(0) must wait for the next tick boundary (40020), returned at {after_t0}"
    );
    assert_eq!(poll.ret, Some(MU_TIMER));
    assert!(
        after_poll <= 40_107,
        "evnt_multi(MU_TIMER, 0) is a poll (two input drains: ev_multi's head and the \
         closing dispatch), returned at {after_poll}"
    );
    assert_eq!(sub.ret, Some(MU_TIMER));
    assert!(
        after_sub >= 40_200 + TICK,
        "evnt_multi(MU_TIMER, 5) is armed as one tick (to 40220), returned at {after_sub}"
    );
    assert_eq!(long.ret, Some(MU_TIMER));
    assert!(
        (40_335..40_340).contains(&after_long),
        "a 30-ms count keeps its length, returned at {after_long}"
    );
}

// ── appl_yield: its own dsptch(), then super()'s ───────────────────────────

/// `appl_yield` is `dsptch()` and nothing else (`gemsuper.c:115-119`), and
/// `super()` dispatches once more after it (`gemsuper.c:458-462`): with a
/// ready peer the call first gives up the CPU *before* completing, and once
/// resumed it completes with 1 and dispatches again. Alone it completes at
/// once.
#[test]
fn appl_yield_gives_up_the_cpu_before_and_after_it_completes() {
    let _g = lock();
    set_clock(60_000);
    let a = Proc::new();
    let b = Proc::new();
    let mut pa = Aespb::new();

    a.run();
    pa.intin.fill(0);
    let alone = pa.call(APPL_YIELD, 0, 1, 0);
    b.ready();
    let first = pa.call(APPL_YIELD, 0, 1, 0);
    // The peer ran; A is resumed and its svc re-executes.
    a.run();
    let second = pa.trap();
    drop(b);

    assert_eq!((alone.ret, alone.switch_to), (Some(1), None), "alone: 1 at once");
    assert_eq!(first.ret, None, "with a ready peer appl_yield dispatches before completing");
    assert!(first.switch_to.is_some() && first.switch_to != Some(a.slot));
    assert_eq!(second.ret, Some(1), "resumed, it completes with TRUE");
    assert!(
        second.switch_to.is_some(),
        "…and the closing dispatch gives the CPU away again (the peer is still ready)"
    );
}
