//! Phase 08 microSD read/write self-test (board-rpi4 / QEMU raspi4b).
//!
//! Initialises the EMMC2 microSD card, reads sector 0 (checking the
//! MBR/FAT boot-sector signature `0x55AA`), reports the capacity from
//! the CSD, and does a **non-destructive** write + read-back on the last
//! sector (original saved and restored).

use crate::drivers::bcm2711::emmc;
use crate::hal::block::BlockDevice;
use crate::kprintln;

/// Run the microSD read/write self-test.
pub fn run() {
    kprintln!("[emmc] === microSD (SD-host) read/write test ===");
    // SAFETY: boot CPU; the VideoCore firmware has finished with the card.
    if !unsafe { emmc::init() } {
        kprintln!("[emmc] card init failed (no card? attach -drive ...,if=sd)");
        return;
    }
    let blocks = emmc::SDCARD.block_count();
    kprintln!(
        "[emmc] card initialised: {} blocks ({} MiB)",
        blocks,
        blocks * 512 / (1024 * 1024),
    );

    // ── read sector 0 ──
    let mut buf = [0u8; 512];
    match emmc::SDCARD.read_blocks(0, &mut buf) {
        Ok(()) => {
            let sig = u16::from_le_bytes([buf[510], buf[511]]);
            kprintln!(
                "[emmc] sector 0: {:02x} {:02x} {:02x} {:02x} ..., sig=0x{:04x} {}",
                buf[0],
                buf[1],
                buf[2],
                buf[3],
                sig,
                if sig == 0xAA55 { "(valid 55AA)" } else { "(no 55AA)" },
            );
        }
        Err(e) => {
            kprintln!("[emmc] sector 0 read FAILED: {:?}", e);
            return;
        }
    }

    // ── non-destructive write + read-back on the last sector ──
    if blocks > 1 {
        let scratch = blocks - 1;
        let mut orig = [0u8; 512];
        let mut test = [0u8; 512];
        let mut back = [0u8; 512];
        for (i, b) in test.iter_mut().enumerate() {
            *b = (i as u8) ^ 0xA5;
        }
        test[..9].copy_from_slice(b"NEWTOS-WR");

        let result = emmc::SDCARD
            .read_blocks(scratch, &mut orig)
            .and_then(|_| emmc::SDCARD.write_blocks(scratch, &test))
            .and_then(|_| emmc::SDCARD.read_blocks(scratch, &mut back))
            .map(|_| back == test);
        // Restore the original contents regardless of outcome.
        let _ = emmc::SDCARD.write_blocks(scratch, &orig);

        match result {
            Ok(true) => kprintln!("[emmc] write+read-back @ sector {} OK (restored)", scratch),
            Ok(false) => kprintln!("[emmc] write+read-back @ sector {} MISMATCH", scratch),
            Err(e) => kprintln!("[emmc] write test @ sector {} FAILED: {:?}", scratch, e),
        }
    }

    r1_rejection_test();
    multi_block_test();

    // ── partition table + BIOS Rwabs over the real trap path ──
    let n = tos_api::bios::mount();
    let map = drvmap();
    kprintln!("[emmc] mounted {} partition(s); Drvmap=0x{:x}", n, map);

    if map != 0 {
        let drv = map.trailing_zeros() as u64; // first present drive
        let letter = (b'A' + drv as u8) as char;
        let mut rbuf = [0u8; 512];
        // Read sector 0 of the partition (its FAT/boot sector).
        let rc = rwabs(0, rbuf.as_mut_ptr() as u64, 1, 0, drv, 0);
        let sig = u16::from_le_bytes([rbuf[510], rbuf[511]]);
        kprintln!(
            "[emmc] Rwabs(read sec 0 of {}:) -> {} ; first {:02x} {:02x}, sig=0x{:04x} {}",
            letter,
            rc,
            rbuf[0],
            rbuf[1],
            sig,
            if rc == 0 { "OK" } else { "FAIL" },
        );

        // ── L1: physical-mode Rwabs (RW_NOTRANSLATE, bit 3) ──
        // Bit 3 set ⇒ `drive` is the physical unit — the SD card is unit 26
        // (`tos_api::bios::drive::SD_UNIT`, BIOS-STOR-002) — and the sector
        // argument an ABSOLUTE device sector with no partition translation.
        // Reading absolute sector 0 must return the raw MBR — byte-identical
        // to a direct block read, and (when the drive starts past sector 0,
        // e.g. C: behind an MBR) different from the logical read of the
        // drive's own sector 0 above.
        const RW_NOTRANSLATE: u64 = 8;
        let mut raw0 = [0u8; 512];
        let mut phys0 = [0u8; 512];
        let raw_ok = emmc::SDCARD.read_blocks(0, &mut raw0).is_ok();
        let unit = tos_api::bios::drive::SD_UNIT as u64;
        let prc = rwabs(RW_NOTRANSLATE, phys0.as_mut_ptr() as u64, 1, 0, unit, 0);
        let matches_raw = phys0 == raw0;
        let differs_logical = phys0 != rbuf;
        kprintln!(
            "[emmc] Rwabs(PHYS abs-sec 0) -> {} ; ==raw_block:{} !=logical:{} {}",
            prc,
            matches_raw,
            differs_logical,
            if prc == 0 && raw_ok && matches_raw { "OK" } else { "FAIL" },
        );

        // ── Getbpb on the FAT partition (C:) and the bare one (D:) ──
        let bp = getbpb(drv);
        if bp != 0 {
            // The BPB is nine consecutive UWORDs at the returned pointer.
            let p = bp as *const u16;
            let f = |i: usize| unsafe { p.add(i).read() };
            kprintln!(
                "[emmc] Getbpb({}:) recsiz={} clsiz={} clsizb={} rdlen={} fsiz={} fatrec={} datrec={} numcl={} flags=0x{:x}",
                letter, f(0), f(1), f(2), f(3), f(4), f(5), f(6), f(7), f(8),
            );
        } else {
            kprintln!("[emmc] Getbpb({}:) -> NULL", letter);
        }
        let bd = getbpb(drv + 1);
        kprintln!(
            "[emmc] Getbpb({}:) -> {}",
            (b'A' + (drv + 1) as u8) as char,
            if bd == 0 { "NULL (no FAT BPB)" } else { "ptr" },
        );

        // ── Phase 09 D1: FAT cluster-chain reader on C: ──
        fat_test(drv as u16, letter);
    }
}

