//! When sectors reach the card, and what a card fault does to the calls after
//! it (TASK/HW08, "Entscheidungen fuer den Fix").
//!
//! * `Fwrite` writes back what EmuTOS writes back: whole records at once,
//!   partial ones on eviction and at `Fclose` (`bdos/fsio.c:208-284`,
//!   `bdos/fsopnclo.c:576-579`). The code of a failure is the result of the
//!   call that met it.
//! * A dirty sector is kept and really tried again by every later write-back
//!   until it lands (`docs/COMPATIBILITY.md`, HW08): one fault does not block
//!   anything once the card is good again.
//! * Every GEMDOS call that meets a device error answers with its code, as
//!   `osif` does after `longjmp_rwabs` (`bdos/bdosmain.c:449-492`), instead of
//!   the "not found", "access denied" or a wrong number its own path falls
//!   into.

mod fatvol;

use core::sync::atomic::Ordering;
use fatvol::*;
use tos_api::bdos::fs::cache;

const EDRVNR: i32 = -2;
const EACCDN: i32 = -36;
const EIHNDL: i32 = -37;
const FATTRIB: u16 = 0x43;
const FDATIME: u16 = 0x57;
const FRENAME: u16 = 0x56;
const FSEEK: u16 = 0x42;
const DFREE: u16 = 0x36;

/// The lock, no fault armed, nothing open, nothing cached, no process: a failed
/// test does not spill into the next.
fn fresh() -> (std::sync::MutexGuard<'static, ()>, &'static DiskHost) {
    let (g, host) = fs();
    host.disk.disarm();
    tos_api::bdos::process::set_current(core::ptr::null_mut());
    tos_api::bdos::fs::file::free_drive_files(2);
    cache::invalidate_drive(2);
    tos_api::bdos::fs::curdir::free_drive(2);
    cache::clear_error();
    (g, host)
}

fn fwrite(h: i32, data: &[u8]) -> i32 {
    call(FWRITE, [h as u64, data.len() as u64, data.as_ptr() as u64, 0, 0, 0, 0, 0])
}

fn fread(h: i32, buf: &mut [u8]) -> i32 {
    call(FREAD, [h as u64, buf.len() as u64, buf.as_mut_ptr() as u64, 0, 0, 0, 0, 0])
}

fn fseek(h: i32, pos: u64, mode: u64) -> i32 {
    call(FSEEK, [pos, h as u64, mode, 0, 0, 0, 0, 0])
}

fn fcreate(name: &str) -> i32 {
    let p = cpath(name);
    gemdos2(FCREATE, p.as_ptr() as u64, 0)
}

fn fopen(name: &str, mode: u64) -> i32 {
    let p = cpath(name);
    gemdos2(FOPEN, p.as_ptr() as u64, mode)
}

fn fclose(h: i32) -> i32 {
    gemdos1(FCLOSE, h as u64)
}

fn fdelete(name: &str) -> i32 {
    let p = cpath(name);
    gemdos1(FDELETE, p.as_ptr() as u64)
}

fn dfree() -> (i32, [u8; 16]) {
    let mut buf = [0xAAu8; 16];
    let rc = gemdos2(DFREE, buf.as_mut_ptr() as u64, 3);
    (rc, buf)
}

/// The directory entry of an 11-byte name in the root, on the card.
fn root_entry_on_card(host: &DiskHost, name: &[u8; 11]) -> Option<[u8; 32]> {
    for s in 0..32 {
        let sec = host.read_sector(root_lba() + s);
        for e in sec.chunks_exact(32) {
            if &e[..11] == name {
                let mut o = [0u8; 32];
                o.copy_from_slice(e);
                return Some(o);
            }
        }
    }
    None
}

fn u16_at(e: &[u8], at: usize) -> u32 {
    u32::from(u16::from_le_bytes([e[at], e[at + 1]]))
}

fn len_of(e: &[u8]) -> u32 {
    u32::from_le_bytes([e[0x1C], e[0x1D], e[0x1E], e[0x1F]])
}

