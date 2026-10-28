//! TASK/FS20 S1 — after a program changes a drive's boot sector, GEMDOS works
//! with the **new** geometry: it meets the media change, logs the drive in
//! again with a fresh `Getbpb` + `log_media` and writes where FAT and
//! directory say (`docs/api/gemdos/_state.md` §4a; EmuTOS `osif`,
//! `references/emutos/bdos/bdosmain.c:449-487`).
//!
//! The sequence is part B of the probe `tosdump/probes/gcprobe.c` (B01-B07),
//! on the in-memory volume of `fatvol`: root entries 512 -> 256 moves the data
//! area from partition sector 73 to 57 and the cluster count from 5000 to
//! 5016. Before FS20 NewTOS kept the old geometry and wrote the new file's
//! first cluster where the boot sector on disk puts another cluster
//! (`measurements/measurement-2026-10-28-bios01-s2/RUNINFO.TXT`, finding 2).
//!
//! A test binary of its own: it leaves the volume with the changed boot
//! sector.

mod fatvol;

use fatvol::*;
use tos_api::trap::{TrapArgs, TrapClass};

const C: u64 = 2;
const DFREE: u16 = 0x36;

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

fn dfree() -> (i32, [u32; 4]) {
    let mut buf = [0u32; 4];
    let rc = gemdos2(DFREE, buf.as_mut_ptr() as u64, 3);
    (rc, buf)
}

const MARKER: &[u8; 32] = b"GCPROBE-STALE-GEOMETRY-MARKER-01";

#[test]
fn b04_b06_a_changed_boot_sector_is_logged_in_and_written_with() {
    let (_g, h) = fs();
    // Old geometry: data area at partition sector 73, 5000 clusters.
    let (rc, d) = dfree();
    assert_eq!(rc, E_OK);
    assert_eq!(d[1], 5000, "total clusters before");
    assert_eq!(mediach(C), 0);

    // B01/B02: root entries 512 -> 256, written back with RW_NOMEDIACH.
    let mut boot = [0u8; SECTOR_BYTES];
    assert_eq!(bios(0x04, [2, boot.as_mut_ptr() as u64, 1, 0, C, 0, 0, 0]), 0, "B01 read sector 0");
    boot[0x11] = 0x00;
    boot[0x12] = 0x01;
    assert_eq!(bios(0x04, [3, boot.as_mut_ptr() as u64, 1, 0, C, 0, 0, 0]), 0, "B02 write it");
    assert_eq!(mediach(C), 2, "B03");

    // B04: the next Dfree runs on the new log-in.
    let (rc, d) = dfree();
    assert_eq!(rc, E_OK, "B04 Dfree");
    assert_eq!(d[1], 5016, "B04 total clusters: the boot sector on disk");
    assert_eq!(mediach(C), 0, "B05");

    // B06: a new file. Clusters 2-4 are taken, so it gets cluster 5:
    // partition sector 57 + 3 = 60 by the new geometry (76 by the old one).
    let path = cpath("C:\\GCPROBE.DAT");
    let fh = gemdos2(FCREATE, path.as_ptr() as u64, 0);
    assert!(fh >= 6, "Fcreate: {fh}");
    let mut dat = [0u8; SECTOR_BYTES];
    for (i, b) in dat.iter_mut().enumerate() {
        *b = MARKER[i % 32];
    }
    let n = call(FWRITE, [fh as u64, 512, dat.as_ptr() as u64, 0, 0, 0, 0, 0]);
    assert_eq!(n, 512, "Fwrite");
    assert_eq!(gemdos1(FCLOSE, fh as u64), E_OK, "Fclose");

    assert_eq!(h.read_sector(part_lba(60)), dat, "the marker is in cluster 5 of the new geometry");
    assert_ne!(h.read_sector(part_lba(76)), dat, "not where the old geometry has cluster 5");

    // The directory entry (new root: partition sectors 41..57) names cluster
    // 5, and both FAT copies end the chain there.
    let root = h.read_sector(part_lba(41));
    let e = root
        .chunks(32)
        .find(|e| &e[..11] == b"GCPROBE DAT")
        .expect("GCPROBE.DAT in the root directory");
    assert_eq!(u16::from_le_bytes([e[0x1A], e[0x1B]]), 5, "start cluster");
    assert_eq!(u32::from_le_bytes([e[0x1C], e[0x1D], e[0x1E], e[0x1F]]), 512, "length");
    for copy in 0..FAT_COPIES {
        let fat = h.read_sector(fat_lba(copy));
        assert_eq!(u16::from_le_bytes([fat[10], fat[11]]), 0xFFFF, "FAT copy {copy}: cluster 5 = EOC");
    }
}
