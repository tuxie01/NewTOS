//! TASK/FS31 — what names a directory entry names it **on one drive**.
//!
//! EmuTOS hangs the open files on the directory node (`d_files`, `bdos/fs.h:224`),
//! and that node belongs to one drive (`d_drv`, `bdos/fs.h:219`), so `ixdel`
//! (`bdos/fsopnclo.c:660-669`) can only ever see the files of the directory it
//! deletes from. NewTOS has no such node: an OFD carries the directory's start
//! cluster and the slot, and every root directory starts at "cluster" 0, every
//! freshly made folder at the first free cluster. Two drives of the same
//! history therefore hand out equal numbers.
//!
//! The image holds two FAT16 volumes of the same geometry, `C:` and `D:`, both
//! empty at the start of every test ([`fresh`]); each test asserts on the raw
//! directory bytes that its two entries really have the same numbers.
//!
//! Besides the three callers of `ixdel` the tests pin down the other
//! drive-sensitive keys of the BDOS: `Frename`'s `move_ofds`, the `Fsnext`
//! resume state, the current directories, a media change.

mod fatvol;

use fatvol::*;
use tos_api::bdos::fs::{cache, curdir, file};
use tos_api::bdos::process;
use tos_api::trap::{TrapArgs, TrapClass};
use tos_types::BasePage;

const C: u16 = 2;
const D: u16 = 3;
const DFREE: u16 = 0x36;
const DSETPATH: u16 = 0x3B;
const FATTRIB: u16 = 0x43;
const FDATIME: u16 = 0x57;
const FRENAME: u16 = 0x56;
const EACCDN: i32 = -36;
const EIHNDL: i32 = -37;
const ENSAME: i32 = -48;
const EINTRN: i32 = -65;

/// EmuTOS `default_handle` (`bdos/console.c:60-68`).
const DEFAULT: [i8; 6] = [-1, -1, -2, -3, -1, -1];

/// A process basepage in host memory, made `run` with [`Pd::run`].
struct Pd(*mut BasePage);
impl Pd {
    fn new() -> Pd {
        let bp = Box::into_raw(Box::new([0u8; 256])) as *mut BasePage;
        // SAFETY: fresh box.
        unsafe { (*bp).p_uft = DEFAULT };
        Pd(bp)
    }
    fn run(&self) {
        process::set_current(self.0);
    }
}
impl Drop for Pd {
    fn drop(&mut self) {
        if process::current() == self.0 {
            process::set_current(core::ptr::null_mut());
        }
        // SAFETY: nothing names the box any more.
        drop(unsafe { Box::from_raw(self.0 as *mut [u8; 256]) });
    }
}

/// The file-system lock, both volumes empty, nothing open, no process running.
fn fresh() -> (std::sync::MutexGuard<'static, ()>, &'static DiskHost) {
    let (g, h) = fs_two_drives();
    process::set_current(core::ptr::null_mut());
    for d in [C, D] {
        file::free_drive_files(d);
        cache::invalidate_drive(d);
        curdir::free_drive(d);
    }
    cache::clear_error();
    h.reset_two_drive_image();
    (g, h)
}

fn fcreate(name: &str) -> i32 {
    let p = cpath(name);
    gemdos2(FCREATE, p.as_ptr() as u64, 0)
}

fn fopen(name: &str, mode: u64) -> i32 {
    let p = cpath(name);
    gemdos2(FOPEN, p.as_ptr() as u64, mode)
}

fn fdelete(name: &str) -> i32 {
    let p = cpath(name);
    gemdos1(FDELETE, p.as_ptr() as u64)
}

fn dcreate(name: &str) -> i32 {
    let p = cpath(name);
    gemdos1(DCREATE, p.as_ptr() as u64)
}

fn ddelete(name: &str) -> i32 {
    let p = cpath(name);
    gemdos1(DDELETE, p.as_ptr() as u64)
}

