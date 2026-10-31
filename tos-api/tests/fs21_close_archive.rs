//! TASK/FS21 — `Fclose` of a changed file sets `FA_ARCHIVE` in its directory
//! entry, as EmuTOS does.
//!
//! EmuTOS `ixclose` (`bdos/fsopnclo.c:530-550`): only when the file's DFD is
//! `O_DIRTY` it rewrites the entry — time/date, start cluster, length — and
//! OR-s `FA_ARCHIVE` into the attribute byte the entry has at that moment
//! (`attr = fcb->f_attrib`, `:537`; `attr |= FA_ARCHIVE`, `:544-545`).
//! `O_DIRTY` is set by `Fcreate` (`fsopnclo.c:206`), by a write that grows the
//! file (`addit`, `fsio.c:42-46`) or gives it its first cluster (`nextcl`,
//! `fsfat.c:279-282`), and by an `Fdatime` set (`fsdir.c:698`). A file only
//! opened, read, or overwritten inside its length is not dirty: its `Fclose`
//! leaves the attribute alone. Spec: `docs/api/gemdos/file/3e-fclose.md`
//! edge case 8.
//!
//! Measured on the oracle: the FS20 probe's `GCPROBE.DAT` (Fcreate, 512-byte
//! Fwrite, Fclose) ends with attribute 0x20 on EmuTOS
//! (`measurements/measurement-2026-10-28-fs20/RUNINFO.TXT`).

mod fatvol;

use fatvol::*;

const FATTRIB: u16 = 0x43;
const FDATIME: u16 = 0x57;
const FA_RO: u8 = 0x01;
const FA_ARCHIVE: u8 = 0x20;

fn fcreate(name: &str, attr: u16) -> i32 {
    let p = cpath(name);
    gemdos2(FCREATE, p.as_ptr() as u64, attr as u64)
}

