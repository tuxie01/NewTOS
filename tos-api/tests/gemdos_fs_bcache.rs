//! FS09 — the block read cache under the whole stack: GEMDOS direct
//! transfers, the BDOS buffer lists, BIOS `Rwabs` in logical and physical
//! mode all reach the card through `bios::bcache::device()`, so a second
//! read of the same data costs no device request, every write is on the card
//! at once (write-through) and every later read sees it, byte for byte.
//!
//! Spec: `docs/analysis/fs09-block-cache-2026-10-07.md` §2. Fixture: the
//! logging FAT16 volume with 4-sector clusters (`tests/fatclus`).

mod fatclus;

use std::sync::OnceLock;

use fatclus::*;
use tos_api::trap::{TrapArgs, TrapClass};

/// Install the system cache once for this binary (4 MiB, every read run
/// inserted), then start each test from an empty cache and logged-out
/// drives — `bios::mount` drops both.
fn setup() -> (std::sync::MutexGuard<'static, ()>, &'static DiskHost) {
    static ARENA: OnceLock<usize> = OnceLock::new();
    let (g, host) = fs();
    ARENA.get_or_init(|| {
        const BYTES: usize = 4 << 20;
        let arena: &'static mut [u64] = Box::leak(vec![0u64; BYTES / 8].into_boxed_slice());
        // SAFETY: leaked, owned by the cache for the life of the binary.
        let s = unsafe { tos_api::bios::bcache::install(arena.as_mut_ptr().cast(), BYTES, 100) }
            .expect("cache installs");
        assert!(s.slots > 7000, "{s:?}");
        arena.as_ptr() as usize
    });
    let before = tos_api::bios::bcache::stats().unwrap().invalidations;
    assert_eq!(tos_api::bios::mount(), 1);
    let s = tos_api::bios::bcache::stats().unwrap();
    assert_eq!(s.invalidations, before + 1, "mount empties the cache");
    // Then it holds what the scan just read: the MBR and C:'s boot sector,
    // read by the boot scan's Getbpb (`bios::bpb::scan`, EmuTOS
    // `pun_info_setup`, BIOS01).
    assert_eq!(s.used, 2, "then holds only the MBR and C:'s boot sector");
    host.clear_log();
    (g, host)
}

fn reads(log: &[Req]) -> usize {
    log.iter().filter(|r| !r.write).count()
}

/// BIOS `Rwabs(r_w, buf, count, first, drive, lfirst)` through the trap
/// dispatcher, as an in-kernel caller.
fn rwabs(r_w: u64, buf: *mut u8, count: u64, first: i64, drive: u64, lfirst: u64) -> i64 {
    let args = TrapArgs {
        regs: [r_w, buf as u64, count, first as u64, drive, lfirst, 0, 0],
        func: 4,
        svc_imm: TrapClass::Bios as u16,
        space: tos_api::uaccess::Space::Kernel,
    };
    tos_api::bios::dispatch(&args)
}

#[test]
fn reading_a_file_again_costs_no_device_request() {
    let (_g, host) = setup();
    for chunk in [512usize, 1000, 4096, 65536] {
        let h = fopen("C:\\SEQ.DAT", 0);
        assert!(h >= 6);
        let got = read_all(h, chunk);
        assert_eq!(fclose(h), 0);
        assert_eq!(got, expected(SEQ_SEED, SEQ_LEN), "chunk {chunk}");
    }
    let first = host.log();
    assert!(reads(&first) > 0);
    // Everything SEQ.DAT needs (directory, FAT, data) is in the cache now;
    // a fresh log-in (as after a drive change in the BDOS) still finds it.
    host.clear_log();
    tos_api::bdos::fs::fat::log_out_all();
    for chunk in [512usize, 4096, SEQ_LEN as usize + 10] {
        let h = fopen("C:\\SEQ.DAT", 0);
        let got = read_all(h, chunk);
        fclose(h);
        assert_eq!(got, expected(SEQ_SEED, SEQ_LEN), "warm chunk {chunk}");
    }
    assert_eq!(reads(&host.log()), 0, "warm pass hit the card: {:?}", host.log());
    let s = tos_api::bios::bcache::stats().unwrap();
    assert!(s.hit_sectors > 0);
}

#[test]
fn a_partly_cached_fragmented_file_reads_only_the_gaps() {
    let (_g, host) = setup();
    // Read the middle of FRAG.DAT (records of the 2nd and 3rd run) first.
    let h = fopen("C:\\FRAG.DAT", 0);
    assert_eq!(fseek(2 * CLSIZB as i64, h, 0), 2 * CLSIZB as i32);
    let mut mid = vec![0u8; 2 * CLSIZB as usize];
    assert_eq!(fread(h, &mut mid), 2 * CLSIZB as i32);
    fclose(h);
    let want = expected(FRAG_SEED, FRAG_LEN);
    assert_eq!(&mid[..], &want[2 * CLSIZB as usize..4 * CLSIZB as usize]);
    host.clear_log();
    // Now the whole file in one Fread: clusters 90 and 62 come from the cache.
    let h = fopen("C:\\FRAG.DAT", 0);
    let mut all = vec![0u8; FRAG_LEN as usize];
    assert_eq!(fread(h, &mut all), FRAG_LEN as i32);
    fclose(h);
    assert_eq!(all, want);
    for r in host.log() {
        assert!(
            !(r.lba < cluster_lba(90) + 4 && r.lba + r.sectors as u64 > cluster_lba(90))
                && !(r.lba < cluster_lba(62) + 4 && r.lba + r.sectors as u64 > cluster_lba(62)),
            "cached clusters read again: {r:?}"
        );
    }
}