fn frename(src: &str, dst: &str) -> i32 {
    let (s, d) = (cpath(src), cpath(dst));
    call(FRENAME, [0, s.as_ptr() as u64, d.as_ptr() as u64, 0, 0, 0, 0, 0])
}

fn fwrite(h: i32, data: &[u8]) -> i32 {
    call(FWRITE, [h as u16 as u64, data.len() as u64, data.as_ptr() as u64, 0, 0, 0, 0, 0])
}

fn fread(h: i32, buf: &mut [u8]) -> i32 {
    call(FREAD, [h as u16 as u64, buf.len() as u64, buf.as_mut_ptr() as u64, 0, 0, 0, 0, 0])
}

fn fclose(h: i32) -> i32 {
    gemdos1(FCLOSE, h as u16 as u64)
}

fn fdatime_set(h: i32, td: &mut [u16; 2]) -> i32 {
    call(FDATIME, [td.as_mut_ptr() as u64, h as u16 as u64, 1, 0, 0, 0, 0, 0])
}

/// A closed file `name` holding `data`.
fn make_file(name: &str, data: &[u8]) {
    let h = fcreate(name);
    assert!(h >= 6, "Fcreate {name}: {h}");
    assert_eq!(fwrite(h, data), data.len() as i32, "Fwrite {name}");
    assert_eq!(fclose(h), E_OK, "Fclose {name}");
}

/// The whole content of `name`, read back through a fresh handle.
fn content(name: &str) -> Vec<u8> {
    let h = fopen(name, 0);
    assert!(h >= 6, "Fopen {name}: {h}");
    let mut out = vec![0u8; 8192];
    let n = fread(h, &mut out);
    assert!(n >= 0, "Fread {name}: {n}");
    out.truncate(n as usize);
    assert_eq!(fclose(h), E_OK, "Fclose {name}");
    out
}

/// `Dfree`: `(free, total)` clusters of `drive`.
fn dfree(drive: u16) -> (u32, u32) {
    let mut buf = [0u32; 4];
    assert_eq!(gemdos2(DFREE, buf.as_mut_ptr() as u64, u64::from(drive) + 1), E_OK, "Dfree");
    (buf[0], buf[1])
}

/// The raw 32-byte directory entry `name` in the first sector at `lba`, with
/// its slot. Reads the image, not the BDOS.
fn raw_entry(h: &DiskHost, lba: u64, name: &[u8; 11]) -> (u32, [u8; 32]) {
    let sec = h.read_sector(lba);
    for (i, e) in sec.chunks_exact(32).enumerate() {
        if &e[..11] == name {
            let mut out = [0u8; 32];
            out.copy_from_slice(e);
            return (i as u32, out);
        }
    }
    panic!("{} not in the sector at LBA {lba}", String::from_utf8_lossy(name));
}

/// The precondition of every collision test: the two entries are at the same
/// slot of their directories.
fn assert_same_slot(h: &DiskHost, lba_c: u64, name_c: &[u8; 11], lba_d: u64, name_d: &[u8; 11]) {
    assert_eq!(
        raw_entry(h, lba_c, name_c).0,
        raw_entry(h, lba_d, name_d).0,
        "the test needs the same slot on both drives"
    );
}

// ── EmuTOS `ixdel` on D: and an open file on C: ─────────────────────────────