/// HW03 / audit finding 3: a command the **card** refuses must be reported
/// as a failure, not as success.
///
/// The lever is the one the audit drove (`docs/api/hw/emmc-audit.md`
/// finding 3): an access far past the end of the card. The card answers
/// `CMD17`/`CMD24` with an R1 status carrying `OUT_OF_RANGE` (bit 31) or
/// `ADDRESS_ERROR` (bit 30) — SD Phys 9.10 Table 4-42,
/// `docs/hardware/emmc-sd.md` §5 — and the controller's `INTERRUPT`
/// register stays at 0, which is why the old driver reported `Ok(())`.
///
/// **Non-destructive on real hardware by construction:** the probe goes
/// 4096 blocks past the capacity the card itself reported in its CSD — the
/// same offset the audit drove — so there is no sector of this card it
/// could reach. A card that refuses the command never enters the data
/// phase, so the buffer below is never transferred.
fn r1_rejection_test() {
    let beyond = emmc::SDCARD.block_count() + 4096;

    let mut buf = [0u8; 512];
    let rd = emmc::SDCARD.read_blocks(beyond, &mut buf);
    let wr = emmc::SDCARD.write_blocks(beyond, &buf);
    let s = emmc::error_stats();
    kprintln!(
        "[emmc] R1 out-of-range probe @ block {}: read -> {:?}, write -> {:?} {}",
        beyond,
        rd,
        wr,
        if rd.is_err() && wr.is_err() { "OK (refused)" } else { "FAIL (accepted!)" },
    );
    crate::kprint!(
        "[emmc] R1 probe: {} card rejection(s), last status=0x{:08x}",
        s.card_errors,
        s.last_card_status,
    );
    for (i, n) in s.card_bits.iter().enumerate() {
        if *n != 0 {
            crate::kprint!(" {}={}", emmc::R1_BIT_NAMES[i], n);
        }
    }
    kprintln!("");
}

/// FS08: one `CMD18` / `CMD25` over several sectors must move the same
/// bytes as one `CMD17` / `CMD24` per sector.
///
/// Works on the **last** [`MB_SECTORS`] sectors of the card, so the read
/// also covers the end-of-card case in which SD Phys 9.10 §4.3.3 tells the
/// host to ignore a spurious `OUT_OF_RANGE` (in the Auto CMD12 response or
/// the next command's). **Non-destructive:** the original contents are
/// read first and written back at the end, then checked.
fn multi_block_test() {
    const MB_SECTORS: usize = 64;
    const SZ: usize = 512;
    let blocks = emmc::SDCARD.block_count();
    if blocks <= MB_SECTORS as u64 {
        return;
    }
    let lba = blocks - MB_SECTORS as u64;
    let before = emmc::error_stats();
    let mut orig = alloc::vec![0u8; MB_SECTORS * SZ];
    let mut single = alloc::vec![0u8; MB_SECTORS * SZ];
    let mut back = alloc::vec![0u8; MB_SECTORS * SZ];
    let mut test = alloc::vec![0u8; MB_SECTORS * SZ];
    for (i, b) in test.iter_mut().enumerate() {
        *b = (i as u8) ^ ((i / SZ) as u8).wrapping_mul(37) ^ 0x5A;
    }

    // Multi-block read vs. one single-block read per sector.
    let read_multi = emmc::SDCARD.read_blocks(lba, &mut orig);
    let mut read_single = Ok(());
    for (i, s) in single.chunks_exact_mut(SZ).enumerate() {
        read_single = read_single.and_then(|_| emmc::SDCARD.read_blocks(lba + i as u64, s));
    }
    let reads_agree = read_multi.is_ok() && read_single.is_ok() && orig == single;
    kprintln!(
        "[emmc] FS08 multi-read {} sectors @ {} (to the card's end): multi {:?}, single {:?}, equal={} {}",
        MB_SECTORS,
        lba,
        read_multi,
        read_single,
        orig == single,
        if reads_agree { "OK" } else { "FAIL" },
    );
    if !reads_agree {
        return; // no write without a trustworthy copy of the original
    }

    // Multi-block write, read back both ways.
    let write_multi = emmc::SDCARD.write_blocks(lba, &test);
    let back_multi = emmc::SDCARD.read_blocks(lba, &mut back);
    let multi_ok = write_multi.is_ok() && back_multi.is_ok() && back == test;
    let mut back_single = Ok(());
    for (i, s) in back.chunks_exact_mut(SZ).enumerate() {
        back_single = back_single.and_then(|_| emmc::SDCARD.read_blocks(lba + i as u64, s));
    }
    let single_ok = back_single.is_ok() && back == test;
    // Restore, and check the restore.
    let restore = emmc::SDCARD.write_blocks(lba, &orig);
    let check = emmc::SDCARD.read_blocks(lba, &mut back);
    let restored = restore.is_ok() && check.is_ok() && back == orig;
    let after = emmc::error_stats();
    kprintln!(
        "[emmc] FS08 multi-write {} sectors: write {:?}, read-back multi={} single={}, restored={} (card rejections +{}, io errors +{}, timeouts +{}) {}",
        MB_SECTORS,
        write_multi,
        multi_ok,
        single_ok,
        restored,
        after.card_errors - before.card_errors,
        after.io_errors - before.io_errors,
        after.timeouts - before.timeouts,
        if multi_ok && single_ok && restored { "OK" } else { "FAIL" },
    );
}

