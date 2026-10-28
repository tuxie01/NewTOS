//! A failed sector access reaches the application, and no sector is lost
//! when its write-back fails (TASK/HW03).
//!
//! **Where this comes from.** `docs/api/hw/emmc-audit.md`, findings 3, 4
//! and 5, on the Pi measurement of 2026-09-25: twelve of twenty-three
//! sector writes came back with an error and nothing above the driver
//! learned of it. `Fwrite` reported the full byte count, `Fclose` reported
//! `E_OK`, and the file was correct only because the block cache silently
//! repeated the write.
//!
//! **What is asserted here.** The whole chain from the block device to the
//! GEMDOS return value:
//!
//! * finding 5 — a device error becomes `EWRITF` (-10) / `EREADF` (-11) at
//!   the application, instead of a short count or `E_OK`
//!   (`docs/api/gemdos/file/40-fwrite.md` and `3f-fread.md`
//!   §"Error conditions"; `docs/api/gemdos/_dispatcher.md` §8: the hard
//!   error "is returned to caller as if it were the function's return
//!   value");
//! * finding 4 — the sector whose write-back failed is **still in the
//!   cache**, still dirty, and lands on the card once the fault goes away;
//! * HW03 point 4 — the repetition is bounded and counted, not endless and
//!   silent.
//!
//! The card's own refusal (the R1 status word of finding 3) cannot be
//! produced on the host: the `RamDisk` stands in for it, refusing the
//! access with the TOS code the real driver derives from R1
//! (`docs/hardware/emmc-sd.md` §5). What is driven here is everything above
//! the driver; the driver's own R1 evaluation is driven in QEMU
//! (`measurements/measurement-2026-09-27-hw03`).

mod fatvol;

use fatvol::*;

/// Make sure a particular sector is no longer cached when the next access
/// asks for it. Since FS08 the cache has two lists (FAT and data, EmuTOS
/// `bufl[BI_FAT]`/`bufl[BI_DATA]`), so both are turned over: `Dfree` reads
/// every sector of the FAT (20 in this image) and a search for a name that
/// does not exist reads every sector of the root directory (32) — each more
/// than a list has slots. The search's own result is not asserted: with a
/// sector whose write-back fails in the data list it may report that fault.
fn turn_the_cache_over() {
    let mut buf = [0u8; 16];
    assert_eq!(gemdos2(0x36, buf.as_mut_ptr() as u64, 3), E_OK, "Dfree C:");
    let mut dta = Dta::new();
    let _ = gemdos1(FSETDTA, dta.ptr());
    let pat = cpath("C:\\NOSUCH.XYZ");
    let _ = gemdos2(FSFIRST, pat.as_ptr() as u64, FA_ANY);
}

/// Create `name`, write `data`, close — and return the data cluster the
/// file was given, read straight out of the root directory.
fn make_file(host: &DiskHost, name: &str, data: &[u8]) -> u32 {
    let path = cpath(name);
    let h = gemdos2(FCREATE, path.as_ptr() as u64, 0);
    assert!(h >= 0, "Fcreate {name} -> {h}");
    let n = call(FWRITE, [h as u64, data.len() as u64, data.as_ptr() as u64, 0, 0, 0, 0, 0]);
    assert_eq!(n, data.len() as i32, "Fwrite {name}");
    assert_eq!(gemdos1(FCLOSE, h as u64), E_OK, "Fclose {name}");

    // The 8.3 entry's first-cluster word sits at offset 0x1A.
    let short: Vec<u8> = {
        let (stem, ext) = name.trim_start_matches("C:\\").split_once('.').unwrap();
        let mut v = stem.as_bytes().to_vec();
        v.resize(8, b' ');
        v.extend_from_slice(ext.as_bytes());
        v.resize(11, b' ');
        v
    };
    let sec = host.read_sector(root_lba());
    for slot in 0..(SECTOR_BYTES / 32) {
        let e = &sec[slot * 32..slot * 32 + 32];
        if e[..11] == short[..] {
            return u16::from_le_bytes([e[0x1A], e[0x1B]]) as u32;
        }
    }
    panic!("{name} not found in the root directory");
}

// ─────────────────────────────────────────────────────────────────────
// Finding 5 — the error reaches the application
// ─────────────────────────────────────────────────────────────────────