/// `Fdelete` (FSWBENCH on the Pi: `log last Fwrite=-36 Fclose=-65`,
/// `measurements/measurement-2026-11-06-hw08-pi2/RUNINFO.TXT`).
#[test]
fn fdelete_on_d_leaves_the_open_file_on_c_alone() {
    let (_g, h) = fresh();
    let log = fcreate("C:\\OPEN.LOG");
    assert!(log >= 6);
    assert_eq!(fwrite(log, b"first "), 6);
    let d_free = dfree(D).0;
    make_file("D:\\VICTIM.DAT", b"victim");
    assert_same_slot(h, two_drive_root_lba(0), b"OPEN    LOG", two_drive_root_lba(1), b"VICTIM  DAT");

    assert_eq!(fdelete("D:\\VICTIM.DAT"), E_OK);

    assert_eq!(fwrite(log, b"second"), 6, "Fwrite on C: after the delete on D:");
    assert_eq!(fclose(log), E_OK, "Fclose on C: after the delete on D:");
    assert_eq!(content("C:\\OPEN.LOG"), b"first second");
    assert_eq!(fopen("D:\\VICTIM.DAT", 0), EFILNF, "the file on D: is gone");
    assert_eq!(dfree(D).0, d_free, "its cluster is free");
    assert_eq!(dfree(C).0, VOLUME_CLUSTERS - 1, "the cluster on C: is not");
}

/// The second code of the FSWBENCH line, on its own: `Fclose` directly after
/// the delete, no `Fwrite` in between; what was written before stays.
#[test]
fn fclose_on_c_after_a_delete_on_d_is_a_good_close() {
    let (_g, h) = fresh();
    let log = fcreate("C:\\OPEN.LOG");
    assert!(log >= 6);
    assert_eq!(fwrite(log, b"first "), 6);
    make_file("D:\\VICTIM.DAT", b"victim");
    assert_same_slot(h, two_drive_root_lba(0), b"OPEN    LOG", two_drive_root_lba(1), b"VICTIM  DAT");

    assert_eq!(fdelete("D:\\VICTIM.DAT"), E_OK);

    assert_eq!(fclose(log), E_OK, "Fclose on C: after the delete on D:");
    assert_eq!(content("C:\\OPEN.LOG"), b"first ");
}

/// The file on `C:` was open before the delete **and only read**: the reader's
/// position survives too, and it reads to the end.
#[test]
fn fdelete_on_d_leaves_a_file_being_read_on_c_alone() {
    let (_g, h) = fresh();
    make_file("C:\\SOURCE.DAT", b"0123456789");
    make_file("D:\\VICTIM.DAT", b"victim");
    assert_same_slot(h, two_drive_root_lba(0), b"SOURCE  DAT", two_drive_root_lba(1), b"VICTIM  DAT");

    let src = fopen("C:\\SOURCE.DAT", 0);
    assert!(src >= 6);
    let mut first = [0u8; 4];
    assert_eq!(fread(src, &mut first), 4);

    assert_eq!(fdelete("D:\\VICTIM.DAT"), E_OK);

    let mut rest = [0u8; 16];
    assert_eq!(fread(src, &mut rest), 6, "Fread on C: after the delete on D:");
    assert_eq!(&rest[..6], b"456789");
    assert_eq!(fclose(src), E_OK);
}

/// `Fcreate` over an existing file: a copy from `C:` to `D:` that overwrites.
/// The source stays readable, the new target is made on `D:` only.
#[test]
fn fcreate_over_a_file_on_d_leaves_the_open_source_on_c_alone() {
    let (_g, h) = fresh();
    make_file("C:\\SOURCE.DAT", b"source bytes");
    make_file("D:\\TARGET.DAT", b"old target");
    assert_same_slot(h, two_drive_root_lba(0), b"SOURCE  DAT", two_drive_root_lba(1), b"TARGET  DAT");

    let src = fopen("C:\\SOURCE.DAT", 0);
    assert!(src >= 6);
    let dst = fcreate("D:\\TARGET.DAT");
    assert!(dst >= 6, "Fcreate over the file on D:: {dst}");

    let mut buf = [0u8; 32];
    let n = fread(src, &mut buf);
    assert_eq!(n, 12, "Fread of the source on C: after the Fcreate on D:");
    assert_eq!(fwrite(dst, &buf[..12]), 12);
    assert_eq!(fclose(src), E_OK, "Fclose of the source");
    assert_eq!(fclose(dst), E_OK, "Fclose of the target");
    assert_eq!(content("C:\\SOURCE.DAT"), b"source bytes");
    assert_eq!(content("D:\\TARGET.DAT"), b"source bytes");
}

