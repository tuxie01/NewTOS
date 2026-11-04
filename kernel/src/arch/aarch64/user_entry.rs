//! EL0 entry — the asm sequence that drops the CPU into user mode.
//!
//! Phase 06 D5 consolidates the EL0-transition code that the
//! Phase-05 EC4 spike and the Phase-06 D3 ELF-loader self-test had
//! independently duplicated. Phase 06 D6 (`Pterm`) reuses the same
//! helper — production and test callers all want the same five
//! steps before `eret`-ing into user space:
//!
//!   1. Capture an EL1 resume PC into a caller-supplied atomic so
//!      the trap handler that fields the user's exit (`Pterm` SVC,
//!      EC4 sentinel SVC, …) knows where to redirect.
//!   2. Save the kernel's current SP into another caller-supplied
//!      atomic so the handler can restore `frame.sp_el0` to it.
//!   3. Mask D/A/I/F — no exception may be taken between the
//!      `ELR_EL1` / `SPSR_EL1` writes and the `eret`, because
//!      exception entry overwrites both (ARCH01) — and switch
//!      `SPSel = 1` so writes to `SP_EL0` are well-defined.
//!   4. Configure `SP_EL0` / `ELR_EL1` / `SPSR_EL1` for the eret.
//!   5. `eret` to EL0; on the handler-redirected return, land at the
//!      captured EL1 resume PC.
//!
//! Sentinel-specific bookkeeping (the EC4 "running flag" that gates
//! `0xEC4`-immediate redirects) is the test driver's responsibility:
//! the driver writes the flag in Rust *before* calling this helper.
//! Pterm has no analogous flag — the trap dispatcher recognises it
//! by the SVC immediate + GEMDOS function ID directly.
//!
//! ## Why the resume-capture sits *inside* the same asm block
//!
//! `adr {pc}, 1f` resolves the address of the `1:` label using a
//! PC-relative `ADR` (±1 MiB range). The label must therefore live
//! in the same translation unit *and* the same asm block as the
//! `adr` — splitting the prelude and the eret into two `asm!` blocks
//! makes `1:` invisible to the prelude (inline-asm local labels are
//! per-block). Keeping everything in one block also matches what the
//! caller actually wants: the handler's eret lands one instruction
//! past the eret-to-EL0, so a single linear asm sequence is the
//! natural shape.
//!
//! ## Why this is `unsafe`
//!
//! * `install_ttbr0` rewrites the per-CPU low-half translation
//!   regime; concurrent EL0 access on another CPU is undefined.
//! * The asm reads / writes raw atomic pointers; the caller must
//!   guarantee they're valid for the duration of the round-trip.
//! * After eret to EL0 control depends on the handler honouring the
//!   sentinel; a buggy handler would leave the CPU spinning in the
//!   user code's `b .` safety net (or worse, faulting in EL0 with
//!   an indirectly-wrong frame).

use core::sync::atomic::{AtomicU64, Ordering};

use tos_api::bdos::process::mmu::install_ttbr0;
use tos_api::bdos::process::ParentFrame;

// ─── Stage-1 boot-table phys (linker symbol) ────────────────────────
//
// `__boot_l0_phys` is a linker-script absolute symbol whose *value*
// is the low-half phys of the Stage-1 boot L0 table (`__BOOT_L0 -
// VA_OFFSET` in the link script). Resolving it via `&raw const` from
// high-half kernel code would emit `ADRP+ADD` (`R_AARCH64_ADR_PREL_PG_HI21`,
// ±4 GiB range) which cannot bridge the high-half→low-half distance.
// `ldr =sym` goes through a literal pool — 64-bit absolute, works
// regardless of the kernel's link address.

unsafe extern "C" {
    static __boot_l0_phys: u8;
}

#[inline]
fn boot_l0_phys() -> u64 {
    let v: u64;
    // SAFETY: pure register load of a link-time constant.
    unsafe {
        core::arch::asm!(
            "ldr {out}, ={sym}",
            out = out(reg) v,
            sym = sym __boot_l0_phys,
            options(nomem, nostack, preserves_flags)
        );
    }
    v
}

