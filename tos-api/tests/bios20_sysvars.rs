//! TASK/BIOS20 — the system variables served as EmuTOS serves them, the parts
//! that move at run time (spec `docs/api/bios/_sysvars-counters-contract.md`,
//! section 17.5; the static cells the loader writes are tested beside
//! `populate_sysvars` in `tos-api/src/bdos/process/loader.rs`).
//!
//! - `_frclock` (`$466`) and `_vbclock` (`$462`) count the host frame clock
//!   from the boot frame; they are published by the timer interrupt
//!   (`tick::advance`), by `Vsync` before it returns, and at every switch-in.
//! - `_vbclock` = `_frclock` minus the frames the process ran with its
//!   `_vblsem` (`$452`) at or below 0 (decision Q1 (c)); a process that was
//!   away loses nothing.
//! - `_vblsem` at or below 0 in the running process gates the frame work
//!   (`tick::vbl_semaphore_down`, read by the kernel's `vb_draw` tick).
//! - `_v_bas_ad` (`$44E`) follows what `Logbase` answers: at a switch-in and
//!   after the process's own `Setscreen`.
//! - The ADR-031 watcher: `dump_vec` is quiet at the `ret` stub and warns on
//!   another value; a non-empty `_vbl_list` slot warns, `$456` itself is not
//!   compared.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, Once};

use tos_api::bdos::process::hooks::{self, Hook, Kind, Via, Warning};
use tos_api::bdos::process::{self, mmu, PROCESS_TABLE};
use tos_api::bios::tick;
use tos_api::host::{install_host, Host, ScreenInfo};
use tos_api::trap::TrapArgs;
use tos_api::xbios::kbdvecs;
use tos_types::BasePage;

// ─── The host double: a settable frame clock and a two-page screen ────────

/// The raw frame-clock value the host reports.
static FRAME: AtomicU64 = AtomicU64::new(1000);
/// When set, every `frame_clock` call advances the clock by one (so `Vsync`,
/// which spins until the clock changes, returns).
static AUTO: AtomicBool = AtomicBool::new(false);
/// The logical screen base the host reports (`ScreenInfo::base`).
static LOGICAL: AtomicU64 = AtomicU64::new(SCREEN_MEM);

const SCREEN_MEM: u64 = 0x1000_0000;
const SCREEN_SIZE: u64 = 640 * 4 * 480;

struct FrameHost;

impl Host for FrameHost {
    fn console_write_byte(&self, _byte: u8) {}
    fn frame_clock(&self) -> u64 {
        if AUTO.load(Ordering::SeqCst) {
            FRAME.fetch_add(1, Ordering::SeqCst) + 1
        } else {
            FRAME.load(Ordering::SeqCst)
        }
    }
    fn screen(&self) -> Option<ScreenInfo> {
        Some(ScreenInfo {
            base: LOGICAL.load(Ordering::SeqCst),
            phys_base: SCREEN_MEM,
            mem_base: SCREEN_MEM,
            mem_bytes: (2 * SCREEN_SIZE) as u32,
            width: 640,
            height: 480,
            bpp: 32,
            stride_bytes: 640 * 4,
            size_bytes: SCREEN_SIZE as u32,
        })
    }
    fn set_screen_logical(&self, base: u64) -> bool {
        let ok = (SCREEN_MEM..SCREEN_MEM + 2 * SCREEN_SIZE).contains(&base) && (base - SCREEN_MEM).is_multiple_of(SCREEN_SIZE);
        if ok {
            LOGICAL.store(base, Ordering::SeqCst);
        }
        ok
    }
}

static FRAME_HOST: FrameHost = FrameHost;
static HOST: &dyn Host = &FRAME_HOST;
static INSTALL: Once = Once::new();
static LOCK: Mutex<()> = Mutex::new(());

/// Serialise the tests (one global clock, one process table), install the
/// host once, record the boot frame anew and park the CPU in kernel code.
fn setup() -> MutexGuard<'static, ()> {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    INSTALL.call_once(|| install_host(&HOST));
    AUTO.store(false, Ordering::SeqCst);
    LOGICAL.store(SCREEN_MEM, Ordering::SeqCst);
    process::set_current(core::ptr::null_mut());
    FRAME.store(1000, Ordering::SeqCst);
    tick::start_frame_clock();
    while hooks::pop().is_some() {}
    g
}

