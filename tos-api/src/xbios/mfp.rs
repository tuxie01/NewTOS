//! `Mfpint` (XBIOS 0x0D) and `Xbtimer` (XBIOS 0x1F) — the MFP vector
//! installers, on a machine without an MFP.
//!
//! Spec: `docs/api/xbios/0d-mfpint.md` (TASK/BIOS15). EmuTOS:
//! `references/emutos/bios/mfp.c:114-120` (`mfpint`), `:163-169` (`xbtimer`).
//!
//! Both install an interrupt handler NewTOS never calls — there is no MFP to
//! raise the interrupt (ADR-031, "trap and CPU vectors via `Setexc`, `Mfpint`,
//! `Xbtimer` … not served, warning"). The call warns once per process
//! ([`hooks::note_running`]) and installs nothing.
//!
//! **The return value** is the oracle's (user decision 2026-10-29, TASK/BIOS15).
//! Both are `void` in EmuTOS; what lands in `d0` is what the compiled code
//! leaves there (disassembly of the pinned build, `obj/mfp.o`; spec
//! "Return value"): the trap glue zeroes `d0` and loads the function number
//! (`references/emutos/bios/vectors.S:1323-1324`); `mfpint` ends in
//! `jenabint`, whose last instruction leaves the **MFP interrupt mask
//! register** it just wrote in `d0` — IMRB (`$FFFFFA15`) for interrupt 0–7,
//! IMRA (`$FFFFFA13`) for 8–15, with the interrupt's bit set — zero-extended;
//! `xbtimer` with a timer outside 0–3 returns after loading `control` into
//! `d0.w`, so `d0` = `control` as an unsigned WORD. Measured on EmuTOS TT and
//! Falcon (tosdump/probes/hkprobe.c, `measurements/measurement-2026-10-29-bios15/`):
//! IMRA = 0x14, IMRB = 0x60 while a program runs, `Mfpint(5)` 0x60,
//! `Mfpint(12)` 0x14, `Xbtimer(2, …)` 0x60, `Xbtimer(4, 0x1234, …)` 0x1234,
//! `Xbtimer(-1, 0x8001, …)` 0x8001.
//!
//! NewTOS keeps the two mask bytes as EmuTOS has them while a program runs
//! ([`IMR_AT_RUN`]) and sets the bit an installer enables, as `jenabint`
//! does, so a sequence of calls answers as on EmuTOS. Nothing else reads or
//! writes this image (`Jdisint`/`Jenabint` are not implemented,
//! `docs/api/xbios/0d-mfpint.md` "Open").

use core::sync::atomic::{AtomicU16, Ordering};

use crate::bdos::process::hooks::{note_running, Hook, Via};
use crate::trap::TrapArgs;

/// IMRA (high byte) and IMRB (low byte) of EmuTOS' MFP while a program runs:
/// IMRA bits 2 and 4 (RS-232 transmit buffer empty, receive buffer full —
/// `mfpint(MFP_TBE/MFP_RBF, …)`, `references/emutos/bios/serport.c:1071-1073`),
/// IMRB bits 5 and 6 (timer C, `references/emutos/bios/mfp.c:208`; the IKBD/MIDI
/// ACIA, `mfpint(6, int_acia)`, `references/emutos/bios/aciavecs.S:217-221`).
/// Measured, TT and Falcon (module doc).
pub const IMR_AT_RUN: u16 = 0x1460;

/// The mask image the installers answer from ([`IMR_AT_RUN`] at boot).
static IMR: AtomicU16 = AtomicU16::new(IMR_AT_RUN);

/// MFP interrupt number of `Xbtimer`'s timers A–D — EmuTOS `timer_num[]`
/// (`references/emutos/bios/mfp.c:161`: `MFP_TIMERA` 13, `MFP_TIMERB` 8,
/// `MFP_200HZ` 5, `MFP_TIMERD` 4, `references/emutos/include/biosdefs.h:181-192`).
const XBTIMER_MFP: [u8; 4] = [13, 8, 5, 4];

/// `jenabint(num)`'s `d0`: set interrupt `num`'s mask bit in the image and
/// answer the mask register it lives in, zero-extended.
fn enable(num: u8) -> i64 {
    let bit = 1u16 << (num & 0x0F);
    let imr = IMR.fetch_or(bit, Ordering::Relaxed) | bit;
    if num & 0x0F < 8 {
        i64::from(imr & 0xFF)
    } else {
        i64::from(imr >> 8)
    }
}

/// `Mfpint(num, vector)` (XBIOS 0x0D): EmuTOS writes `vector` to MFP vector
/// `0x40 + (num & 0x0F)` and enables the interrupt. NewTOS warns that the
/// handler is never called (ADR-031) and answers EmuTOS' `d0`.
pub fn mfpint(args: &TrapArgs) -> i64 {
    let num = (args.arg_i16(0) & 0x0F) as u8;
    note_running(Hook::Vector(0x40 + num), args.arg_u64(1), Via::Mfpint);
    enable(num)
}

/// `Xbtimer(timer, control, data, vector)` (XBIOS 0x1F): for timer 0–3
/// EmuTOS programs the timer and then does `mfpint(timer_num[timer],
/// vector)`; for any other `timer` it returns at once with `control` in
/// `d0.w`. NewTOS warns about the handler (ADR-031) and answers EmuTOS' `d0`.
pub fn xbtimer(args: &TrapArgs) -> i64 {
    let timer = args.arg_i16(0);
    let Some(&num) = usize::try_from(timer).ok().and_then(|t| XBTIMER_MFP.get(t)) else {
        return i64::from(args.arg_u16(1));
    };
    note_running(Hook::Vector(0x40 + num), args.arg_u64(3), Via::Xbtimer);
    enable(num)
}

/// Back to the boot image — host tests only.
#[cfg(test)]
pub(crate) fn reset_for_test() {
    IMR.store(IMR_AT_RUN, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(regs: [u64; 4]) -> TrapArgs {
        TrapArgs::kernel([regs[0], regs[1], regs[2], regs[3], 0, 0, 0, 0], 0, 14)
    }

    #[test]
    fn answers_as_the_oracle_measured() {
        // One test owns the image (single global, like the hardware).
        reset_for_test();
        assert_eq!(mfpint(&args([5, 0, 0, 0])), 0x60, "M02");
        assert_eq!(mfpint(&args([0x15, 0, 0, 0])), 0x60, "M03: num & 15");
        assert_eq!(mfpint(&args([12, 0, 0, 0])), 0x14, "M04: IMRA");
        assert_eq!(xbtimer(&args([2, 0x50, 192, 0])), 0x60, "M05: timer C = MFP 5");
        assert_eq!(xbtimer(&args([4, 0x1234, 0, 0])), 0x1234, "M06: no timer");
        assert_eq!(xbtimer(&args([0xFFFF, 0x8001, 0, 0])), 0x8001, "M07: WORD, zero-extended");
        // Enabling another interrupt sets its bit, as jenabint does.
        assert_eq!(mfpint(&args([0, 0, 0, 0])), 0x61);
        assert_eq!(xbtimer(&args([0, 0, 0, 0])), 0x34, "timer A = MFP 13: IMRA bit 5");
        reset_for_test();
    }
}