/// Phase 09 D1: mount the FAT volume on `drive` and walk the cluster
/// chain of the first directory entry, verifying the chain and the data.
fn fat_test(drive: u16, letter: char) {
    use tos_api::bdos::fs::fat::Volume;

    let vol = match Volume::mount(drive) {
        Some(v) => v,
        None => {
            kprintln!("[emmc] FAT mount({}:) -> not a FAT volume", letter);
            return;
        }
    };
    kprintln!(
        "[emmc] FAT mount({}:) clsizb={} numcl={} root@{}(rel)",
        letter,
        vol.clsizb,
        vol.numcl,
        vol.root_sector(),
    );

    // Look up HELLO.TXT for the chain walk (the first raw root entry is
    // the volume label, cluster 0).
    use tos_api::bdos::fs::dir;
    let entry = match dir::fcb_name("hello.txt").and_then(|n| dir::find_in_root(&vol, &n)) {
        Some(e) => e,
        None => {
            kprintln!("[emmc] HELLO.TXT not found");
            return;
        }
    };
    let (start, size) = (entry.cluster, entry.size);
    kprintln!(
        "[emmc] HELLO.TXT cluster={} size={}",
        start,
        size,
    );

    // Walk the FAT chain and read the first cluster's content.
    let mut cl = start;
    let mut chain = 0u32;
    let mut first = [0u8; 64];
    while vol.is_data_cluster(cl) && chain < 64 {
        if chain == 0 {
            let mut buf = [0u8; 2048];
            if vol.read_cluster(cl, &mut buf) {
                first.copy_from_slice(&buf[..64]);
            }
        }
        chain += 1;
        match vol.fat_entry(cl) {
            Some(next) if vol.is_eoc(next) => break,
            Some(next) => cl = next,
            None => break,
        }
    }
    let want = size.div_ceil(vol.clsizb).max(1);
    kprintln!(
        "[emmc] chain length={} (expected {} clusters) {}",
        chain,
        want,
        if chain == want { "OK" } else { "MISMATCH" },
    );
    kprintln!(
        "[emmc] cluster[0] first bytes: \"{}\"",
        core::str::from_utf8(&first[..32]).unwrap_or("?"),
    );

    // FAT type + next-cluster sanity (genuine FAT16: fat_entry(2) == 3).
    kprintln!("[emmc] fat_entry(2) -> {:?} (expect Some(3))", vol.fat_entry(2));

    // ── directory traversal (D5 foundation): list root dir + lookup ──
    let mut count = 0u32;
    for e in dir::DirIter::root(&vol) {
        kprintln!(
            "[emmc]   entry: \"{}\" attr=0x{:02x} cl={} size={}",
            core::str::from_utf8(&e.name).unwrap_or("?"),
            e.attrib,
            e.cluster,
            e.size,
        );
        count += 1;
    }
    let hit = dir::fcb_name("hello.txt")
        .and_then(|n| dir::find_in_root(&vol, &n))
        .map(|e| e.cluster);
    kprintln!(
        "[emmc] root entries={}; find(\"hello.txt\") -> {}",
        count,
        hit.unwrap_or_default(),
    );

    // ── Phase 09 D4: GEMDOS Fopen/Fread/Fseek/Fclose (EC1) ──
    gemdos_file_test();
}

/// Phase 09 D4 / EC1: open `c:/hello.txt` through GEMDOS, read it across
/// a cluster boundary, seek, and close — all over the real `svc #1` path.
fn gemdos_file_test() {
    let path = b"c:/hello.txt\0";
    let h = gemdos_fopen(path.as_ptr() as u64, 0);
    kprintln!("[emmc] Fopen(\"c:/hello.txt\") -> {}", h);
    if h < 6 {
        kprintln!("[emmc] Fopen FAILED");
        return;
    }
    let handle = h as u16;

    // Read 64 bytes from the start.
    let mut buf = [0u8; 64];
    let n = gemdos_fread(handle, buf.len() as i64, buf.as_mut_ptr() as u64);
    kprintln!(
        "[emmc] Fread(64) -> {} : \"{}\"",
        n,
        core::str::from_utf8(&buf[..32]).unwrap_or("?"),
    );

    // Seek to size-via-END, then back to an offset spanning cluster 2->3
    // (cluster boundary is at clsizb = 512 bytes).
    let size = gemdos_fseek(0, handle, 2);
    kprintln!("[emmc] Fseek(0,END) -> size={} (expect 1000)", size);
    let pos = gemdos_fseek(508, handle, 0); // 4 bytes before cluster boundary
    let mut b2 = [0u8; 32];
    let n2 = gemdos_fread(handle, b2.len() as i64, b2.as_mut_ptr() as u64);
    kprintln!(
        "[emmc] Fseek(508,SET)->{} ; Fread(32) -> {} across cluster 2->3: \"{}\"",
        pos,
        n2,
        core::str::from_utf8(&b2[..n2.max(0) as usize]).unwrap_or("?"),
    );

    // Read at EOF returns 0.
    gemdos_fseek(0, handle, 2);
    let eof = gemdos_fread(handle, 16, buf.as_mut_ptr() as u64);
    kprintln!("[emmc] Fread at EOF -> {} (expect 0)", eof);

    let rc = gemdos_fclose(handle);
    kprintln!("[emmc] Fclose -> {} {}", rc, if rc == 0 { "OK" } else { "FAIL" });

    // ── Phase 09 D5: Fsfirst/Fsnext enumeration (EC2) ──
    fsearch_test();
}