/// `Ddelete` is the third caller of `ixdel` (`bdos/fsdir.c:377`).
#[test]
fn ddelete_on_d_leaves_the_open_file_on_c_alone() {
    let (_g, h) = fresh();
    let log = fcreate("C:\\OPEN.LOG");
    assert!(log >= 6);
    assert_eq!(fwrite(log, b"first "), 6);
    assert_eq!(dcreate("D:\\EMPTY"), E_OK);
    assert_same_slot(h, two_drive_root_lba(0), b"OPEN    LOG", two_drive_root_lba(1), b"EMPTY      ");

    assert_eq!(ddelete("D:\\EMPTY"), E_OK);

    assert_eq!(fwrite(log, b"second"), 6, "Fwrite on C: after the Ddelete on D:");
    assert_eq!(fclose(log), E_OK, "Fclose on C: after the Ddelete on D:");
    assert_eq!(content("C:\\OPEN.LOG"), b"first second");
}

/// The file of **another process** on the other drive is no reason to refuse
/// (`ixdel` `EACCDN`, `bdos/fsopnclo.c:667-668`, is about the directory's own
/// files), and that process's handle stays good.
#[test]
fn a_file_open_elsewhere_on_c_does_not_refuse_the_delete_on_d() {
    let (_g, h) = fresh();
    let (a, b) = (Pd::new(), Pd::new());
    a.run();
    let log = fcreate("C:\\OPEN.LOG");
    assert!(log >= 6);
    assert_eq!(fwrite(log, b"first "), 6);
    b.run();
    make_file("D:\\VICTIM.DAT", b"victim");
    make_file("D:\\NEWONE.DAT", b"new");
    assert_same_slot(h, two_drive_root_lba(0), b"OPEN    LOG", two_drive_root_lba(1), b"VICTIM  DAT");

    assert_eq!(fdelete("D:\\VICTIM.DAT"), E_OK, "Fdelete on D: by the other process");
    let again = fcreate("D:\\NEWONE.DAT");
    assert!(again >= 6, "Fcreate over a file on D: at the slot after: {again}");
    assert_eq!(fclose(again), E_OK);

    a.run();
    assert_eq!(fwrite(log, b"second"), 6);
    assert_eq!(fclose(log), E_OK);
    assert_eq!(content("C:\\OPEN.LOG"), b"first second");
}

/// Two folders made in the same order start at the same cluster: not only the
/// roots collide.
#[test]
fn a_subfolder_with_the_same_start_cluster_is_another_directory() {
    let (_g, h) = fresh();
    assert_eq!(dcreate("C:\\FOLDER"), E_OK);
    assert_eq!(dcreate("D:\\FOLDER"), E_OK);
    let cl = |i| {
        let e = raw_entry(h, two_drive_root_lba(i), b"FOLDER     ").1;
        u32::from(u16::from_le_bytes([e[0x1A], e[0x1B]]))
    };
    assert_eq!(cl(0), cl(1), "the test needs the same start cluster on both drives");

    let log = fcreate("C:\\FOLDER\\OPEN.LOG");
    assert!(log >= 6);
    assert_eq!(fwrite(log, b"first "), 6);
    make_file("D:\\FOLDER\\VICTIM.DAT", b"victim");
    assert_same_slot(
        h,
        two_drive_cluster_lba(0, cl(0)),
        b"OPEN    LOG",
        two_drive_cluster_lba(1, cl(1)),
        b"VICTIM  DAT",
    );

    assert_eq!(fdelete("D:\\FOLDER\\VICTIM.DAT"), E_OK);

    assert_eq!(fwrite(log, b"second"), 6);
    assert_eq!(fclose(log), E_OK);
    assert_eq!(content("C:\\FOLDER\\OPEN.LOG"), b"first second");
}

