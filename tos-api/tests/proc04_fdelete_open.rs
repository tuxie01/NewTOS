//! TASK/PROC04 point 5 — `Fdelete` (and every other caller of EmuTOS `ixdel`:
//! `Fcreate` over an existing file, `Ddelete`) on a file that is still open.
//!
//! EmuTOS `ixdel` (`bdos/fsopnclo.c:652-669`) walks the OFDs listed on the
//! directory (`d_files`, newest first, `makopn` `:294-295`) at the entry's
//! position and, for every `sft[]` entry on such an OFD, `ixclose`s it if
//! `f_own == run` and answers `EACCDN` otherwise (no rollback). `ixclose`
//! (`:511-581`) takes the OFD off `d_files` but leaves `sft[]` alone: the
//! caller's handle — and every `Fdup`/`Fforce` alias — stays valid, and a later
//! `Fclose` answers `EINTRN` and frees the entry (`:552-566`, `:484-496`).
//!
//! NewTOS deviation (user decision 2026-10-18, variant b,
//! `docs/COMPATIBILITY.md` "Sanctioned deviations"): once the file is deleted,
//! nothing done through such a handle reaches the card any more — no write into
//! the freed clusters, no rewrite of the (possibly reused) directory slot on
//! `Fclose`. `Fread` answers 0, `Fwrite` `EACCDN`, `Fseek` / `Fdatime` keep
//! EmuTOS' values from the OFD alone. Spec: `docs/api/gemdos/file/41-fdelete.md`
//! edge case 3 and "PROC04 point 5".

mod fatvol;

use fatvol::*;
use tos_api::bdos::process;
use tos_types::BasePage;

