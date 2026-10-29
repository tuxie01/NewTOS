//! BIOS01 / BIOS-STOR-001 — the `forcechange` handshake: writing logical
//! sector 0 forces a media change.
//!
//! Spec: `docs/api/bios/04-rwabs.md` (edge case 2, the order table of the
//! logical path, verification criteria), `docs/api/bios/09-mediach.md` (edge
//! case 3), `docs/api/bios/07-getbpb.md` (the reset at `blkdev.c:589-590`).
//! EmuTOS: `references/emutos/bios/blkdev.c:450-453` (test, arm),
//! `:473-477` (the media-change test that fails the arming write itself),
//! `:802-803` (`Mediach`), `:590` (`Getbpb` clears it).
//!
//! The case ids are the rows of `tosdump/cases/bios-r2.tsv` (W01-W11,
//! X12-X16), whose oracle answers are in
//! `tosdump/reference/EMUTOS-1a7484ab-BIOS-r2-disk.BIN`; the cases without an
//! id are consequences of the same check order (`04-rwabs.md`, "Consequences
//! (read from the order …)").

mod fatvol;

use fatvol::*;
use std::sync::atomic::Ordering;
use tos_api::trap::{TrapArgs, TrapClass};

const C: u64 = 2;
const E_CHNG: i64 = -14;
const ESECNF: i64 = -8;
const EUNDEV: i64 = -15;

const RW_WRITE: u64 = 1;
const RW_NOMEDIACH: u64 = 2;
const RW_NOTRANSLATE: u64 = 8;
/// The SD card's physical unit for `RW_NOTRANSLATE` (BIOS-STOR-002).
const SD_UNIT: u64 = 26;

fn bios(func: u64, regs: [u64; 8]) -> i64 {
    tos_api::bios::dispatch(&TrapArgs {
        regs,
        func,
        svc_imm: TrapClass::Bios as u16,
        space: tos_api::uaccess::Space::Kernel,
    })
}

/// `Rwabs(r_w, buf, count, first, drive, lfirst)`.
fn rwabs(r_w: u64, buf: *mut u8, count: u64, first: i64, drive: u64, lfirst: u64) -> i64 {
    bios(0x04, [r_w, buf as u64, count, first as u64, drive, lfirst, 0, 0])
}

fn mediach(drive: u64) -> i64 {
    bios(0x09, [drive, 0, 0, 0, 0, 0, 0, 0])
}

fn getbpb(drive: u64) -> i64 {
    bios(0x07, [drive, 0, 0, 0, 0, 0, 0, 0])
}

/// The test drive with no pending change: the tests of this binary share the
/// BIOS statics and run in any order, so each starts from a `Getbpb`.
fn settled() -> (std::sync::MutexGuard<'static, ()>, &'static DiskHost) {
    let (g, h) = fs();
    assert_ne!(getbpb(C), 0, "Getbpb(C:)");
    assert_eq!(mediach(C), 0, "C: settled");
    (g, h)
}

/// Poisoned 512-byte buffer — what a request that transfers nothing leaves.
fn poison() -> Vec<u8> {
    vec![0xA5; SECTOR_BYTES]
}

// ── the measured sequence, W01-W08 ───────────────────────────────────────

#[test]
fn w01_w08_sector0_write_fails_e_chng_writes_nothing_until_getbpb() {
    let (_g, h) = settled();
    let boot = part_lba(0);
    let before = h.read_sector(boot);
    let writes = h.disk.writes.load(Ordering::Relaxed);

    // W01: the write of sector 0 arms forcechange and fails the media-change
    // test of the same call: E_CHNG, nothing transferred. The buffer holds
    // other bytes than the disk, so a transfer would show in the image.
    let mut other = before;
    other[0x100] ^= 0xFF;
    assert_eq!(rwabs(RW_WRITE, other.as_mut_ptr(), 1, 0, C, 0), E_CHNG, "W01");
    assert_eq!(h.read_sector(boot), before, "W01 writes nothing");
    assert_eq!(h.disk.writes.load(Ordering::Relaxed), writes, "W01: no device write");

    // W02, W05: Mediach reports MEDIACHANGE, each time.
    assert_eq!(mediach(C), 2, "W02");

    // W03, W04: every logical request fails E_CHNG, RW_NOMEDIACH included;
    // the buffer is untouched.
    let mut b = poison();
    assert_eq!(rwabs(0, b.as_mut_ptr(), 1, 1, C, 0), E_CHNG, "W03");
    assert_eq!(b, poison(), "W03 buffer untouched");
    assert_eq!(rwabs(RW_NOMEDIACH, b.as_mut_ptr(), 1, 1, C, 0), E_CHNG, "W04");
    assert_eq!(b, poison(), "W04 buffer untouched");
    assert_eq!(mediach(C), 2, "W05");

    // W06: Getbpb returns the BPB and clears the state; W07, W08.
    assert_ne!(getbpb(C), 0, "W06");
    assert_eq!(mediach(C), 0, "W07");
    assert_eq!(rwabs(0, b.as_mut_ptr(), 1, 1, C, 0), 0, "W08");
    assert_eq!(b[..], h.read_sector(part_lba(1))[..], "W08 data");
}