/// Advance the host frame clock by `n` frames.
fn frames(n: u64) {
    FRAME.fetch_add(n, Ordering::SeqCst);
}

// ─── A process with a page 0 as the loader leaves the cells under test ────

struct Proc {
    slot: usize,
    bp: *mut BasePage,
}

impl Proc {
    fn new() -> Proc {
        let slot = process::allocate_slot().expect("a slot");
        let bp = Box::into_raw(Box::new([0u8; 256])) as *mut BasePage;
        let p = &PROCESS_TABLE[slot];
        p.basepage_kva.store(bp, Ordering::Release);
        // SAFETY: under the lock; the slot is not an installed root.
        unsafe { mmu::init_slot_tables(slot) }.unwrap();
        // SAFETY: under the lock; the slot's page 0 belongs to this process.
        unsafe {
            let page0 = mmu::page0_bytes(slot).unwrap();
            page0.fill(0);
            tos_api::bios::vectors::write_page0_vectors(page0);
            // The page-0 hook cells hold the loader's values.
            for h in hooks::PAGE0_HOOKS.iter() {
                let at = h.addr as usize;
                page0[at..at + 4].copy_from_slice(&h.default.to_le_bytes());
            }
        }
        // `_vblsem` = 1, as `populate_sysvars` leaves it; nothing lost yet.
        tick::reset_frames(slot);
        p.set_state(process::State::Running);
        let me = Proc { slot, bp };
        me.set_vblsem(1);
        me
    }

    fn run(&self) {
        process::set_current(process::pd_of(self.slot));
    }

    fn page0(&self) -> &'static mut [u8] {
        // SAFETY: under the lock; the process's own page 0.
        unsafe { mmu::page0_bytes(self.slot) }.unwrap()
    }

    fn rd32(&self, addr: usize) -> u32 {
        u32::from_le_bytes(self.page0()[addr..addr + 4].try_into().unwrap())
    }

    fn poke(&self, addr: usize, v: u32) {
        self.page0()[addr..addr + 4].copy_from_slice(&v.to_le_bytes());
    }

    /// Write `_vblsem`, as the program does in super state.
    fn set_vblsem(&self, v: i16) {
        self.page0()[0x452..0x454].copy_from_slice(&v.to_le_bytes());
    }

    fn frclock(&self) -> u32 {
        self.rd32(0x466)
    }

    fn vbclock(&self) -> u32 {
        self.rd32(0x462)
    }
}

impl Drop for Proc {
    fn drop(&mut self) {
        process::set_current(core::ptr::null_mut());
        process::free_slot(self.slot);
        // SAFETY: allocated in `new` with `Box::into_raw`.
        drop(unsafe { Box::from_raw(self.bp as *mut [u8; 256]) });
    }
}

// ─── `_frclock` / `_vbclock` ─────────────────────────────────────────────

/// C07 (host form): the counters start at 0 at the boot frame and advance
/// with the host frame clock; with `_vblsem` = 1 both are equal.
#[test]
fn the_frame_counters_start_at_0_and_follow_the_frame_clock() {
    let _g = setup();
    assert_eq!(tick::frclock(), 0, "0 at the boot frame");
    let a = Proc::new();
    a.run();
    assert_eq!((a.frclock(), a.vbclock()), (0, 0), "the switch-in publishes the boot frame");
    frames(5);
    tick::advance(1);
    assert_eq!((a.frclock(), a.vbclock()), (5, 5), "the timer interrupt publishes");
    frames(60);
    tick::advance(1);
    assert_eq!((a.frclock(), a.vbclock()), (65, 65));
    // `_hz_200` keeps being published beside them.
    assert_eq!(a.rd32(0x4BA), tick::hz_200());
}

/// No process runs: nothing is published, the counter itself goes on.
#[test]
fn nothing_is_published_without_a_running_process() {
    let _g = setup();
    let a = Proc::new();
    frames(7);
    tick::advance(1);
    assert_eq!((a.frclock(), a.vbclock()), (0, 0), "not running: untouched");
    assert_eq!(tick::frclock(), 7);
    assert!(!tick::vbl_semaphore_down(), "the gate does not apply in EL1");
}

