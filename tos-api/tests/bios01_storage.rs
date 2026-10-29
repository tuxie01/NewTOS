//! BIOS01 (rest) — the BIOS storage findings BIOS-STOR-002 … -007 of the
//! audit `docs/audits/2026-10-28-bios-storage.md`.
//!
//! Spec: `docs/api/bios/04-rwabs.md` (order table of the logical path, edge
//! cases 7 and 8, "Decisions"), `docs/api/bios/07-getbpb.md` (edge cases 8
//! and 9). EmuTOS: `references/emutos/bios/blkdev.c:406-540`
//! (`blkdev_rwabs`), `:569-777` (`blkdev_getbpb`), `:112-163`
//! (`pun_info_setup`).
//!
//! The case ids are the rows of the DUMPBIOS disk tables whose oracle answers
//! are pinned in `tosdump/reference/EMUTOS-1a7484ab-BIOS-r2*-disk.BIN`:
//! R09/R10, F03/F03b (`bios-r2.tsv`), X01* (`bios-r2-x004.tsv`), X06*
//! (`bios-r2-x005.tsv`), X07*/X08* (`bios-r2-x006.tsv`), P* (`bios-r2-phys.tsv`).
//! The host image here is `fatvol`'s (partition at LBA 64), not the test
//! image of those tables, so the sector numbers differ; the answers do not.

mod fatvol;

use fatvol::*;
use std::sync::atomic::Ordering;
use tos_api::trap::{TrapArgs, TrapClass};

const C: u64 = 2;
const ESECNF: i64 = -8;
const EUNDEV: i64 = -15;

const RW_WRITE: u64 = 1;
const RW_NOMEDIACH: u64 = 2;
const RW_NOTRANSLATE: u64 = 8;
/// The SD card's physical unit (BIOS-STOR-002): `2 + 8*SDMMC_BUS + 0`.
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
fn rwabs(r_w: u64, buf: *mut u8, count: u64, first: i64, drive: u64, lfirst: i64) -> i64 {
    bios(0x04, [r_w, buf as u64, count, first as u64, drive, lfirst as u64, 0, 0])
}

fn mediach(drive: u64) -> i64 {
    bios(0x09, [drive, 0, 0, 0, 0, 0, 0, 0])
}

fn getbpb(drive: u64) -> i64 {
    bios(0x07, [drive, 0, 0, 0, 0, 0, 0, 0])
}

/// The nine BPB words behind a `Getbpb` pointer.
fn bpb_words(p: i64) -> [u16; 9] {
    assert_ne!(p, 0);
    // SAFETY: an in-kernel caller's `Getbpb` returns the kernel address of
    // the drive's 18-byte BPB slot, which lives as long as the test binary.
    unsafe { core::ptr::read_unaligned(p as *const [u16; 9]) }
}

/// The partition length of C: and the device size, from the MBR.
fn sizes(h: &DiskHost) -> (u64, u64) {
    let mbr = h.read_sector(0);
    let start = u32::from_le_bytes([mbr[0x1C6], mbr[0x1C7], mbr[0x1C8], mbr[0x1C9]]) as u64;
    let count = u32::from_le_bytes([mbr[0x1CA], mbr[0x1CB], mbr[0x1CC], mbr[0x1CD]]) as u64;
    (count, start + count)
}

/// The test drive settled: its BPB valid and no change pending. The tests of
/// this binary share the BIOS statics and run in any order.
fn settled() -> (std::sync::MutexGuard<'static, ()>, &'static DiskHost) {
    let (g, h) = fs();
    assert_ne!(getbpb(C), 0, "Getbpb(C:)");
    assert_eq!(mediach(C), 0, "C: settled");
    (g, h)
}

fn poison() -> Vec<u8> {
    vec![0xA5; SECTOR_BYTES]
}