/// Phase 09 D5 / EC2: enumerate the root directory via Fsetdta +
/// Fsfirst/Fsnext, both with a `*.TXT` filter and a `*.*` wildcard.
fn fsearch_test() {
    let mut dta = [0u8; 44];
    gemdos_fsetdta(dta.as_mut_ptr() as u64);

    let pat = b"c:/*.txt\0";
    let mut rc = gemdos_fsfirst(pat.as_ptr() as u64, 0);
    kprintln!("[emmc] Fsfirst(\"c:/*.txt\") -> {}", rc);
    let mut found = 0;
    while rc == 0 {
        let size = u32::from_le_bytes([dta[26], dta[27], dta[28], dta[29]]);
        kprintln!("[emmc]   match: \"{}\" attr=0x{:02x} size={}", dta_name(&dta), dta[21], size);
        found += 1;
        rc = gemdos_fsnext();
    }
    kprintln!("[emmc] *.txt matches={} ; final rc={} (expect -49 ENMFIL)", found, rc);

    // Wildcard over everything (volume label excluded by att=0).
    let all = b"c:/*.*\0";
    let mut rc2 = gemdos_fsfirst(all.as_ptr() as u64, 0);
    let mut n2 = 0;
    while rc2 == 0 {
        n2 += 1;
        rc2 = gemdos_fsnext();
    }
    kprintln!("[emmc] *.* matches={} (expect 3, label excluded)", n2);

    // Volume-label-exclusive search (att = FA_VOL = 0x08).
    let rc3 = gemdos_fsfirst(all.as_ptr() as u64, 0x08);
    kprintln!("[emmc] Fsfirst(\"c:/*.*\", FA_VOL) -> {} name=\"{}\"", rc3, dta_name(&dta));

    // ── Phase 09 D6: Dgetdrv / Dsetdrv / Dfree ──
    ddrive_test();

    // ── Phase 09 D7: write side (Fcreate/Fwrite/Fdelete) ──
    fwrite_test();

    // ── Subdirectories: findit path resolution ──
    subdir_test();

    // ── Fattrib / Frename / Fdatime ──
    fmeta_test();

    // ── Phase 09 D2: sector-cache effectiveness ──
    let (hits, misses) = tos_api::bdos::fs::cache::stats();
    let total = hits + misses;
    let pct = (hits * 100).checked_div(total).unwrap_or(0);
    kprintln!(
        "[emmc] sector cache: {} hits / {} misses ({}% hit rate)",
        hits, misses, pct,
    );
}

/// Phase 09 D6: current-drive state and free-space query.
fn ddrive_test() {
    kprintln!("[emmc] Dgetdrv -> {} (default C:)", gemdos_dgetdrv());
    let map = gemdos_dsetdrv(2); // select C:
    kprintln!("[emmc] Dsetdrv(C:) -> drvmap=0x{:x} ; Dgetdrv -> {}", map, gemdos_dgetdrv());

    let mut info = [0u8; 16];
    let rc = gemdos_dfree(info.as_mut_ptr() as u64, 3); // drv 3 = C: (1-based)
    let free = u32::from_le_bytes([info[0], info[1], info[2], info[3]]);
    let total = u32::from_le_bytes([info[4], info[5], info[6], info[7]]);
    let recsiz = u32::from_le_bytes([info[8], info[9], info[10], info[11]]);
    let clsiz = u32::from_le_bytes([info[12], info[13], info[14], info[15]]);
    kprintln!(
        "[emmc] Dfree(C:) -> {} ; free={} total={} recsiz={} clsiz={} ; free={} KiB",
        rc, free, total, recsiz, clsiz, (free as u64 * recsiz as u64 * clsiz as u64) / 1024,
    );
}

