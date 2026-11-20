//! XBIOS11 — `Random` (XBIOS 0x11) seeds its first value from the 200 Hz
//! counter `_hz_200`, as EmuTOS `random()` does (`bios/xbios.c:421-430`):
//! `rseed = (hz_200 << 16) + hz_200`, then `rseed = rseed * 3141592621 + 1`,
//! result `(rseed >> 8) & 0xFFFFFF`. Spec: `docs/api/xbios/11-random.md`,
//! "Required behaviour" (seed source, result width).
//!
//! Through the trap dispatcher with the kernel counter advanced by the tick
//! path (`bios::tick::advance`), not a frame clock. One test only: `Random`'s
//! state and the counter are globals of this test binary.

use tos_api::trap::{TrapArgs, TrapClass};

const RANDOM: u16 = 0x11;

fn xbios(func: u16) -> i64 {
    let args = TrapArgs {
        regs: [0u64; 8],
        func: func as u64,
        svc_imm: TrapClass::Xbios as u16,
        space: tos_api::uaccess::Space::Kernel,
    };
    tos_api::xbios::dispatch(&args)
}

fn lcg(s: u32) -> u32 {
    s.wrapping_mul(3_141_592_621).wrapping_add(1)
}

#[test]
fn first_random_is_seeded_from_hz_200() {
    // hz_200 = 2573: the value the oracle's first Random() saw on Hatari TT,
    // which returned 7d9272
    // (measurements/measurement-2026-10-28-xbios-system-audit/runs/SEED-EMUTOS-tt/XSSEED.LOG).
    tos_api::bios::tick::advance(2573);
    assert_eq!(tos_api::bios::tick::hz_200(), 2573);
    let first = xbios(RANDOM);
    assert_eq!(first, 0x7D_9272, "first Random() = LCG step of (2573 << 16) + 2573");

    // The second call steps on from the state; the counter moving does not
    // re-seed a non-zero state.
    tos_api::bios::tick::advance(1000);
    let s1 = lcg((2573u32 << 16) + 2573);
    let second = xbios(RANDOM);
    assert_eq!(second, i64::from((lcg(s1) >> 8) & 0xFF_FFFF));
    assert!((0..=0xFF_FFFF).contains(&second), "24-bit result, bits 24-63 zero");
}