/// Write C:'s boot sector physically (never arms `forcechange`, reaches the
/// disk whatever the drive's BPB state), as the closing tables' `+set` does
/// logically.
fn put_boot(h: &DiskHost, bytes: &[u8; SECTOR_BYTES]) {
    let mut b = *bytes;
    let lba = part_lba(0) as i64;
    assert_eq!(rwabs(RW_NOTRANSLATE | RW_WRITE, b.as_mut_ptr(), 1, -1, SD_UNIT, lba), 0);
    assert_eq!(h.read_sector(part_lba(0)), *bytes);
}

// ── BIOS-STOR-003: count 0 is range-checked ───────────────────────────────

#[test]
fn stor003_count_zero_is_range_checked() {
    let (_g, h) = settled();
    let (psize, _) = sizes(h);
    let writes = h.disk.writes.load(Ordering::Relaxed);
    let mut b = poison();
    // R09: in range -> 0, nothing transferred.
    assert_eq!(rwabs(0, b.as_mut_ptr(), 0, 0, C, 0), 0, "R09");
    // At the end exactly: lrecnr + 0 > size is false -> 0 (`>`, blkdev.c:461-462).
    assert_eq!(rwabs(0, b.as_mut_ptr(), 0, -1, C, psize as i64), 0, "count 0 at the end");
    // R10: past the end -> ESECNF.
    assert_eq!(rwabs(0, b.as_mut_ptr(), 0, -1, C, psize as i64 + 10), ESECNF, "R10");
    assert_eq!(rwabs(0, b.as_mut_ptr(), 0, -1, C, psize as i64 + 1), ESECNF);
    // A negative lrecnr -> ESECNF with count 0 too (blkdev.c:461).
    assert_eq!(rwabs(0, b.as_mut_ptr(), 0, -1, C, -5), ESECNF);
    // A count-0 write past the end transfers nothing either.
    assert_eq!(rwabs(RW_WRITE, b.as_mut_ptr(), 0, -1, C, psize as i64 + 10), ESECNF);
    assert_eq!(b, poison(), "buffer untouched");
    assert_eq!(h.disk.writes.load(Ordering::Relaxed), writes, "nothing written");
}

// ── BIOS-STOR-007: a NULL buffer on A:/B: returns 0 ───────────────────────

#[test]
fn stor007_null_buffer_on_a_or_b_returns_zero() {
    let (_g, h) = settled();
    let writes = h.disk.writes.load(Ordering::Relaxed);
    let null = core::ptr::null_mut();
    // F03, F03b.
    assert_eq!(rwabs(0, null, 2, 0, 0, 0), 0, "F03");
    assert_eq!(rwabs(0, null, 0, 0, 0, 0), 0, "F03b");
    // B:, a write, the physical mode: step 1 is ahead of all of them.
    assert_eq!(rwabs(0, null, 1, 0, 1, 0), 0);
    assert_eq!(rwabs(RW_WRITE, null, 1, 0, 1, 0), 0);
    assert_eq!(rwabs(RW_NOTRANSLATE, null, 1, 0, 0, 0), 0);
    assert_eq!(h.disk.writes.load(Ordering::Relaxed), writes, "no I/O");
    // A: and B: still do not exist.
    assert_eq!(mediach(0), EUNDEV);
    assert_eq!(mediach(1), EUNDEV);
    assert_eq!(getbpb(0), 0);
    let mut b = poison();
    assert_eq!(rwabs(0, b.as_mut_ptr(), 1, 0, 0, 0), EUNDEV, "A: with a buffer");
    assert_eq!(rwabs(0, b.as_mut_ptr(), 1, 0, 1, 0), EUNDEV, "B: with a buffer");
    // Not a floppy: a NULL buffer on an absent drive or drive -1 is EUNDEV.
    assert_eq!(rwabs(0, null, 1, 0, 5, 0), EUNDEV);
    assert_eq!(rwabs(0, null, 1, 0, 0xFFFF, 0), EUNDEV, "drive -1");
    // C: is untouched by the feature.
    assert_eq!(mediach(C), 0);
}