/// Deleting on `C:` with a same-slot file open on both drives takes exactly
/// the one on `C:`.
#[test]
fn a_delete_takes_the_open_file_of_its_own_drive_only() {
    let (_g, h) = fresh();
    let on_c = fcreate("C:\\GOES.LOG");
    assert!(on_c >= 6);
    assert_eq!(fwrite(on_c, b"goes"), 4);
    let on_d = fcreate("D:\\STAYS.LOG");
    assert!(on_d >= 6);
    assert_eq!(fwrite(on_d, b"first "), 6);
    assert_same_slot(h, two_drive_root_lba(0), b"GOES    LOG", two_drive_root_lba(1), b"STAYS   LOG");

    assert_eq!(fdelete("C:\\GOES.LOG"), E_OK);

    assert_eq!(fwrite(on_c, b"more"), EACCDN, "the deleted file, PROC04");
    assert_eq!(fclose(on_c), EINTRN, "the deleted file, PROC04");
    assert_eq!(fwrite(on_d, b"second"), 6, "the file on the other drive");
    assert_eq!(fclose(on_d), E_OK);
    assert_eq!(content("D:\\STAYS.LOG"), b"first second");
}

// ── Controls: the same drive as before (PROC04, `docs/COMPATIBILITY.md`) ────

#[test]
fn control_fdelete_of_the_callers_open_file_on_the_same_drive() {
    let (_g, _h) = fresh();
    let f = fcreate("D:\\OPEN.LOG");
    assert!(f >= 6);
    assert_eq!(fwrite(f, b"bytes"), 5);
    assert_eq!(fdelete("D:\\OPEN.LOG"), E_OK);
    assert_eq!(fwrite(f, b"more"), EACCDN);
    let mut b = [0u8; 4];
    assert_eq!(fread(f, &mut b), 0);
    assert_eq!(fclose(f), EINTRN);
    assert_eq!(fopen("D:\\OPEN.LOG", 0), EFILNF);
    assert_eq!(dfree(D).0, VOLUME_CLUSTERS, "no cluster left allocated");
}

#[test]
fn control_a_file_open_elsewhere_on_the_same_drive_refuses_the_delete() {
    let (_g, _h) = fresh();
    let (a, b) = (Pd::new(), Pd::new());
    a.run();
    let f = fcreate("D:\\OPEN.LOG");
    assert!(f >= 6);
    b.run();
    assert_eq!(fdelete("D:\\OPEN.LOG"), EACCDN);
    assert_eq!(fcreate("D:\\OPEN.LOG"), EACCDN);
    a.run();
    assert_eq!(fclose(f), E_OK);
}

// ── The other drive-sensitive keys: green before FS31 too ───────────────────

/// `move_ofds` compares the drive: moving a file on `D:` from a folder takes
/// the open file on `D:` along and leaves the one on `C:` where it is.
#[test]
fn frename_across_folders_on_d_moves_the_open_file_of_d_only() {
    let (_g, h) = fresh();
    assert_eq!(dcreate("C:\\FOLDER"), E_OK);
    assert_eq!(dcreate("D:\\FOLDER"), E_OK);
    let on_c = fcreate("C:\\FOLDER\\OPEN.LOG");
    assert!(on_c >= 6);
    assert_eq!(fwrite(on_c, b"on c "), 5);
    let on_d = fcreate("D:\\FOLDER\\MOVED.LOG");
    assert!(on_d >= 6);
    assert_eq!(fwrite(on_d, b"on d "), 5);
    assert_same_slot(
        h,
        two_drive_cluster_lba(0, 2),
        b"OPEN    LOG",
        two_drive_cluster_lba(1, 2),
        b"MOVED   LOG",
    );

    assert_eq!(frename("D:\\FOLDER\\MOVED.LOG", "D:\\MOVED.LOG"), E_OK);

    assert_eq!(fwrite(on_c, b"more"), 4);
    assert_eq!(fwrite(on_d, b"more"), 4);
    assert_eq!(fclose(on_c), E_OK);
    assert_eq!(fclose(on_d), E_OK);
    assert_eq!(content("C:\\FOLDER\\OPEN.LOG"), b"on c more");
    assert_eq!(content("D:\\MOVED.LOG"), b"on d more");
}