fn fopen(name: &str, mode: u16) -> i32 {
    let p = cpath(name);
    gemdos2(FOPEN, p.as_ptr() as u64, mode as u64)
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

fn fdatime(buf: &mut [u16; 2], h: i32, wrt: u16) -> i32 {
    call(FDATIME, [buf.as_mut_ptr() as u64, h as u16 as u64, wrt as u64, 0, 0, 0, 0, 0])
}

fn fattrib_set(name: &str, attr: u8) -> i32 {
    let p = cpath(name);
    call(FATTRIB, [p.as_ptr() as u64, 1, attr as u64, 0, 0, 0, 0, 0])
}

/// The attribute byte of the live root-directory entry `name11`, read straight
/// from the image (offset 0x0B).
fn root_attr(host: &DiskHost, name11: &[u8; 11]) -> u8 {
    for s in 0..32u64 {
        let sec = host.read_sector(root_lba() + s);
        for e in sec.chunks(32) {
            if &e[0..11] == name11 {
                return e[0x0B];
            }
        }
    }
    panic!("no root entry {:?}", core::str::from_utf8(name11));
}

/// A closed file with content and attribute 0: created, written, closed (that
/// sets `FA_ARCHIVE`), then cleared by `Fattrib`.
fn plain_file(host: &DiskHost, path: &str, name11: &[u8; 11]) {
    let h = fcreate(path, 0);
    assert!(h >= 6, "Fcreate {path} -> {h}");
    assert_eq!(fwrite(h, b"hello"), 5);
    assert_eq!(fclose(h), E_OK);
    assert_eq!(root_attr(host, name11), FA_ARCHIVE);
    assert_eq!(fattrib_set(path, 0), 0);
    assert_eq!(root_attr(host, name11), 0, "Fattrib cleared it");
}

/// The FS20 probe's sequence: Fcreate, Fwrite, Fclose → `FA_ARCHIVE`.
#[test]
fn written_file_gets_the_archive_bit_on_close() {
    let (_g, host) = fs();
    let h = fcreate("C:\\WR1.DAT", 0);
    assert!(h >= 6, "Fcreate -> {h}");
    assert_eq!(root_attr(host, b"WR1     DAT"), 0, "Fcreate writes the requested attribute");
    assert_eq!(fwrite(h, &[0xDE; 512]), 512);
    assert_eq!(fclose(h), E_OK);
    assert_eq!(root_attr(host, b"WR1     DAT"), FA_ARCHIVE);
}

/// `Fcreate` marks the new DFD `O_DIRTY` (`fsopnclo.c:206`): its `Fclose`
/// sets the bit even when nothing was written.
#[test]
fn created_file_closed_without_a_write_gets_the_archive_bit() {
    let (_g, host) = fs();
    let h = fcreate("C:\\EMPTY.DAT", 0);
    assert!(h >= 6);
    assert_eq!(fclose(h), E_OK);
    assert_eq!(root_attr(host, b"EMPTY   DAT"), FA_ARCHIVE);
}

/// The bit is OR-ed into the attribute the entry has: a read-only create
/// keeps `FA_RO` (0x01 → 0x21).
#[test]
fn archive_bit_is_ored_into_the_created_attribute() {
    let (_g, host) = fs();
    let h = fcreate("C:\\RO.DAT", FA_RO as u16);
    assert!(h >= 6);
    assert_eq!(fwrite(h, b"x"), 1, "EmuTOS does not enforce the access code");
    assert_eq!(fclose(h), E_OK);
    assert_eq!(root_attr(host, b"RO      DAT"), FA_RO | FA_ARCHIVE);
}

/// A file only opened and read, or only opened and closed, is not dirty: the
/// attribute stays as it was.
#[test]
fn file_only_opened_or_read_keeps_its_attribute() {
    let (_g, host) = fs();
    plain_file(host, "C:\\RD.DAT", b"RD      DAT");

    let h = fopen("C:\\RD.DAT", 0);
    assert!(h >= 6);
    assert_eq!(fclose(h), E_OK);
    assert_eq!(root_attr(host, b"RD      DAT"), 0, "open + close");

    let h = fopen("C:\\RD.DAT", 0);
    let mut buf = [0u8; 16];
    assert_eq!(fread(h, &mut buf), 5);
    assert_eq!(fclose(h), E_OK);
    assert_eq!(root_attr(host, b"RD      DAT"), 0, "open + read + close");

    let h = fopen("C:\\RD.DAT", 2);
    let mut td = [0u16; 2];
    assert_eq!(fdatime(&mut td, h, 0), E_OK, "Fdatime get marks nothing");
    assert_eq!(fclose(h), E_OK);
    assert_eq!(root_attr(host, b"RD      DAT"), 0, "read-write open + Fdatime get + close");
}

/// A write inside the file's length grows nothing and allocates nothing
/// (`addit` only marks `O_DIRTY` when `o_bytnum > o_fileln`): no bit.
#[test]
fn overwrite_inside_the_length_keeps_the_attribute() {
    let (_g, host) = fs();
    plain_file(host, "C:\\OVR.DAT", b"OVR     DAT");
    let h = fopen("C:\\OVR.DAT", 2);
    assert!(h >= 6);
    assert_eq!(fwrite(h, b"HE"), 2);
    assert_eq!(fclose(h), E_OK);
    assert_eq!(root_attr(host, b"OVR     DAT"), 0);
}

/// A write that grows an existing file sets the bit.
#[test]
fn write_that_grows_an_existing_file_sets_the_bit() {
    let (_g, host) = fs();
    plain_file(host, "C:\\GROW.DAT", b"GROW    DAT");
    let h = fopen("C:\\GROW.DAT", 2);
    assert!(h >= 6);
    assert_eq!(fwrite(h, b"hello!"), 6, "one byte past the old length");
    assert_eq!(fclose(h), E_OK);
    assert_eq!(root_attr(host, b"GROW    DAT"), FA_ARCHIVE);
}

/// `Fdatime` set marks the DFD `O_DIRTY` (`fsdir.c:698`), so the close sets
/// the bit even on a file opened read-only.
#[test]
fn fdatime_set_sets_the_bit_on_close() {
    let (_g, host) = fs();
    plain_file(host, "C:\\TD.DAT", b"TD      DAT");
    let h = fopen("C:\\TD.DAT", 0);
    assert!(h >= 6);
    let mut td = [0x1234u16, 0x5678];
    assert_eq!(fdatime(&mut td, h, 1), E_OK);
    assert_eq!(fclose(h), E_OK);
    assert_eq!(root_attr(host, b"TD      DAT"), FA_ARCHIVE);
}

/// `ixclose` reads the attribute from the entry at close time: an `Fattrib`
/// that clears the bit while the dirty file is open does not survive the
/// close, and one that sets `FA_RO` is kept.
#[test]
fn close_ors_into_the_attribute_fattrib_left_while_open() {
    let (_g, host) = fs();
    let h = fcreate("C:\\FAT.DAT", 0);
    assert!(h >= 6);
    assert_eq!(fwrite(h, b"abc"), 3);
    assert_eq!(fattrib_set("C:\\FAT.DAT", FA_RO), FA_RO as i32);
    assert_eq!(root_attr(host, b"FAT     DAT"), FA_RO);
    assert_eq!(fclose(h), E_OK);
    assert_eq!(root_attr(host, b"FAT     DAT"), FA_RO | FA_ARCHIVE);
}