/// Subdirectory path resolution: read a file inside SUB/, enumerate the
/// subdirectory, create a file in it, and check a bad path → EPTHNF.
fn subdir_test() {
    // Read a file in a subdirectory.
    let p = b"c:/SUB/INNER.TXT\0";
    let h = gemdos_fopen(p.as_ptr() as u64, 0);
    let mut buf = [0u8; 64];
    let n = if h >= 6 {
        gemdos_fread(h as u16, buf.len() as i64, buf.as_mut_ptr() as u64)
    } else {
        -1
    };
    if h >= 6 {
        gemdos_fclose(h as u16);
    }
    kprintln!(
        "[emmc] Fopen(\"c:/SUB/INNER.TXT\")={} read={} : \"{}\"",
        h, n, core::str::from_utf8(&buf[..n.clamp(0, 40) as usize]).unwrap_or("?"),
    );

    // Enumerate the subdirectory (att=0 excludes "." / ".." subdirs).
    let mut dta = [0u8; 44];
    gemdos_fsetdta(dta.as_mut_ptr() as u64);
    let pat = b"c:/SUB/*.*\0";
    let mut rc = gemdos_fsfirst(pat.as_ptr() as u64, 0);
    let mut count = 0;
    while rc == 0 {
        kprintln!("[emmc]   SUB entry: \"{}\"", dta_name(&dta));
        count += 1;
        rc = gemdos_fsnext();
    }
    kprintln!("[emmc] Fsfirst(\"c:/SUB/*.*\")={} entries (expect 1: INNER.TXT)", count);

    // Create a file inside the subdirectory, then read it back.
    let np = b"c:/SUB/CREATED.TXT\0";
    let hc = gemdos_fcreate(np.as_ptr() as u64, 0);
    if hc >= 6 {
        gemdos_fwrite(hc as u16, 11, c"in subdir!\n".as_ptr() as u64);
        gemdos_fclose(hc as u16);
        let hr = gemdos_fopen(np.as_ptr() as u64, 0);
        let mut b2 = [0u8; 16];
        let r2 = gemdos_fread(hr as u16, 11, b2.as_mut_ptr() as u64);
        gemdos_fclose(hr as u16);
        kprintln!(
            "[emmc] Fcreate(\"c:/SUB/CREATED.TXT\")={} ; read back {} : \"{}\"",
            hc, r2, core::str::from_utf8(&b2[..r2.max(0) as usize]).unwrap_or("?"),
        );
        gemdos_fdelete(np.as_ptr() as u64);
    } else {
        kprintln!("[emmc] Fcreate in subdir FAILED -> {}", hc);
    }

    // Bad path component → EPTHNF (-34).
    let bad = b"c:/NOPE/x.txt\0";
    let hb = gemdos_fopen(bad.as_ptr() as u64, 0);
    kprintln!("[emmc] Fopen(\"c:/NOPE/x.txt\") -> {} (expect -34 EPTHNF)", hb);

    // ── Dcreate / Ddelete ──
    let dir = b"c:/MYDIR\0";
    let c = gemdos_dcreate(dir.as_ptr() as u64);
    // Create a file inside the new directory, then read it back.
    let f = b"c:/MYDIR/FILE.TXT\0";
    let hf = gemdos_fcreate(f.as_ptr() as u64, 0);
    if hf >= 6 {
        gemdos_fwrite(hf as u16, 7, c"nested\n".as_ptr() as u64);
        gemdos_fclose(hf as u16);
    }
    // Ddelete on a non-empty directory must fail (EACCDN -36).
    let de_full = gemdos_ddelete(dir.as_ptr() as u64);
    // Remove the file, then the now-empty directory should delete.
    gemdos_fdelete(f.as_ptr() as u64);
    let de_empty = gemdos_ddelete(dir.as_ptr() as u64);
    let reopen = gemdos_dcreate(dir.as_ptr() as u64); // can re-create → was freed
    gemdos_ddelete(dir.as_ptr() as u64); // clean up
    kprintln!(
        "[emmc] Dcreate(MYDIR)={} ; file={} ; Ddelete(non-empty)={} (exp -36) ; Ddelete(empty)={} ; re-Dcreate={}",
        c, hf, de_full, de_empty, reopen,
    );

    // ── Dsetpath / Dgetpath + current-dir-relative paths ──
    let mut pbuf = [0u8; 64];
    let set = gemdos_dsetpath(c"c:/SUB".as_ptr() as u64);
    gemdos_dgetpath(pbuf.as_mut_ptr() as u64, 3); // drv 3 = C:
    let cur = cstr_of(&pbuf);
    // Relative open: "INNER.TXT" should resolve inside the current dir.
    let hrel = gemdos_fopen(c"INNER.TXT".as_ptr() as u64, 0);
    let mut rb = [0u8; 32];
    let rn = if hrel >= 6 {
        let n = gemdos_fread(hrel as u16, 20, rb.as_mut_ptr() as u64);
        gemdos_fclose(hrel as u16);
        n
    } else {
        -1
    };
    kprintln!(
        "[emmc] Dsetpath(c:/SUB)={} Dgetpath=\"{}\" ; rel Fopen(INNER.TXT)={} read={}",
        set, cur, hrel, rn,
    );
    // ".." back to root, then reset to root.
    gemdos_dsetpath(c"..".as_ptr() as u64);
    let mut pbuf2 = [0u8; 64];
    gemdos_dgetpath(pbuf2.as_mut_ptr() as u64, 3);
    kprintln!("[emmc] Dsetpath(..) -> Dgetpath=\"{}\" (expect root \"\")", cstr_of(&pbuf2));
}