// ── another sector does not arm, W09-W11 ─────────────────────────────────

#[test]
fn w09_w11_writing_another_sector_does_not_arm() {
    let (_g, h) = settled();
    // The last sector of C: — the partition length from the MBR.
    let mbr = h.read_sector(0);
    let count = u32::from_le_bytes([mbr[0x1CA], mbr[0x1CB], mbr[0x1CC], mbr[0x1CD]]) as u64;
    let rel = count - 1;
    let mut b = poison();
    assert_eq!(rwabs(RW_NOMEDIACH, b.as_mut_ptr(), 1, -1, C, rel), 0);
    assert_eq!(rwabs(RW_WRITE, b.as_mut_ptr(), 1, -1, C, rel), 0, "W09");
    assert_eq!(mediach(C), 0, "W10");
    assert_eq!(rwabs(0, b.as_mut_ptr(), 1, 1, C, 0), 0, "W11");
}

// ── sector 0 with RW_NOMEDIACH, X12-X16 ──────────────────────────────────

#[test]
fn x12_x16_sector0_write_with_nomediach_writes_and_arms() {
    let (_g, h) = settled();
    let boot = part_lba(0);
    let orig = h.read_sector(boot);

    // X12: written (the changed byte reaches the image) and 0 …
    let mut changed = orig;
    changed[0x100] ^= 0xFF;
    assert_eq!(rwabs(RW_WRITE | RW_NOMEDIACH, changed.as_mut_ptr(), 1, 0, C, 0), 0, "X12");
    assert_eq!(h.read_sector(boot), changed, "X12 reached the image");
    // … and armed: X13, X14.
    assert_eq!(mediach(C), 2, "X13");
    let mut b = poison();
    assert_eq!(rwabs(RW_NOMEDIACH, b.as_mut_ptr(), 1, 1, C, 0), E_CHNG, "X14");
    assert_eq!(b, poison(), "X14 buffer untouched");
    // The restore itself is refused while armed — RW_NOMEDIACH does not get
    // past step 5.
    let mut o = orig;
    assert_eq!(rwabs(RW_WRITE | RW_NOMEDIACH, o.as_mut_ptr(), 1, 0, C, 0), E_CHNG);
    assert_eq!(h.read_sector(boot), changed);

    // X15, X16.
    assert_ne!(getbpb(C), 0, "X15");
    assert_eq!(mediach(C), 0, "X16");

    // Put the image back (arms again), and settle.
    assert_eq!(rwabs(RW_WRITE | RW_NOMEDIACH, o.as_mut_ptr(), 1, 0, C, 0), 0);
    assert_eq!(h.read_sector(boot), orig);
    assert_eq!(mediach(C), 2);
    assert_ne!(getbpb(C), 0);
    assert_eq!(mediach(C), 0);
}

// ── consequences of the check order (04-rwabs.md, order table) ───────────

#[test]
fn count_zero_is_not_exempt() {
    let (_g, _h) = settled();
    let mut b = poison();
    // A count-0 write of sector 0 arms, then fails step 10 …
    assert_eq!(rwabs(RW_WRITE, b.as_mut_ptr(), 0, 0, C, 0), E_CHNG);
    assert_eq!(mediach(C), 2);
    // … and an armed drive refuses a count-0 request (step 5, edge case 7).
    assert_eq!(rwabs(0, b.as_mut_ptr(), 0, 0, C, 0), E_CHNG);
    assert_ne!(getbpb(C), 0);
    // With RW_NOMEDIACH the count-0 write of sector 0 returns 0 and arms.
    assert_eq!(rwabs(RW_WRITE | RW_NOMEDIACH, b.as_mut_ptr(), 0, 0, C, 0), 0);
    assert_eq!(mediach(C), 2);
    assert_ne!(getbpb(C), 0);
    assert_eq!(mediach(C), 0);
}

#[test]
fn arming_precedes_a_failing_range_check() {
    let (_g, h) = settled();
    let mbr = h.read_sector(0);
    let count = u32::from_le_bytes([mbr[0x1CA], mbr[0x1CB], mbr[0x1CC], mbr[0x1CD]]) as u64;
    let writes = h.disk.writes.load(Ordering::Relaxed);
    // Sector 0 with a count past the end: ESECNF (step 8) — but step 6 armed.
    let mut big = vec![0u8; (count as usize + 1) * SECTOR_BYTES];
    assert_eq!(
        rwabs(RW_WRITE | RW_NOMEDIACH, big.as_mut_ptr(), count + 1, 0, C, 0),
        ESECNF
    );
    assert_eq!(h.disk.writes.load(Ordering::Relaxed), writes, "nothing written");
    assert_eq!(mediach(C), 2);
    assert_ne!(getbpb(C), 0);
}

