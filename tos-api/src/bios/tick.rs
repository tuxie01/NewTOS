//! The 200 Hz system tick and the `_hz_200` system variable (`$4BA`).
//!
//! Spec: [`docs/api/bios/_state.md`] §1 (`0x4ba hz_200`, `LONG`, volatile —
//! part of the **contract**, i.e. programs observe it) and
//! [`docs/api/bios/06-tickcal.md`] §"The 200 Hz / 20 ms tick derivation".
//!
//! # EmuTOS reference
//!
//! * `bios/mfp.c:193` `init_system_timer()` — programs MFP timer C as
//!   `xbtimer(2, 0x50, 192, int_timerc)`: prescaler ÷64, counter 192, i.e.
//!   2 457 600 / 64 / 192 = **200 Hz**, and sets `timer_ms = 20`.
//! * `bios/vectors.S:501-503` `_int_timerc` — the interrupt handler. Its
//!   **first** action, on *every* interrupt, is `addq.l #1, _hz_200.w`.
//! * `bios/vectors.S:521` — `rol.w _timer_c_sieve` then `jpl timerc_end`
//!   gates everything *after* the counter increment to every 4th interrupt
//!   (50 Hz): keyboard repeat (`kb_timerc_int`), `dosound` (`sndirq`) and
//!   the `etv_timer` hook. See "Deviations" below.
//! * `tosvars.ld:81` — `_hz_200 = 0x4ba`, a `LONG`.
//!
//! # What NewTOS does
//!
//! The kernel's ARM-generic-timer driver (`kernel::drivers::timer`) fires a
//! periodic interrupt and calls [`advance`] with the number of 5 ms periods
//! that elapsed. [`advance`] bumps the canonical counter and mirrors it into
//! the **running** process's low-memory copy of the sysvar area, which the
//! loader placed at TPA offset `$4BA` (`bdos::process::loader::populate_sysvars`,
//! B1/Option 1) — so an EL0 program reading `*(volatile long *)0x4ba` sees a
//! live 200 Hz counter, exactly as on an Atari.
//!
//! # Deviations from EmuTOS (deliberate, listed — not rationalised away)
//!
//! * **Drift-free counting.** EmuTOS increments by exactly one per delivered
//!   interrupt and silently *loses* ticks whenever the m68k IPL masks timer C
//!   (its own drivers even bound themselves to `MAX_IPL 5` because of it —
//!   `bios/xhdi.h:59`). NewTOS' interrupt latency is unbounded in the same way
//!   (IRQs are masked inside the synchronous SVC handler), but the ARM generic
//!   timer's free-running counter lets the driver tell *how many* periods
//!   elapsed, so a late interrupt credits all of them. The counter therefore
//!   equals "5 ms periods since boot" and cannot drift away from wall time.
//! * **The 50 Hz half of the sieve is not run in this interrupt.**
//!   - *Keyboard repeat* (`kb_timerc_int`) runs **polled**, in the input pump
//!     (`kernel::ikbd::poll_repeat`, `kernel::key_repeat`): it derives the
//!     50 Hz steps from this counter (every multiple of 4, the sieve's
//!     phase). It cannot run here, because on the Pi the USB-HID boot
//!     keyboard reports changes only and does **not** repeat by itself, and
//!     its break reaches the decoder only through polling — an
//!     interrupt-driven repeat would emit ghost keys after the release.
//!     Sanctioned deviation, `docs/api/xbios/_state.md` §"Key-repeat handler"
//!     (KEYS01, 2026-10-03). INPUT01's pump in this interrupt (while a
//!     program runs at EL0, `kernel::input_gate`) moves the pointer only:
//!     key bytes stay held for the trap-side pump, and the repeat with them.
//!   - `dosound` (no YM2149) and the `etv_timer` dispatch (`bios::vectors`
//!     holds the shared cell but never calls it; ADR-031, BIOS-CORE-002
//!     ACCEPTED) have no NewTOS counterpart (`dosound`: not yet). Tracked as a gap in
//!     `docs/api/bios/_state.md`, not silently absorbed.

use core::sync::atomic::{AtomicU32, Ordering};

/// Tick rate of the system clock counter, in Hz (EmuTOS timer C: 200 Hz —
/// `bios/mfp.c:205`; `include/biosdefs.h:17` `CLOCKS_PER_SEC`).
pub const TICK_HZ: u32 = 200;

/// Offset of the `_hz_200` `LONG` inside the classic low-memory system-
/// variable area — i.e. its address in a process's TPA, since the TPA base
/// is user VA 0 (`tosvars.ld:81`, `docs/api/bios/_state.md` §1).
pub const HZ_200_ADDR: usize = 0x4BA;

/// The canonical 200 Hz counter. A TOS `LONG`, so 32 bits wide and wrapping
/// — the same value width a program reads at `$4BA`.
static HZ_200: AtomicU32 = AtomicU32::new(0);

/// Advance the system-tick counter by `ticks` 5 ms periods and republish it
/// to the running process. Called from the kernel's timer interrupt handler;
/// must stay interrupt-context clean (no locks, no allocation, no I/O).
///
/// EmuTOS `_int_timerc`'s `addq.l #1, _hz_200.w` (`bios/vectors.S:503`),
/// generalised to "one increment per elapsed period" (see the module docs).
pub fn advance(ticks: u32) {
    if ticks == 0 {
        return;
    }
    // `fetch_add` returns the *previous* value; the published one is the new.
    let value = HZ_200.fetch_add(ticks, Ordering::Relaxed).wrapping_add(ticks);
    publish(value);
}

/// Current value of the 200 Hz system-tick counter (`_hz_200`, `$4BA`).
///
/// Kernel-side readers get the exact counter; a program reads the mirrored
/// copy in its own low memory, which is refreshed on every tick.
#[inline]
pub fn hz_200() -> u32 {
    HZ_200.load(Ordering::Relaxed)
}

/// Mirror `value` into the running process's `_hz_200` cell.
///
/// The write goes through the **kernel alias of the running process's page
/// 0** (its private `[0, 0x1000)`, a static kernel page per slot), not
/// through the user mapping — so it is independent of which `TTBR0` happens
/// to be installed and works whether the interrupt was taken from EL0 or EL1.
///
/// `$4BA` is not 4-byte aligned (it is `2 mod 4`), exactly as on m68k, hence
/// the unaligned store. AArch64 permits unaligned accesses to Normal memory;
/// the cell lies inside one 16-byte block, and the m68k original is equally
/// non-atomic (`addq.l` to an odd-word address is two bus cycles), so this
/// matches the classic contract rather than tightening it.
fn publish(value: u32) {
    let Some(page0) = crate::bdos::process::current_page0_kva() else {
        // No user process is running (kernel/EL1 code — boot, shell loop);
        // nothing to mirror into. The counter itself keeps running.
        return;
    };
    // SAFETY: `page0` is the running slot's page 0, a static kernel page;
    // `HZ_200_ADDR + 4` lies inside it, in the loader-populated sysvar area
    // (`populate_sysvars`). The page itself never goes away; the slot cannot
    // change under us because `Pterm` clears `RUN` inside the synchronous
    // SVC handler, which runs with IRQs masked (that masking *is* the BDOS
    // mutual exclusion on this single-CPU kernel — see
    // `gemdos::memory::maddalt`).
    unsafe {
        page0
            .add(HZ_200_ADDR)
            .cast::<u32>()
            .write_unaligned(value);
    }
}
