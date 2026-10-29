//! TASK/BIOS15 — ADR-031 "OS hooks installed by programs" (`docs/MEMORY.md`):
//! the warning for hooks NewTOS does not call, the owner of a hook in a shared
//! cell, and the `prt_*` default.
//!
//! One test per detection point (rule 5): `Setexc`, the page-0 cells (leaving
//! the super state, a trap in super state, the end in super state), the
//! `Kbdvbase` slots at an input event, `Initmous`/`Mfpint`/`Xbtimer` and the
//! `vex_*` calls. Each checks: one warning for the first installation, none
//! for the second of the same hook by the same process, a new one for a
//! second process. Then the ownership rule (rule 3): restored when still on
//! top of the chain, kept with a warning when another hook chains on top,
//! untouched after the process unhooked itself, kept after `Ptermres`.
//!
//! The warnings are read where the kernel reads them (`hooks::pop`), after the
//! checks the kernel runs after every trap (`hooks::after_trap`).

#[path = "vdi_reference/harness.rs"]
mod harness;

use std::sync::atomic::Ordering;
use std::sync::{Mutex, MutexGuard, Once};

use tos_api::bdos::process::hooks::{self, Cell, Hook, Kind, Via, Warning};
use tos_api::bdos::process::{self, mmu, pexec, pterm, standin, superstate, Abi, PROCESS_TABLE};
use tos_api::bios::vectors::{self, setexc, ETV_CRITIC, ETV_DEFAULT_CRITIC, ETV_DEFAULT_JUST_RTS, ETV_TERM, ETV_TIMER};
use tos_api::trap::TrapArgs;
use tos_api::xbios::kbdvecs;
use tos_types::BasePage;

static LOCK: Mutex<()> = Mutex::new(());
static SETUP: Once = Once::new();

fn lock() -> MutexGuard<'static, ()> {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    SETUP.call_once(|| {
        // SAFETY: once per binary, before any process; boot drive C: (2).
        let published = unsafe { standin::init(2) };
        assert!(published.is_some(), "the stand-in page is published");
    });
    // Whatever an earlier test left queued is not this test's business.
    while hooks::pop().is_some() {}
    g
}

