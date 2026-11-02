//! AArch64 architecture support: boot stub, exception vectors,
//! register access, low-level halt.

pub mod boot;
pub mod crash;
pub mod dtb;
pub mod el0;
pub mod exceptions;
pub mod mmu;
pub mod user_entry;

/// Spin forever in the lowest-power state. Used by the panic handler and the
/// secondary-core parking loop in `boot::_start`.
#[inline(always)]
pub fn halt() -> ! {
    loop {
        // SAFETY: `wfe` has no side effects beyond entering low-power wait.
        unsafe { core::arch::asm!("wfe", options(nomem, nostack, preserves_flags)) };
    }
}

/// Busy-wait `ms` milliseconds using the ARM generic timer
/// (`CNTPCT_EL0` / `CNTFRQ_EL0`). Coarse but adequate for boot-screen
/// pacing.
pub fn delay_ms(ms: u64) {
    // SAFETY: read-only system-register reads, always permitted at EL1.
    let (start, frq): (u64, u64);
    unsafe {
        core::arch::asm!(
            "mrs {s}, cntpct_el0",
            "mrs {f}, cntfrq_el0",
            s = out(reg) start,
            f = out(reg) frq,
            options(nomem, nostack),
        );
    }
    let target = start + frq / 1000 * ms;
    loop {
        let now: u64;
        // SAFETY: as above.
        unsafe {
            core::arch::asm!("mrs {n}, cntpct_el0", n = out(reg) now, options(nomem, nostack));
        }
        if now >= target {
            break;
        }
        core::hint::spin_loop();
    }
}

// Debug feature `crash-walk` (TASK/PROC08). Declared at the end of the file
// so that with the feature off nothing above moves (panic-location lines
// included; CLAUDE.md "Testing", debug features).
#[cfg(feature = "crash-walk")]
pub mod crash_walk;