#[test]
fn fwrite_reports_ewritf_when_the_sector_does_not_reach_the_card() {
    let (_g, host) = fs();
    let cl = make_file(host, "C:\\WFAIL.TXT", b"first");
    let lba = cluster_lba(cl);

    let path = cpath("C:\\WFAIL.TXT");
    let h = gemdos2(FOPEN, path.as_ptr() as u64, 2);
    assert!(h >= 0, "Fopen -> {h}");

    host.disk.fail_write_at(lba, EWRITF);
    let payload = b"second";
    let rc = call(FWRITE, [h as u64, payload.len() as u64, payload.as_ptr() as u64, 0, 0, 0, 0, 0]);
    host.disk.disarm();
    // Before HW03 this returned 6 — the full byte count — for data that
    // never left the machine.
    assert_eq!(rc, EWRITF, "Fwrite must report the write fault, not a count");

    let _ = gemdos1(FCLOSE, h as u64);
}

#[test]
fn fread_reports_ereadf_when_the_sector_cannot_be_read() {
    let (_g, host) = fs();
    // `HELLO.TXT` of the fixture image lives in cluster 3.
    let lba = cluster_lba(3);
    turn_the_cache_over();

    let path = cpath("C:\\HELLO.TXT");
    let h = gemdos2(FOPEN, path.as_ptr() as u64, 0);
    assert!(h >= 0, "Fopen -> {h}");

    host.disk.fail_read_at(lba, EREADF);
    let mut buf = [0u8; 5];
    let rc = call(FREAD, [h as u64, buf.len() as u64, buf.as_mut_ptr() as u64, 0, 0, 0, 0, 0]);
    host.disk.disarm();
    assert_eq!(rc, EREADF, "Fread must report the read fault, not a short count");

    let _ = gemdos1(FCLOSE, h as u64);
}

#[test]
fn rwabs_passes_the_drivers_own_code_through() {
    let (_g, host) = fs();
    // Partition-relative sector 0 of C: is the boot sector.
    let lba = part_lba(0);
    let mut buf = [0u8; SECTOR_BYTES];

    // BIOS `Rwabs` (trap 13, function 0x04): r_w, buffer, count, first,
    // drive, lfirst — driven through the BIOS dispatcher, not the GEMDOS one.
    host.disk.fail_read_at(lba, EREADF);
    let rc = tos_api::bios::dispatch(&tos_api::trap::TrapArgs {
        regs: [0, buf.as_mut_ptr() as u64, 1, 0, 2, 0, 0, 0],
        func: 0x04,
        svc_imm: tos_api::trap::TrapClass::Bios as u16,
        space: tos_api::uaccess::Space::Kernel,
    });
    host.disk.disarm();
    assert_eq!(rc, EREADF as i64, "Rwabs read must report the driver's code");

    // A write of logical sector 0 reaches the driver only with `RW_NOMEDIACH`
    // (r_w 3): without it the write arms `forcechange` and fails `E_CHNG`
    // before any transfer (BIOS-STOR-001, `docs/api/bios/04-rwabs.md` steps
    // 6 and 10).
    host.disk.fail_write_at(lba, EWRITF);
    let rc = tos_api::bios::dispatch(&tos_api::trap::TrapArgs {
        regs: [3, buf.as_ptr() as u64, 1, 0, 2, 0, 0, 0],
        func: 0x04,
        svc_imm: tos_api::trap::TrapClass::Bios as u16,
        space: tos_api::uaccess::Space::Kernel,
    });
    host.disk.disarm();
    assert_eq!(rc, EWRITF as i64, "Rwabs write must report the driver's code");
    // The write armed `forcechange` (step 6, before the transfer failed);
    // `Getbpb` clears it, so the other tests of this binary find C: settled.
    let bpb = tos_api::bios::dispatch(&tos_api::trap::TrapArgs {
        regs: [2, 0, 0, 0, 0, 0, 0, 0],
        func: 0x07,
        svc_imm: tos_api::trap::TrapClass::Bios as u16,
        space: tos_api::uaccess::Space::Kernel,
    });
    assert_ne!(bpb, 0, "Getbpb(C:) after the failed write");
}