/// The chain of the file's first cluster, read from one FAT copy on the card.
fn chain_on_card(host: &DiskHost, copy: u32, start: u32) -> Vec<u32> {
    let mut out = Vec::new();
    let mut cl = start;
    while (2..0xFFF0).contains(&cl) && out.len() < 100 {
        out.push(cl);
        let sec = host.read_sector(fat_lba(copy) + u64::from(cl * 2 / 512));
        cl = u16_at(&sec, (cl * 2 % 512) as usize);
    }
    out
}

/// Let the data list forget the root sectors (partial reads of nine sectors
/// of a file: a list has eight slots) and then the FAT list sector 0 (`Dfree`
/// walks the whole FAT, 20 sectors in this image).
fn turn_the_cache_over() {
    let h = match fopen("C:\\TURN.DAT", 0) {
        h if h >= 0 => h,
        _ => {
            let h = fcreate("C:\\TURN.DAT");
            assert!(h >= 0);
            assert_eq!(fwrite(h, &[0u8; 512 * 12]), 512 * 12);
            assert_eq!(fclose(h), E_OK);
            fopen("C:\\TURN.DAT", 0)
        }
    };
    let mut b = [0u8; 1];
    for i in 0..9u64 {
        assert_eq!(fseek(h, i * 512, 0), (i * 512) as i32);
        assert_eq!(fread(h, &mut b), 1);
    }
    assert_eq!(fclose(h), E_OK);
    assert_eq!(dfree().0, E_OK);
}

fn remove(names: &[&str]) {
    for n in names {
        let _ = fdelete(n);
    }
}

// ─────────────────────────────────────────────────────────────────────
// Decision 2: no write-back per Fwrite
// ─────────────────────────────────────────────────────────────────────

#[test]
fn a_partial_fwrite_stays_in_the_cache_until_fclose() {
    let (_g, host) = fresh();
    let h = fcreate("C:\\PART.TXT");
    assert!(h >= 0);
    let before = host.disk.writes.load(Ordering::Relaxed);
    assert_eq!(fwrite(h, b"abc"), 3);
    // `xrw` tail bytes: getrec + memcpy, no device access (`bdos/fsio.c:261-278`).
    assert_eq!(host.disk.writes.load(Ordering::Relaxed), before, "Fwrite must not write back");
    assert_eq!(fwrite(h, b"def"), 3);
    assert_eq!(host.disk.writes.load(Ordering::Relaxed), before, "nor must the second one");

    assert_eq!(fclose(h), E_OK);
    let e = root_entry_on_card(host, b"PART    TXT").expect("entry on the card");
    assert_eq!(len_of(&e), 6);
    assert_eq!(&host.read_sector(cluster_lba(u16_at(&e, 0x1A)))[..6], b"abcdef");
    remove(&["C:\\PART.TXT"]);
}

#[test]
fn a_whole_record_goes_to_the_card_at_once_the_fat_at_fclose() {
    let (_g, host) = fresh();
    let h = fcreate("C:\\WHOLE.TXT");
    assert!(h >= 0);
    let data = vec![0x5Au8; 1024];
    let before = host.disk.writes.load(Ordering::Relaxed);
    assert_eq!(fwrite(h, &data), 1024);
    // Two clusters of one sector each, straight through (`usrio`,
    // `bdos/fsio.c:57-74`); the FAT sector stays dirty in its list.
    assert_eq!(host.disk.writes.load(Ordering::Relaxed) - before, 2, "the data, not the FAT");
    assert_eq!(fclose(h), E_OK);
    // The FAT sector in both copies and the directory sector.
    assert_eq!(host.disk.writes.load(Ordering::Relaxed) - before, 5);
    remove(&["C:\\WHOLE.TXT"]);
}

#[test]
fn a_second_handle_reads_what_the_first_has_only_in_the_cache() {
    let (_g, _host) = fresh();
    let h = fcreate("C:\\TWO.TXT");
    assert!(h >= 0);
    assert_eq!(fwrite(h, b"12345"), 5);
    assert_eq!(fclose(h), E_OK);

    let w = fopen("C:\\TWO.TXT", 2);
    assert!(w >= 0);
    assert_eq!(fwrite(w, b"XY"), 2);
    let r = fopen("C:\\TWO.TXT", 0);
    assert!(r >= 0);
    let mut buf = [0u8; 5];
    assert_eq!(fread(r, &mut buf), 5);
    assert_eq!(&buf, b"XY345", "the dirty sector, not the card's");
    assert_eq!(fclose(r), E_OK);
    assert_eq!(fclose(w), E_OK);
    remove(&["C:\\TWO.TXT"]);
}