// ── BIOS-STOR-006: Rwabs after a failed Getbpb ────────────────────────────

#[test]
fn stor006_logical_rwabs_after_a_failed_getbpb_is_esecnf() {
    let (_g, h) = settled();
    let (psize, _) = sizes(h);
    let orig = h.read_sector(part_lba(0));
    // X07: sectors per FAT 0, written with RW_NOMEDIACH (arms forcechange).
    let mut bad = orig;
    bad[0x16] = 0;
    bad[0x17] = 0;
    let mut w = bad;
    assert_eq!(rwabs(RW_WRITE | RW_NOMEDIACH, w.as_mut_ptr(), 1, 0, C, 0), 0, "X07");
    assert_eq!(mediach(C), 2, "X07a");
    assert_eq!(getbpb(C), 0, "X07b: FAT32 test, recsiz marked 0");
    assert_eq!(mediach(C), 0, "X07c");
    // X08-X08e: every logical request is ESECNF, buffer untouched.
    let mut b = poison();
    assert_eq!(rwabs(0, b.as_mut_ptr(), 1, 1, C, 0), ESECNF, "X08");
    assert_eq!(rwabs(RW_NOMEDIACH, b.as_mut_ptr(), 1, 0, C, 0), ESECNF, "X08a");
    assert_eq!(rwabs(0, b.as_mut_ptr(), 1, -1, C, psize as i64 - 1), ESECNF, "X08b");
    assert_eq!(rwabs(0, b.as_mut_ptr(), 0, 0, C, 0), ESECNF, "X08c count 0");
    assert_eq!(b, poison(), "buffer untouched");
    assert_eq!(getbpb(C), 0, "X08d");
    assert_eq!(rwabs(0, b.as_mut_ptr(), 1, 1, C, 0), ESECNF, "X08e");
    // A write is refused the same way (step 4 is ahead of step 6).
    assert_eq!(rwabs(RW_WRITE, b.as_mut_ptr(), 1, 1, C, 0), ESECNF);
    assert_eq!(mediach(C), 0, "step 4 is ahead of the arming");
    // Absent drives stay EUNDEV (step 3 first).
    assert_eq!(rwabs(0, b.as_mut_ptr(), 1, 0, 5, 0), EUNDEV);
    // The physical unit is not affected.
    assert_eq!(rwabs(RW_NOTRANSLATE, b.as_mut_ptr(), 1, -1, SD_UNIT, part_lba(1) as i64), 0);
    assert_eq!(b[..], h.read_sector(part_lba(1))[..]);
    // Restore physically; a successful Getbpb makes the drive valid again.
    put_boot(h, &orig);
    assert_eq!(rwabs(0, b.as_mut_ptr(), 1, 1, C, 0), ESECNF, "still invalid before Getbpb");
    assert_ne!(getbpb(C), 0);
    assert_eq!(rwabs(0, b.as_mut_ptr(), 1, 1, C, 0), 0);
}

#[test]
fn stor006_a_boot_scan_that_refuses_leaves_the_bpb_invalid() {
    let (_g, h) = settled();
    let orig = h.read_sector(part_lba(0));
    let mut bad = orig;
    bad[0x16] = 0;
    bad[0x17] = 0;
    put_boot(h, &bad);
    // A new partition scan runs the boot scan's Getbpb (pun_info_setup,
    // blkdev.c:145): the refused boot sector leaves C:'s BPB invalid.
    assert_eq!(tos_api::bios::mount(), 1);
    let mut b = poison();
    assert_eq!(rwabs(0, b.as_mut_ptr(), 1, 1, C, 0), ESECNF);
    put_boot(h, &orig);
    assert_eq!(tos_api::bios::mount(), 1);
    assert_eq!(rwabs(0, b.as_mut_ptr(), 1, 1, C, 0), 0, "the boot scan logged it in");
}

