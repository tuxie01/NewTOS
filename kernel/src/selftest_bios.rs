//! BIOS self-test through the real `svc #13` trap path.
//!
//! Covers the character devices (`Bconstat`/`Bconin`/`Bconout`/`Bcostat`)
//! plus the error/boundary paths added with Task #24: `Kbshift` query,
//! the out-of-range dispatcher return convention, `Setexc` non-null
//! `etv_*` defaults, `Getmpb` MPB fill, `Bcostat(5) == 0`, and the
//! non-blocking handle-4 (IKBD) input slot.

use tos_api::bdos::mem::init::stram_range;
use tos_api::bios::getmpb::{PublicMd, PublicMpb};
use tos_api::bios::vectors::{ETV_DEFAULT_JUST_RTS, ETV_TIMER};

use crate::{console, kprintln};

/// Issue a BIOS call: `svc #13`, function id in `x8`, args in `x0`/`x1`.
fn bios(func: u16, a0: u64, a1: u64) -> i64 {
    let ret: i64;
    // SAFETY: BIOS trap ABI; results come back in x0.
    unsafe {
        core::arch::asm!(
            "svc #13",
            in("x8") func as u64,
            inout("x0") a0 => ret,
            in("x1") a1,
            options(nostack),
        );
    }
    ret
}

/// Run the BIOS self-test.
pub fn run() {
    kprintln!("[bios] === BIOS trap-13 test (Bcon* / Kbshift / Setexc / Getmpb) ===");
    let mut fails = 0;
    let mut check = |cond: bool, label: &str| {
        kprintln!("[bios]   {}: {}", label, if cond { "OK" } else { "FAIL" });
        if !cond {
            fails += 1;
        }
    };

    // ── Bcostat (0x08): output-device status ──────────────────────────
    check(bios(0x08, 2, 0) == -1, "Bcostat(CON=2) ready = -1");
    check(bios(0x08, 0, 0) == 0, "Bcostat(PRT=0) absent = 0");
    // F5: handle 5 (raw-CON) output *status* is char_dummy = 0, even
    // though Bconout(5) itself writes and returns 1 (checked below).
    check(bios(0x08, 5, 0) == 0, "Bcostat(RAW_CON=5) = 0 (F5)");

    // ── Bconout (0x03): CON + raw-CON write and return 1 ──────────────
    check(bios(0x03, 2, b'X' as u64) == 1, "Bconout(CON,'X') = 1");
    check(bios(0x03, 5, b'Y' as u64) == 1, "Bconout(RAW_CON,'Y') = 1");
    check(bios(0x03, 0, b'X' as u64) == 0, "Bconout(PRT,'X') = 0");

    // ── Handle-4 (IKBD) input must NOT block and must return 0 ────────
    // A regression here (handle 4 wired as console input) would hang the
    // whole boot on the blocking Bconin(4) — this line reaching the check
    // is itself the proof it no longer blocks.
    check(bios(0x01, 4, 0) == 0, "Bconstat(IKBD=4) = 0 (no input slot)");
    check(bios(0x02, 4, 0) == 0, "Bconin(IKBD=4) = 0 immediately (handle-4 hang fix)");

    // ── Bconstat / Bconin (CON=2): no regress on the real console ─────
    check(bios(0x01, 2, 0) == 0, "Bconstat(CON) no input = 0");
    console::push_rx(u32::from(b'Z'));
    check(bios(0x01, 2, 0) == -1, "Bconstat(CON) with input = -1");
    check(bios(0x02, 2, 0) == 0x5A, "Bconin(CON) returns 'Z' = 0x5A");

    // ── Kbshift (0x0B): query + set/get round-trip (F1) ───────────────
    let flag_query = (-1i64) as u64;
    let prev = bios(0x0B, flag_query, 0);
    // The result is the previous shift byte, zero-extended: a value in
    // 0..=0xFF with bits 8-31 clear.
    check(
        prev >= 0 && prev == (prev & 0xFF),
        "Kbshift(-1) returns a zero-extended byte bitmask",
    );
    // Set MODE_CTRL (0x04), read it back, then restore the prior state.
    let old = bios(0x0B, 0x04, 0);
    let now = bios(0x0B, flag_query, 0);
    bios(0x0B, old as u64, 0); // restore
    check(
        old == prev && now == 0x04,
        "Kbshift set/get round-trip (0x04) + returns old",
    );

    // ── Dispatcher out-of-range return convention (F2) ────────────────
    // IDs > 0x0B return the function number itself (EmuTOS trampoline),
    // not EINVFN.
    check(bios(0x0C, 0, 0) == 0x0C, "Bios(0x0C) out-of-range = func# 0x0C (F2)");
    check(bios(0x20, 0, 0) == 0x20, "Bios(0x20) out-of-range = func# 0x20 (F2)");

    // ── Setexc (0x05): etv_* defaults are non-null (F3) ───────────────
    let etv = bios(0x05, ETV_TIMER as u64, flag_query); // query only
    check(
        etv == ETV_DEFAULT_JUST_RTS && etv != 0,
        "Setexc(etv_timer,-1) non-null default (F3)",
    );
    // Set/get round-trip on etv_term, then restore its default.
    const TEST_VEC: u64 = 0x0000_0000_ABCD_1234;
    let old_term = bios(0x05, 0x102, TEST_VEC);
    let now_term = bios(0x05, 0x102, flag_query);
    bios(0x05, 0x102, old_term as u64); // restore
    check(
        old_term == ETV_DEFAULT_JUST_RTS && now_term as u64 == TEST_VEC,
        "Setexc(etv_term) set/get round-trip + old non-null",
    );

    // ── Getmpb (0x00): fills the caller's MPB (F4) ────────────────────
    // The block and descriptor have the program-visible layout
    // (`docs/api/bios/00-getmpb.md` V1, BIOS06), not the BDOS-internal MD.
    let (st_start, st_end) = stram_range();
    const ROVER_SENTINEL: u64 = 0xDEAD_BEEF;
    let mut mpb = PublicMpb { mp_free: 0x1111, mp_used: 0x2222, mp_rover: ROVER_SENTINEL };
    bios(0x00, (&mut mpb as *mut PublicMpb) as u64, 0);
    let mfl = mpb.mp_free as *const PublicMd;
    let mpb_ok = !mfl.is_null()
        && mpb.mp_used == 0
        && mpb.mp_rover == ROVER_SENTINEL
        // SAFETY: an EL1 caller gets the kernel VA of `getmpb`'s static
        // descriptor in mp_free.
        && unsafe { *mfl }
            == PublicMd {
                md_next: 0,
                md_start: i64::from(st_start),
                md_length: i64::from(st_end) - i64::from(st_start),
                md_owner: 0,
            };
    check(mpb_ok, "Getmpb fills MPB from ST-RAM extent (F4)");

    if fails == 0 {
        kprintln!("[bios] === BIOS trap-13 test PASS ===");
    } else {
        kprintln!("[bios] === BIOS trap-13 test FAILED ({}) ===", fails);
    }
}