/// C08 (host form): with `_vblsem` = 0 for n frames, `_frclock - _vbclock`
/// grows by exactly n; at 1 again both advance together.
#[test]
fn vbclock_stops_while_vblsem_is_at_or_below_0() {
    let _g = setup();
    let a = Proc::new();
    a.run();
    frames(10);
    tick::advance(1);
    assert_eq!((a.frclock(), a.vbclock()), (10, 10));
    a.set_vblsem(0);
    frames(3);
    tick::advance(1);
    assert_eq!((a.frclock(), a.vbclock()), (13, 10), "3 frames with the semaphore down");
    a.set_vblsem(-1);
    frames(2);
    tick::advance(1);
    assert_eq!((a.frclock(), a.vbclock()), (15, 10), "below 0 counts as down");
    a.set_vblsem(1);
    frames(4);
    tick::advance(1);
    assert_eq!((a.frclock(), a.vbclock()), (19, 14), "both advance again");
    assert_eq!(a.frclock() - a.vbclock(), 5);
}

/// C09 (host form): a process that was away loses nothing; the switch-in
/// refreshes its counters at once, so they are strictly larger.
#[test]
fn a_switched_in_process_is_refreshed_and_loses_nothing_while_away() {
    let _g = setup();
    let a = Proc::new();
    let b = Proc::new();
    a.run();
    frames(4);
    tick::advance(1);
    a.set_vblsem(0); // the semaphore is down while `a` is away
    frames(2);
    b.run(); // `a` is accounted up to the switch: 2 frames lost
    let f_a = a.frclock();
    assert_eq!((f_a, a.vbclock()), (6, 4));
    frames(30);
    tick::advance(1);
    assert_eq!((b.frclock(), b.vbclock()), (36, 36), "b runs with its semaphore up");
    assert_eq!(a.frclock(), f_a, "a's copy does not move while it is away");
    a.run();
    assert_eq!(a.frclock(), 36, "refreshed at the switch-in");
    assert!(a.frclock() > f_a);
    assert_eq!(a.vbclock(), 34, "the 30 frames away are not charged");
    // Running again with the semaphore down charges from here on.
    frames(1);
    tick::advance(1);
    assert_eq!((a.frclock(), a.vbclock()), (37, 34));
}

/// C07: `Vsync` publishes before it returns — `a = *_frclock; Vsync();
/// b = *_frclock` gives `b - a >= 1`, and `_vbclock` with it.
#[test]
fn vsync_publishes_the_new_frame_before_it_returns() {
    let _g = setup();
    let a = Proc::new();
    a.run();
    let (f0, v0) = (a.frclock(), a.vbclock());
    AUTO.store(true, Ordering::SeqCst);
    assert_eq!(tos_api::xbios::system::vsync(), 0);
    AUTO.store(false, Ordering::SeqCst);
    assert!(a.frclock().wrapping_sub(f0) >= 1, "frclock {} -> {}", f0, a.frclock());
    assert!(a.vbclock().wrapping_sub(v0) >= 1, "vbclock {} -> {}", v0, a.vbclock());
    // Three Vsyncs with the semaphore down: frclock moves, vbclock does not.
    a.set_vblsem(0);
    let (f1, v1) = (a.frclock(), a.vbclock());
    AUTO.store(true, Ordering::SeqCst);
    for _ in 0..3 {
        tos_api::xbios::system::vsync();
    }
    AUTO.store(false, Ordering::SeqCst);
    assert!(a.frclock().wrapping_sub(f1) >= 3);
    assert_eq!(a.vbclock(), v1, "d462 == 0");
}

// ─── `_vblsem` as the frame-work gate ────────────────────────────────────

/// The kernel's `vb_draw` tick asks this: down at 0 and below, up above 0.
#[test]
fn the_frame_work_gate_reads_the_running_processs_vblsem() {
    let _g = setup();
    let a = Proc::new();
    a.run();
    assert!(!tick::vbl_semaphore_down(), "1: frame work runs");
    a.set_vblsem(0);
    assert!(tick::vbl_semaphore_down(), "0: skipped");
    a.set_vblsem(-5);
    assert!(tick::vbl_semaphore_down(), "negative: skipped");
    a.set_vblsem(2);
    assert!(!tick::vbl_semaphore_down(), "2: runs");
    process::set_current(core::ptr::null_mut());
    assert!(!tick::vbl_semaphore_down(), "no process: runs");
}

// ─── `_v_bas_ad` ─────────────────────────────────────────────────────────

