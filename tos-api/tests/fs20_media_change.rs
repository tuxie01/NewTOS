//! TASK/FS20 — GEMDOS meets a media change and logs the drive in again
//! (EmuTOS `osif`'s media-change block), with the boot sector unchanged.
//!
//! Spec: `docs/api/gemdos/_state.md` §4a. EmuTOS (`references/emutos`, pin
//! `1a7484ab`): a logical `Rwabs` write of sector 0 arms `forcechange`
//! (`bios/blkdev.c:452-453`); the next buffer access of the BDOS on the drive
//! meets it — `Mediach` on a hit (`bdos/fsbuf.c:179-191`), `E_CHNG` from the
//! `Rwabs` of a miss (`bdos/fs.h:60-77`) — and `osif` frees the drive's open
//! files (`offree`), its current directories (`freetree`), its buffers
//! (`mark_bcbs_invalid`), logs it in with `Getbpb` + `log_media` and runs the
//! call again (`bdos/bdosmain.c:449-487`).
//!
//! The probe steps named here (A05-A07) are those of `tosdump/probes/gcprobe.c`,
//! measured on the oracle in `measurements/measurement-2026-10-28-fs20/RUNINFO.TXT`.
//! The image is left as it was by every test (the boot sector is written back
//! unchanged); the changed geometry is `fs20_media_change_geometry.rs`.

mod fatvol;

use std::sync::Once;

use fatvol::*;
use tos_api::bdos::fs::curdir;
use tos_api::bdos::process::{self, standin};
use tos_api::trap::{TrapArgs, TrapClass};

const C: u64 = 2;
const DFREE: u16 = 0x36;
const DSETPATH: u16 = 0x3B;
const EIHNDL: i32 = -37;

const RW_READ_NOMEDIACH: u64 = 2;
const RW_WRITE_NOMEDIACH: u64 = 3;

fn bios(func: u64, regs: [u64; 8]) -> i64 {
    tos_api::bios::dispatch(&TrapArgs {
        regs,
        func,
        svc_imm: TrapClass::Bios as u16,
        space: tos_api::uaccess::Space::Kernel,
    })
}

fn mediach(drive: u64) -> i64 {
    bios(0x09, [drive, 0, 0, 0, 0, 0, 0, 0])
}

fn getbpb(drive: u64) -> i64 {
    bios(0x07, [drive, 0, 0, 0, 0, 0, 0, 0])
}

/// `Rwabs(flag, buf, 1, 0, C:, 0)` — logical sector 0.
fn rwabs_sector0(flag: u64, buf: &mut [u8; SECTOR_BYTES]) -> i64 {
    bios(0x04, [flag, buf.as_mut_ptr() as u64, 1, 0, C, 0, 0, 0])
}

/// Probe steps A03/A04: read logical sector 0 and write it back unchanged,
/// both with `RW_NOMEDIACH`. The write goes through and arms `forcechange`.
fn arm_forcechange() {
    let mut boot = [0u8; SECTOR_BYTES];
    assert_eq!(rwabs_sector0(RW_READ_NOMEDIACH, &mut boot), 0, "A03 read sector 0");
    assert_eq!(rwabs_sector0(RW_WRITE_NOMEDIACH, &mut boot), 0, "A04 write it back");
    assert_eq!(mediach(C), 2, "A05 Mediach after the sector-0 write");
}

/// `Dfree(C:)`: `(rc, [free, total, secsiz, clsiz])`.
fn dfree() -> (i32, [u32; 4]) {
    let mut buf = [0u32; 4];
    let rc = gemdos2(DFREE, buf.as_mut_ptr() as u64, 3);
    (rc, buf)
}

static SETUP: Once = Once::new();

/// The file-system lock, C: logged in and settled (no pending change), and a
/// stand-in `run` PD for the current-directory calls.
fn settled() -> (std::sync::MutexGuard<'static, ()>, &'static DiskHost) {
    let (g, h) = fs();
    SETUP.call_once(|| {
        // SAFETY: once per binary, before any program; boot drive C: (2).
        assert!(unsafe { standin::init(2) }.is_some());
    });
    process::set_current(core::ptr::null_mut());
    assert_ne!(getbpb(C), 0, "Getbpb(C:)");
    assert_eq!(mediach(C), 0, "C: settled");
    assert_eq!(dfree().0, E_OK, "Dfree(C:) logs C: in");
    (g, h)
}