// ── BIOS-STOR-004: the sector-size limit after boot ───────────────────────

#[test]
fn stor004_getbpb_refuses_a_sector_size_above_the_boot_maximum() {
    let (_g, h) = settled();
    let orig = h.read_sector(part_lba(0));
    // X01: bps 512 -> 1024.
    let mut big = orig;
    big[0x0C] = 0x04;
    let mut w = big;
    assert_eq!(rwabs(RW_WRITE | RW_NOMEDIACH, w.as_mut_ptr(), 1, 0, C, 0), 0, "X01");
    assert_eq!(h.read_sector(part_lba(0)), big);
    assert_eq!(mediach(C), 2, "X01a");
    // X01b: the boot scan logged in 512-byte sectors only, so the limit is
    // max(512, CONF_LOGSEC_SIZE) = 512 (blkdev.c:159, 642-646).
    assert_eq!(getbpb(C), 0, "X01b");
    assert_eq!(mediach(C), 0, "X01c");
    let mut b = poison();
    assert_eq!(rwabs(0, b.as_mut_ptr(), 1, 1, C, 0), ESECNF, "X01d");
    assert_eq!(rwabs(RW_NOMEDIACH, b.as_mut_ptr(), 1, 0, C, 0), ESECNF, "X01e");
    assert_eq!(getbpb(C), 0, "X01g");
    assert_eq!(b, poison());

    // The limit is the boot scan's: a scan that meets the 1024-byte sector
    // logs it in (limit MAX_LOGSEC_SIZE while it runs, blkdev.c:122) and
    // raises the run-time limit to 1024 (blkdev.c:147-150, 159).
    assert_eq!(tos_api::bios::mount(), 1);
    let p = getbpb(C);
    assert_eq!(bpb_words(p)[0], 1024, "logged in with 1024-byte sectors");
    // ... until a scan that sees 512 again lowers it (restore first).
    put_boot(h, &orig);
    assert_eq!(tos_api::bios::mount(), 1);
    assert_eq!(bpb_words(getbpb(C))[0], 512);
    put_boot(h, &big);
    assert_eq!(getbpb(C), 0, "limit back to 512");
    put_boot(h, &orig);
    assert_ne!(getbpb(C), 0);
    assert_eq!(rwabs(0, b.as_mut_ptr(), 1, 1, C, 0), 0);
}

// ── BIOS-STOR-005: tot16 = tot32 = 0 ──────────────────────────────────────

#[test]
fn stor005_no_total_sectors_logs_in_with_zero_clusters() {
    let (_g, h) = settled();
    let (psize, _) = sizes(h);
    let orig = h.read_sector(part_lba(0));
    let good = bpb_words(getbpb(C));
    // X06: tot16 (0x13) = tot32 (0x20) = 0.
    let mut zero = orig;
    for o in [0x13, 0x14, 0x20, 0x21, 0x22, 0x23] {
        zero[o] = 0;
    }
    let mut w = zero;
    assert_eq!(rwabs(RW_WRITE | RW_NOMEDIACH, w.as_mut_ptr(), 1, 0, C, 0), 0, "X06");
    assert_eq!(mediach(C), 2, "X06a");
    // X06b: a BPB, numcl 0 and no B_16 (blkdev.c:730-733, 743-752); the other
    // fields are the boot sector's.
    let got = bpb_words(getbpb(C));
    assert_eq!(got[..7], good[..7], "X06b recsiz..datrec");
    assert_eq!(got[7], 0, "X06b numcl 0");
    assert_eq!(got[8], 0, "X06b no B_16");
    assert_eq!(mediach(C), 0, "X06c");
    // X06d-X06f: recsiz stays 512, the logical path works, the range is the
    // partition's, not the BPB's.
    let mut b = poison();
    assert_eq!(rwabs(0, b.as_mut_ptr(), 1, 0, C, 0), 0, "X06d");
    assert_eq!(b[..], zero[..]);
    assert_eq!(rwabs(0, b.as_mut_ptr(), 1, 1, C, 0), 0, "X06e");
    assert_eq!(rwabs(0, b.as_mut_ptr(), 1, -1, C, psize as i64 - 1), 0, "X06f");
    // X06g-X06k: the restore, logically.
    let mut o = orig;
    assert_eq!(rwabs(RW_WRITE | RW_NOMEDIACH, o.as_mut_ptr(), 1, 0, C, 0), 0, "X06g");
    assert_eq!(mediach(C), 2, "X06h");
    assert_eq!(bpb_words(getbpb(C)), good, "X06i");
    assert_eq!(mediach(C), 0, "X06j");
    assert_eq!(h.read_sector(part_lba(0)), orig);
}