#[test]
fn written_data_is_on_the_card_at_once_and_read_back_from_the_cache() {
    let (_g, host) = setup();
    let name = "C:\\WTHRU.DAT";
    let data: Vec<u8> = (0..20_000u32).map(|i| (i * 31 + i / 7) as u8).collect();
    let h = fcreate(name);
    assert!(h >= 6, "Fcreate -> {h}");
    // Mixed sizes: partial records through the BDOS buffers, whole records
    // direct — both through the block cache.
    let mut off = 0;
    for n in [100usize, 3000, 512, 8192, 1] {
        assert_eq!(fwrite(h, &data[off..off + n]), n as i32);
        off += n;
    }
    assert_eq!(fwrite(h, &data[off..]), (data.len() - off) as i32);
    assert_eq!(fclose(h), 0);
    // Patch across a record boundary.
    let h = fopen(name, 2);
    assert_eq!(fseek(1020, h, 0), 1020);
    assert_eq!(fwrite(h, b"PATCHED!"), 8);
    fclose(h);
    let mut want = data.clone();
    want[1020..1028].copy_from_slice(b"PATCHED!");

    host.clear_log();
    let h = fopen(name, 0);
    let got = read_all(h, 777);
    fclose(h);
    assert_eq!(got, want);
    assert_eq!(reads(&host.log()), 0, "written sectors come from the cache: {:?}", host.log());

    // And the card itself holds every byte (write-through): drop the cache
    // and the BDOS state, read again from the card.
    tos_api::bios::bcache::invalidate_all();
    tos_api::bdos::fs::fat::log_out_all();
    host.clear_log();
    let h = fopen(name, 0);
    let got = read_all(h, 4096);
    fclose(h);
    assert_eq!(got, want, "card content");
    assert!(reads(&host.log()) > 0);
    assert_eq!(fdelete(name), 0);
}

#[test]
fn logical_and_physical_rwabs_share_one_copy() {
    let (_g, host) = setup();
    const C: u64 = 2; // drive C:
    const PHYS: u64 = 8; // RW_NOTRANSLATE
    const SD_UNIT: u64 = 26; // the SD card's physical unit (BIOS-STOR-002)
    let rel = 5u64; // a partition-relative sector (inside the first FAT)
    let abs = part_lba(rel as u32);
    let mut orig = vec![0u8; SECTOR];
    assert_eq!(rwabs(0, orig.as_mut_ptr(), 1, rel as i64, C, 0), 0);
    host.clear_log();
    // The same sector, physically addressed: a hit.
    let mut phys = vec![0u8; SECTOR];
    assert_eq!(rwabs(PHYS, phys.as_mut_ptr(), 1, abs as i64, SD_UNIT, 0), 0);
    assert_eq!(phys, orig);
    assert_eq!(reads(&host.log()), 0, "physical read of a logically cached sector");
    // Physical write of new bytes; a logical read sees them, from the cache.
    let mut new: Vec<u8> = (0..SECTOR).map(|i| (i as u8) ^ 0xC3).collect();
    assert_eq!(rwabs(PHYS | 1, new.as_mut_ptr(), 1, abs as i64, SD_UNIT, 0), 0);
    assert_eq!(host.raw(abs), new, "on the card at once");
    host.clear_log();
    let mut back = vec![0u8; SECTOR];
    assert_eq!(rwabs(0, back.as_mut_ptr(), 1, rel as i64, C, 0), 0);
    assert_eq!(back, new);
    assert_eq!(reads(&host.log()), 0);
    // Restore through the logical path (keeps the FAT intact for the others).
    assert_eq!(rwabs(1, orig.as_mut_ptr(), 1, rel as i64, C, 0), 0);
    assert_eq!(host.raw(abs), orig);
    let mut again = vec![0u8; SECTOR];
    assert_eq!(rwabs(PHYS, again.as_mut_ptr(), 1, abs as i64, SD_UNIT, 0), 0);
    assert_eq!(again, orig);
}

#[test]
fn a_new_partition_scan_drops_the_cache() {
    let (_g, host) = setup();
    let mut buf = vec![0u8; 4 * SECTOR];
    assert_eq!(rwabs(0, buf.as_mut_ptr(), 4, 100, 2, 0), 0);
    host.clear_log();
    assert_eq!(rwabs(0, buf.as_mut_ptr(), 4, 100, 2, 0), 0);
    assert_eq!(reads(&host.log()), 0);
    assert_eq!(tos_api::bios::mount(), 1);
    host.clear_log();
    assert_eq!(rwabs(0, buf.as_mut_ptr(), 4, 100, 2, 0), 0);
    assert_eq!(
        host.log(),
        vec![Req { write: false, lba: part_lba(100), sectors: 4 }],
        "after bios::mount the card is asked again"
    );
}