const FDUP: u16 = 0x45;
const FFORCE: u16 = 0x46;
const FSEEK: u16 = 0x42;
const FDATIME: u16 = 0x57;
const EACCDN: i32 = -36;
const EIHNDL: i32 = -37;
const ERANGE: i32 = -64;
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
    fn uft(&self) -> [i8; 6] {
        // SAFETY: the box is alive.
        unsafe { (*self.0).p_uft }
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

fn fcreate(name: &str) -> i32 {
    let p = cpath(name);
    gemdos2(FCREATE, p.as_ptr() as u64, 0)
}

fn fopen(name: &str) -> i32 {
    let p = cpath(name);
    gemdos2(FOPEN, p.as_ptr() as u64, 2)
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

fn fwrite(h: i32, data: &[u8]) -> i32 {
    call(FWRITE, [h as u16 as u64, data.len() as u64, data.as_ptr() as u64, 0, 0, 0, 0, 0])
}

fn fread(h: i32, buf: &mut [u8]) -> i32 {
    call(FREAD, [h as u16 as u64, buf.len() as u64, buf.as_mut_ptr() as u64, 0, 0, 0, 0, 0])
}

fn fseek(off: i32, h: i32, mode: u16) -> i32 {
    call(FSEEK, [off as u32 as u64, h as u16 as u64, mode as u64, 0, 0, 0, 0, 0])
}

fn fdatime(buf: &mut [u16; 2], h: i32, wrt: u16) -> i32 {
    call(FDATIME, [buf.as_mut_ptr() as u64, h as u16 as u64, wrt as u64, 0, 0, 0, 0, 0])
}

fn fclose(h: i32) -> i32 {
    gemdos1(FCLOSE, h as u16 as u64)
}

fn fdup(h: i32) -> i32 {
    gemdos1(FDUP, h as u64)
}

fn fforce(std: i32, h: i32) -> i32 {
    gemdos2(FFORCE, std as u16 as u64, h as u16 as u64)
}

/// The whole content of `name`, read back through a fresh handle.
fn content(name: &str) -> Vec<u8> {
    let h = fopen(name);
    assert!(h >= 6, "Fopen {name} -> {h}");
    let mut buf = [0u8; 2048];
    let n = fread(h, &mut buf);
    assert!(n >= 0, "Fread {name} -> {n}");
    assert_eq!(fclose(h), E_OK);
    buf[..n as usize].to_vec()
}

/// Free entries in the first FAT copy, read straight from the image.
fn free_clusters(host: &DiskHost) -> usize {
    let mut free = 0;
    let mut cl = 0usize;
    // 5000 data clusters + the two reserved entries, 256 entries per sector.
    for s in 0..20u64 {
        let sec = host.read_sector(fat_lba(0) + s);
        for e in sec.chunks(2) {
            if (2..5002).contains(&cl) && e == [0, 0] {
                free += 1;
            }
            cl += 1;
        }
    }
    free
}

/// The raw 32-byte root-directory entry named `name11`, if a live one exists.
fn root_entry(host: &DiskHost, name11: &[u8; 11]) -> Option<[u8; 32]> {
    for s in 0..32u64 {
        let sec = host.read_sector(root_lba() + s);
        for e in sec.chunks(32) {
            if &e[0..11] == name11 {
                let mut out = [0u8; 32];
                out.copy_from_slice(e);
                return Some(out);
            }
        }
    }
    None
}

fn writes(host: &DiskHost) -> u32 {
    host.disk.writes.load(std::sync::atomic::Ordering::Relaxed)
}

fn reads(host: &DiskHost) -> u32 {
    host.disk.reads.load(std::sync::atomic::Ordering::Relaxed)
}

/// Own open file: `Fdelete` = `E_OK`, the handle stays valid (`ixclose` leaves
/// `sft[]` alone), the name is gone; `Fseek` keeps EmuTOS' values from the
/// OFD; `Fread` 0, `Fwrite` `EACCDN` (deviation); `Fclose` `EINTRN` (the OFD
/// is no longer on `d_files`, `fsopnclo.c:552-566`) and frees the entry.
#[test]
fn own_open_file_is_deleted_and_its_handle_stays_valid() {
    let (_g, _host) = fs();
    let p = Pd::new();
    p.run();

    let h = fcreate("C:\\FD1.TXT");
    assert!(h >= 6, "Fcreate -> {h}");
    assert_eq!(fwrite(h, b"hello"), 5);
    assert_eq!(fdelete("C:\\FD1.TXT"), E_OK, "ixdel: own OFD -> ixclose, E_OK");
    assert_eq!(fopen("C:\\FD1.TXT"), EFILNF, "the name is gone");

    assert_eq!(fseek(0, h, 1), 5, "SEEK_CUR: the OFD's position (o_bytnum)");
    assert_eq!(fseek(0, h, 2), 5, "SEEK_END: the OFD's length (o_fileln)");
    assert_eq!(fseek(6, h, 0), ERANGE, "past o_fileln (ixlseek :360-361)");
    assert_eq!(fseek(-1, h, 0), ERANGE);
    assert_eq!(fseek(2, h, 0), 2);
    let mut buf = [0u8; 8];
    assert_eq!(fread(h, &mut buf), 0, "deleted: nothing to read (deviation)");
    assert_eq!(fwrite(h, b"x"), EACCDN, "deleted: no write (deviation)");
    assert_eq!(fwrite(h, b""), 0, "a zero-length write moves nothing, as on EmuTOS");
    assert_eq!(fseek(0, h, 1), 2, "neither Fread nor Fwrite moved the position");

    assert_eq!(fclose(h), EINTRN, "OFD not on d_files any more");
    assert_eq!(fclose(h), EIHNDL, "the entry was freed by that close");
}

/// A file written but not yet closed has its start cluster only in the OFD
/// (`O_DIRTY`); `ixdel`'s `ixclose` writes it back before the chain is freed
/// (the chain walk reads `f->f_clust` after it), so no cluster is lost.
#[test]
fn deleting_a_dirty_open_file_frees_its_whole_chain() {
    let (_g, host) = fs();
    let p = Pd::new();
    p.run();

    let before = free_clusters(host);
    let h = fcreate("C:\\FD2.TXT");
    assert!(h >= 6);
    assert_eq!(fwrite(h, &[0x5A; 1500]), 1500, "three 512-byte clusters");
    // The FAT sector is still dirty in the cache (`Fwrite` does not write back,
    // TASK/HW08): the three clusters show through `Dfree`, not on the card.
    let mut df = [0u8; 16];
    assert_eq!(gemdos2(0x36, df.as_mut_ptr() as u64, 3), E_OK);
    assert_eq!(u32::from_le_bytes([df[0], df[1], df[2], df[3]]) as usize, before - 3);
    assert_eq!(fdelete("C:\\FD2.TXT"), E_OK);
    assert_eq!(free_clusters(host), before, "the whole chain is free again");
    assert_eq!(fclose(h), EINTRN);
    assert_eq!(free_clusters(host), before);
}

/// After the delete nothing done through the handle reaches the card: no
/// sector is written or read by `Fwrite`, `Fseek`, `Fread`, `Fdatime` (get and
/// set) or `Fclose`, and the freed clusters stay free.
#[test]
fn nothing_through_a_deleted_handle_touches_the_card() {
    let (_g, host) = fs();
    let p = Pd::new();
    p.run();

    let before = free_clusters(host);
    let h = fcreate("C:\\FD3.TXT");
    assert!(h >= 6);
    assert_eq!(fwrite(h, &[0x33; 1000]), 1000);
    let mut td = [0u16; 2];
    assert_eq!(fdatime(&mut td, h, 0), E_OK);
    assert_eq!(fdelete("C:\\FD3.TXT"), E_OK);
    assert_eq!(free_clusters(host), before);

    let (w0, r0) = (writes(host), reads(host));
    assert_eq!(fwrite(h, &[0x77; 2000]), EACCDN);
    assert_eq!(fseek(0, h, 0), 0);
    let mut buf = [0u8; 100];
    assert_eq!(fread(h, &mut buf), 0);
    let mut got = [0u16; 2];
    assert_eq!(fdatime(&mut got, h, 0), E_OK, "get: from the OFD (xgsdtof)");
    assert_eq!(got, td, "the file's date/time as it was at the delete");
    let mut new = [0x1234u16, 0x5678];
    assert_eq!(fdatime(&mut new, h, 1), E_OK, "set: into the OFD only");
    let mut back = [0u16; 2];
    assert_eq!(fdatime(&mut back, h, 0), E_OK);
    assert_eq!(back, [0x1234, 0x5678]);
    assert_eq!(fclose(h), EINTRN);
    assert_eq!(writes(host), w0, "no sector written after the delete");
    assert_eq!(reads(host), r0, "no sector read after the delete");
    assert_eq!(free_clusters(host), before, "the freed chain stays free");
}

/// The slot and clusters of the deleted file go to the next file created; the
/// stale handle's later `Fwrite`/`Fclose` must not touch that file (EmuTOS'
/// `ixclose` would rewrite the reused slot from a dirty OFD, `:530-549`).
#[test]
fn a_stale_close_leaves_the_file_that_reused_the_slot_alone() {
    let (_g, host) = fs();
    let p = Pd::new();
    p.run();

    let old = fcreate("C:\\FD4A.TXT");
    assert!(old >= 6);
    assert_eq!(fwrite(old, &[0xAA; 700]), 700);
    assert_eq!(fdelete("C:\\FD4A.TXT"), E_OK);

    let new = fcreate("C:\\FD4B.TXT");
    assert!(new >= 6);
    assert_eq!(fwrite(new, b"new file"), 8);
    assert_eq!(fclose(new), E_OK);
    let entry = root_entry(host, b"FD4B    TXT").expect("FD4B entry");

    let w0 = writes(host);
    assert_eq!(fwrite(old, &[0xBB; 300]), EACCDN);
    assert_eq!(fclose(old), EINTRN);
    assert_eq!(writes(host), w0);
    assert_eq!(root_entry(host, b"FD4B    TXT"), Some(entry), "entry untouched");
    assert_eq!(content("C:\\FD4B.TXT"), b"new file");
}

/// `Fforce`/`Fdup` aliases of a deleted file stay valid too: every `sft[]`
/// entry on the OFD keeps it, every close is an `ixclose` of an unlisted OFD
/// (`EINTRN`) that frees its entry when the use count reaches 0.
#[test]
fn aliases_of_a_deleted_file_stay_valid() {
    let (_g, _host) = fs();
    let p = Pd::new();
    p.run();

    let h = fcreate("C:\\FD5.TXT");
    assert!(h >= 6);
    assert_eq!(fforce(1, h), E_OK);
    let d = fdup(1);
    assert!(d >= 6 && d != h);
    assert_eq!(fwrite(1, b"abc"), 3);
    assert_eq!(fdelete("C:\\FD5.TXT"), E_OK);

    assert_eq!(fwrite(1, b"x"), EACCDN, "handle 1 still names the OFD");
    assert_eq!(fwrite(d, b"x"), EACCDN, "the duplicate too");
    assert_eq!(fseek(0, d, 1), 3, "one position for all of them");
    assert_eq!(fclose(d), EINTRN, "the duplicate's entry goes");
    assert_eq!(fclose(1), EINTRN, "reverts handle 1, use 2 -> 1");
    assert_eq!(p.uft()[1], -1);
    assert_eq!(fclose(h), EINTRN, "use 1 -> 0");
    assert_eq!(fclose(h), EIHNDL);
}

/// A file open in another process: `Fdelete` and `Fcreate` over it answer
/// `EACCDN` (`fsopnclo.c:667-668`, `:163-164`); the owner's handle is
/// untouched and the file intact.
#[test]
fn a_file_open_in_another_process_is_not_deleted() {
    let (_g, _host) = fs();
    let a = Pd::new();
    let b = Pd::new();
    a.run();
    let h = fcreate("C:\\FD6.TXT");
    assert!(h >= 6);
    assert_eq!(fwrite(h, b"keep"), 4);

    b.run();
    assert_eq!(fdelete("C:\\FD6.TXT"), EACCDN);
    assert_eq!(fcreate("C:\\FD6.TXT"), EACCDN);

    a.run();
    assert_eq!(fwrite(h, b"!"), 1, "the owner's handle still writes");
    assert_eq!(fclose(h), E_OK, "and is still listed: a first close");
    assert_eq!(content("C:\\FD6.TXT"), b"keep!");
}

/// `ixdel` walks `d_files` newest first and does not roll back: when the
/// caller's own OFD is the newer one it is `ixclose`d before the other
/// process's OFD stops the delete (its later `Fclose` is `EINTRN`); when the
/// foreign OFD is newer, `EACCDN` comes first and the caller's OFD stays
/// listed.
#[test]
fn ixdel_order_newest_ofd_first_without_rollback() {
    let (_g, _host) = fs();
    let a = Pd::new();
    let b = Pd::new();
    a.run();
    let h = fcreate("C:\\FD7.TXT");
    assert!(h >= 6);
    assert_eq!(fclose(h), E_OK);

    // (1) A first, B newer: B's own OFD is ixclose'd, then A's says EACCDN.
    let ha = fopen("C:\\FD7.TXT");
    b.run();
    let hb = fopen("C:\\FD7.TXT");
    assert!(ha >= 6 && hb >= 6);
    assert_eq!(fdelete("C:\\FD7.TXT"), EACCDN);
    assert_eq!(fclose(hb), EINTRN, "B's OFD was taken off d_files before the EACCDN");
    a.run();
    assert_eq!(fclose(ha), E_OK);

    // (2) B first, A newer: A's OFD says EACCDN at once.
    b.run();
    let hb = fopen("C:\\FD7.TXT");
    a.run();
    let ha = fopen("C:\\FD7.TXT");
    b.run();
    assert_eq!(fdelete("C:\\FD7.TXT"), EACCDN);
    assert_eq!(fclose(hb), E_OK, "B's OFD was never reached");
    a.run();
    assert_eq!(fclose(ha), E_OK);
    assert_eq!(content("C:\\FD7.TXT"), b"", "the file is still there");
}

/// `Fcreate` over the caller's own open file: `ixcreat` → `ixdel` closes the
/// old OFD and the new file is created; the old handle is a deleted file's.
#[test]
fn fcreate_over_an_own_open_file() {
    let (_g, host) = fs();
    let p = Pd::new();
    p.run();

    let before = free_clusters(host);
    let h1 = fcreate("C:\\FD8.TXT");
    assert!(h1 >= 6);
    assert_eq!(fwrite(h1, &[0x11; 600]), 600);
    let h2 = fcreate("C:\\FD8.TXT");
    assert!(h2 >= 6 && h2 != h1, "Fcreate over the own open file -> {h2}");
    assert_eq!(free_clusters(host), before, "the old chain is free");
    assert_eq!(fwrite(h1, b"zz"), EACCDN, "the old handle names the deleted file");
    assert_eq!(fwrite(h2, b"second"), 6);
    assert_eq!(fclose(h2), E_OK);
    assert_eq!(fclose(h1), EINTRN);
    assert_eq!(content("C:\\FD8.TXT"), b"second");
    let e = root_entry(host, b"FD8     TXT").expect("FD8 entry");
    assert_eq!(u32::from_le_bytes([e[0x1C], e[0x1D], e[0x1E], e[0x1F]]), 6);
}

/// `Ddelete` goes through the same `ixdel`: a folder created in the slot a
/// deleted-while-open file left behind is not disturbed by the stale handle
/// and is deleted normally (the deleted file's OFD is not on `d_files`).
#[test]
fn ddelete_of_a_folder_in_the_slot_of_a_deleted_open_file() {
    let (_g, host) = fs();
    let p = Pd::new();
    p.run();

    assert_eq!(dcreate("C:\\FD9D"), E_OK);
    let h = fcreate("C:\\FD9D\\F.TXT");
    assert!(h >= 6);
    assert_eq!(fwrite(h, b"abc"), 3);
    assert_eq!(fdelete("C:\\FD9D\\F.TXT"), E_OK);
    assert_eq!(dcreate("C:\\FD9D\\G"), E_OK, "takes the erased slot");

    let w0 = writes(host);
    assert_eq!(fwrite(h, b"def"), EACCDN);
    assert_eq!(fclose(h), EINTRN);
    assert_eq!(writes(host), w0, "the folder's entry is not rewritten");
    assert_eq!(ddelete("C:\\FD9D\\G"), E_OK);
    assert_eq!(ddelete("C:\\FD9D"), E_OK);
}
