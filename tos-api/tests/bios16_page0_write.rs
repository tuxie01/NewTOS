//! TASK/BIOS16 — a vector cell written straight into page 0 becomes the
//! shared cell (option B, user decision 2026-10-30; `docs/api/bios/05-setexc.md`
//! "Page 0 written directly"). EmuTOS has one vector table, so a direct write
//! IS the vector and `Setexc(n, -1)` answers it
//! (`references/emutos/bios/bios.c:1070-1080`).
//!
//! * `$008`–`$407` (vectors 2…0xFF, `etv_timer`, `etv_critic`), written by a
//!   process in super state, are taken over before its next call is served
//!   (`hooks::before_trap`) and after it (`hooks::after_trap`), when it leaves
//!   the super state and when it ends in it — every page-0 copy gets the value;
//! * the process owns the value like a `Setexc` write (ADR-031 rule 3): its
//!   `Pterm` puts the previous value back while the cell still holds it,
//!   `Ptermres` keeps it, a hook chained on top keeps it with a warning;
//! * an unserved hook warns once per process, via page 0; nothing is called;
//! * `$408` (`etv_term`, served) is not taken over: warning, `Setexc` keeps
//!   the value set through it; `$000`/`$004` and `etv_xtra` (`$40C`) neither.

use std::sync::atomic::Ordering;
use std::sync::{Mutex, MutexGuard, Once};

use tos_api::bdos::process::hooks::{self, Cell, Hook, Kind, Via, Warning};
use tos_api::bdos::process::{self, mmu, pexec, pterm, superstate, PROCESS_TABLE};
use tos_api::bios::vectors::{self, setexc, ETV_CRITIC, ETV_TERM, ETV_TIMER, ETV_XTRA, JUST_RTS};
use tos_api::trap::TrapArgs;
use tos_types::BasePage;

static LOCK: Mutex<()> = Mutex::new(());
static SETUP: Once = Once::new();

fn lock() -> MutexGuard<'static, ()> {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    SETUP.call_once(|| {
        // SAFETY: once per binary, before any process; boot drive C: (2).
        let published = unsafe { process::standin::init(2) };
        assert!(published.is_some(), "the stand-in page is published");
    });
    while hooks::pop().is_some() {}
    g
}

/// A loaded, running process whose page 0 holds what the loader puts at the
/// vector cells and the page-0 hook cells, and zeros elsewhere.
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
            vectors::write_page0_vectors(page0);
            // The page-0 hook cells hold the loader's values (TASK/BIOS17:
            // `swv_vec` is the reset entry, not 0).
            for h in hooks::PAGE0_HOOKS.iter() {
                let at = h.addr as usize;
                page0[at..at + 4].copy_from_slice(&h.default.to_le_bytes());
            }
        }
        p.set_state(process::State::Running);
        p.parent_frame.elr().store(1, Ordering::Release);
        // SAFETY: under the lock; the slot holds a basepage.
        unsafe { pexec::set_kernel_parent(slot) };
        let me = Proc { slot, bp };
        me.run();
        me
    }

    fn run(&self) {
        process::set_current(process::pd_of(self.slot));
    }

    fn peek(&self, addr: usize) -> u32 {
        // SAFETY: under the lock; the process's own page 0.
        let p0 = unsafe { mmu::page0_bytes(self.slot) }.unwrap();
        u32::from_le_bytes(p0[addr..addr + 4].try_into().unwrap())
    }

    /// Write the LONG at page-0 address `addr`, as the program does in super
    /// state.
    fn poke(&self, addr: usize, v: u32) {
        // SAFETY: under the lock; the process's own page 0.
        let p0 = unsafe { mmu::page0_bytes(self.slot) }.unwrap();
        p0[addr..addr + 4].copy_from_slice(&v.to_le_bytes());
    }

    fn super_on(&self) {
        // SAFETY: under the lock; built root.
        unsafe { superstate::set_super(self.slot, true) };
    }

    fn super_off(&self) {
        // SAFETY: as above.
        unsafe { superstate::set_super(self.slot, false) };
    }

    fn pterm(&self) {
        self.run();
        // SAFETY: under the lock; `RUN` is this process.
        unsafe { pterm::terminate(0) }.expect("Pterm");
    }

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