/// Fattrib / Frename / Fdatime over a freshly created file.
fn fmeta_test() {
    let f = c"c:/META.TXT";
    // A previous run that stopped between SET(0x21) and the clear below
    // leaves a read-only META.TXT that Fcreate cannot replace; clear it so the
    // test is re-runnable on the same card (EFILNF when absent, harmless).
    gemdos_fattrib(f.as_ptr() as u64, 1, 0x00);
    let hc = gemdos_fcreate(f.as_ptr() as u64, 0);
    if hc >= 6 {
        gemdos_fwrite(hc as u16, 5, c"meta\n".as_ptr() as u64);
        gemdos_fclose(hc as u16);
    }

    // Fattrib: get (0 — created with attr 0), set read-only (0x21), get.
    let g0 = gemdos_fattrib(f.as_ptr() as u64, 0, 0);
    let s1 = gemdos_fattrib(f.as_ptr() as u64, 1, 0x21);
    let g1 = gemdos_fattrib(f.as_ptr() as u64, 0, 0);
    // Reject FA_SUBDIR via Fattrib → EACCDN.
    let bad = gemdos_fattrib(f.as_ptr() as u64, 1, 0x10);
    kprintln!(
        "[emmc] Fattrib get={:#x} set(0x21)={:#x} get={:#x} ; set(SUBDIR)={} (exp -36)",
        g0, s1, g1, bad,
    );
    // Hard checks (docs/api/gemdos/file/43-fattrib.md "Return value"): a
    // set returns the attribute byte it stored, a following get reads it
    // back, and a set with a bit outside FA_NORM is EACCDN (-36).
    assert_eq!(g0, 0, "[emmc] fmeta Fattrib GET on a freshly created file");
    assert_eq!(s1, 0x21, "[emmc] fmeta Fattrib SET(0x21) returns the stored byte");
    assert_eq!(g1, 0x21, "[emmc] fmeta Fattrib GET after SET(0x21)");
    assert_eq!(bad, -36, "[emmc] fmeta Fattrib SET(FA_SUBDIR) must be EACCDN");
    // Clear read-only again so we can rename/delete it.
    gemdos_fattrib(f.as_ptr() as u64, 1, 0x00);

    // ── Fattrib TOS4/FreeMiNT dir-GET extension (QED clipboard / cflib) ──
    // GET on a folder must return its attribute (FA_SUBDIR = 0x10 set), for
    // both the trailing-backslash form cflib passes and the plain form; SET on
    // a folder must stay rejected (EACCDN); GET on a normal file and a missing
    // name are unchanged. Create a CLIPBRD directory to mirror QED's probe.
    let cdir = c"c:/CLIPBRD";
    let dc = gemdos_dcreate(cdir.as_ptr() as u64);
    let dir_bs = gemdos_fattrib(c"C:\\CLIPBRD\\".as_ptr() as u64, 0, 0); // (a) trailing '\'
    let dir_ns = gemdos_fattrib(cdir.as_ptr() as u64, 0, 0); //             (a) plain form
    let file_get = gemdos_fattrib(f.as_ptr() as u64, 0, 0); //             (b) normal file
    let dir_set = gemdos_fattrib(cdir.as_ptr() as u64, 1, 0x00); //        (c) SET on folder
    let miss = gemdos_fattrib(c"c:/NOSUCH.XYZ".as_ptr() as u64, 0, 0); //  (d) non-existent
    kprintln!(
        "[emmc] Fattrib dir GET(\"C:\\CLIPBRD\\\")={:#x} (exp>=0 &0x10) dir GET(\"C:\\CLIPBRD\")={:#x} \
         ; file GET={:#x} (&0x10==0) ; dir SET={} (exp -36) ; missing GET={} (exp -33)",
        dir_bs, dir_ns, file_get, dir_set, miss,
    );
    // Hard checks, one per case, so a regression stops the run with the
    // case name instead of printing `false` (TASK/T06; the Q07 defect was
    // exactly (a): the trailing-separator spelling). Clean up first so a
    // failing assertion does not leave the folder behind on the card.
    let _ = dc;
    gemdos_ddelete(cdir.as_ptr() as u64);
    assert!(
        dir_bs >= 0 && (dir_bs & 0x10) != 0,
        "[emmc] fmeta (a) Fattrib GET on \"C:\\CLIPBRD\\\" (trailing '\\') must return the folder attribute with FA_SUBDIR, got {}",
        dir_bs,
    );
    assert!(
        dir_ns >= 0 && (dir_ns & 0x10) != 0,
        "[emmc] fmeta (a) Fattrib GET on \"C:\\CLIPBRD\" (plain) must return the folder attribute with FA_SUBDIR, got {}",
        dir_ns,
    );
    assert!(
        file_get >= 0 && (file_get & 0x10) == 0,
        "[emmc] fmeta (b) Fattrib GET on a normal file must be >= 0 without FA_SUBDIR, got {}",
        file_get,
    );
    assert_eq!(dir_set, -36, "[emmc] fmeta (c) Fattrib SET on a folder must be EACCDN");
    assert_eq!(miss, -33, "[emmc] fmeta (d) Fattrib GET on a missing name must be EFILNF");

    // Fdatime: set a known stamp on the open file, close, reopen, get.
    let h = gemdos_fopen(f.as_ptr() as u64, 0);
    let stamp: [u16; 2] = [0x6000, 0x5AA1]; // time=12:00:02, date=2025-05-01
    gemdos_fdatime(stamp.as_ptr() as u64, h as u16, 1);
    gemdos_fclose(h as u16);
    let h2 = gemdos_fopen(f.as_ptr() as u64, 0);
    let mut got = [0u16; 2];
    gemdos_fdatime(got.as_mut_ptr() as u64, h2 as u16, 0);
    gemdos_fclose(h2 as u16);
    kprintln!(
        "[emmc] Fdatime set=[{:#06x},{:#06x}] get=[{:#06x},{:#06x}] match={}",
        stamp[0], stamp[1], got[0], got[1], got == stamp,
    );

    // Frename META.TXT → RENAMED.TXT, verify old gone / new present.
    let nf = c"c:/RENAMED.TXT";
    let rn = gemdos_frename(f.as_ptr() as u64, nf.as_ptr() as u64);
    let old_open = gemdos_fopen(f.as_ptr() as u64, 0);
    let new_open = gemdos_fopen(nf.as_ptr() as u64, 0);
    if new_open >= 6 {
        gemdos_fclose(new_open as u16);
    }
    kprintln!(
        "[emmc] Frename={} ; old open={} (exp -33) ; new open={} (exp >=6)",
        rn, old_open, new_open,
    );
    gemdos_fdelete(nf.as_ptr() as u64); // clean up
}

/// GEMDOS `Fattrib` (0x43): x0=path, x1=wrt, x2=mod.
fn gemdos_fattrib(path: u64, wrt: u16, mode: u16) -> i64 {
    let r: i64;
    // SAFETY: svc #1, GEMDOS Fattrib ABI; `path` is a kernel string.
    unsafe { core::arch::asm!("svc #1", in("x8") 0x43u64, inout("x0") path => r, in("x1") wrt as u64, in("x2") mode as u64, options(nostack)); }
    r
}

/// GEMDOS `Frename` (0x56): x0=n(unused), x1=src, x2=dst.
fn gemdos_frename(src: u64, dst: u64) -> i64 {
    let r: i64;
    // SAFETY: svc #1, GEMDOS Frename ABI; src/dst are kernel strings.
    unsafe { core::arch::asm!("svc #1", in("x8") 0x56u64, inout("x0") 0u64 => r, in("x1") src, in("x2") dst, options(nostack)); }
    r
}

/// GEMDOS `Fdatime` (0x57): x0=buf, x1=handle, x2=wrt.
fn gemdos_fdatime(buf: u64, handle: u16, wrt: u16) -> i64 {
    let r: i64;
    // SAFETY: svc #1, GEMDOS Fdatime ABI; `buf` is a 4-byte kernel buffer.
    unsafe { core::arch::asm!("svc #1", in("x8") 0x57u64, inout("x0") buf => r, in("x1") handle as u64, in("x2") wrt as u64, options(nostack)); }
    r
}