// ── BIOS-STOR-002: RW_NOTRANSLATE addresses physical unit 26 ──────────────

#[test]
fn stor002_physical_mode_addresses_unit_26() {
    let (_g, h) = settled();
    let (_, total) = sizes(h);
    let mut b = poison();
    // P02: physical sector 0 is the MBR.
    assert_eq!(rwabs(RW_NOTRANSLATE, b.as_mut_ptr(), 1, 0, SD_UNIT, 0), 0, "P02");
    assert_eq!(b[..], h.read_sector(0)[..]);
    // P06: the partition's first sector = C:'s boot sector.
    assert_eq!(
        rwabs(RW_NOTRANSLATE, b.as_mut_ptr(), 1, -1, SD_UNIT, part_lba(0) as i64),
        0,
        "P06"
    );
    assert_eq!(b[..], h.read_sector(part_lba(0))[..]);
    // P08, P09: RW_NOMEDIACH, RW_NORETRIES accepted.
    assert_eq!(rwabs(RW_NOTRANSLATE | 2, b.as_mut_ptr(), 1, 0, SD_UNIT, 0), 0, "P08");
    assert_eq!(rwabs(RW_NOTRANSLATE | 4, b.as_mut_ptr(), 1, 0, SD_UNIT, 0), 0, "P09");
    // P11: count 0 -> 0.
    assert_eq!(rwabs(RW_NOTRANSLATE, b.as_mut_ptr(), 0, -1, SD_UNIT, 63), 0, "P11");
    // P12, P13: negative / past any device -> ESECNF.
    assert_eq!(rwabs(RW_NOTRANSLATE, b.as_mut_ptr(), 1, -1, SD_UNIT, -5), ESECNF, "P12");
    assert_eq!(
        rwabs(RW_NOTRANSLATE, b.as_mut_ptr(), 1, -1, SD_UNIT, 0x7FFF_FFF0),
        ESECNF,
        "P13"
    );
    // The device bound is `>=` (blkdev.c:488-489): the last device sector is
    // refused, the one before it is not (spec edge case 8).
    assert_eq!(
        rwabs(RW_NOTRANSLATE, b.as_mut_ptr(), 1, -1, SD_UNIT, total as i64 - 1),
        ESECNF
    );
    assert_eq!(rwabs(RW_NOTRANSLATE, b.as_mut_ptr(), 1, -1, SD_UNIT, total as i64 - 2), 0);
    // Every other unit is EUNDEV — the drive letters, the other buses' first
    // units (ACSI 2, SCSI 10, IDE 18), the floppies, 27, -1.
    for unit in [C, 3, 10, 18, 0, 1, 25, 27, 0xFFFF] {
        let mut p = poison();
        assert_eq!(rwabs(RW_NOTRANSLATE, p.as_mut_ptr(), 1, 0, unit, 0), EUNDEV, "unit {unit}");
        assert_eq!(p, poison(), "unit {unit}: buffer untouched");
    }
    // Logical mode does not know unit 26: there is no drive 26.
    assert_eq!(rwabs(0, b.as_mut_ptr(), 1, 0, SD_UNIT, 0), EUNDEV);
}
