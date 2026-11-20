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

use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

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
    // The frame counters of the same process, from the same interrupt
    // (BIOS20, spec section 5 rule 1).
    publish_frames_running();
}

// ─── `_frclock`, `_vbclock` and the `_vblsem` input (BIOS20) ─────────────
//
// EmuTOS counts both in its VBL interrupt: `_frclock` on every VBL
// (`references/emutos/bios/vectors.S:330`), `_vbclock` only when the VBL
// body runs, i.e. when `_vblsem` (`$452`) was greater than 0 on entry
// (`:338-339`, the count at `:348`). NewTOS has no VBL interrupt; the frame
// is the ~60 Hz frame clock `Vsync` waits on (`Host::frame_clock`), counted
// from the frame recorded at boot ([`start_frame_clock`]), so both cells
// start at 0 (owner decision C1, spec
// `docs/api/bios/_sysvars-counters-contract.md` sections 5, 6 and 17).
//
// The cells follow `_hz_200`'s model: they are written into the **running**
// process's page 0, at three points — this timer interrupt ([`advance`]),
// `Vsync` before it returns ([`publish_frames_running`]) and whenever a
// process gets the CPU ([`switch_frames`], from `process::set_current`).
// `_vbclock` is `_frclock` minus the frames the process ran with its own
// `_vblsem` at or below 0 ([`VBL_LOST`], decision Q1 (c)): at each
// publish the frames since the last one ([`ACCT_FC`]) are charged when the
// semaphore is down; at a switch-in they are not, so a process that was away
// loses nothing. All of it is lock-free and allocation-free: one counter
// read, a few atomics and unaligned stores into a static kernel page.

/// `_vblsem` (`$452`, WORD): "if > 0, the VBL routine is executed"
/// (`references/emutos/tosvars.ld:56`). Boot value 1 (`bios/bios.c:553`).
/// Read by the kernel as a documented input (`docs/api/_super-state.md` S10).
pub const VBLSEM_ADDR: usize = 0x452;

/// `_vbclock` (`$462`, LONG): VBLs whose body ran (`tosvars.ld:61`).
pub const VBCLOCK_ADDR: usize = 0x462;

/// `_frclock` (`$466`, LONG): every VBL (`tosvars.ld:62`).
pub const FRCLOCK_ADDR: usize = 0x466;

/// The raw frame-clock value recorded at boot ([`start_frame_clock`]); the
/// cells count from it, so they start at 0 (decision C1).
static FRAME_BASE: AtomicU64 = AtomicU64::new(0);

/// The frame number up to which the running process's `_vblsem` has been
/// accounted (spec section 6, consequence 2: the global word `acct_fc`).
static ACCT_FC: AtomicU32 = AtomicU32::new(0);

/// Per process slot: the frames the process ran with its `_vblsem` at or
/// below 0 (spec section 6, consequence 2: the per-process word `lost`); its
/// `_vbclock` is `_frclock` minus this. Kernel-private, wrapping like the
/// `LONG` cells; set to 0 when a process is loaded into the slot
/// ([`reset_frames`]).
static VBL_LOST: [AtomicU32; crate::bdos::process::MAX_PROCESSES] =
    [const { AtomicU32::new(0) }; crate::bdos::process::MAX_PROCESSES];

/// A new process in `slot` has lost no frames: its `_vbclock` equals
/// `_frclock`. Called by the loader when it sets the slot up.
pub fn reset_frames(slot: usize) {
    if let Some(l) = VBL_LOST.get(slot) {
        l.store(0, Ordering::Relaxed);
    }
}

/// Record the boot frame: from now on [`frclock`] counts from 0. Called once
/// by the kernel right after it installs the host.
pub fn start_frame_clock() {
    if let Some(h) = crate::host::host() {
        FRAME_BASE.store(h.frame_clock(), Ordering::Relaxed);
    }
}

/// The current `_frclock` value: frames of the host frame clock since
/// [`start_frame_clock`], truncated to a wrapping 32-bit `LONG` (0 without a
/// host, whose frame clock stands still).
pub fn frclock() -> u32 {
    crate::host::host().map_or(0, |h| h.frame_clock().wrapping_sub(FRAME_BASE.load(Ordering::Relaxed)) as u32)
}