// ─────────────────────────────────────────────────────────────────────
// Finding 4 — the sector survives a failed write-back
// ─────────────────────────────────────────────────────────────────────

#[test]
fn a_failed_write_back_keeps_the_sector_and_lands_it_afterwards() {
    let (_g, host) = fs();
    let cl = make_file(host, "C:\\KEEP.TXT", b"old");
    let lba = cluster_lba(cl);

    let path = cpath("C:\\KEEP.TXT");
    let h = gemdos2(FOPEN, path.as_ptr() as u64, 2);
    assert!(h >= 0, "Fopen -> {h}");

    host.disk.fail_write_at(lba, EWRITF);
    let payload = b"new";
    let rc = call(FWRITE, [h as u64, payload.len() as u64, payload.as_ptr() as u64, 0, 0, 0, 0, 0]);
    assert_eq!(rc, EWRITF, "the write must be reported as failed");

    // The sector is not on the card…
    assert_eq!(&host.read_sector(lba)[..3], b"old", "the card must still hold the old bytes");
    // …and it has not been dropped either: it is still a dirty slot.
    let (_f, _r, _x, dirty) = tos_api::bdos::fs::cache::writeback_stats();
    assert!(dirty > 0, "the failed sector must still be in the cache, dirty");

    // Force the cache to look for victims all the way round: the dirty
    // slot must survive that too (before HW03 the eviction path marked it
    // clean and overwrote it — the silent loss of finding 4).
    turn_the_cache_over();
    assert_eq!(&host.read_sector(lba)[..3], b"old", "still not on the card");

    // Fault gone: the sector the cache kept is the one that lands.
    host.disk.disarm();
    assert_eq!(gemdos1(FCLOSE, h as u64), E_OK, "Fclose after the fault cleared");
    assert_eq!(&host.read_sector(lba)[..3], b"new", "the kept sector must reach the card");
}

// ─────────────────────────────────────────────────────────────────────
// HW03 point 4 — the repetition is bounded and counted
// ─────────────────────────────────────────────────────────────────────

#[test]
fn the_write_back_retry_is_bounded_and_visible() {
    use std::sync::atomic::Ordering;

    let (_g, host) = fs();
    let cl = make_file(host, "C:\\BOUND.TXT", b"x");
    let lba = cluster_lba(cl);

    let path = cpath("C:\\BOUND.TXT");
    let h = gemdos2(FOPEN, path.as_ptr() as u64, 2);
    assert!(h >= 0, "Fopen -> {h}");

    let before = tos_api::bdos::fs::cache::writeback_stats();
    host.disk.refused_writes.store(0, Ordering::Relaxed);
    host.disk.fail_write_at(lba, EWRITF);

    let payload = b"y";
    let rc = call(FWRITE, [h as u64, payload.len() as u64, payload.as_ptr() as u64, 0, 0, 0, 0, 0]);
    assert_eq!(rc, EWRITF);

    // Ten more flush-driven attempts at the same, permanently failing
    // sector. `Fdatime` (set) is a flushing call that touches nothing else
    // of interest.
    let mut td = [0u8; 4];
    for _ in 0..10 {
        let rc = call(0x57, [td.as_mut_ptr() as u64, h as u64, 1, 0, 0, 0, 0, 0]);
        assert_eq!(rc, EWRITF, "every later call must keep reporting the failure");
    }

    let refused = host.disk.refused_writes.load(Ordering::Relaxed);
    assert!(
        refused <= 3,
        "the cache must stop re-issuing the write after 3 attempts, saw {refused}"
    );
    assert!(refused >= 1, "it must have tried at least once, saw {refused}");

    let after = tos_api::bdos::fs::cache::writeback_stats();
    assert!(after.0 > before.0, "the failure counter must move");
    assert!(after.2 > before.2, "the slot must be recorded as exhausted");
    assert!(after.3 > 0, "and the sector must still be in the cache");

    // Writing the sector again is a new request and re-arms the retry —
    // the sector is not stuck for ever once the card comes back.
    host.disk.disarm();
    let rc = call(FWRITE, [h as u64, payload.len() as u64, payload.as_ptr() as u64, 0, 0, 0, 0, 0]);
    assert_eq!(rc, 1, "after the fault clears the write goes through again");
    assert_eq!(gemdos1(FCLOSE, h as u64), E_OK);
}