fn setexc_call(num: u64, vector: u64) -> i64 {
    setexc(&TrapArgs::kernel([num, vector, 0, 0, 0, 0, 0, 0], 0x05, 13))
}

fn query(num: u64) -> i64 {
    setexc_call(num, u64::MAX)
}

/// The queued warnings, as the kernel prints them.
fn drain() -> Vec<Warning> {
    std::iter::from_fn(hooks::pop).collect()
}

fn not_served(ws: &[Warning]) -> Vec<(usize, Hook, u64, Via)> {
    ws.iter()
        .filter_map(|w| match w.kind {
            Kind::NotServed(h, via) => Some((w.slot, h, w.value, via)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_direct_write_is_what_the_next_setexc_answers() {
    let _g = lock();
    let a = Proc::new();
    let b = Proc::new(); // a parent's page 0, say
    a.run();
    a.super_on();
    a.poke(0x080, 0x0001_2000); // trap #0 = vector 0x20
    // The next system call: taken over before it is served.
    hooks::before_trap();
    assert_eq!(query(0x20), 0x0001_2000);
    assert_eq!(b.peek(0x080), 0x0001_2000, "every page 0 carries the cell");
    assert_eq!(not_served(&drain()), vec![(a.slot, Hook::Vector(0x20), 0x0001_2000, Via::Page0)]);
    // A second direct write: the cell follows, no second warning.
    a.poke(0x080, 0x0001_2100);
    hooks::before_trap();
    assert_eq!(query(0x20), 0x0001_2100);
    assert!(drain().is_empty(), "once per hook and process");
    // A Setexc write after a direct one answers the direct value as old.
    a.poke(0x080, 0x0001_2200);
    hooks::before_trap();
    assert_eq!(setexc_call(0x20, 0x0001_2300), 0x0001_2200);
    assert_eq!(a.peek(0x080), 0x0001_2300);
    // Pterm: the value before the first write comes back.
    a.pterm();
    drop(a);
    assert_eq!(vectors::vector(0x20), JUST_RTS);
    assert_eq!(b.peek(0x080), JUST_RTS as u32);
    drop(b);
    drain();
}

#[test]
fn the_range_is_008_to_407_and_etv_term_stays_the_setexc_value() {
    let _g = lock();
    let a = Proc::new();
    a.super_on();
    a.poke(0x000, 0x0001_3000); // vector 0: no hook, not checked
    a.poke(0x004, 0x0001_3004); // vector 1: likewise
    a.poke(0x008, 0x0001_3008); // vector 2, the first cell taken over
    a.poke(0x3FC, 0x0001_33FC); // vector 0xFF
    a.poke(0x400, 0x0001_3400); // etv_timer
    a.poke(0x404, 0x0001_3404); // etv_critic, the last
    a.poke(0x408, 0x0001_3408); // etv_term: not taken over
    a.poke(0x40C, 0x0001_340C); // etv_xtra: outside the check
    hooks::before_trap();
    assert_eq!(query(2), 0x0001_3008);
    assert_eq!(query(0xFF), 0x0001_33FC);
    assert_eq!(query(ETV_TIMER as u64), 0x0001_3400);
    assert_eq!(query(ETV_CRITIC as u64), 0x0001_3404);
    assert_eq!(query(ETV_TERM as u64), JUST_RTS, "etv_term: the value set through Setexc");
    assert_eq!(query(ETV_XTRA as u64), 0);
    assert_ne!(query(0), 0x0001_3000);
    assert_ne!(query(1), 0x0001_3004);
    let mut got = not_served(&drain());
    got.sort_by_key(|w| w.2);
    assert_eq!(
        got,
        vec![
            (a.slot, Hook::Vector(2), 0x0001_3008, Via::Page0),
            (a.slot, Hook::Vector(0xFF), 0x0001_33FC, Via::Page0),
            (a.slot, Hook::EtvTimer, 0x0001_3400, Via::Page0),
            (a.slot, Hook::EtvCritic, 0x0001_3404, Via::Page0),
            (a.slot, Hook::EtvTermDirect, 0x0001_3408, Via::Page0),
        ]
    );
    // etv_term keeps warning once; the copy keeps its value until Setexc.
    hooks::before_trap();
    assert!(drain().is_empty());
    assert_eq!(a.peek(0x408), 0x0001_3408);
    // Put the cells the test wrote but no one owns back by hand.
    a.poke(0x000, vectors::vector(0) as u32);
    a.poke(0x004, vectors::vector(1) as u32);
    a.poke(0x408, JUST_RTS as u32);
    a.poke(0x40C, 0);
    a.pterm();
    drop(a);
    for n in [2, 0xFF, ETV_TIMER, ETV_CRITIC, ETV_TERM] {
        assert_eq!(vectors::vector(n), JUST_RTS, "cell 0x{:x} given back", n);
    }
    drain();
}

#[test]
fn every_edge_of_a_call_takes_the_write_over() {
    let _g = lock();
    let a = Proc::new();
    // After a call (a Supexec callback returns through Psigreturn).
    a.super_on();
    a.poke(0x084, 0x0001_4000); // trap #1 = vector 0x21
    hooks::after_trap();
    assert_eq!(vectors::vector(0x21), 0x0001_4000);
    // Leaving the super state.
    a.poke(0x088, 0x0001_4100); // vector 0x22
    a.super_off();
    assert_eq!(vectors::vector(0x22), 0x0001_4100);
    // In user state a trap does not look at page 0 (EL0 cannot write it).
    a.poke(0x08C, 0x0001_4200); // vector 0x23, the test writes it anyway
    hooks::before_trap();
    hooks::after_trap();
    assert_eq!(vectors::vector(0x23), JUST_RTS);
    a.poke(0x08C, JUST_RTS as u32);
    // The end in super state.
    a.super_on();
    a.poke(0x090, 0x0001_4300); // vector 0x24
    a.pterm();
    drop(a);
    for n in [0x21, 0x22, 0x24] {
        assert_eq!(vectors::vector(n), JUST_RTS, "cell 0x{:x} given back at Pterm", n);
    }
    drain();
}

#[test]
fn ownership_follows_the_setexc_rules() {
    let _g = lock();
    // Ptermres keeps the value.
    let tsr = Proc::new();
    tsr.super_on();
    tsr.poke(0x400, 0x0001_5000);
    tsr.ptermres();
    drop(tsr);
    assert_eq!(vectors::vector(ETV_TIMER), 0x0001_5000, "the resident hook stays");
    setexc_call(ETV_TIMER as u64, JUST_RTS as u64);
    drain();

    // A hook chained on top (XBRA): A's value stays, with a warning.
    let a = Proc::new();
    setexc_call(0x101, 0x0001_5100);
    let b = Proc::new();
    b.super_on();
    b.poke(0x404, 0x0001_5200);
    hooks::before_trap();
    drain();
    a.pterm();
    let ws = drain();
    assert_eq!(ws.len(), 1);
    assert_eq!(ws[0].kind, Kind::LeftInstalled(Cell::Vector(0x101)));
    assert_eq!((ws[0].slot, ws[0].value, ws[0].other), (a.slot, 0x0001_5100, 0x0001_5200));
    drop(a);
    // B ends on top: its previous value, A's, comes back.
    b.pterm();
    drop(b);
    assert_eq!(vectors::vector(ETV_CRITIC), 0x0001_5100);
    setexc_call(ETV_CRITIC as u64, JUST_RTS as u64);
    drain();

    // Unhooked by a direct write of the old value: no warning, nothing to do.
    let a = Proc::new();
    a.super_on();
    a.poke(0x094, 0x0001_5300); // vector 0x25
    hooks::before_trap();
    a.poke(0x094, JUST_RTS as u32);
    hooks::before_trap();
    let c = Proc::new();
    setexc_call(0x25, 0x0001_5400); // someone else's hook now
    drain();
    a.pterm();
    assert!(drain().is_empty(), "A unhooked itself");
    assert_eq!(vectors::vector(0x25), 0x0001_5400);
    drop(a);
    c.pterm();
    drop(c);
    assert_eq!(vectors::vector(0x25), JUST_RTS);
    drain();
}