/// Read a NUL-terminated string from a byte buffer.
fn cstr_of(b: &[u8]) -> &str {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    core::str::from_utf8(&b[..end]).unwrap_or("?")
}

/// GEMDOS `Dsetpath` (0x3B): x0=path.
fn gemdos_dsetpath(path: u64) -> i64 {
    let r: i64;
    // SAFETY: svc #1, GEMDOS Dsetpath ABI; `path` is a kernel string.
    unsafe { core::arch::asm!("svc #1", in("x8") 0x3Bu64, inout("x0") path => r, options(nostack)); }
    r
}

/// GEMDOS `Dgetpath` (0x47): x0=buf, x1=drv.
fn gemdos_dgetpath(buf: u64, drv: u16) -> i64 {
    let r: i64;
    // SAFETY: svc #1, GEMDOS Dgetpath ABI; `buf` is a kernel buffer.
    unsafe { core::arch::asm!("svc #1", in("x8") 0x47u64, inout("x0") buf => r, in("x1") drv as u64, options(nostack)); }
    r
}

/// GEMDOS `Dcreate` (0x39): x0=path.
fn gemdos_dcreate(path: u64) -> i64 {
    let r: i64;
    // SAFETY: svc #1, GEMDOS Dcreate ABI; `path` is a kernel string.
    unsafe { core::arch::asm!("svc #1", in("x8") 0x39u64, inout("x0") path => r, options(nostack)); }
    r
}

/// GEMDOS `Ddelete` (0x3A): x0=path.
fn gemdos_ddelete(path: u64) -> i64 {
    let r: i64;
    // SAFETY: svc #1, GEMDOS Ddelete ABI; `path` is a kernel string.
    unsafe { core::arch::asm!("svc #1", in("x8") 0x3Au64, inout("x0") path => r, options(nostack)); }
    r
}

/// Phase 09 D7 / EC3: create a file, write data spanning two clusters,
/// close, re-open and read it back, then delete it.
fn fwrite_test() {
    let path = b"c:/WRITE.TXT\0";
    // Build a 1500-byte payload (> 2 clusters of 512 bytes) so cluster
    // allocation *and* chain linking are exercised. Each byte encodes its
    // index so a corrupted cluster boundary is detectable.
    let mut payload = [0u8; 1500];
    for (i, b) in payload.iter_mut().enumerate() {
        *b = b'!' + (i % 90) as u8; // printable, period 90
    }

    let h = gemdos_fcreate(path.as_ptr() as u64, 0);
    kprintln!("[emmc] Fcreate(\"c:/WRITE.TXT\") -> {}", h);
    if h < 6 {
        kprintln!("[emmc] Fcreate FAILED");
        return;
    }
    let n = gemdos_fwrite(h as u16, payload.len() as i64, payload.as_ptr() as u64);
    let rc = gemdos_fclose(h as u16);
    kprintln!("[emmc] Fwrite({}) -> {} ; Fclose -> {}", payload.len(), n, rc);

    // Re-open, check size, and read back the whole file.
    let h2 = gemdos_fopen(path.as_ptr() as u64, 0);
    let size = gemdos_fseek(0, h2 as u16, 2);
    gemdos_fseek(0, h2 as u16, 0);
    let mut back = [0u8; 1500];
    let r = gemdos_fread(h2 as u16, back.len() as i64, back.as_mut_ptr() as u64);
    let ok = r == payload.len() as i64 && back == payload;
    kprintln!(
        "[emmc] re-read size={} -> {} bytes, match={} (spans {} clusters)",
        size, r, ok, (payload.len() as u32).div_ceil(512),
    );
    gemdos_fclose(h2 as u16);

    // Fdelete on a throwaway file (leave WRITE.TXT for host verification).
    let dp = b"c:/DELME.TXT\0";
    let hd = gemdos_fcreate(dp.as_ptr() as u64, 0);
    gemdos_fwrite(hd as u16, 4, c"bye\n".as_ptr() as u64);
    gemdos_fclose(hd as u16);
    let d = gemdos_fdelete(dp.as_ptr() as u64);
    let h3 = gemdos_fopen(dp.as_ptr() as u64, 0);
    kprintln!(
        "[emmc] Fdelete(DELME.TXT) -> {} ; re-open -> {} (expect -33 EFILNF)",
        d, h3,
    );
}

/// GEMDOS `Fcreate` (0x3C): x0=name, x1=attr.
fn gemdos_fcreate(name: u64, attr: u16) -> i64 {
    let r: i64;
    // SAFETY: svc #1, GEMDOS Fcreate ABI; `name` is a kernel string.
    unsafe { core::arch::asm!("svc #1", in("x8") 0x3Cu64, inout("x0") name => r, in("x1") attr as u64, options(nostack)); }
    r
}

/// GEMDOS `Fwrite` (0x40): x0=handle, x1=len, x2=buf.
fn gemdos_fwrite(handle: u16, len: i64, buf: u64) -> i64 {
    let r: i64;
    // SAFETY: svc #1, GEMDOS Fwrite ABI; `buf` is a kernel buffer.
    unsafe { core::arch::asm!("svc #1", in("x8") 0x40u64, inout("x0") handle as u64 => r, in("x1") len, in("x2") buf, options(nostack)); }
    r
}

/// GEMDOS `Fdelete` (0x41): x0=name.
fn gemdos_fdelete(name: u64) -> i64 {
    let r: i64;
    // SAFETY: svc #1, GEMDOS Fdelete ABI; `name` is a kernel string.
    unsafe { core::arch::asm!("svc #1", in("x8") 0x41u64, inout("x0") name => r, options(nostack)); }
    r
}

/// GEMDOS `Dgetdrv` (0x19): no args.
fn gemdos_dgetdrv() -> i64 {
    let r: i64;
    // SAFETY: svc #1, GEMDOS Dgetdrv ABI.
    unsafe { core::arch::asm!("svc #1", in("x8") 0x19u64, out("x0") r, options(nostack)); }
    r
}