fn read_hello() -> [u8; 5] {
    let p = cpath("C:\\HELLO.TXT");
    let h = gemdos2(FOPEN, p.as_ptr() as u64, 0);
    assert!(h >= 6, "Fopen HELLO.TXT: {h}");
    let mut out = [0u8; 5];
    let n = call(FREAD, [h as u64, 5, out.as_mut_ptr() as u64, 0, 0, 0, 0, 0]);
    assert_eq!(n, 5, "Fread HELLO.TXT");
    assert_eq!(gemdos1(FCLOSE, h as u64), E_OK);
    out
}

// ── A05-A07: the first GEMDOS access after the arming re-logs the drive ──

#[test]
fn a07_a_buffer_access_relogs_and_clears_the_pending_change() {
    let (_g, _h) = settled();
    let (rc, before) = dfree();
    assert_eq!(rc, E_OK);

    arm_forcechange();
    // A06: the FAT buffers of the first Dfree are hits; their media test
    // meets the change, osif logs C: in again and runs Dfree once more.
    let (rc, after) = dfree();
    assert_eq!(rc, E_OK, "A06 Dfree(C:) after the arming");
    assert_eq!(after, before, "same boot sector, same answer");
    // A07: the re-login's Getbpb cleared forcechange.
    assert_eq!(mediach(C), 0, "A07 Mediach after that GEMDOS call");
}

#[test]
fn a_call_that_touches_no_buffer_does_not_notice_the_change() {
    let (_g, _h) = settled();
    arm_forcechange();
    // Dgetpath reads no record (xgetdir walks the DND tree in memory,
    // ckdrv(FALSE)): the change stays pending.
    let mut path = [0xAAu8; 128];
    assert_eq!(gemdos2(DGETPATH, path.as_mut_ptr() as u64, 3), E_OK);
    assert_eq!(mediach(C), 2, "still pending after Dgetpath");
    // The next buffer access takes it.
    assert_eq!(dfree().0, E_OK);
    assert_eq!(mediach(C), 0);
}

// ── offree: handles on the drive are gone (EIHNDL) ──

#[test]
fn open_handles_on_the_changed_drive_answer_eihndl() {
    let (_g, _h) = settled();
    let p = cpath("C:\\HELLO.TXT");
    let h = gemdos2(FOPEN, p.as_ptr() as u64, 0);
    assert!(h >= 6, "Fopen: {h}");

    arm_forcechange();
    // Fread meets the change on its data record; the re-login frees the
    // handle (offree), the restarted Fread finds a free entry: EIHNDL
    // ("invalid handle: media change, etc", bdos/bdosmain.c:591-592).
    let mut out = [0u8; 5];
    let n = call(FREAD, [h as u64, 5, out.as_mut_ptr() as u64, 0, 0, 0, 0, 0]);
    assert_eq!(n, EIHNDL, "Fread on a handle of the changed drive");
    assert_eq!(mediach(C), 0, "the re-login cleared the change");
    assert_eq!(gemdos1(FCLOSE, h as u64), EIHNDL, "Fclose: getofd NULL");
    // A fresh Fopen on the new log-in works.
    assert_eq!(read_hello(), *b"hello");
}

// ── freetree: current directories on the drive become the root ──

#[test]
fn current_directories_on_the_changed_drive_become_the_root() {
    let (_g, _h) = settled();
    let sub = cpath("C:\\SUB");
    assert_eq!(gemdos1(DSETPATH, sub.as_ptr() as u64), E_OK);
    let mut path = [0u8; 128];
    assert_eq!(gemdos2(DGETPATH, path.as_mut_ptr() as u64, 3), E_OK);
    assert_eq!(&path[..5], b"\\SUB\0");

    arm_forcechange();
    assert_eq!(dfree().0, E_OK, "the access that re-logs C:");
    let mut path = [0xAAu8; 128];
    assert_eq!(gemdos2(DGETPATH, path.as_mut_ptr() as u64, 3), E_OK);
    assert_eq!(path[0], 0, "C:'s directory is the root after the re-login");
}

// ── TASK/FS28: Dsetpath that meets the change keeps the path it set ──

/// `Dgetpath(C:)` as bytes up to the NUL.
fn dgetpath_c() -> Vec<u8> {
    let mut path = [0xAAu8; 128];
    assert_eq!(gemdos2(DGETPATH, path.as_mut_ptr() as u64, 3), E_OK);
    let n = path.iter().position(|&b| b == 0).expect("NUL-terminated");
    path[..n].to_vec()
}