#[test]
fn dfree_counts_clusters_whose_fat_sector_is_still_dirty() {
    let (_g, host) = fresh();
    let (rc, b0) = dfree();
    assert_eq!(rc, E_OK);
    let free0 = u32::from_le_bytes([b0[0], b0[1], b0[2], b0[3]]);
    let h = fcreate("C:\\COUNT.TXT");
    let before = host.disk.writes.load(Ordering::Relaxed);
    assert_eq!(fwrite(h, &[1u8; 700]), 700);
    let (rc, b1) = dfree();
    assert_eq!(rc, E_OK);
    assert_eq!(u32::from_le_bytes([b1[0], b1[1], b1[2], b1[3]]), free0 - 2);
    // One whole record went out; `Dfree` writes nothing itself unless it
    // evicts, and the one dirty FAT sector in this image is sector 0 of 20:
    // the walk may evict it, a device write of the FAT is not the question
    // here, the count is.
    assert!(host.disk.writes.load(Ordering::Relaxed) > before);
    assert_eq!(fclose(h), E_OK);
    remove(&["C:\\COUNT.TXT"]);
}

#[test]
fn fdatime_set_reaches_the_card_with_fclose_not_before() {
    let (_g, host) = fresh();
    let h = fcreate("C:\\DATE.TXT");
    assert!(h >= 0);
    let before = host.disk.writes.load(Ordering::Relaxed);
    // `xgsdtof` only sets `o_td` and O_DIRTY (`bdos/fsdir.c:684-707`).
    let mut td = [0x34u8, 0x12, 0x78, 0x56];
    assert_eq!(call(FDATIME, [td.as_mut_ptr() as u64, h as u64, 1, 0, 0, 0, 0, 0]), E_OK);
    assert_eq!(host.disk.writes.load(Ordering::Relaxed), before, "Fdatime set writes nothing");
    assert_eq!(fclose(h), E_OK);
    let e = root_entry_on_card(host, b"DATE    TXT").unwrap();
    assert_eq!(&e[0x16..0x1A], &td);
    remove(&["C:\\DATE.TXT"]);
}

#[test]
fn a_process_ending_without_fclose_leaves_the_file_as_fclose_would() {
    let (_g, host) = fresh();
    // A PD in host memory, `run` for the calls below (as tests/fs31_two_drives.rs).
    let bp = Box::into_raw(Box::new([0u8; 256])) as *mut tos_types::BasePage;
    // SAFETY: fresh box.
    unsafe { (*bp).p_uft = [-1, -1, -2, -3, -1, -1] };
    tos_api::bdos::process::set_current(bp);
    let h = fcreate("C:\\NOCLOSE.TXT");
    assert!(h >= 0);
    assert_eq!(fwrite(h, &[7u8; 700]), 700);
    assert!(root_entry_on_card(host, b"NOCLOSE TXT").is_some_and(|e| len_of(&e) == 0));
    // `ixterm` closes what the process left open (`bdos/proc.c:117-123`).
    // SAFETY: `bp` is live.
    unsafe { tos_api::bdos::fs::file::close_process_files(bp) };
    tos_api::bdos::process::set_current(core::ptr::null_mut());

    let e = root_entry_on_card(host, b"NOCLOSE TXT").expect("the entry on the card");
    assert_eq!(len_of(&e), 700);
    let first = u16_at(&e, 0x1A);
    assert_eq!(chain_on_card(host, 0, first).len(), 2);
    assert_eq!(chain_on_card(host, 1, first).len(), 2);
    assert_eq!(cache::writeback_stats().2, 0, "nothing dirty is left behind");
    assert_eq!(host.read_sector(cluster_lba(first))[0], 7);
    remove(&["C:\\NOCLOSE.TXT"]);
    // SAFETY: nothing names the box any more.
    unsafe { drop(Box::from_raw(bp as *mut [u8; 256])) };
}

// ─────────────────────────────────────────────────────────────────────
// Decision 1: a kept sector is really tried again
// ─────────────────────────────────────────────────────────────────────

