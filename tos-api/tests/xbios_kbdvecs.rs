//! KEYS02 — `Kbdvbase` (XBIOS 0x22), the per-process IKBD vector table and
//! the `kbdvec` hook's queue.
//!
//! Spec: `docs/api/xbios/22-kbdvbase.md` (approved 2026-10-04, D1–D8).
//! EmuTOS: `bios/aciavecs.S:126-141` (layout), `:146-223` (defaults),
//! `:407-414` (`ikbdraw` → `kbdvec`), `:372-376` (`_kbdvec` → `kbd_int`).
//!
//! What runs on the host here: the table the loader writes, the address
//! `Kbdvbase` returns, the decision "default `kbdvec` → decode directly /
//! program hook → queue for upcall" (`route_key`), the queue's order and
//! overflow rule, its drain at process exit, `Ssystem(S_NEWTOS_KBDVEC)`, the
//! give-way of a blocking AES wait and of a blocking key read when hook bytes
//! are waiting. The EL0 upcall delivery itself (trap frame, `Psigreturn`
//! chain) is kernel code and is measured in QEMU
//! (`measurements/measurement-2026-10-04-keys02/`).

use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::{Mutex, MutexGuard, Once};

use tos_api::bdos::process::loader::KBDVECS_AREA_VA;
use tos_api::bdos::process::{free_slot, sched, set_current, State, PROCESS_TABLE};
use tos_api::host::Host;
use tos_api::trap::{TrapArgs, TrapClass};
use tos_api::xbios::kbdvecs::{
    self, KBDVEC_DEFAULT_VA, OFF_DRVSTAT, OFF_IKBDSYS, OFF_KBDLENGTH, OFF_KBDVBASE, OFF_KBDVEC,
    OFF_MOUSEXVEC, RET_STUB_VA, RING_LEN, S_NEWTOS_KBDVEC,
};
use tos_types::BasePage;

// ─── rig ─────────────────────────────────────────────────────────────────

/// Process table, `RUN`, the rings and the host cells are process-wide.
static LOCK: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Bytes the host's key decoder (`Host::kbd_int`) received.
static DECODED: Mutex<Vec<u8>> = Mutex::new(Vec::new());
/// Bytes the console echoed.
static ECHOED: Mutex<Vec<u8>> = Mutex::new(Vec::new());
/// `true` = this host has no key decoder (`kbd_int` answers `false`).
static NO_DECODER: AtomicBool = AtomicBool::new(false);
/// A key byte the next `pump_input` feeds through `route_key` (0 = none) —
/// the stand-in for a USB report decoded inside an AES wait.
static PUMP_FEEDS: AtomicU16 = AtomicU16::new(0);
/// The key word the blocking reads return when they do not give way.
const KEY_WORD: u16 = 0x1E61; // 'a'

struct KbdHost;

impl Host for KbdHost {
    fn console_write_byte(&self, byte: u8) {
        ECHOED.lock().unwrap().push(byte);
    }
    fn console_read_key_blocking(&self) -> u16 {
        KEY_WORD
    }
    fn console_read_key_restartable(&self) -> Option<u16> {
        // The kernel host's rule: give way while hook bytes wait.
        if kbdvecs::pending_for_running() {
            None
        } else {
            Some(KEY_WORD)
        }
    }
    fn pump_input(&self) {
        let b = PUMP_FEEDS.swap(0, Ordering::AcqRel);
        if b != 0 {
            let routed = kbdvecs::route_key(b as u8);
            assert!(routed, "the rig only feeds while a hook is installed");
        }
    }
    fn kbd_int(&self, scancode: u8) -> bool {
        if NO_DECODER.load(Ordering::Acquire) {
            return false;
        }
        DECODED.lock().unwrap().push(scancode);
        true
    }
}

static HOST: Once = Once::new();

