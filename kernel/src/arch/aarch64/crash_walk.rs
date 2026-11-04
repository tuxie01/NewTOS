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
//! runs (then the ladder, below):
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
//!
//! ## The ladder (TASK/PROC08 "Stand Leiter")
//!
//! After the collection — the walk only reads, the ladder changes the
//! TLB — and only when the first `AT` on the faulting address failed,
//! [`report`] runs the rungs of [`crash_diag::RUNGS`] through
//! [`crash_diag::run_ladder`]; each is followed by `AT S1E0R` on that
//! address (`at; isb; mrs par_el1`, above) and the ladder stops at the
//! first success. The decision and the lines are tos-api's; the
//! instructions are here ([`Ladder`]). Barriers, ARM ARM DDI 0487 M.d:
//!
//! * L1 (`AT` again) and L2 (`AT` on the page of `SP_EL0` and on the
//!   private page `PRIV_PAGE_VA`, then on the address): no maintenance.
//! * L3 `dsb ish; isb` — the barrier part of L5–L7 alone, as the control.
//! * L4 `dc civac, <line>` for every descriptor the walk(s) read (kernel
//!   address of the entry; "No alignment restrictions apply to this VA",
//!   C5.3.21, C5-1128), then `dsb ish`. A data cache maintenance
//!   instruction "is only guaranteed to be complete, and its effects
//!   visible to other observers, following a DSB instruction executed by
//!   the PE", a DSB of access type loads and stores (D7.5.9.15, D7-7667);
//!   such a DSB completes every cache maintenance instruction before it,
//!   and no later instruction performs any part of its function until it
//!   completes (B2.6.9, B2-302/303) — so the following `AT` walks after the
//!   clean. Clean *and* invalidate: a dirty line is written back first,
//!   nothing is lost.
//! * L5 `tlbi vaale1is, <Xt>` — VAAL: stage 1 entries of the final lookup
//!   level for the address, global and any ASID (`RJXKNN`, D8-7869);
//!   L6 `tlbi vaae1is, <Xt>` — VAA: entries from **any** lookup level for
//!   the address, global and any ASID (`RZSXRQ`, D8-7869);
//!   L7 `tlbi vmalle1is` — VMALL: all stage 1 entries of the EL1&0 regime
//!   from any lookup level (`RPMNQC`, D8-7867). `IS`: all TLBs in the
//!   Inner Shareable domain (`IDMCXY`, D8-7863). `<Xt>` for VAA/VAAL:
//!   bits [63:48] RES0 (no ASID argument), [47:44] RES0 without FEAT_TTL
//!   (optional from Armv8.3, A2-128; the A72 implements Armv8-A, TRM 1.2.1,
//!   S. 1-15), [43:0] = VA[55:12] (`IZVNJG`, D8-7863/7864): `(va >> 12)`
//!   masked to 44 bits. Each followed by `dsb ish; isb`: without FEAT_ETS2
//!   (A72: `ID_AA64MMFR1_EL1` RES0, TRM Table 10-10, S. 10-364) a TLB
//!   maintenance instruction "is guaranteed to be complete after the
//!   execution of DSB by that PE, followed by a Context synchronization
//!   event" (`IBLDZX`, D8-7875); an `ISB` is one (Glossary, "Context
//!   Synchronization event", S. 17126).
//!
//! Nothing runs between the rungs (no output, no allocation); the
//! exception entry masked IRQs.

use tos_api::bdos::process::crash_diag::{self, CrashDiag, HwProbe, LadderHw, Rung};
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
    let mut diag = unsafe { crash_diag::collect(slot, report, ttbr0, hw, crate::arch::aarch64::mmu::empty_l0_pa()) };
    // Then the ladder, which changes the TLB (module docs).
    diag.ladder = crash_diag::run_ladder(&mut Ladder, &diag, report.usp);
    let _ = crash_diag::write_serial(&mut crate::hal::uart::UartWriter(&crate::console::CONSOLE), &diag);
    diag
}

/// `TLBI` operand of the VA forms without ASID: VA[55:12] in bits [43:0],
/// bits [63:44] zero (module docs, `IZVNJG`).
fn tlbi_va_operand(va: u64) -> u64 {
    (va >> 12) & ((1 << 44) - 1)
}

/// The instructions of the ladder (module docs: barriers and cites).
struct Ladder;

impl LadderHw for Ladder {
    fn at_s1e0r(&mut self, va: u64) -> u64 {
        at_s1e0r(va)
    }

    fn maintain(&mut self, rung: Rung, va: u64, lines: &[u64]) {
        let x = tlbi_va_operand(va);
        match rung {
            Rung::Again | Rung::Neighbours => {}
            // SAFETY: EL1; barriers only.
            Rung::Barrier => unsafe {
                core::arch::asm!("dsb ish", "isb", options(nostack, preserves_flags));
            },
            Rung::CleanLines => {
                for &line in lines {
                    // SAFETY: EL1; `line` is a kernel address through which
                    // the walk has just read a descriptor (mapped); clean
                    // before invalidate, so no data is lost.
                    unsafe {
                        core::arch::asm!("dc civac, {a}", a = in(reg) line, options(nostack, preserves_flags));
                    }
                }
                // SAFETY: EL1; a barrier only.
                unsafe { core::arch::asm!("dsb ish", options(nostack, preserves_flags)) };
            }
            // SAFETY (the three TLBIs): EL1; they drop cached translations
            // only, every later access walks the tables again, and the crash
            // path does not change them.
            Rung::Vaale1is => unsafe {
                core::arch::asm!("tlbi vaale1is, {x}", "dsb ish", "isb", x = in(reg) x, options(nostack, preserves_flags));
            },
            // SAFETY: as above.
            Rung::Vaae1is => unsafe {
                core::arch::asm!("tlbi vaae1is, {x}", "dsb ish", "isb", x = in(reg) x, options(nostack, preserves_flags));
            },
            // SAFETY: as above.
            Rung::Vmalle1is => unsafe {
                core::arch::asm!("tlbi vmalle1is", "dsb ish", "isb", options(nostack, preserves_flags));
            },
        }
    }
}

/// The screen block for `diag` into the console writer `w` of the report.
pub fn screen<W: core::fmt::Write>(w: &mut W, diag: &CrashDiag) {
    let _ = crash_diag::write_screen(w, diag);
}