/// `b3` of `measurements/measurement-2026-11-06-hw08/results/table.txt`.
#[test]
fn a_fat_fault_blocks_nothing_once_the_card_is_good_again() {
    let (_g, host) = fresh();
    let data = vec![9u8; 1500];
    let h = fcreate("C:\\RESULTS.PDF");
    assert!(h >= 0);

    // Every write of the FAT sector fails, in the copy written first.
    host.disk.fail_write_at(fat_lba(1), EWRITF);
    assert_eq!(fwrite(h, &data), 1500, "the fault is in the FAT, not met yet");
    assert_eq!(fclose(h), EWRITF, "Fclose carries it");

    // Far more than any former attempt limit: each save meets the fault again.
    for i in 0..6 {
        let n = format!("C:\\P{i}.FBP");
        let rc = fcreate(&n);
        assert_eq!(rc, EWRITF, "Fcreate {i} while the card refuses");
    }

    host.disk.disarm();
    let h = fcreate("C:\\R1.FBP");
    assert!(h >= 0, "the card is good again: {h}");
    assert_eq!(fwrite(h, &[1u8; 5281]), 5281);
    assert_eq!(fclose(h), E_OK);

    // Nothing was lost: the FAT sector of RESULTS.PDF landed in both copies.
    let e = root_entry_on_card(host, b"RESULTS PDF").unwrap();
    assert_eq!(len_of(&e), 1500);
    assert_eq!(chain_on_card(host, 0, u16_at(&e, 0x1A)).len(), 3);
    assert_eq!(chain_on_card(host, 1, u16_at(&e, 0x1A)).len(), 3);
    assert_eq!(cache::writeback_stats().2, 0);

    let mut names = vec!["C:\\RESULTS.PDF".to_string(), "C:\\R1.FBP".to_string()];
    names.extend((0..6).map(|i| format!("C:\\P{i}.FBP")));
    for n in &names {
        let _ = fdelete(n);
    }
}

#[test]
fn every_flush_really_tries_the_sector_again() {
    let (_g, host) = fresh();
    let h = fcreate("C:\\RETRY.TXT");
    assert!(h >= 0);
    assert_eq!(fwrite(h, b"x"), 1);
    assert_eq!(fclose(h), E_OK);
    let e = root_entry_on_card(host, b"RETRY   TXT").unwrap();
    let lba = cluster_lba(u16_at(&e, 0x1A));

    let h = fopen("C:\\RETRY.TXT", 2);
    assert_eq!(fwrite(h, b"y"), 1);
    host.disk.refused_writes.store(0, Ordering::Relaxed);
    host.disk.fail_write_at(lba, EDRVNR);
    let (f0, r0, _) = cache::writeback_stats();
    let p = cpath("C:\\RETRY.TXT");
    for i in 1..=10u32 {
        // `Fattrib` set is a flushing call (`bdos/fsdir.c:432`).
        assert_eq!(call(FATTRIB, [p.as_ptr() as u64, 1, 0, 0, 0, 0, 0, 0]), EDRVNR, "try {i}");
        assert_eq!(host.disk.refused_writes.load(Ordering::Relaxed), i, "a real device write");
    }
    let (f1, r1, dirty) = cache::writeback_stats();
    assert_eq!(f1 - f0, 10);
    assert_eq!(r1 - r0, 9);
    assert!(dirty > 0);

    host.disk.disarm();
    assert_eq!(fclose(h), E_OK);
    assert_eq!(host.read_sector(lba)[0], b'y');
    assert_eq!(cache::writeback_stats().2, 0);
    remove(&["C:\\RETRY.TXT"]);
}

#[test]
fn a_failed_fclose_frees_the_handle_the_data_stays_dirty() {
    let (_g, host) = fresh();
    let h = fcreate("C:\\HANDLE.TXT");
    assert!(h >= 0);
    assert_eq!(fwrite(h, &[3u8; 600]), 600);
    host.disk.fail_write_at(fat_lba(1), EWRITF);
    assert_eq!(fclose(h), EWRITF);
    assert_eq!(fclose(h), EIHNDL, "the handle is gone (named deviation, EmuTOS keeps it)");
    host.disk.disarm();
    let (rc, _) = dfree();
    assert_eq!(rc, E_OK);
    let h = fcreate("C:\\HANDLE2.TXT");
    assert!(h >= 0);
    assert_eq!(fclose(h), E_OK);
    assert_eq!(chain_on_card(host, 1, u16_at(&root_entry_on_card(host, b"HANDLE  TXT").unwrap(), 0x1A)).len(), 2);
    remove(&["C:\\HANDLE.TXT", "C:\\HANDLE2.TXT"]);
}

