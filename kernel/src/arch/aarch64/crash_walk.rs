//! Crash diagnosis of the debug feature `crash-walk` (TASK/PROC08): extra
//! serial lines, prefixed `[crashwalk]`, right after the `[crash]` line of a
//! program crash ([`super::crash::report_and_wait`]), and a compact block on
//! the screen crash report, after the report text and before the key prompt
//! (readable on a photo when the serial line is off). Both answer one
//! question for the fault address (and, for an EL0 crash, the page of the
//! PC): was the page mapped in the tables NewTOS keeps for the faulting slot
//! while the hardware reported the fault, or not?
//!
//! For each address four views, taken in this order before anything else
//! runs:
//!
//! 1. **hardware now** — `AT S1E0R` with the faulting root still in
//!    `TTBR0_EL1`, result from `PAR_EL1` (below);
//! 2. **roots** — `TTBR0_EL1` (table address and ASID) against the root
//!    NewTOS expects for the slot
//!    ([`l0_pa_for`](tos_api::bdos::process::mmu::l0_pa_for)) and the one
//!    the slot records
//!    ([`Process::l0_pa`](tos_api::bdos::process::Process::l0_pa));
//! 3. **tables** — a checked software walk through the slot's tables
//!    ([`walk`](tos_api::bdos::process::mmu::diag::walk): descriptor and
//!    table kind at every level, where it stops), and through the root in
//!    `TTBR0_EL1` too if that is another slot's;
//! 4. **model** — the rights model's view of the page for the slot
//!    ([`page_view`](tos_api::bdos::process::memprot::diag::page_view):
//!    block, mode, owners, memory space, the descriptor the model expects).
//!
//! The data is collected once ([`report`], the hardware first) into a
//! [`CrashDiag`]; the serial lines ([`crash_diag::write_serial`], through
//! the UART writer behind `kprintln!`, so subject to `serial=on`) and the
//! screen block ([`crash_diag::write_screen`], through the console, so
//! independent of `serial=on`) both render that one value — the tables are
//! not walked twice. Nothing allocates, every table pointer is checked
//! before it is read, and the existing report text is unchanged. Off by
//! default: with the feature off this module does not exist.
//!
//! ## The hardware probe (ARM ARM DDI 0487 M.d)
//!
//! * `AT S1E0R, <Xt>` — stage 1 translation of `Xt` "with permissions as if
//!   reading from the given virtual address from EL0", EL1&0 regime here
//!   (C5.4.5, C5-1179; encoding `op0=0b01 op1=0b000 CRn=0b0111 CRm=0b1000
//!   op2=0b010`, executable at EL1). A fault is not taken as an exception
//!   but reported in `PAR_EL1`, except a synchronous External abort on the
//!   walk (`RNHWXL`, D8-7842) — not expected for tables in DRAM.
//! * It is IMPLEMENTATION DEFINED whether the result comes from a TLB or
//!   from a table walk (`RNPZWX`, D8-7733): the probe tells what the MMU
//!   *now* answers for the address, not which of the two it used.
//! * A context synchronization event is needed before `PAR_EL1` shows the
//!   result (`IJTYVP`, D8-7734): `isb` between `AT` and `MRS`.
//! * `PAR_EL1`, 64-bit format (the A72 has no FEAT_D128; D24.2.141):
//!   bit 0 `F` = 0 success, then `PA[47:12]` in bits [47:12] (D24-9519);
//!   `F` = 1 abort, then `FST` in bits [6:1] with the Data Abort fault
//!   status encoding — `0b0001LL` translation fault level `LL`,
//!   `0b0010LL` access flag fault, `0b0011LL` permission fault (D24-9526) —,
//!   `S` bit 9 (stage 2) and `PTW` bit 8 (D24-9525). `MRS <Xt>, PAR_EL1` is
//!   `op0=0b11 op1=0b000 CRn=0b0111 CRm=0b0100 op2=0b000` (D24-9528).
//! * `TTBR0_EL1`, 64-bit format: `ASID` in bits [63:48], `BADDR` in
//!   [47:1] (D24.2.208, D24-10043).

use tos_api::bdos::process::crash_diag::{self, CrashDiag, HwProbe};
use tos_api::bios::crash::CrashReport;

/// `TTBR0_EL1` as it is now — in the crash path still the faulting root.
fn read_ttbr0() -> u64 {
    let v: u64;
    // SAFETY: a system-register read, permitted at EL1.
    unsafe { core::arch::asm!("mrs {v}, ttbr0_el1", v = out(reg) v, options(nomem, nostack, preserves_flags)) };
    v
}

/// `AT S1E0R` on `va`, then `PAR_EL1` (module docs: C5.4.5, `IJTYVP`).
fn at_s1e0r(va: u64) -> u64 {
    let par: u64;
    // SAFETY: EL1; the instruction only translates and writes PAR_EL1,
    // which nothing else in the kernel uses. A translation, access flag or
    // permission fault is reported in PAR_EL1, not raised (`RNHWXL`).
    unsafe {
        core::arch::asm!(
            "at s1e0r, {va}",
            "isb",
            "mrs {par}, par_el1",
            va = in(reg) va,
            par = out(reg) par,
            options(nostack, preserves_flags),
        );
    }
    par
}

/// Collect the diagnosis for the crash `report` of `slot` and print the
/// serial lines (module docs). Called by [`super::crash::report_and_wait`]
/// right after its `[crash]` line, before the report text and the key wait;
/// the returned value goes to [`screen`] after the report text.
pub fn report(slot: usize, report: &CrashReport<'_>) -> CrashDiag {
    // The hardware first, closest to the fault: nothing below changes
    // TTBR0_EL1 or the TLB, but the probe should see what the fault saw.
    let ttbr0 = read_ttbr0();
    let mut hw: [Option<HwProbe>; 2] = [None, None];
    for (out, a) in hw.iter_mut().zip(crash_diag::probe_addresses(report)) {
        if let Some((label, va)) = a {
            *out = Some(HwProbe { label, va, par: at_s1e0r(va) });
        }
    }
    // SAFETY: the crash path runs in the trap of the faulting program,
    // under the BDOS lock; the collection only reads.
    let diag = unsafe { crash_diag::collect(slot, report, ttbr0, hw, crate::arch::aarch64::mmu::empty_l0_pa()) };
    let _ = crash_diag::write_serial(&mut crate::hal::uart::UartWriter(&crate::console::CONSOLE), &diag);
    diag
}

/// The screen block for `diag` into the console writer `w` of the report.
pub fn screen<W: core::fmt::Write>(w: &mut W, diag: &CrashDiag) {
    let _ = crash_diag::write_screen(w, diag);
}