/// `xrename` compares the drives after its source checks (`fsdir.c:989-990`):
/// the same parent number on both drives is no same-directory rename.
#[test]
fn frename_from_c_to_d_is_ensame() {
    let (_g, _h) = fresh();
    make_file("C:\\A.DAT", b"a");
    assert_eq!(frename("C:\\A.DAT", "D:\\B.DAT"), ENSAME);
    assert_eq!(content("C:\\A.DAT"), b"a");
    assert_eq!(fopen("D:\\B.DAT", 0), EFILNF);
}

/// `Fdatime` and the close go through the OFD's own volume, not through the
/// numbers alone.
#[test]
fn fdatime_writes_the_entry_on_the_handles_drive() {
    let (_g, h) = fresh();
    make_file("C:\\A.DAT", b"a");
    make_file("D:\\B.DAT", b"b");
    assert_same_slot(h, two_drive_root_lba(0), b"A       DAT", two_drive_root_lba(1), b"B       DAT");
    let c_before = raw_entry(h, two_drive_root_lba(0), b"A       DAT").1;

    let f = fopen("D:\\B.DAT", 2);
    assert!(f >= 6);
    let mut td = [0x1234u16, 0x5678u16];
    assert_eq!(fdatime_set(f, &mut td), E_OK);
    assert_eq!(fclose(f), E_OK);

    let d_after = raw_entry(h, two_drive_root_lba(1), b"B       DAT").1;
    assert_eq!(&d_after[0x16..0x1A], &[0x34, 0x12, 0x78, 0x56], "the entry on D:");
    assert_eq!(raw_entry(h, two_drive_root_lba(0), b"A       DAT").1, c_before, "the entry on C:");
}

/// `Fattrib` by path on `D:` leaves the entry at the same slot on `C:` alone.
#[test]
fn fattrib_changes_the_entry_on_the_paths_drive() {
    let (_g, h) = fresh();
    make_file("C:\\A.DAT", b"a");
    make_file("D:\\B.DAT", b"b");
    let c_before = raw_entry(h, two_drive_root_lba(0), b"A       DAT").1;
    let p = cpath("D:\\B.DAT");
    assert_eq!(call(FATTRIB, [p.as_ptr() as u64, 1, 0x01, 0, 0, 0, 0, 0]), 0x01);
    assert_eq!(raw_entry(h, two_drive_root_lba(1), b"B       DAT").1[0x0B], 0x01);
    assert_eq!(raw_entry(h, two_drive_root_lba(0), b"A       DAT").1, c_before);
}

/// The `Fsnext` resume state holds the drive: two searches at the same
/// directory number and slot, each goes on where it was.
#[test]
fn fsnext_goes_on_on_the_drive_of_its_dta() {
    let (_g, _h) = fresh();
    make_file("C:\\C1.DAT", b"x");
    make_file("C:\\C2.DAT", b"x");
    make_file("D:\\D1.DAT", b"x");
    make_file("D:\\D2.DAT", b"x");
    let (mut on_c, mut on_d) = (Dta::new(), Dta::new());
    let all_c = cpath("C:\\*.*");
    let all_d = cpath("D:\\*.*");

    assert_eq!(gemdos1(FSETDTA, on_c.ptr()), E_OK);
    assert_eq!(gemdos2(FSFIRST, all_c.as_ptr() as u64, FA_ANY), E_OK);
    assert_eq!(on_c.name(), "C1.DAT");
    assert_eq!(gemdos1(FSETDTA, on_d.ptr()), E_OK);
    assert_eq!(gemdos2(FSFIRST, all_d.as_ptr() as u64, FA_ANY), E_OK);
    assert_eq!(on_d.name(), "D1.DAT");
    assert_eq!(gemdos1(FSETDTA, on_c.ptr()), E_OK);
    assert_eq!(gemdos1(FSNEXT, 0), E_OK);
    assert_eq!(on_c.name(), "C2.DAT");
    assert_eq!(gemdos1(FSETDTA, on_d.ptr()), E_OK);
    assert_eq!(gemdos1(FSNEXT, 0), E_OK);
    assert_eq!(on_d.name(), "D2.DAT");
}