#[test]
fn fclose_that_cannot_reach_the_directory_keeps_the_handle_and_the_length() {
    let (_g, host) = fresh();
    let h = fcreate("C:\\KEEPLEN.TXT");
    assert!(h >= 0);
    assert_eq!(fwrite(h, &[4u8; 600]), 600);
    // Nothing dirty but the FAT, and the root sector is out of the data list.
    turn_the_cache_over();

    host.disk.fail_read_at(root_lba(), EREADF);
    // EmuTOS: `ixlseek`/`ixgetfcb` long-jumps out of `ixclose` before the
    // OFD is touched (`bdos/fsopnclo.c:535-536`).
    assert_eq!(fclose(h), EREADF);
    host.disk.disarm();
    assert_eq!(fclose(h), E_OK, "the same handle closes for real now");
    let e = root_entry_on_card(host, b"KEEPLEN TXT").unwrap();
    assert_eq!(len_of(&e), 600, "the length was not lost");
    remove(&["C:\\KEEPLEN.TXT"]);
}

// ─────────────────────────────────────────────────────────────────────
// Decisions 4 and 5: the device's code, not a guess
// ─────────────────────────────────────────────────────────────────────

/// The Pi: report -2, program -36 (`measurements/measurement-2026-11-06-hw08-pi/RUNINFO.TXT`).
#[test]
fn fcreate_answers_the_device_code_when_the_directory_cannot_be_read() {
    let (_g, host) = fresh();
    turn_the_cache_over();
    host.disk.fail_read_at(root_lba(), EDRVNR);
    let rc = fcreate("C:\\NEWONE.TXT");
    host.disk.disarm();
    assert_eq!(rc, EDRVNR, "not EACCDN ({EACCDN})");
    assert!(root_entry_on_card(host, b"NEWONE  TXT").is_none());
}

#[test]
fn fcreate_answers_the_device_code_when_no_slot_can_be_made_free() {
    let (_g, host) = fresh();
    // The data list full of dirty sectors that cannot be written, and a
    // directory sector that is not in it.
    let h = fcreate("C:\\FILL.TXT");
    assert!(h >= 0);
    let mut cls = Vec::new();
    for _ in 0..8 {
        assert_eq!(fwrite(h, &[0u8; 512]), 512);
    }
    assert_eq!(fclose(h), E_OK);
    let first = u16_at(&root_entry_on_card(host, b"FILL    TXT").unwrap(), 0x1A);
    cls.extend(chain_on_card(host, 0, first));
    let h = fopen("C:\\FILL.TXT", 2);
    for i in 0..8u64 {
        assert_eq!(fseek(h, i * 512, 0), (i * 512) as i32);
        assert_eq!(fwrite(h, b"z"), 1);
    }
    host.disk.fail_write_at(cluster_lba(cls[0]), EDRVNR);
    // The root sector is out of the list by now; the slot it needs can only
    // come from evicting that one.
    let rc = fcreate("C:\\NEWX.TXT");
    host.disk.disarm();
    assert_eq!(rc, EDRVNR, "not EACCDN ({EACCDN})");
    assert_eq!(fclose(h), E_OK);
    remove(&["C:\\FILL.TXT"]);
}

/// `a3`: EmuTOS has no such file afterwards, its buffer went with the error.
#[test]
fn a_refused_fcreate_leaves_no_file_behind() {
    let (_g, host) = fresh();
    host.disk.fail_write_at(root_lba(), EWRITF);
    assert_eq!(fcreate("C:\\NOFILE.TXT"), EWRITF);
    host.disk.disarm();
    let h = fcreate("C:\\AFTER.TXT");
    assert!(h >= 0);
    assert_eq!(fclose(h), E_OK);
    assert!(root_entry_on_card(host, b"NOFILE  TXT").is_none(), "no 0-byte file");
    assert!(root_entry_on_card(host, b"AFTER   TXT").is_some());
    remove(&["C:\\AFTER.TXT"]);
}