/// Reset `TTBR0_EL1` to the empty no-process root ([`mmu::EMPTY_L0`]) and
/// flush the TLB (`tlbi vmalle1`), dropping any cached entries from the dying
/// slot before a future per-process install.
///
/// **ADR-014:** this used to restore the Stage-1 boot identity map. After the
/// high-half MMIO switch the kernel runs entirely through TTBR1 and never
/// needs the low-half identity — and on real hardware the boot map's stale
/// 1-GiB UXN blocks *shadow* a process/slot mapping (EL0 fetch permission
/// fault) and corrupt the kernel on the `Pterm` resume of a kernel-launched
/// program (the AUTO folder). So we install the empty L0 here, matching what
/// `install_stage2_ttbr1` leaves in TTBR0 for the no-process state.
///
/// # Safety
///
/// Caller must be the boot CPU (single-CPU contract). After this call,
/// low-half VAs raise a translation fault until the next per-process install.
pub unsafe fn restore_boot_ttbr0() {
    let empty_l0 = crate::arch::aarch64::mmu::empty_l0_pa();
    if tos_api::bdos::process::mmu::asids_enabled() {
        // ARCH03 stage 4 (`asid=on`): every process root carries its own
        // ASID and the empty root ASID 0, so nothing of the dying or
        // departing root can be matched under the next one — no TLBI
        // (ARM ARM Example D8-1, D8-7700; V0a B8). What a torn-down root left
        // behind is retired with its ASID (`mmu::flush_tlb_slot_released`,
        // `mmu::init_slot_tables`).
        // SAFETY: writing TTBR0_EL1 is permitted at EL1.
        unsafe {
            core::arch::asm!(
                "dsb ishst",
                "msr ttbr0_el1, {ttbr0}",
                "isb",
                ttbr0 = in(reg) empty_l0,
                options(nostack, preserves_flags),
            );
        }
        return;
    }
    // ASIDs off: every root runs with ASID 0, so whatever the TLB and the
    // A72's intermediate table walk caches hold of the departing root would
    // match under the next one (Cortex-A72 TRM 100095_0003_06 §5.6, p. 5-282:
    // cached intermediate entries are tied to an ASID; with one shared ASID
    // "you must invalidate the TLB to deallocate the translation table
    // memory"). The `isb` between `msr` and `tlbi` is required (TASK/PROC08):
    // a direct write to TTBR0_EL1 is "not required to be observed by indirect
    // reads until completion of a Context synchronization event" (ARM ARM
    // DDI 0487 M.d `IRBMTM`, D8-7700), so without it a walk issued after the
    // `tlbi` may still start from the departing root and cache an
    // intermediate entry the `tlbi` no longer removes — any entry that does
    // not fault "is permitted to be cached … as the result of an explicit or
    // speculative access" (`RSQBCS`, D8-7852). The `tlbi` is complete only
    // after `dsb` and a Context synchronization event (`IBLDZX`, D8-7875;
    // the A72 has no FEAT_ETS2: its ID_AA64MMFR1_EL1 is RES0, TRM Table
    // 10-10, p. 10-364), and until then speculative accesses may use
    // the old translation information (`ICPZSM`, D8-7875): hence the closing
    // `dsb ish; isb`. Same shape as `mmu::size_asids` (`RVNRFW`).
    // SAFETY: writing TTBR0_EL1 + TLBI is permitted at EL1.
    unsafe {
        core::arch::asm!(
            "dsb ishst",
            "msr ttbr0_el1, {ttbr0}",
            "isb",
            "tlbi vmalle1",
            "dsb ish",
            "isb",
            ttbr0 = in(reg) empty_l0,
            options(nostack, preserves_flags),
        );
    }
}