/// `dirtbl[]` holds the drive beside the path: the same path on two drives is
/// two current directories (`bdos/fsdir.c:1164-1188`, `dcrack` `:1695-1731`).
#[test]
fn the_same_current_directory_path_on_two_drives_is_two_directories() {
    let (_g, _h) = fresh();
    assert_eq!(dcreate("C:\\FOLDER"), E_OK);
    assert_eq!(dcreate("D:\\FOLDER"), E_OK);
    let pd = Pd::new();
    pd.run();
    let on_c = cpath("C:\\FOLDER");
    assert_eq!(gemdos1(DSETPATH, on_c.as_ptr() as u64), E_OK);
    assert_eq!(curdir::curdir(C).as_str(), "\\FOLDER");
    assert_eq!(curdir::curdir(D).as_str(), "", "C:'s directory is not D:'s");

    make_file("D:REL.DAT", b"root of d");
    assert_eq!(content("D:\\REL.DAT"), b"root of d", "a relative path on D:");

    let on_d = cpath("D:\\FOLDER");
    assert_eq!(gemdos1(DSETPATH, on_d.as_ptr() as u64), E_OK);
    assert_eq!(curdir::uses_of(C, "\\FOLDER"), 1);
    assert_eq!(curdir::uses_of(D, "\\FOLDER"), 1);
    make_file("D:INNER.DAT", b"in d folder");
    assert_eq!(content("D:\\FOLDER\\INNER.DAT"), b"in d folder");
    assert_eq!(fopen("C:\\FOLDER\\INNER.DAT", 0), EFILNF);
    assert_eq!(gemdos1(DSETPATH, cpath("C:\\").as_ptr() as u64), E_OK);
    assert_eq!(gemdos1(DSETPATH, cpath("D:\\").as_ptr() as u64), E_OK);
}

/// A media change on `D:` frees the files of `D:` only (`offree`,
/// `bdos/bdosmain.c:387-405`).
#[test]
fn a_media_change_on_d_frees_the_open_files_of_d_only() {
    let (_g, _h) = fresh();
    let on_c = fcreate("C:\\OPEN.LOG");
    assert!(on_c >= 6);
    assert_eq!(fwrite(on_c, b"first "), 6);
    let on_d = fcreate("D:\\OPEN.LOG");
    assert!(on_d >= 6);
    assert_eq!(fwrite(on_d, b"goes"), 4);

    // `Rwabs` write of logical sector 0 of D: arms `forcechange`
    // (`bios/blkdev.c:452-453`, TASK/FS20).
    let bios = |func: u64, regs: [u64; 8]| {
        tos_api::bios::dispatch(&TrapArgs {
            regs,
            func,
            svc_imm: TrapClass::Bios as u16,
            space: tos_api::uaccess::Space::Kernel,
        })
    };
    let mut boot = [0u8; SECTOR_BYTES];
    let ptr = boot.as_mut_ptr() as u64;
    assert_eq!(bios(0x04, [2, ptr, 1, 0, u64::from(D), 0, 0, 0]), 0, "read D: sector 0");
    assert_eq!(bios(0x04, [3, ptr, 1, 0, u64::from(D), 0, 0, 0]), 0, "write it back");
    dfree(D); // the access that re-logs D:

    assert_eq!(fwrite(on_d, b"more"), EIHNDL, "the file on D: is gone");
    assert_eq!(fwrite(on_c, b"second"), 6, "the file on C: is not");
    assert_eq!(fclose(on_c), E_OK);
    assert_eq!(content("C:\\OPEN.LOG"), b"first second");
}
