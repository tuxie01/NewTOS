//! The per-process **super state** — `Super` (GEMDOS 0x20) and the state
//! `Supexec` (XBIOS 0x26) and the `kbdvec` hook run in (TASK/MEM12).
//!
//! Spec: `docs/api/_super-state.md` (approved 2026-10-27), S1–S13. EmuTOS
//! reference: `bdos/rwa.S:159-184` (the TRAP #1 front end catches 0x20),
//! `x20_usr` `:280-301`, `x20_sup` `:305-336`, `x20_inq` `:340-346`;
//! `bios/vectors.S:1376-1378` (`Supexec`).
//!
//! On the m68k, supervisor mode is a bit in the status register. On NewTOS a
//! program always runs at EL0, and the super state is one bit per process
//! that does exactly one thing: it makes the process's page 0 — the classic
//! system variables — EL0-writable (S2). Nothing else becomes privileged: no
//! EL1 code, no kernel memory, no other process's memory, no MMIO; memory
//! protection rights are unchanged. In user state page 0 is EL0 read-only,
//! and a write there is a permission fault that ends the program like a bus
//! error on the Atari (S9).

use core::sync::atomic::Ordering;

use super::{mmu, MAX_PROCESSES, PROCESS_TABLE};

/// Is `slot` in super state? `false` for an out-of-range slot.
pub fn is_super(slot: usize) -> bool {
    PROCESS_TABLE
        .get(slot)
        .is_some_and(|p| p.super_state.load(Ordering::Acquire))
}

/// Enter (`on`) or leave the super state of `slot`: record it and switch the
/// slot's page 0 between EL0 read-write and read-only, with the TLB
/// maintenance [`mmu::set_page0_writable`] documents. A no-op when the state
/// does not change.
///
/// # Safety
///
/// Caller holds the BDOS giant lock (trap context); EL1; the slot's root is
/// built ([`mmu::init_slot_tables`]).
pub unsafe fn set_super(slot: usize, on: bool) {
    if slot >= MAX_PROCESSES {
        return;
    }
    let p = &PROCESS_TABLE[slot];
    if p.super_state.swap(on, Ordering::AcqRel) == on {
        return;
    }
    // SAFETY: caller contract.
    unsafe { mmu::set_page0_writable(slot, on) };
}

/// `Super(stack)` for the process in `slot` (S3–S5, ABI S12): `sp_el0` is
/// the caller's stack pointer at the `svc`.
///
/// * `stack == 1` — the inquiry (`x20_inq`): `0` in user state, `-1` in
///   super state; nothing changes (S3).
/// * any other `stack`, user state — enter the super state; the result is
///   the caller's `SP_EL0`, the token it hands back (S4). The stack does not
///   change, and an explicit `stack` is not used as one (documented
///   deviation O2 = a; EmuTOS `x20_usr` makes it the new SSP).
/// * any other `stack`, super state — leave it (`SuperToUser`, S5); the
///   result is again the caller's `SP_EL0` (EmuTOS's `x20_sup` leaves `d0`
///   undefined).
///
/// # Safety
///
/// As [`set_super`].
pub unsafe fn super_call(slot: usize, stack: u64, sp_el0: u64) -> i64 {
    let sup = is_super(slot);
    if stack == 1 {
        return if sup { -1 } else { 0 };
    }
    // SAFETY: caller contract.
    unsafe { set_super(slot, !sup) };
    sp_el0 as i64
}