fn rig() {
    HOST.call_once(|| {
        let slot: &'static &'static dyn Host = Box::leak(Box::new(&KbdHost as &'static dyn Host));
        tos_api::host::install_host(slot);
    });
    DECODED.lock().unwrap().clear();
    ECHOED.lock().unwrap().clear();
    NO_DECODER.store(false, Ordering::Release);
    PUMP_FEEDS.store(0, Ordering::Release);
    kbdvecs::take_restart();
}

/// A process in `SLOT` as the loader leaves it (only the KEYS02 part): its
/// basepage (here a box of its own, which is all `slot_for` needs) and its
/// page 0 — since ARCH03 stage 1 the slot's own kernel page
/// (`process::mmu::page0_kva`), separate from the basepage.
struct Page0(Box<[u64; 32]>);

impl Page0 {
    fn loaded() -> Page0 {
        kbdvecs::write_defaults(page0());
        let p = Page0(Box::new([0; 32]));
        // The standard handles `init_pd_files` gives every loaded PD (here the
        // kernel's `default_handle`): the console calls read their device from
        // `p_uft[]` (TASK/PROC04 point 1), so stdin must be CON.
        // SAFETY: the box is 256 bytes, a whole basepage.
        unsafe { (*p.kva()).p_uft = tos_api::bdos::process::standin::DEFAULT_HANDLE };
        p
    }
    fn bytes(&self) -> &[u8] {
        page0()
    }
    fn slot_value(&self, off: u64) -> u64 {
        let at = (KBDVECS_AREA_VA + off) as usize;
        u64::from_le_bytes(self.bytes()[at..at + 8].try_into().unwrap())
    }
    /// What a program does: `((void **)Kbdvbase())[-1] = hook`.
    fn set_slot(&mut self, off: u64, v: u64) {
        let at = (KBDVECS_AREA_VA + off) as usize;
        page0()[at..at + 8].copy_from_slice(&v.to_le_bytes());
    }
    fn kva(&self) -> *mut BasePage {
        self.0.as_ptr() as *mut BasePage
    }
}

const SLOT: usize = 6;

/// `SLOT`'s page 0.
fn page0() -> &'static mut [u8; 4096] {
    // SAFETY: the tests run one at a time under `lock()`; nothing else in this
    // binary touches `SLOT`'s page 0.
    unsafe { tos_api::bdos::process::mmu::page0_bytes(SLOT) }.expect("page 0")
}

/// Make `page` the running process in `SLOT` (the identity `slot_for` uses is
/// `basepage_kva`; the kernel reads the vector table from the slot's page 0).
fn run(page: &Page0) {
    for (i, p) in PROCESS_TABLE.iter().enumerate() {
        sched::unready(i);
        p.wait_cond.store(0, Ordering::Release);
        p.set_state(State::Free);
        p.basepage_kva.store(core::ptr::null_mut(), Ordering::Release);
    }
    sched::set_kernel_parked(false);
    kbdvecs::release_slot(SLOT);
    PROCESS_TABLE[SLOT].set_state(State::Running);
    PROCESS_TABLE[SLOT].basepage_kva.store(page.kva(), Ordering::Release);
    set_current(page.kva());
}

fn no_process() {
    set_current(core::ptr::null_mut());
}

fn drain(slot: usize) -> Vec<u8> {
    std::iter::from_fn(|| kbdvecs::pop(slot)).collect()
}

fn xbios(func: u16) -> i64 {
    tos_api::xbios::dispatch(&TrapArgs {
        regs: [0; 8],
        func: func as u64,
        svc_imm: TrapClass::Xbios as u16,
        space: tos_api::uaccess::Space::Kernel,
    })
}

fn gemdos(func: u16, a0: u64, a1: u64, a2: u64) -> i32 {
    tos_api::gemdos::dispatch(&TrapArgs {
        regs: [a0, a1, a2, 0, 0, 0, 0, 0],
        func: func as u64,
        svc_imm: TrapClass::Gemdos as u16,
        space: tos_api::uaccess::Space::Kernel,
    })
}

fn bios(func: u16, a0: u64) -> i32 {
    tos_api::bios::dispatch(&TrapArgs {
        regs: [a0, 0, 0, 0, 0, 0, 0, 0],
        func: func as u64,
        svc_imm: TrapClass::Bios as u16,
        space: tos_api::uaccess::Space::Kernel,
    }) as i32
}

const KBDVBASE: u16 = 0x22;
const SSYSTEM: u16 = 0x154;
const E_OK: i32 = 0;
const ERROR: i32 = -1;
const EINVAL: i32 = -25;

/// A program's own `kbdvec` routine (any EL0 VA that is not the default).
const HOOK: u64 = 0x0001_2340;

// IKBD scancodes (bit 7 = break).
const UP: u8 = 0x48;
const RIGHT: u8 = 0x4D;
const BRK: u8 = 0x80;

// ─── the table ───────────────────────────────────────────────────────────

#[test]
fn loader_writes_the_table_and_the_two_stubs() {
    let _g = lock();
    let p = Page0::loaded();
    // B1 layout: mousexvec, kbdvec, then the nine `struct kbdvecs` slots.
    assert_eq!(OFF_KBDVEC, OFF_KBDVBASE - 8, "kbdvec sits right before Kbdvbase()");
    assert_eq!(OFF_MOUSEXVEC, OFF_KBDVEC - 8, "mousexvec right before kbdvec");
    assert_eq!(p.slot_value(OFF_KBDVEC), KBDVEC_DEFAULT_VA);
    assert_eq!(p.slot_value(OFF_MOUSEXVEC), RET_STUB_VA);
    for i in 0..9 {
        assert_eq!(p.slot_value(OFF_KBDVBASE + 8 * i), RET_STUB_VA, "slot {i} is just_rts");
    }
    assert_eq!(OFF_IKBDSYS, OFF_KBDVBASE + 0x40);
    // D5: drvstat and kbdlength are 0.
    let b = p.bytes();
    assert_eq!(b[(KBDVECS_AREA_VA + OFF_DRVSTAT) as usize], 0);
    assert_eq!(b[(KBDVECS_AREA_VA + OFF_KBDLENGTH) as usize], 0);
    // The stubs: `ret`, and mov x1,x0 / mov x0,#2002 / mov x2,#0 /
    // mov x8,#0x154 / svc #1 / ret.
    let word = |va: u64| u32::from_le_bytes(b[va as usize..va as usize + 4].try_into().unwrap());
    assert_eq!(word(RET_STUB_VA), 0xD65F_03C0);
    let stub: Vec<u32> = (0..6).map(|i| word(KBDVEC_DEFAULT_VA + 4 * i)).collect();
    assert_eq!(
        stub,
        [0xAA00_03E1, 0xD280_FA40, 0xD280_0002, 0xD280_2A88, 0xD400_0021, 0xD65F_03C0]
    );
    assert_eq!(S_NEWTOS_KBDVEC, 2002);
    // Neither stub address is 0 — every slot is a valid chain target (B8).
    assert_ne!(RET_STUB_VA, 0);
    assert_ne!(KBDVEC_DEFAULT_VA, 0);
}

#[test]
fn kbdvbase_is_the_processes_own_el0_table() {
    let _g = lock();
    rig();
    let p = Page0::loaded();
    run(&p);
    let a = xbios(KBDVBASE);
    assert_eq!(a as u64, KBDVECS_AREA_VA + OFF_KBDVBASE, "EL0 VA of midivec");
    assert_eq!(xbios(KBDVBASE), a, "stable");
    // With no process (desktop, boot) a kernel table answers, same layout.
    no_process();
    let k = xbios(KBDVBASE) as u64;
    assert_ne!(k, 0);
    // SAFETY: `k - 8 .. k + 0x48` is the kernel table.
    unsafe {
        assert_eq!(((k - 8) as *const u64).read(), KBDVEC_DEFAULT_VA);
        for i in 0..9 {
            assert_eq!(((k + 8 * i) as *const u64).read(), RET_STUB_VA);
        }
    }
}

// ─── default vs. hooked ──────────────────────────────────────────────────

#[test]
fn default_kbdvec_and_no_process_decode_directly() {
    let _g = lock();
    rig();
    let p = Page0::loaded();
    run(&p);
    // Default still in place: the decoder runs as before (fast path).
    assert!(!kbdvecs::route_key(UP));
    assert!(!kbdvecs::route_key(UP | BRK));
    assert_eq!(kbdvecs::pending(SLOT), 0);
    // No process: same.
    no_process();
    assert!(!kbdvecs::route_key(UP));
    assert!(!kbdvecs::pending_for_running());
}

#[test]
fn hooked_kbdvec_gets_make_and_break_in_order() {
    let _g = lock();
    rig();
    let mut p = Page0::loaded();
    p.set_slot(OFF_KBDVEC, HOOK);
    run(&p);
    // Four bytes in one poll — the case `post_upcall` (last writer wins)
    // would have lost: up make, right make, right break, up break.
    for b in [UP, RIGHT, RIGHT | BRK, UP | BRK] {
        assert!(kbdvecs::route_key(b), "{b:#x} belongs to the hook");
    }
    assert!(kbdvecs::pending_for_running());
    assert_eq!(kbdvecs::kbdvec_of(SLOT), Some(HOOK), "the upcall target");
    assert_eq!(drain(SLOT), vec![UP, RIGHT, RIGHT | BRK, UP | BRK]);
    assert!(DECODED.lock().unwrap().is_empty(), "nothing reached the default");
    // Restoring the default (what a program does at exit) ends the routing.
    p.set_slot(OFF_KBDVEC, KBDVEC_DEFAULT_VA);
    assert!(!kbdvecs::route_key(UP));
}

#[test]
fn full_ring_drops_the_newest_byte() {
    let _g = lock();
    rig();
    let mut p = Page0::loaded();
    p.set_slot(OFF_KBDVEC, HOOK);
    run(&p);
    for i in 0..RING_LEN + 6 {
        kbdvecs::route_key((i % 0x70) as u8);
    }
    assert_eq!(kbdvecs::pending(SLOT), RING_LEN);
    let got = drain(SLOT);
    let want: Vec<u8> = (0..RING_LEN).map(|i| (i % 0x70) as u8).collect();
    assert_eq!(got, want, "the oldest 64 survive, in order (D3a)");
    // After draining, the ring accepts again and wraps correctly.
    for b in [UP, UP | BRK] {
        kbdvecs::route_key(b);
    }
    assert_eq!(drain(SLOT), vec![UP, UP | BRK]);
}

#[test]
fn process_exit_drains_the_ring() {
    let _g = lock();
    rig();
    let mut p = Page0::loaded();
    p.set_slot(OFF_KBDVEC, HOOK);
    run(&p);
    kbdvecs::route_key(UP);
    kbdvecs::route_key(UP | BRK);
    assert_eq!(kbdvecs::pending(SLOT), 2);
    no_process();
    free_slot(SLOT); // Pterm's slot release (D8)
    assert_eq!(kbdvecs::pending(SLOT), 0);
    assert_eq!(kbdvecs::pop(SLOT), None);
    // The next process in that slot starts with the defaults and routes
    // nothing to a hook.
    let q = Page0::loaded();
    run(&q);
    assert!(!kbdvecs::route_key(UP));
}

#[test]
fn ikbdsys_value_for_the_repeat_stop() {
    let _g = lock();
    rig();
    no_process();
    assert_eq!(kbdvecs::current_ikbdsys(), RET_STUB_VA, "no process: the default");
    let mut p = Page0::loaded();
    run(&p);
    assert_eq!(kbdvecs::current_ikbdsys(), RET_STUB_VA, "a fresh process: the same default");
    p.set_slot(OFF_IKBDSYS, 0x0004_5670);
    assert_eq!(kbdvecs::current_ikbdsys(), 0x0004_5670, "a program's own ikbdsys");
    no_process();
}

// ─── the default `kbdvec` entry: Ssystem(2002) ───────────────────────────

#[test]
fn ssystem_kbdvec_runs_the_default_decoder() {
    let _g = lock();
    rig();
    let p = Page0::loaded();
    run(&p);
    let m = u64::from(S_NEWTOS_KBDVEC);
    assert_eq!(gemdos(SSYSTEM, m, u64::from(UP), 0), E_OK);
    assert_eq!(gemdos(SSYSTEM, m, u64::from(UP | BRK), 0), E_OK);
    assert_eq!(*DECODED.lock().unwrap(), vec![UP, UP | BRK]);
    // Not a byte, or reserved arg2 set: refused, nothing decoded.
    assert_eq!(gemdos(SSYSTEM, m, 0x148, 0), EINVAL);
    assert_eq!(gemdos(SSYSTEM, m, u64::from(UP), 1), EINVAL);
    assert_eq!(DECODED.lock().unwrap().len(), 2);
    // A host without a key decoder says so.
    NO_DECODER.store(true, Ordering::Release);
    assert_eq!(gemdos(SSYSTEM, m, u64::from(UP), 0), ERROR);
    no_process();
}

// ─── waits give way to the hook (O1) ─────────────────────────────────────

#[test]
fn aes_wait_gives_way_when_hook_bytes_are_queued() {
    let _g = lock();
    rig();
    let mut p = Page0::loaded();
    p.set_slot(OFF_KBDVEC, HOOK);
    run(&p);

    // (a) Bytes already queued when the wait starts: give way at once, the
    // slot never goes WAITING.
    kbdvecs::route_key(RIGHT);
    match sched::sleep(sched::wait_input()) {
        sched::SleepOutcome::Yield(sw) => {
            assert!(sw.upcall_restart, "a restart for the hook, not a peer switch");
            assert_eq!(sw.slot, SLOT);
        }
        sched::SleepOutcome::Recheck => panic!("queued hook bytes must end the wait"),
    }
    assert_eq!(PROCESS_TABLE[SLOT].wait_cond.load(Ordering::Acquire), 0);
    drain(SLOT);

    // (b) The byte arrives during the wait's own input pump: give way right
    // after it, the slot READY again.
    PUMP_FEEDS.store(u16::from(RIGHT | BRK), Ordering::Release);
    match sched::sleep(sched::wait_input()) {
        sched::SleepOutcome::Yield(sw) => assert!(sw.upcall_restart),
        sched::SleepOutcome::Recheck => panic!("a hook byte from the pump must end the wait"),
    }
    assert_eq!(PROCESS_TABLE[SLOT].wait_cond.load(Ordering::Acquire), 0);
    assert!(PROCESS_TABLE[SLOT].runnable.load(Ordering::Acquire));
    assert_eq!(drain(SLOT), vec![RIGHT | BRK]);

    // (c) Nothing for the hook: an ordinary idle pass.
    assert!(matches!(sched::sleep(sched::wait_input()), sched::SleepOutcome::Recheck));
    no_process();
}

#[test]
fn single_key_reads_give_way_without_side_effects() {
    let _g = lock();
    rig();
    let mut p = Page0::loaded();
    p.set_slot(OFF_KBDVEC, HOOK);
    run(&p);
    const CCONIN: u16 = 0x01;
    const CRAWCIN: u16 = 0x07;
    const CNECIN: u16 = 0x08;
    const BCONIN: u16 = 0x02;

    // Hook byte waiting: each read asks for a restart and does nothing else
    // (no echo, no key consumed, no Ctrl-C bookkeeping).
    kbdvecs::route_key(UP);
    for f in [CCONIN, CRAWCIN, CNECIN] {
        assert_eq!(gemdos(f, 0, 0, 0), 0);
        assert!(kbdvecs::take_restart(), "GEMDOS {f:#x} must request the restart");
    }
    assert_eq!(bios(BCONIN, 2), 0);
    assert!(kbdvecs::take_restart(), "Bconin(2) must request the restart");
    assert!(ECHOED.lock().unwrap().is_empty(), "Cconin echoed nothing");
    assert_eq!(kbdvecs::take_restart(), false);

    // After the hook ran (ring empty) the read no longer gives way.
    drain(SLOT);
    assert!(!kbdvecs::pending_for_running());
    // Completing it with a process running would take the `conterm` fold's
    // unprivileged load of the EL0 `$484`, which a host test cannot map; the
    // completed read itself is the pre-KEYS02 path, shown here without RUN.
    no_process();
    assert_eq!(gemdos(CCONIN, 0, 0, 0), 0x001E_0061);
    assert!(!kbdvecs::take_restart());
    assert_eq!(*ECHOED.lock().unwrap(), vec![0x61]);
    assert_eq!(bios(BCONIN, 2), 0x001E_0061);
    assert!(!kbdvecs::take_restart());
}