/// Drop to EL0 at `entry_va`, with the per-process L0 at `l0_pa`
/// installed in `TTBR0_EL1`, `SP_EL0 = sp_el0` and `x0 = basepage_va`
/// (the process-entry contract: the child finds its basepage in `x0`). The function
/// returns when the trap handler that fields the user's exit
/// (`Pterm` SVC, EC4 sentinel SVC, …) rewrites the trap frame's
/// `elr_el1` / `sp_el0` / `spsr` to the EL1 resume PC captured by
/// this helper, so the *Rust* caller observes a normal return.
///
/// `parent_frame` is the slot's [`ParentFrame`] snapshot. The asm
/// captures the EL1 resume PC into `parent_frame.elr()` and the
/// current kernel SP into `parent_frame.sp_el0()`; the Rust code
/// before the asm sets `parent_frame.spsr() = 0b0100` (EL1t) so
/// the Pterm handler's full-frame restore brings us back to EL1
/// on the kernel stack we left. The remaining fields (`gpr`, `lr`)
/// stay zero — the kernel-resume path doesn't depend on them
/// because the Rust function epilogue restores its callee-saved
/// regs from the local stack frame.
///
/// Sentinel-specific bookkeeping (e.g. the EC4 "running" flag that
/// gates `0xEC4`-immediate redirects) is the caller's responsibility:
/// the test driver sets the flag in Rust before calling this helper.
/// Pterm needs no equivalent — the trap dispatcher matches on the
/// SVC immediate + GEMDOS function ID directly.
///
/// # Safety
///
/// * `l0_pa` must be the phys of a 4-KiB-aligned, fully-initialised
///   per-process L0 page table.
/// * `parent_frame` must point at the [`ParentFrame`] of a slot
///   whose `parent_l0_pa` is already set to the kernel-resume
///   target (typically `0` for the boot path).
/// * Single-CPU contract — no other CPU is allowed to touch
///   `TTBR0_EL1` while this function is in flight.
/// * `entry_va` must lie within the slot's mapped low-half VA range
///   (otherwise the eret faults at the user PC).
///
/// `#[inline(never)]`: the EL0 excursion in the asm clobbers caller-saved
/// registers, so the caller's code must not be inlined across it — only this
/// function's own epilogue (which reloads callee-saved from the stack) may run
/// after the resume.
#[inline(never)]
pub unsafe fn enter_el0_with_resume_capture(
    entry_va: u64,
    basepage_va: u64,
    l0_pa: u64,
    sp_el0: u64,
    parent_frame: &ParentFrame,
) {
    // Pre-set the SPSR to EL1t so the handler-side eret returns
    // here on the kernel stack with `SPSel = 0` (active SP =
    // SP_EL0 = saved kernel SP).
    parent_frame
        .spsr()
        .store(0b0100, Ordering::Release);

    // SAFETY: caller contract — single-CPU, valid l0_pa.
    unsafe { install_ttbr0(l0_pa) };

    let elr_ptr: *const AtomicU64 = parent_frame.elr();
    let sp_ptr: *const AtomicU64 = parent_frame.sp_el0();

    // SAFETY: parent_frame outlives the round-trip; the asm uses
    // raw pointers into the atomics, with single-CPU exclusion
    // honoured by the BDOS lock.
    unsafe {
        core::arch::asm!(
            // Preserve the kernel's callee-saved registers + FP/LR across the
            // EL0 excursion. The Pterm handler resumes us at label 1 via the
            // generic trap-frame restore, which loads the *child's* EL0 GPRs —
            // so without this, x19..x30 come back as garbage. LLVM assumes an
            // asm block preserves callee-saved regs and won't let us list the
            // LLVM-reserved ones (x18/x19/x29) as clobbers, so we save them on
            // the kernel stack here; the captured SP points at them so the
            // resume restores them. (Latent miscompile — surfaced only once
            // unrelated code shifted the register allocation.)
            "stp x19, x20, [sp, #-96]!",
            "stp x21, x22, [sp, #16]",
            "stp x23, x24, [sp, #32]",
            "stp x25, x26, [sp, #48]",
            "stp x27, x28, [sp, #64]",
            "stp x29, x30, [sp, #80]",

            // Capture EL1 resume PC into parent_frame.elr.
            "adr   {tmp}, 1f",
            "str   {tmp}, [{resume_pc}]",

            // Save the current SP (= SP_EL0 since SPSel=0) via
            // `mov xN, sp` — the only legal way while SPSel=0. It now points
            // at the saved registers, so the resume restores from there.
            "mov   {kstack}, sp",
            "str   {kstack}, [{kernel_sp}]",

            // Mask D/A/I/F for the rest of the sequence. From the first
            // `MSR` below until the `eret`, `ELR_EL1` / `SPSR_EL1` hold the
            // *user* return state; an exception taken in that window would
            // overwrite both with the interrupted EL1 state (ARM ARM
            // DDI 0487M.c, D1.4.2 "Exception entry", rule R_DQXFW: entry
            // writes SPSR_ELx from PSTATE and the preferred return address
            // to ELR_ELx). The handler returns into the middle of this
            // block, the remaining instructions reinstate `SPSR_EL1` = EL0t
            // — and the `eret` then drops to EL0 on the *kernel* PC the IRQ
            // left in `ELR_EL1`, which faults as an Instruction Abort from a
            // lower EL (ARCH01: seen once in seven launches, and reproduced
            // on demand with a delay inserted into this window).
            //
            // `MSR DAIFSet, #0xf` ORs the immediate into PSTATE.{D,A,I,F}
            // (C6.2.288 "MSR (immediate)"). Asynchronous exceptions are the
            // only ones that can enter this window — the instructions here
            // touch no memory and cannot fault — so masking all four closes
            // it completely. The mask does not outlive the `eret`: an
            // exception return sets PSTATE from `SPSR_EL1` (D1.4.4, rule
            // R_BWCFK; C6.2.157 "ERET"), and the SPSR written below is `0`,
            // so EL0 starts with DAIF = 0 and takes the pending interrupt
            // right away.
            "msr   daifset, #0xf",

            // Switch SPSel = 1 so the subsequent `MSR SP_EL0, …` is
            // well-defined (writing SP_EL0 while it's the active SP
            // is UNPREDICTABLE — ARM ARM D17.2.130).
            "msr   spsel, #1",

            // Configure the eret target.
            "msr   sp_el0, {sp}",
            "msr   elr_el1, {entry}",
            "mov   {tmp}, #0",          // SPSR.M = EL0t, DAIF = 0
            "msr   spsr_el1, {tmp}",

            // Drop to EL0.
            "eret",

            // Resume here. The Pterm handler restored
            // `parent_frame.spsr.M = EL1t` and `parent_frame.sp_el0
            // = saved kernel SP` into the trap frame, so the
            // handler-side eret lands us at EL1 with `SP_EL0 = our
            // original kernel stack` — pointing at the saved registers.
            "1:",
            "ldp x19, x20, [sp]",
            "ldp x21, x22, [sp, #16]",
            "ldp x23, x24, [sp, #32]",
            "ldp x25, x26, [sp, #48]",
            "ldp x27, x28, [sp, #64]",
            "ldp x29, x30, [sp, #80]",
            "add sp, sp, #96",
            entry      = in(reg) entry_va,
            sp         = in(reg) sp_el0,
            // The child's `x0` at entry = its basepage VA — the NewTOS
            // AArch64 process-entry contract the `Pexec` path already keeps
            // (`exceptions.rs` `handle_pexec`: `frame.gpr[0] =
            // child.basepage_va`). The SDK `crt0` reads the basepage from
            // `x0` (ARCH03 stage 0d), so the kernel-launch path must supply
            // it too. `inout … => _`: the EL0 excursion returns with the
            // child's `x0`, not this value.
            inout("x0") basepage_va => _,
            resume_pc  = in(reg) elr_ptr,
            kernel_sp  = in(reg) sp_ptr,
            kstack     = out(reg) _,
            tmp        = out(reg) _,
        );
    }
}