/// gemdos-mc-fs24 V1a (cases 82-89): `Dsetpath(C:\SUB)` with the change
/// pending meets it, C: is logged in again (`freetree` empties the slot the
/// PD byte names), the call restarts and sets `\SUB`. EmuTOS `xchdir` reads
/// `olddir` before `findit` (`bdos/fsdir.c:1129-1134`), whose `ckdrv` has
/// repaired the stale byte to a root slot (`bdos/fsdrive.c:121-130`), so the
/// final `decr_curdir_usage(olddir)` cannot drop the slot just taken for
/// `\SUB`; the oracle's Dgetpath answers `\SUB` (case 87).
#[test]
fn fs28_dsetpath_that_meets_the_change_sets_the_path() {
    let (_g, _h) = settled();
    let sub = cpath("C:\\SUB");
    assert_eq!(gemdos1(DSETPATH, sub.as_ptr() as u64), E_OK);
    assert_eq!(dgetpath_c(), b"\\SUB");

    arm_forcechange();
    assert_eq!(gemdos1(DSETPATH, sub.as_ptr() as u64), E_OK, "V1a Dsetpath(C:\\SUB)");
    assert_eq!(mediach(C), 0, "the Dsetpath met the change (case 86)");
    assert_eq!(dgetpath_c(), b"\\SUB", "case 87: the path the call set");
    assert_eq!(curdir::uses_of(2, "\\SUB"), 1, "one PD names the \\SUB slot");
}

/// The same after a re-login by another call (the PD byte stale when
/// Dsetpath starts): a byte that names no live slot is the root and has no
/// use to drop.
#[test]
fn fs28_dsetpath_after_a_relogin_by_another_call_sets_the_path() {
    let (_g, _h) = settled();
    let sub = cpath("C:\\SUB");
    assert_eq!(gemdos1(DSETPATH, sub.as_ptr() as u64), E_OK);

    arm_forcechange();
    assert_eq!(dfree().0, E_OK, "the access that re-logs C:");
    assert_eq!(dgetpath_c(), b"", "the stale byte reads as the root");
    assert_eq!(gemdos1(DSETPATH, sub.as_ptr() as u64), E_OK);
    assert_eq!(dgetpath_c(), b"\\SUB");
    assert_eq!(curdir::uses_of(2, "\\SUB"), 1, "one PD names the \\SUB slot");
}

// ── mark_bcbs_invalid: the drive's buffers are read again ──

#[test]
fn the_drives_buffers_are_dropped_by_the_relogin() {
    let (_g, h) = settled();
    assert_eq!(read_hello(), *b"hello", "HELLO.TXT's record is now cached");

    // Change the record behind the cache's back: a cached copy hides it.
    let lba = cluster_lba(3);
    let orig = h.read_sector(lba);
    let mut changed = orig;
    changed[..5].copy_from_slice(b"HELLO");
    h.write_sector(lba, &changed);
    assert_eq!(read_hello(), *b"hello", "served from the cache");

    arm_forcechange();
    // Fopen meets the change on a cached directory record; after the
    // re-login no record of C: is cached any more.
    assert_eq!(read_hello(), *b"HELLO", "read from the disk again");
    assert_eq!(mediach(C), 0);

    // Leave the image as it was, and the cache without the changed copy.
    h.write_sector(lba, &orig);
    arm_forcechange();
    assert_eq!(read_hello(), *b"hello");
    assert_eq!(mediach(C), 0);
}

// ── a refused boot sector: E_CHNG to the caller, the drive logged out ──

#[test]
fn a_refused_boot_sector_returns_e_chng_and_leaves_the_drive_logged_out() {
    let (_g, h) = settled();
    let boot_lba = part_lba(0);
    let orig = h.read_sector(boot_lba);

    // Cluster size 0: Getbpb refuses the boot sector (bios/blkdev.c:651).
    let mut boot = orig;
    assert_eq!(rwabs_sector0(RW_READ_NOMEDIACH, &mut boot), 0);
    boot[0x0D] = 0;
    assert_eq!(rwabs_sector0(RW_WRITE_NOMEDIACH, &mut boot), 0);
    assert_eq!(mediach(C), 2);

    // osif: Getbpb NULL -> drvsel bit cleared, return E_CHNG (-14).
    let (rc, _) = dfree();
    assert_eq!(rc, -14, "Dfree returns E_CHNG when the re-login's Getbpb fails");
    // The next call logs C: in through ckdrv: Getbpb NULL again -> Dfree ERR.
    assert_eq!(dfree().0, -1, "C: cannot be logged in");

    // Restore: the boot sector back by hand, then a log-in.
    h.write_sector(boot_lba, &orig);
    assert_ne!(getbpb(C), 0);
    assert_eq!(dfree().0, E_OK);
}