#[test]
fn an_existing_name_behind_a_read_error_is_not_created_twice() {
    let (_g, host) = fresh();
    let h = fcreate("C:\\ONCE.TXT");
    assert!(h >= 0);
    assert_eq!(fclose(h), E_OK);
    turn_the_cache_over();
    host.disk.fail_read_at(root_lba(), EREADF);
    assert_eq!(fcreate("C:\\ONCE.TXT"), EREADF);
    host.disk.disarm();
    let mut n = 0;
    for s in 0..32 {
        n += host.read_sector(root_lba() + s).chunks_exact(32).filter(|e| &e[..11] == b"ONCE    TXT").count();
    }
    assert_eq!(n, 1);
    remove(&["C:\\ONCE.TXT"]);
}

#[test]
fn the_other_calls_answer_the_device_code_too() {
    let (_g, host) = fresh();
    let h = fcreate("C:\\SITE.TXT");
    assert!(h >= 0);
    assert_eq!(fclose(h), E_OK);
    let old = cpath("C:\\SITE.TXT");
    let new = cpath("C:\\SITE2.TXT");
    let dir = cpath("C:\\SITEDIR");
    let cases: [(&str, u16, [u64; 8]); 6] = [
        ("Fopen", FOPEN, [old.as_ptr() as u64, 0, 0, 0, 0, 0, 0, 0]),
        ("Fattrib get", FATTRIB, [old.as_ptr() as u64, 0, 0, 0, 0, 0, 0, 0]),
        ("Fattrib set", FATTRIB, [old.as_ptr() as u64, 1, 0, 0, 0, 0, 0, 0]),
        ("Fdelete", FDELETE, [old.as_ptr() as u64, 0, 0, 0, 0, 0, 0, 0]),
        ("Frename", FRENAME, [0, old.as_ptr() as u64, new.as_ptr() as u64, 0, 0, 0, 0, 0]),
        ("Dcreate", DCREATE, [dir.as_ptr() as u64, 0, 0, 0, 0, 0, 0, 0]),
    ];
    for (what, f, regs) in cases {
        turn_the_cache_over();
        host.disk.fail_read_at(root_lba(), EREADF);
        let rc = call(f, regs);
        host.disk.disarm();
        assert_eq!(rc, EREADF, "{what}");
    }
    remove(&["C:\\SITE.TXT"]);
}

#[test]
fn the_first_code_of_a_call_is_its_result() {
    let (_g, host) = fresh();
    let h = fcreate("C:\\FIRST.TXT");
    assert!(h >= 0);
    assert_eq!(fwrite(h, &[5u8; 600]), 600);
    // EmuTOS jumps out at the first failed access: the FAT, -2; the directory
    // would then have failed with -10.
    host.disk.fail_write_at(fat_lba(1), EDRVNR);
    assert_eq!(fclose(h), EDRVNR);
    host.disk.disarm();
    let (rc, _) = dfree();
    assert_eq!(rc, E_OK);
    let h = fcreate("C:\\FIRST2.TXT");
    assert_eq!(fclose(h), E_OK);
    remove(&["C:\\FIRST.TXT", "C:\\FIRST2.TXT"]);
}

/// `countfree16` long-jumps at the first FAT record it cannot get and has not
/// stored anything by then (`bdos/fsfat.c:299-367`).
#[test]
fn dfree_answers_the_code_and_leaves_the_buffer_alone() {
    let (_g, host) = fresh();
    turn_the_cache_over();
    let (rc, good) = dfree();
    assert_eq!(rc, E_OK);
    turn_the_cache_over();
    host.disk.fail_read_at(fat_lba(1), EREADF);
    let (rc, buf) = dfree();
    host.disk.disarm();
    assert_eq!(rc, EREADF);
    assert_eq!(buf, [0xAAu8; 16], "nothing stored");
    let (rc, again) = dfree();
    assert_eq!(rc, E_OK);
    assert_eq!(again, good, "the number is right again");
}