/// A started process with a built root, zeroed page 0 and a fresh
/// `Kbdvbase` table, the running one.
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
        // SAFETY: under the lock; the slot's pages belong to this process.
        unsafe {
            mmu::page0_bytes(slot).unwrap().fill(0);
            let private = mmu::priv_page_bytes(slot).unwrap();
            private.fill(0);
            kbdvecs::write_table(private);
        }
        p.set_state(process::State::Running);
        // A resume record, as the launcher leaves one.
        p.parent_frame.elr().store(1, Ordering::Release);
        // SAFETY: under the lock; the slot holds a basepage.
        unsafe { pexec::set_kernel_parent(slot) };
        let me = Proc { slot, bp };
        me.run();
        me
    }

    fn pd(&self) -> *mut BasePage {
        process::pd_of(self.slot)
    }

    /// Give this process the CPU.
    fn run(&self) {
        process::set_current(self.pd());
    }

    /// Write the LONG at page-0 address `addr`, as the program does in super
    /// state.
    fn poke(&self, addr: usize, v: u32) {
        // SAFETY: under the lock; the process's own page 0.
        let p0 = unsafe { mmu::page0_bytes(self.slot) }.unwrap();
        p0[addr..addr + 4].copy_from_slice(&v.to_le_bytes());
    }

    /// Write `Kbdvbase` slot `off` (a table offset), as the program does.
    fn set_kbd_slot(&self, off: u64, v: u64) {
        let page = mmu::priv_page_kva(self.slot).unwrap() as u64;
        let at = page + (kbdvecs_table_off() + off);
        // SAFETY: the process's own private page; 8-aligned slot.
        unsafe { (at as *mut u64).write_volatile(v) };
    }

    fn super_on(&self) {
        // SAFETY: under the lock; built root.
        unsafe { superstate::set_super(self.slot, true) };
    }

    fn super_off(&self) {
        // SAFETY: as above.
        unsafe { superstate::set_super(self.slot, false) };
    }

    /// `Pterm` through the real `terminate` (the slot kept until drop).
    fn pterm(&self) {
        self.run();
        // SAFETY: under the lock; `RUN` is this process.
        unsafe { pterm::terminate(0) }.expect("Pterm");
    }

    /// `Ptermres` through the real `terminate_resident`.
    fn ptermres(&self) {
        self.run();
        // SAFETY: as above.
        unsafe { pterm::terminate_resident(0x100, 0) }.expect("Ptermres");
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

/// Byte offset of the `Kbdvbase` table in the private page.
fn kbdvecs_table_off() -> u64 {
    process::loader::KBDVECS_AREA_VA - mmu::PRIV_PAGE_VA
}

/// What the kernel would print now: the after-trap checks, then the queue.
fn warnings() -> Vec<Warning> {
    hooks::after_trap();
    std::iter::from_fn(hooks::pop).collect()
}

/// The not-served warnings among `ws`, as (slot, hook, value, via).
fn not_served(ws: &[Warning]) -> Vec<(usize, Hook, u64, Via)> {
    ws.iter()
        .filter_map(|w| match w.kind {
            Kind::NotServed(h, via) => Some((w.slot, h, w.value, via)),
            _ => None,
        })
        .collect()
}

fn setexc_call(num: u64, vector: u64) -> i64 {
    setexc(&TrapArgs::kernel([num, vector, 0, 0, 0, 0, 0, 0], 0x05, 13))
}

fn xbios_call(func: u64, regs: [u64; 4]) -> i64 {
    let r = [regs[0], regs[1], regs[2], regs[3], 0, 0, 0, 0];
    tos_api::xbios::dispatch(&TrapArgs::kernel(r, func, 14))
}

// ─── Setexc ──────────────────────────────────────────────────────────────

#[test]
fn setexc_warns_once_per_unserved_hook_and_process() {
    let _g = lock();
    let a = Proc::new();
    let old_timer = setexc_call(0x100, 0x0001_2340);
    assert_eq!(old_timer, ETV_DEFAULT_JUST_RTS, "the boot value comes back");
    let ws = warnings();
    assert_eq!(not_served(&ws), vec![(a.slot, Hook::EtvTimer, 0x0001_2340, Via::Setexc)]);
    // The same hook again: no second warning. A query never warns.
    setexc_call(0x100, 0x0001_2380);
    setexc_call(0x100, u64::MAX);
    assert!(warnings().is_empty(), "once per hook and process");
    // etv_critic and a CPU vector warn; etv_term (served) and etv_xtra
    // (EmuTOS never calls it) do not; a system variable above it neither.
    setexc_call(0x101, 0x0001_3000);
    setexc_call(0x45, 0x0001_3100);
    setexc_call(0x102, 0x0001_3200);
    setexc_call(0x103, 0x0001_3300);
    setexc_call(0x108, 0x0001_3400);
    assert_eq!(
        not_served(&warnings()),
        vec![
            (a.slot, Hook::EtvCritic, 0x0001_3000, Via::Setexc),
            (a.slot, Hook::Vector(0x45), 0x0001_3100, Via::Setexc),
        ]
    );
    // Restoring a boot value is no installation.
    setexc_call(0x46, 0);
    assert!(warnings().is_empty());

    // A second process warns again for the same hook.
    let b = Proc::new();
    setexc_call(0x100, 0x0002_0000);
    assert_eq!(not_served(&warnings()), vec![(b.slot, Hook::EtvTimer, 0x0002_0000, Via::Setexc)]);

    // Clean up the shared shadow: both end, their hooks go back.
    b.pterm();
    drop(b);
    a.pterm();
    drop(a);
    assert_eq!(vectors::vector(ETV_TIMER), ETV_DEFAULT_JUST_RTS);
    assert_eq!(vectors::vector(ETV_CRITIC), ETV_DEFAULT_CRITIC);
    assert_eq!(vectors::vector(ETV_TERM), ETV_DEFAULT_JUST_RTS);
    assert_eq!(vectors::vector(0x45), 0);
    // 0x108 is a system variable, not a hook: no owner, put back by hand.
    assert_eq!(vectors::vector(0x108), 0x0001_3400);
    setexc_call(0x108, 0);
    while hooks::pop().is_some() {}
}

#[test]
fn setexc_into_page_0_checks_the_page_0_hooks() {
    let _g = lock();
    let a = Proc::new();
    // bell_hook is $5AC = Setexc(0x16B): reached through the page-0 regime.
    setexc_call(0x16B, 0x0001_4000);
    // prt_stat/prt_vec ($506/$50A): EmuTOS never calls them — no warning.
    setexc_call(0x142, 0x0001_4100);
    assert_eq!(not_served(&warnings()), vec![(a.slot, Hook::BellHook, 0x0001_4000, Via::Page0)]);
}

// ─── Page 0 (rule 4) ─────────────────────────────────────────────────────

#[test]
fn a_page_0_hook_warns_when_the_process_leaves_the_super_state() {
    let _g = lock();
    let a = Proc::new();
    a.super_on();
    a.poke(0x47E, 0x0001_5000); // hdv_mediach
    a.poke(0x070, 0x0001_5100); // VBL vector, written directly
    a.poke(0x46A, 0x0001_5200); // hdv_init: EmuTOS never calls it
    a.poke(0x506, 0x0001_5300); // prt_stat: neither
    a.super_off();
    let mut got = not_served(&warnings());
    got.sort_by_key(|w| format!("{:?}", w.1));
    assert_eq!(
        got,
        vec![
            (a.slot, Hook::HdvMediach, 0x0001_5000, Via::Page0),
            (a.slot, Hook::Vector(0x1C), 0x0001_5100, Via::Page0),
        ]
    );
    // In and out again with the hooks still there: no second warning.
    a.super_on();
    a.super_off();
    assert!(warnings().is_empty());

    // A second process warns again.
    let b = Proc::new();
    b.super_on();
    b.poke(0x47E, 0x0002_5000);
    b.super_off();
    assert_eq!(not_served(&warnings()), vec![(b.slot, Hook::HdvMediach, 0x0002_5000, Via::Page0)]);
}

#[test]
fn a_hook_restored_before_leaving_is_seen_at_a_trap_in_super_state() {
    // TeraDesk's `force_mediach`: Super, hdv_* hooked, Fopen, restored, Super.
    let _g = lock();
    let a = Proc::new();
    a.super_on();
    a.poke(0x476, 0x0001_6000); // hdv_rw
    a.poke(0x472, 0x0001_6100); // hdv_bpb
    a.poke(0x47E, 0x0001_6200); // hdv_mediach
    let at_trap = warnings(); // the Fopen trap
    a.poke(0x476, 0);
    a.poke(0x472, 0);
    a.poke(0x47E, 0);
    a.super_off();
    let mut got = not_served(&at_trap);
    got.sort_by_key(|w| w.2);
    assert_eq!(
        got,
        vec![
            (a.slot, Hook::HdvRw, 0x0001_6000, Via::Page0),
            (a.slot, Hook::HdvBpb, 0x0001_6100, Via::Page0),
            (a.slot, Hook::HdvMediach, 0x0001_6200, Via::Page0),
        ]
    );
    assert!(warnings().is_empty(), "nothing new when it leaves");
    // In user state a trap does not look at page 0 (it cannot change there).
    a.poke(0x502, 0x0001_6300);
    assert!(warnings().is_empty());
}

#[test]
fn a_process_that_ends_in_super_state_is_checked_at_its_end() {
    let _g = lock();
    let a = Proc::new();
    a.super_on();
    a.poke(0x4FE, 0x0001_7000); // exec_os
    a.pterm();
    assert_eq!(not_served(&warnings()), vec![(a.slot, Hook::ExecOs, 0x0001_7000, Via::Page0)]);
}

// ─── Kbdvbase slots (rule 5) ─────────────────────────────────────────────

#[test]
fn a_kbdvbase_hook_warns_at_the_first_event_that_would_call_it() {
    let _g = lock();
    let a = Proc::new(); // the innermost started process: the hook owner
    assert_eq!(kbdvecs::hook_owner(), Some(a.slot));
    a.set_kbd_slot(kbdvecs::OFF_MOUSEVEC, 0x0001_8000);
    a.set_kbd_slot(kbdvecs::OFF_CLOCKVEC, 0x0001_8100);
    // No event, no warning.
    assert!(warnings().is_empty());
    // A mouse packet would call mousevec, not clockvec.
    kbdvecs::saw_event(kbdvecs::EV_BYTE | kbdvecs::EV_MOUSE);
    assert_eq!(not_served(&warnings()), vec![(a.slot, Hook::Mousevec, 0x0001_8000, Via::Kbdvbase)]);
    kbdvecs::saw_event(kbdvecs::EV_BYTE | kbdvecs::EV_MOUSE);
    assert!(warnings().is_empty(), "once per hook and process");
    // ikbdsys runs for every byte on EmuTOS (the ACIA interrupt).
    a.set_kbd_slot(kbdvecs::OFF_IKBDSYS, 0x0001_8200);
    kbdvecs::saw_event(kbdvecs::EV_BYTE | kbdvecs::EV_KEY);
    assert_eq!(not_served(&warnings()), vec![(a.slot, Hook::Ikbdsys, 0x0001_8200, Via::Kbdvbase)]);
    // The owner's own kbdvec is served: no warning.
    a.set_kbd_slot(kbdvecs::OFF_KBDVEC, 0x0001_8300);
    kbdvecs::saw_event(kbdvecs::EV_BYTE | kbdvecs::EV_KEY);
    assert!(warnings().is_empty());
    drop(a);
    // The next process in the innermost place warns again.
    let b = Proc::new();
    b.set_kbd_slot(kbdvecs::OFF_MOUSEVEC, 0x0002_8000);
    kbdvecs::saw_event(kbdvecs::EV_MOUSE);
    assert_eq!(not_served(&warnings()), vec![(b.slot, Hook::Mousevec, 0x0002_8000, Via::Kbdvbase)]);
}

#[test]
fn an_accessory_kbdvec_warns_at_the_first_key() {
    let _g = lock();
    let prog = Proc::new();
    let acc = Proc::new();
    // An accessory is never the innermost started process.
    PROCESS_TABLE[acc.slot].start_seq.store(0, Ordering::Release);
    tos_api::aes::acc::register_process(acc.slot);
    acc.set_kbd_slot(kbdvecs::OFF_KBDVEC, 0x0003_8000);
    kbdvecs::saw_event(kbdvecs::EV_MOUSE);
    assert!(warnings().is_empty(), "a mouse packet calls no kbdvec");
    kbdvecs::saw_event(kbdvecs::EV_BYTE | kbdvecs::EV_KEY);
    assert_eq!(
        not_served(&warnings()),
        vec![(acc.slot, Hook::AccessoryKbdvec, 0x0003_8000, Via::Kbdvbase)]
    );
    kbdvecs::saw_event(kbdvecs::EV_KEY);
    assert!(warnings().is_empty());
    tos_api::aes::acc::forget_process(acc.slot);
    drop(acc);
    drop(prog);
}

// ─── XBIOS installers (rule 5) ───────────────────────────────────────────

#[test]
fn initmous_mfpint_xbtimer_warn_in_the_call_and_answer_as_emutos() {
    let _g = lock();
    let a = Proc::new();
    // Initmous(0) disables the mouse, type 3 is an error: no hook installed.
    assert_eq!(xbios_call(0x00, [0, 0, 0x0001_9000, 0]), 0);
    assert_eq!(xbios_call(0x00, [3, 0, 0x0001_9000, 0]), 0);
    assert!(warnings().is_empty());
    // Initmous(1, param, vec): the function number as before, one warning.
    assert_eq!(xbios_call(0x00, [1, 0x0001_9100, 0x0001_9200, 0]), 0);
    // Mfpint(13 | 0x10, vec): vector 0x40 + 13; EmuTOS' d0 = IMRA with bit
    // 13 - 8 set (0x14 | 0x20; xbios::mfp, the only user of the image here).
    assert_eq!(xbios_call(0x0D, [0x1D, 0x0001_9300, 0, 0]), 0x34);
    // Xbtimer(2 = timer C, ...): MFP 5, IMRB 0x60. Xbtimer(4): no timer,
    // nothing installed, d0 = control.
    assert_eq!(xbios_call(0x1F, [2, 0x50, 192, 0x0001_9400]), 0x60);
    assert_eq!(xbios_call(0x1F, [4, 0x50, 192, 0x0001_9500]), 0x50);
    assert_eq!(
        not_served(&warnings()),
        vec![
            (a.slot, Hook::Mousevec, 0x0001_9200, Via::Initmous),
            (a.slot, Hook::Vector(0x4D), 0x0001_9300, Via::Mfpint),
            (a.slot, Hook::Vector(0x45), 0x0001_9400, Via::Xbtimer),
        ]
    );
    // Xbtimer(0) = timer A = MFP 13: the hook Mfpint already reported.
    assert_eq!(xbios_call(0x1F, [0, 0x50, 192, 0x0001_9600]), 0x34);
    assert!(warnings().is_empty());
    let b = Proc::new();
    assert_eq!(xbios_call(0x0D, [13, 0x0002_9300, 0, 0]), 0x34);
    assert_eq!(not_served(&warnings()), vec![(b.slot, Hook::Vector(0x4D), 0x0002_9300, Via::Mfpint)]);
}

// ─── vex_* (rule 5) and their owner (rule 3) ─────────────────────────────

const VEX_TIMV: i16 = 118;
const VEX_BUTV: i16 = 125;

/// A parameter block with room for an LP64 pointer slot 1 (CONTRL[11..15]).
struct WPb {
    contrl: Box<[i16; 16]>,
    _rest: [Box<[i16; 16]>; 4],
    block: Box<[u64; 5]>,
}

impl WPb {
    fn new() -> WPb {
        let contrl = Box::new([0i16; 16]);
        let rest = [Box::new([0i16; 16]), Box::new([0; 16]), Box::new([0; 16]), Box::new([0; 16])];
        let mut block = Box::new([0u64; 5]);
        block[0] = contrl.as_ptr() as u64;
        for i in 0..4 {
            block[i + 1] = rest[i].as_ptr() as u64;
        }
        WPb { contrl, _rest: rest, block }
    }

    /// `vex_*(handle, new, &old)` at LP64: the `old` gemlib reads back.
    fn vex(&mut self, op: i16, handle: i16, new: u64) -> u64 {
        self.contrl.fill(0);
        self.contrl[0] = op;
        self.contrl[6] = handle;
        let base = self.contrl[7..].as_mut_ptr() as *mut u8;
        // SAFETY: pointer slot 0 is CONTRL[7..11], slot 1 CONTRL[11..15].
        unsafe {
            (base as *mut u64).write_unaligned(new);
            tos_api::vdi::dispatch(self.block.as_ptr() as u64, Abi::Lp64);
            (self.contrl[11..].as_ptr() as *const u64).read_unaligned()
        }
    }
}

#[test]
fn vex_vectors_warn_once_and_go_back_when_their_owner_ends() {
    let _g = lock();
    let _v = harness::VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let wk = harness::workstation();
    let mut pb = WPb::new();
    process::set_current(core::ptr::null_mut());
    // The VDI's own values, as the kernel left them (no process: no owner).
    let but0 = pb.vex(VEX_BUTV, wk.handle, 0);
    pb.vex(VEX_BUTV, wk.handle, but0);
    let tim0 = pb.vex(VEX_TIMV, wk.handle, 0);
    pb.vex(VEX_TIMV, wk.handle, tim0);
    assert!(warnings().is_empty(), "the kernel's own vectors");

    let a = Proc::new();
    assert_eq!(pb.vex(VEX_BUTV, wk.handle, 0x0001_A000), but0);
    pb.vex(VEX_BUTV, wk.handle, 0x0001_A100);
    pb.vex(VEX_TIMV, wk.handle, 0x0001_A200);
    assert_eq!(
        not_served(&warnings()),
        vec![
            (a.slot, Hook::VexButv, 0x0001_A000, Via::Vex),
            (a.slot, Hook::VexTimv, 0x0001_A200, Via::Vex),
        ]
    );
    // Its Pterm gives both back.
    a.pterm();
    drop(a);
    process::set_current(core::ptr::null_mut());
    assert_eq!(pb.vex(VEX_BUTV, wk.handle, but0), but0, "vex_butv back at the VDI's value");
    assert_eq!(pb.vex(VEX_TIMV, wk.handle, tim0), tim0, "vex_timv back at the VDI's value");
    assert!(warnings().is_empty());
}

// ─── Ownership (rule 3) ──────────────────────────────────────────────────

#[test]
fn pterm_gives_a_hook_back_only_while_it_is_on_top_of_its_chain() {
    let _g = lock();
    // On top: given back.
    let a = Proc::new();
    setexc_call(0x101, 0x0001_B000);
    a.pterm();
    drop(a);
    assert_eq!(vectors::vector(ETV_CRITIC), ETV_DEFAULT_CRITIC);
    while hooks::pop().is_some() {}

    // Not on top: B (a child, say) chained onto A's hook; A ends first.
    let a = Proc::new();
    setexc_call(0x101, 0x0001_B100);
    let b = Proc::new();
    assert_eq!(setexc_call(0x101, 0x0001_B200), 0x0001_B100);
    while hooks::pop().is_some() {}
    a.pterm();
    let ws = warnings();
    assert_eq!(ws.len(), 1);
    assert_eq!(ws[0].kind, Kind::LeftInstalled(Cell::Vector(0x101)));
    assert_eq!((ws[0].slot, ws[0].value, ws[0].other), (a.slot, 0x0001_B100, 0x0001_B200));
    assert_eq!(vectors::vector(ETV_CRITIC), 0x0001_B200, "B's hook stays");
    drop(a);
    // B ends on top of the chain: its previous value (A's) comes back.
    b.pterm();
    drop(b);
    assert_eq!(vectors::vector(ETV_CRITIC), 0x0001_B100);
    setexc_call(0x101, ETV_DEFAULT_CRITIC as u64);
    while hooks::pop().is_some() {}

    // Unhooked by the program itself: left alone, no warning.
    let a = Proc::new();
    let old = setexc_call(0x101, 0x0001_B300);
    setexc_call(0x101, old as u64);
    let c = Proc::new();
    setexc_call(0x101, 0x0001_B400); // someone else's hook now
    while hooks::pop().is_some() {}
    a.pterm();
    assert!(warnings().is_empty(), "A unhooked itself");
    assert_eq!(vectors::vector(ETV_CRITIC), 0x0001_B400);
    drop(a);
    c.pterm();
    drop(c);
    assert_eq!(vectors::vector(ETV_CRITIC), ETV_DEFAULT_CRITIC);
    while hooks::pop().is_some() {}
}

#[test]
fn after_ptermres_the_hook_stays() {
    let _g = lock();
    let tsr = Proc::new();
    setexc_call(0x100, 0x0001_C000);
    tsr.ptermres();
    drop(tsr);
    assert_eq!(vectors::vector(ETV_TIMER), 0x0001_C000, "the resident hook stays");
    // A later program chains on top and ends: the TSR's hook comes back.
    let p = Proc::new();
    setexc_call(0x100, 0x0001_C100);
    p.pterm();
    drop(p);
    assert_eq!(vectors::vector(ETV_TIMER), 0x0001_C000);
    // Leave the shadow as found.
    setexc_call(0x100, ETV_DEFAULT_JUST_RTS as u64);
    while hooks::pop().is_some() {}
}

#[test]
fn a_warning_reads_as_one_log_line() {
    let _g = lock();
    let a = Proc::new();
    setexc_call(0x100, 0x0001_D000);
    let ws = warnings();
    assert_eq!(ws.len(), 1);
    let line = ws[0].to_string();
    assert!(
        line.starts_with(&format!(
            "[hook] warning: slot {} (PD 0x{:08x}) installed etv_timer = 0x1d000 via Setexc; NewTOS never calls it:",
            a.slot,
            a.pd() as u64
        )),
        "{line}"
    );
    assert!(!line.contains('\n'));
    a.pterm();
    drop(a);
    assert_eq!(vectors::vector(ETV_TIMER), ETV_DEFAULT_JUST_RTS);
}
