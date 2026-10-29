//! Cookie-jar / `Ssystem` self-test: exercise the cookie subfunctions
//! through the real GEMDOS trap (`svc #1`, function id `0x154` in `x8`).

use crate::kprintln;

const NTOS: u32 = u32::from_be_bytes(*b"NTOS");
const MCH: u32 = u32::from_be_bytes(*b"_MCH");
const TEST: u32 = u32::from_be_bytes(*b"TEST");
const ZZZZ: u32 = u32::from_be_bytes(*b"ZZZZ");

/// Issue `Ssystem(mode, arg1, arg2)`: GEMDOS trap, id `0x154` in `x8`.
fn ssystem(mode: u16, arg1: u32, arg2: u64) -> i64 {
    let ret: i64;
    // SAFETY: GEMDOS trap ABI; result comes back in x0.
    unsafe {
        core::arch::asm!(
            "svc #1",
            in("x8") 0x154u64,
            inout("x0") mode as u64 => ret,
            in("x1") arg1 as u64,
            in("x2") arg2,
            options(nostack),
        );
    }
    ret
}

const S_GETCOOKIE: u16 = 8;
const S_SETCOOKIE: u16 = 9;
const S_DELCOOKIE: u16 = 26;

/// FreeMiNT-verbatim error codes returned by the `Ssystem` cookie subset
/// (D-073). `EERROR` (get-miss) and `EINVAL` (bad tag / bad index /
/// del-miss) are deliberately distinct.
const E_OK: i64 = 0;
const EERROR: i64 = -1;
const EINVAL: i64 = -25;
const EINVFN: i64 = -32;

/// The jar's fixed total slot capacity (`JAR_CAP` in `tos_api::cookie`).
/// `S_GETCOOKIE(tag == 0)` returns this, not the occupied count (D-073).
const JAR_CAP: i64 = 32;

/// Run the cookie-jar self-test. Verifies the D-072/D-073 fixes end-to-end
/// through the real GEMDOS trap.
pub fn run() {
    kprintln!("[cookie] === Cookie jar / Ssystem test ===");
    let mut fails = 0;
    let mut total = 0;
    let mut check = |cond: bool, label: &str| {
        total += 1;
        kprintln!("[cookie]   {}: {}", label, if cond { "OK" } else { "FAIL" });
        if !cond {
            fails += 1;
        }
    };

    // D-072: `mode == -1` "does this call exist" probe → E_OK, for exactly
    // -1 and regardless of arg1/arg2.
    check(ssystem(0xFFFF, 0, 0) == E_OK, "Ssystem(-1, 0, 0) = E_OK (probe)");
    check(ssystem(0xFFFF, NTOS, 0xDEAD) == E_OK, "Ssystem(-1, .., ..) = E_OK (args ignored)");

    // Seeded set: NTOS, _MCH, _IDT, _AKP, EdDI (5 cookies).
    check(ssystem(S_GETCOOKIE, NTOS, 0) == 0x0000_0100, "Getcookie(NTOS) = version 0x100");
    // `_MCH` = 0xFFFFFFFF (`tos_api::cookie::MCH_NOHARD`, ADR-012 amendment
    // 2026-10-28). The value form answers it as -1 in x0 (sign-extended),
    // which is also EERROR; the pointer form is the unambiguous read: E_OK
    // and the four bytes 0xFFFFFFFF written to the buffer.
    check(ssystem(S_GETCOOKIE, MCH, 0) == -1, "Getcookie(_MCH) value form = -1 (0xFFFFFFFF)");
    let mut mch: u32 = 0x5A5A_5A5A;
    let rc = ssystem(S_GETCOOKIE, MCH, &raw mut mch as u64);
    check(rc == E_OK && mch == 0xFFFF_FFFF, "Getcookie(_MCH, &v) = E_OK, v = 0xFFFFFFFF");
    kprintln!("[cookie]   _MCH: value form x0 = {:#018x}, pointer form rc = {}, v = {:#010x}",
        ssystem(S_GETCOOKIE, MCH, 0) as u64, rc, mch);

    // D-073: `tag == 0` returns the jar CAPACITY (32), not the occupied
    // count (which is 5 here) — the fix's headline behaviour change.
    check(ssystem(S_GETCOOKIE, 0, 0) == JAR_CAP, "Getcookie(0) = capacity 32 (not count)");

    // Get-miss → EERROR (-1); unchanged, must stay distinct from EINVAL.
    check(ssystem(S_GETCOOKIE, ZZZZ, 0) == EERROR, "Getcookie(unknown tag) = EERROR (-1)");

    // Slot enumeration: slot 1 is the first seeded cookie (NTOS); an index
    // past the end → EINVAL (D-073), NOT EERROR.
    check(ssystem(S_GETCOOKIE, 1, 0) == NTOS as i64, "Getcookie(slot 1) = NTOS tag");
    check(ssystem(S_GETCOOKIE, 9999, 0) == EINVAL, "Getcookie(slot 9999) = EINVAL (-25)");

    // D-073: invalid tag (a zero byte, and all-zero) → EINVAL, not EINVFN.
    check(ssystem(S_SETCOOKIE, 0x0041_4243, 1) == EINVAL, "Setcookie(invalid tag) = EINVAL (-25)");
    check(ssystem(S_SETCOOKIE, 0, 1) == EINVAL, "Setcookie(tag 0) = EINVAL (-25)");

    // D-073: delete of an absent tag → EINVAL, not EERROR.
    check(ssystem(S_DELCOOKIE, ZZZZ, 0) == EINVAL, "Delcookie(unknown) = EINVAL (-25)");

    // Round-trip: Setcookie adds; Getcookie reads it back; capacity is
    // fixed (still 32) regardless of occupancy; Delcookie removes it.
    check(ssystem(S_SETCOOKIE, TEST, 0xCAFE) == E_OK, "Setcookie(TEST) = E_OK");
    check(ssystem(S_GETCOOKIE, TEST, 0) == 0xCAFE, "Getcookie(TEST) = 0xCAFE");
    check(ssystem(S_GETCOOKIE, 0, 0) == JAR_CAP, "Getcookie(0) = capacity 32 (after add)");
    check(ssystem(S_DELCOOKIE, TEST, 0) == E_OK, "Delcookie(TEST) = E_OK");
    check(ssystem(S_GETCOOKIE, TEST, 0) == EERROR, "Getcookie(TEST) after del = EERROR (-1)");

    // Unimplemented mode still → EINVFN (-32).
    check(ssystem(99, 0, 0) == EINVFN, "Ssystem(unimplemented mode) = EINVFN (-32)");

    let passed = total - fails;
    if fails == 0 {
        kprintln!("[cookie] === Cookie jar PASS ({}/{}) ===", passed, total);
    } else {
        kprintln!("[cookie] === Cookie jar FAILED ({}/{}) ===", passed, total);
    }
}