/// Is the `_vblsem` cell of page 0 `page0` at or below 0? A signed WORD,
/// compared with 0 only, as the EmuTOS VBL tests the sign after its
/// `subq.w #1` (`references/emutos/bios/vectors.S:338-339`).
///
/// # Safety
///
/// `page0` is the kernel alias of a slot's 4-KiB page 0.
unsafe fn vblsem_down(page0: *const u8) -> bool {
    // SAFETY: caller's contract; `$452 + 2` lies inside the page.
    let v = unsafe { page0.add(VBLSEM_ADDR).cast::<i16>().read_unaligned() };
    v <= 0
}

/// Account `slot`'s frames up to `fc` and write its two counters into its
/// page 0 `page0`. With `charge` the frames since [`ACCT_FC`] count as lost
/// when the process's `_vblsem` is down; without it (a switch-in) they are
/// the time the process was away and count for nothing.
///
/// # Safety
///
/// `page0` is the kernel alias of `slot`'s 4-KiB page 0.
unsafe fn write_frames(slot: usize, page0: *mut u8, fc: u32, charge: bool) {
    let Some(lost) = VBL_LOST.get(slot) else {
        return;
    };
    // SAFETY: caller's contract.
    if charge && unsafe { vblsem_down(page0) } {
        let since = fc.wrapping_sub(ACCT_FC.load(Ordering::Relaxed));
        lost.fetch_add(since, Ordering::Relaxed);
    }
    ACCT_FC.store(fc, Ordering::Relaxed);
    let vbclock = fc.wrapping_sub(lost.load(Ordering::Relaxed));
    // SAFETY: caller's contract; `$462..$46A` lies inside the page (2 mod 4,
    // unaligned stores as for `_hz_200`).
    unsafe {
        page0.add(FRCLOCK_ADDR).cast::<u32>().write_unaligned(fc);
        page0.add(VBCLOCK_ADDR).cast::<u32>().write_unaligned(vbclock);
    }
}

/// Publish `_frclock` and `_vbclock` into the running process's page 0,
/// charging the frames since the last publish to its `_vblsem` (spec section
/// 6, consequence 2). Nothing when no process runs. Called by the timer
/// interrupt ([`advance`]) and by `Vsync` before it returns (spec section 5
/// rule 3); interrupt-context clean.
pub fn publish_frames_running() {
    let Some(slot) = crate::bdos::process::slot_for(crate::bdos::process::current()) else {
        return;
    };
    let Some(page0) = crate::bdos::process::mmu::page0_kva(slot) else {
        return;
    };
    // SAFETY: the running slot's static page 0 (see `publish`).
    unsafe { write_frames(slot, page0, frclock(), true) };
}

/// The counters at a change of the running process (`process::set_current`):
/// `old` (the slot leaving the CPU, if any and if it is not `new`) is
/// accounted up to now, then `new` (the slot getting it, if any) is refreshed
/// without charging the frames it spent away (spec section 5 rule 2, section
/// 6 consequence 2). With `old == new` it is an ordinary publish.
pub fn switch_frames(old: Option<usize>, new: Option<usize>) {
    use crate::bdos::process::mmu::page0_kva;
    let fc = frclock();
    if let Some(o) = old {
        if let Some(page0) = page0_kva(o) {
            // SAFETY: `o`'s static page 0.
            unsafe { write_frames(o, page0, fc, true) };
        }
    }
    if let Some(n) = new.filter(|&n| Some(n) != old) {
        if let Some(page0) = page0_kva(n) {
            // SAFETY: `n`'s static page 0.
            unsafe { write_frames(n, page0, fc, false) };
        }
    }
}

/// Is the running process's `_vblsem` at or below 0? Then the frame work of
/// the EmuTOS VBL body is skipped for this frame: `_vbclock` does not count
/// and the kernel's mouse draw (`vb_draw`) does not run (decision Q2 (b),
/// ADR-031 amendment of 2026-11-20 item 4, `docs/api/_super-state.md` S10).
/// `false` when no process runs (EL1): the gate does not apply.
/// Interrupt-context clean (one page-0 read).
pub fn vbl_semaphore_down() -> bool {
    match crate::bdos::process::current_page0_kva() {
        // SAFETY: the running slot's static page 0.
        Some(page0) => unsafe { vblsem_down(page0) },
        None => false,
    }
}