#[test]
fn only_a_logical_write_starting_at_sector_0_arms() {
    let (_g, h) = settled();
    let mut b = poison();
    // A read of sector 0 does not arm.
    assert_eq!(rwabs(0, b.as_mut_ptr(), 1, 0, C, 0), 0);
    assert_eq!(mediach(C), 0);
    // A physical (RW_NOTRANSLATE) write of the partition's first sector does
    // not arm: the arming is in the logical branch. Write back what is there.
    let abs = part_lba(0);
    let mut s = h.read_sector(abs);
    assert_eq!(rwabs(RW_NOTRANSLATE | RW_WRITE, s.as_mut_ptr(), 1, abs as i64, SD_UNIT, 0), 0);
    assert_eq!(mediach(C), 0);
    // A logical write starting at sector 1 does not arm.
    let mut s1 = h.read_sector(part_lba(1));
    assert_eq!(rwabs(RW_WRITE, s1.as_mut_ptr(), 1, 1, C, 0), 0);
    assert_eq!(mediach(C), 0);
}

#[test]
fn invalid_drives_stay_eundev_while_c_is_armed() {
    let (_g, _h) = settled();
    let mut b = poison();
    assert_eq!(rwabs(RW_WRITE, b.as_mut_ptr(), 0, 0, C, 0), E_CHNG);
    assert_eq!(mediach(C), 2);
    // An absent drive is still EUNDEV, for Mediach and Rwabs (step 3 first).
    assert_eq!(mediach(3), EUNDEV);
    assert_eq!(rwabs(0, b.as_mut_ptr(), 1, 0, 3, 0), EUNDEV);
    assert_eq!(mediach(16), EUNDEV);
    assert_ne!(getbpb(C), 0);
    assert_eq!(mediach(C), 0);
}

#[test]
fn a_getbpb_that_returns_null_clears_it_too() {
    let (_g, h) = settled();
    let boot = part_lba(0);
    let orig = h.read_sector(boot);
    // A boot sector Getbpb refuses: sectors/FAT 0 (blkdev.c:736-742).
    let mut bad = orig;
    bad[0x16] = 0;
    bad[0x17] = 0;
    assert_eq!(rwabs(RW_WRITE | RW_NOMEDIACH, bad.as_mut_ptr(), 1, 0, C, 0), 0);
    assert_eq!(mediach(C), 2);
    assert_eq!(getbpb(C), 0, "the refused boot sector");
    assert_eq!(mediach(C), 0, "cleared before the boot-sector test (blkdev.c:589-590)");
    // The refused Getbpb left the BPB invalid (recsiz 0, blkdev.c:601,
    // 736-741): no logical way back (BIOS-STOR-006) ...
    let mut o = orig;
    assert_eq!(rwabs(RW_WRITE | RW_NOMEDIACH, o.as_mut_ptr(), 1, 0, C, 0), ESECNF);
    assert_eq!(h.read_sector(boot), bad);
    // ... so restore through the physical unit, which never arms.
    assert_eq!(
        rwabs(RW_NOTRANSLATE | RW_WRITE, o.as_mut_ptr(), 1, -1, SD_UNIT, boot),
        0
    );
    assert_eq!(h.read_sector(boot), orig);
    assert_eq!(mediach(C), 0);
    assert_ne!(getbpb(C), 0);
}

// ── the GEMDOS side: a log-in is a Getbpb ────────────────────────────────

#[test]
fn a_bdos_log_in_clears_it_like_getbpb() {
    let (_g, _h) = settled();
    // Log every drive out, so the next GEMDOS access to C: logs it in
    // (EmuTOS `ckdrv` calls `Getbpb` for a drive not in `drvsel`,
    // bdos/fsdrive.c:94-105).
    assert_eq!(tos_api::bios::mount(), 1);
    let mut b = poison();
    assert_eq!(rwabs(RW_WRITE | RW_NOMEDIACH, b.as_mut_ptr(), 0, 0, C, 0), 0);
    assert_eq!(mediach(C), 2);
    let mut dta = Dta::new();
    assert_eq!(gemdos1(FSETDTA, dta.ptr()), E_OK);
    let star = cpath("C:\\*.*");
    assert_eq!(gemdos2(FSFIRST, star.as_ptr() as u64, FA_ANY), E_OK);
    assert_eq!(mediach(C), 0, "the log-in's Getbpb cleared forcechange");
}
