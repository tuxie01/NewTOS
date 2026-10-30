//! Machine restart — EmuTOS `warm_reset`/`cold_reset` (`bios/panicasm.S:
//! 102-140`) for NewTOS (TASK/RST01).
//!
//! Spec: `docs/api/bios/_reset-warm-cold.md`. User decisions (RST01,
//! 2026-10-17): **cold** = a real restart of the Pi through the PM watchdog
//! ([`crate::drivers::bcm2711::pm`]); **warm** = the same for now (the
//! EmuTOS-faithful warm start that keeps the system variables is TASK/RST02);
//! before it, only the data cache is cleaned, as EmuTOS does — no file
//! buffers are written.
//!
//! The three entries that end here:
//! * Ctrl+Alt+Del / Ctrl+Alt+RShift+Del in the default `kbdvec`
//!   ([`crate::ikbd`], EmuTOS `kbd_int`, `bios/ikbd.c:734-747`);
//! * the OSHEADER reset entries `reseth` (+0x04) / `os_res1` (+0x10), through
//!   the page-0 reset entry and `Ssystem(S_NEWTOS_RESET)`
//!   (`tos_api::bios::reset`, `Host::system_restart`).

use tos_api::bios::reset::ResetKind;

/// Restart the machine. Returns **only** on a board that has no restart
/// mechanism (`board-qemu-virt`: no reset source has been extracted for it —
/// QEMU `virt` offers PSCI, but no PSCI specification is in `references/`,
/// so none is used), after logging that nothing happened.
///
/// `why` names the trigger for the serial log.
pub fn restart(kind: ResetKind, why: &str) {
    crate::kprintln!("[reset] {} ({:?}) requested", why, kind);
    restart_board(kind);
}

#[cfg(feature = "board-rpi4")]
fn restart_board(kind: ResetKind) {
    // `warm` = `cold` until RST02 (user decision 2); both are the full
    // restart. The data cache goes to memory first — EmuTOS `_warm_reset`
    // (`jsr _flush_data_cache`, `panicasm.S:105`) and `_cold_reset`
    // (`panicasm.S:133`). No file buffers are written (decision 3).
    let _ = kind;
    crate::kprintln!("[reset] PM watchdog full reset");
    // Give the UART time to send what it still holds of the lines above:
    // the restart cuts it off mid-character otherwise (log only).
    crate::arch::aarch64::delay_ms(10);
    // SAFETY: EL1, the reset path; nothing runs after this but the restart.
    unsafe { crate::arch::aarch64::mmu::clean_dcache_all() };
    crate::drivers::bcm2711::pm::reboot()
}

#[cfg(not(feature = "board-rpi4"))]
fn restart_board(kind: ResetKind) {
    let _ = kind;
    crate::kprintln!("[reset] no restart mechanism on this board; ignored");
}