/// GEMDOS `Dsetdrv` (0x0E): x0=drive.
fn gemdos_dsetdrv(drv: u16) -> i64 {
    let r: i64;
    // SAFETY: svc #1, GEMDOS Dsetdrv ABI.
    unsafe { core::arch::asm!("svc #1", in("x8") 0x0Eu64, inout("x0") drv as u64 => r, options(nostack)); }
    r
}

/// GEMDOS `Dfree` (0x36): x0=buf, x1=drv.
fn gemdos_dfree(buf: u64, drv: u16) -> i64 {
    let r: i64;
    // SAFETY: svc #1, GEMDOS Dfree ABI; `buf` is a 16-byte kernel buffer.
    unsafe { core::arch::asm!("svc #1", in("x8") 0x36u64, inout("x0") buf => r, in("x1") drv as u64, options(nostack)); }
    r
}

/// Extract the NUL-terminated `d_fname` (offset 30) from a 44-byte DTA.
fn dta_name(d: &[u8; 44]) -> &str {
    let s = &d[30..44];
    let end = s.iter().position(|&c| c == 0).unwrap_or(s.len());
    core::str::from_utf8(&s[..end]).unwrap_or("?")
}

/// GEMDOS `Fsetdta` (0x1A): x0=addr.
fn gemdos_fsetdta(addr: u64) {
    // SAFETY: svc #1, GEMDOS Fsetdta ABI.
    unsafe {
        core::arch::asm!("svc #1", in("x8") 0x1Au64, in("x0") addr, lateout("x0") _, options(nostack));
    }
}

/// GEMDOS `Fsfirst` (0x4E): x0=name, x1=att.
fn gemdos_fsfirst(name: u64, att: u16) -> i64 {
    let r: i64;
    // SAFETY: svc #1, GEMDOS Fsfirst ABI; `name` is a kernel string.
    unsafe {
        core::arch::asm!("svc #1", in("x8") 0x4Eu64, inout("x0") name => r, in("x1") att as u64, options(nostack));
    }
    r
}

/// GEMDOS `Fsnext` (0x4F): no args.
fn gemdos_fsnext() -> i64 {
    let r: i64;
    // SAFETY: svc #1, GEMDOS Fsnext ABI.
    unsafe {
        core::arch::asm!("svc #1", in("x8") 0x4Fu64, out("x0") r, options(nostack));
    }
    r
}

/// GEMDOS `Fopen` (0x3D) via `svc #1`: x0=name, x1=mode.
fn gemdos_fopen(name: u64, mode: u64) -> i64 {
    let r: i64;
    // SAFETY: svc #1, GEMDOS Fopen ABI; `name` is a kernel string pointer.
    unsafe {
        core::arch::asm!("svc #1", in("x8") 0x3Du64, inout("x0") name => r, in("x1") mode, options(nostack));
    }
    r
}

/// GEMDOS `Fread` (0x3F) via `svc #1`: x0=handle, x1=len, x2=buf.
fn gemdos_fread(handle: u16, len: i64, buf: u64) -> i64 {
    let r: i64;
    // SAFETY: svc #1, GEMDOS Fread ABI; `buf` is a kernel buffer.
    unsafe {
        core::arch::asm!("svc #1", in("x8") 0x3Fu64, inout("x0") handle as u64 => r, in("x1") len, in("x2") buf, options(nostack));
    }
    r
}

/// GEMDOS `Fseek` (0x42) via `svc #1`: x0=offset, x1=handle, x2=flag.
fn gemdos_fseek(offset: i64, handle: u16, flag: u16) -> i64 {
    let r: i64;
    // SAFETY: svc #1, GEMDOS Fseek ABI.
    unsafe {
        core::arch::asm!("svc #1", in("x8") 0x42u64, inout("x0") offset => r, in("x1") handle as u64, in("x2") flag as u64, options(nostack));
    }
    r
}

/// GEMDOS `Fclose` (0x3E) via `svc #1`: x0=handle.
fn gemdos_fclose(handle: u16) -> i64 {
    let r: i64;
    // SAFETY: svc #1, GEMDOS Fclose ABI.
    unsafe {
        core::arch::asm!("svc #1", in("x8") 0x3Eu64, inout("x0") handle as u64 => r, options(nostack));
    }
    r
}

/// BIOS `Getbpb` via `svc #13` (function id 0x07) — returns a `BPB *`.
#[inline(always)]
fn getbpb(drive: u64) -> i64 {
    let r: i64;
    // SAFETY: svc #13, BIOS Getbpb (func 0x07), drive in x0.
    unsafe {
        core::arch::asm!("svc #13", in("x8") 0x07u64, inout("x0") drive => r, options(nostack));
    }
    r
}

/// BIOS `Drvmap` via `svc #13` (function id 0x0A).
#[inline(always)]
fn drvmap() -> u32 {
    let r: i64;
    // SAFETY: svc #13, BIOS Drvmap (func 0x0A), no arguments.
    unsafe {
        core::arch::asm!("svc #13", in("x8") 0x0Au64, out("x0") r, options(nostack));
    }
    r as u32
}

/// BIOS `Rwabs` via `svc #13` (function id 4 in x8).
#[inline(always)]
fn rwabs(r_w: u64, adr: u64, numb: u64, first: u64, drive: u64, lfirst: u64) -> i64 {
    let r: i64;
    // SAFETY: svc #13, BIOS Rwabs ABI (func 4); kernel buffer in `adr`.
    unsafe {
        core::arch::asm!(
            "svc #13",
            in("x8") 4u64,
            inout("x0") r_w => r,
            in("x1") adr,
            in("x2") numb,
            in("x3") first,
            in("x4") drive,
            in("x5") lfirst,
            options(nostack),
        );
    }
    r
}