/// C02/C03 (host form): the switch-in writes the address `Logbase` answers a
/// process (the user address of the logical screen, `caller_view`); after the
/// process's own `Setscreen(L, -1, -1)` the cell is `L`. `Logbase` itself is
/// not called here: for a process it publishes the screen uncached, whose
/// cache maintenance (`dc civac`) is not executable in a host test; the probe
/// on the board compares the cell with `Logbase()` (C02).
#[test]
fn v_bas_ad_follows_logbase_at_switch_in_and_after_setscreen() {
    let _g = setup();
    let a = Proc::new();
    a.run();
    let expect = process::kva_to_user_va(SCREEN_MEM) as u32;
    assert_eq!(a.rd32(0x44E), expect, "switch-in: the logical screen");
    assert_eq!(tos_api::xbios::screen::v_bas_ad_value(), Some(expect));
    let l = SCREEN_MEM + SCREEN_SIZE;
    let args = TrapArgs::kernel([l, u64::MAX, u64::MAX, 0, 0, 0, 0, 0], 0x05, 14);
    assert_eq!(tos_api::xbios::screen::setscreen(&args), 0);
    assert_eq!(a.rd32(0x44E) as u64, l, "Setscreen moved it");
    assert_eq!(LOGICAL.load(Ordering::SeqCst), l, "the display's logical base moved with it");
    // Another process sees the moved screen at its next switch-in.
    let b = Proc::new();
    assert_eq!(b.rd32(0x44E), 0, "not yet running");
    b.run();
    assert_eq!(b.rd32(0x44E) as u64, l);
}

/// `Setscreen` with `logLoc <= 0` leaves the cell alone.
#[test]
fn setscreen_without_a_logical_base_leaves_v_bas_ad() {
    let _g = setup();
    let a = Proc::new();
    a.run();
    a.poke(0x44E, 0x1234_5000);
    let args = TrapArgs::kernel([u64::MAX, u64::MAX, u64::MAX, 0, 0, 0, 0, 0], 0x05, 14);
    tos_api::xbios::screen::setscreen(&args);
    assert_eq!(a.rd32(0x44E), 0x1234_5000);
}

// ─── The ADR-031 watcher (amendment of 2026-11-20, items 2 and 3) ─────────

fn not_served(ws: &[Warning]) -> Vec<(usize, Hook, u64, Via)> {
    ws.iter()
        .filter_map(|w| match w.kind {
            Kind::NotServed(h, via) => Some((w.slot, h, w.value, via)),
            _ => None,
        })
        .collect()
}

fn checked(slot: usize) -> Vec<(usize, Hook, u64, Via)> {
    hooks::check_page0(slot);
    not_served(&std::iter::from_fn(hooks::pop).collect::<Vec<_>>())
}

/// C10 (watcher): `dump_vec` at the `ret` stub is quiet; another value warns.
#[test]
fn dump_vec_warns_only_when_it_differs_from_the_stub() {
    let _g = setup();
    let a = Proc::new();
    let stub = kbdvecs::RET_STUB_VA as u32;
    assert_eq!(a.rd32(0x502), stub, "the loader's default");
    assert!(checked(a.slot).is_empty(), "quiet at the default");
    a.poke(0x502, 0x0001_6300);
    assert_eq!(checked(a.slot), vec![(a.slot, Hook::DumpVec, 0x0001_6300, Via::Page0)]);
    // 0 is no longer the default either (a fresh process).
    let b = Proc::new();
    b.poke(0x502, 0);
    assert_eq!(checked(b.slot), vec![(b.slot, Hook::DumpVec, 0, Via::Page0)]);
}

/// C06 (watcher): empty slots are quiet, a routine in any of the eight warns;
/// `$456` itself is not compared.
#[test]
fn a_routine_in_a_vbl_list_slot_warns() {
    let _g = setup();
    let a = Proc::new();
    a.poke(0x456, 0x4CE);
    assert!(checked(a.slot).is_empty(), "eight empty slots");
    a.poke(0x456, 0x0001_9000);
    assert!(checked(a.slot).is_empty(), "the cell $456 is not compared");
    a.poke(0x4EA, 0x0001_9100); // slot 7
    assert_eq!(checked(a.slot), vec![(a.slot, Hook::Vblqueue, 0x0001_9100, Via::Page0)]);
    let b = Proc::new();
    b.poke(0x4CE, 0x0001_9200); // slot 0, the classic installer's choice
    assert_eq!(checked(b.slot), vec![(b.slot, Hook::Vblqueue, 0x0001_9200, Via::Page0)]);
}
