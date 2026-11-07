//! TASK/V34 — `Scrdmp` (XBIOS 0x14) re-arms `dumpflg` and `v_hardcopy` (VDI
//! escape 17) runs `Scrdmp` (VDI-TEXT-011, VDI-TEXT-012,
//! `docs/audits/2026-11-06-v18-t2-vdi-text.md`).
//!
//! EmuTOS: `scrdmp` calls `dump_vec` and then sets `dumpflg = -1`
//! (`references/emutos/bios/xbios.c:490-494`); `escfn17` is nothing but
//! `Scrdmp()` (`references/emutos/vdi/vdi_esc.c:260-263`). Measured by probe
//! T18A (`tosdump/probes/vdprobe.c`): with `dumpflg` ($4EE) poked to 5 in super
//! state, the oracle reads -1 after `v_hardcopy` and after `Scrdmp`.
//! `dump_vec` itself is not called on NewTOS (ADR-031, `docs/MEMORY.md`).
//!
//! `dumpflg` lives in the calling process's page 0 (the loader seeds it with
//! -1, `bios/bios.c:357`); the tests read it there, as the program does.

#[path = "vdi_reference/harness.rs"]
mod harness;

use std::sync::atomic::Ordering;
use std::sync::{Mutex, MutexGuard, Once};

use tos_api::bdos::process::{self, mmu, pexec, standin, PROCESS_TABLE};
use tos_api::trap::TrapArgs;
use tos_types::BasePage;

static LOCK: Mutex<()> = Mutex::new(());
static SETUP: Once = Once::new();

/// `dumpflg`, a WORD at $4EE (`references/emutos/tosvars.ld:87`).
const DUMPFLG: usize = 0x4EE;

fn lock() -> MutexGuard<'static, ()> {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    SETUP.call_once(|| {
        // SAFETY: once per binary, before any process; boot drive C: (2).
        let published = unsafe { standin::init(2) };
        assert!(published.is_some(), "the stand-in page is published");
    });
    g
}

/// A started process with a zeroed page 0, the running one.
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
        unsafe { mmu::page0_bytes(slot) }.unwrap().fill(0);
        p.set_state(process::State::Running);
        p.parent_frame.elr().store(1, Ordering::Release);
        // SAFETY: under the lock; the slot holds a basepage.
        unsafe { pexec::set_kernel_parent(slot) };
        process::set_current(process::pd_of(slot));
        Proc { slot, bp }
    }

    /// The WORD at page-0 address `addr`, as the program reads it.
    fn word(&self, addr: usize) -> u16 {
        // SAFETY: under the lock; the process's own page 0.
        let p0 = unsafe { mmu::page0_bytes(self.slot) }.unwrap();
        u16::from_le_bytes([p0[addr], p0[addr + 1]])
    }

    /// Write the WORD at page-0 address `addr`, as the program does in super
    /// state (probe T18A: `*(int16_t *)0x4EE = 5` under `Supexec`).
    fn poke_word(&self, addr: usize, v: u16) {
        // SAFETY: as above.
        let p0 = unsafe { mmu::page0_bytes(self.slot) }.unwrap();
        p0[addr..addr + 2].copy_from_slice(&v.to_le_bytes());
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

fn scrdmp() -> i64 {
    tos_api::xbios::dispatch(&TrapArgs::kernel([0; 8], 0x14, 14))
}

#[test]
fn scrdmp_sets_dumpflg_to_minus_one() {
    let _g = lock();
    let a = Proc::new();
    // The neighbours stay as they are: only the WORD at $4EE is written.
    a.poke_word(DUMPFLG - 2, 0x1234);
    a.poke_word(DUMPFLG + 2, 0x5678);
    a.poke_word(DUMPFLG, 5);
    assert_eq!(scrdmp(), 0, "void");
    assert_eq!(a.word(DUMPFLG), 0xFFFF, "dumpflg = -1 (bios/xbios.c:493)");
    assert_eq!(a.word(DUMPFLG - 2), 0x1234);
    assert_eq!(a.word(DUMPFLG + 2), 0x5678);
    // 0 is the value at which EmuTOS' VBL would dump (bios/vectors.S:467-472);
    // Scrdmp re-arms it as well.
    a.poke_word(DUMPFLG, 0);
    scrdmp();
    assert_eq!(a.word(DUMPFLG), 0xFFFF);
}

#[test]
fn scrdmp_without_a_process_returns_void() {
    let _g = lock();
    process::set_current(core::ptr::null_mut());
    assert_eq!(scrdmp(), 0);
}

#[test]
fn v_hardcopy_runs_scrdmp() {
    let _g = lock();
    let _v = harness::VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let wk = harness::workstation();
    let a = Proc::new();
    a.poke_word(DUMPFLG, 5);
    let mut pb = harness::Pb::new();
    pb.contrl[0] = 5; // v_escape
    pb.contrl[1] = 0;
    pb.contrl[3] = 0;
    pb.contrl[5] = 17; // v_hardcopy
    pb.contrl[6] = wk.handle;
    pb.call();
    assert_eq!(a.word(DUMPFLG), 0xFFFF, "escfn17 -> Scrdmp (vdi/vdi_esc.c:262)");
}
