//! BIOS drive table — maps TOS drive letters to partitions on the block
//! device (the SD card).
//!
//! TOS numbers drives `A:`=0, `B:`=1, `C:`=2, … Floppies would be A:/B:
//! (we have none), so the SD card's partitions are assigned from **C:**
//! upward. The MBR (sector 0) is parsed at [`mount`]; `Rwabs` then maps
//! `(drive, sector)` to an absolute LBA via [`resolve`].

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};

const MAX_DRIVES: usize = 16;
/// First hard-disk drive letter (`C:`); A:/B: are reserved for floppies.
const FIRST_HD: usize = 2;

/// The physical unit number of the SD card — the `drive` argument of an
/// `Rwabs` with `RW_NOTRANSLATE` (BIOS-STOR-002, `docs/api/bios/04-rwabs.md`,
/// "Decisions"). EmuTOS numbers physical units `NUMFLOPPIES + 8*bus + device`
/// (`GET_UNITNUM`, `references/emutos/bios/disk.h:18,20,30`); the SD/MMC bus is
/// bus 3 (`SDMMC_BUS`, `bios/machine.h:39-42`) and its driver serves device 0
/// only (`sd_rw`, `bios/sd.c:160-161`): `2 + 8*3 + 0` = 26. The Pi has one
/// block device, so this is the only physical unit there is.
pub const SD_UNIT: u16 = 2 + 8 * 3;

/// A mounted partition: its absolute start sector, length, and MBR type.
#[derive(Clone, Copy)]
pub struct Partition {
    /// Absolute starting LBA on the block device.
    pub start_lba: u32,
    /// Length in 512-byte sectors.
    pub count: u32,
    /// MBR partition type byte (e.g. 0x06 FAT16, 0x0B/0x0C FAT32).
    pub kind: u8,
}

struct TableCell(UnsafeCell<[Option<Partition>; MAX_DRIVES]>);
// SAFETY: filled once at boot (`mount`) and read from the single-CPU,
// IRQ-off trap path (`Rwabs`); never concurrently.
unsafe impl Sync for TableCell {}

static TABLE: TableCell = TableCell(UnsafeCell::new([None; MAX_DRIVES]));

/// MBR partition type bytes that denote a container/extended partition
/// (not a directly-mountable filesystem) — skipped in bring-up.
fn is_extended(kind: u8) -> bool {
    matches!(kind, 0x05 | 0x0F | 0x85)
}

/// Read 16-byte MBR/EBR partition entry `i` from a sector buffer:
/// `(kind, start_lba, count)`. `start_lba` is relative to the table's
/// own base (caller adds the appropriate offset).
fn entry(sec: &[u8; 512], i: usize) -> (u8, u32, u32) {
    let e = 0x1BE + i * 16;
    let kind = sec[e + 4];
    let start = u32::from_le_bytes([sec[e + 8], sec[e + 9], sec[e + 10], sec[e + 11]]);
    let count = u32::from_le_bytes([sec[e + 12], sec[e + 13], sec[e + 14], sec[e + 15]]);
    (kind, start, count)
}

/// Parse the MBR on the installed block device and (re)build the drive
/// table. Returns the number of drives mounted. Primary partitions map to
/// `C:`+ in MBR order; an extended partition's logical chain (EBRs) is
/// then walked, appending each logical volume. A device with no MBR
/// partitions (a "super-floppy" FAT image) is mounted whole as `C:`.
pub fn mount() -> usize {
    let blk = match crate::bios::bcache::device() {
        Some(b) => b,
        None => return 0,
    };
    let mut sec0 = [0u8; 512];
    if blk.read_blocks(0, &mut sec0).is_err() {
        return 0;
    }

    // SAFETY: boot CPU, single-threaded.
    let table = unsafe { &mut *TABLE.0.get() };
    *table = [None; MAX_DRIVES];

    let mut drive = FIRST_HD;
    let mut mounted = 0;

    if sec0[510] == 0x55 && sec0[511] == 0xAA {
        // Primary entries first (in MBR order), remembering the extended
        // partition's start for the logical-chain walk afterwards.
        let mut ext_base = 0u32;
        for i in 0..4 {
            let (kind, start, count) = entry(&sec0, i);
            if kind == 0 || count == 0 {
                continue;
            }
            if is_extended(kind) {
                ext_base = start;
            } else if drive < MAX_DRIVES {
                table[drive] = Some(Partition { start_lba: start, count, kind });
                drive += 1;
                mounted += 1;
            }
        }

        // Walk the extended partition's EBR chain. Each EBR's first entry
        // is a logical volume (start relative to that EBR); the second
        // points to the next EBR (start relative to `ext_base`).
        if ext_base != 0 {
            let mut ebr = ext_base;
            // Bounded by MAX_DRIVES iterations to defend against a loop.
            for _ in 0..MAX_DRIVES {
                let mut buf = [0u8; 512];
                if blk.read_blocks(ebr as u64, &mut buf).is_err()
                    || buf[510] != 0x55
                    || buf[511] != 0xAA
                {
                    break;
                }
                let (lk, ls, lc) = entry(&buf, 0);
                if lk != 0 && lc != 0 && drive < MAX_DRIVES {
                    table[drive] = Some(Partition {
                        start_lba: ebr + ls,
                        count: lc,
                        kind: lk,
                    });
                    drive += 1;
                    mounted += 1;
                }
                let (_nk, ns, nc) = entry(&buf, 1);
                if ns == 0 || nc == 0 {
                    break;
                }
                ebr = ext_base + ns;
            }
        }

        if mounted > 0 {
            return mounted;
        }
    }

    // No partitions → the whole device is one volume on C:.
    let count = blk.block_count().min(u32::MAX as u64) as u32;
    table[FIRST_HD] = Some(Partition { start_lba: 0, count, kind: 0 });
    1
}

/// Resolve a TOS drive number to its partition.
pub fn resolve(drive: u16) -> Option<Partition> {
    if drive as usize >= MAX_DRIVES {
        return None;
    }
    // SAFETY: single trap path; table is only mutated at boot.
    unsafe { (*TABLE.0.get())[drive as usize] }
}

/// Per-drive `forcechange` flag — EmuTOS `blkdev[dev].forcechange`
/// (`references/emutos/bios/blkdev.h:141-159`): "force EmuTOS to recognise a
/// media change when logical sector zero is written to". Armed by a logical
/// `Rwabs` write starting at sector 0 (`bios/blkdev.c:452-453`), tested by
/// `Rwabs` (`:450-451`) and `Mediach` (`:802-803`), cleared only by `Getbpb`
/// (`:590`). A partition rescan ([`mount`]) leaves it alone, as EmuTOS'
/// `add_partition` does (`bios/blkdev.c:354-381` writes `mediachange`, not
/// `forcechange`). Spec: `docs/api/bios/04-rwabs.md` (order table, steps 5
/// and 6), `09-mediach.md`, `07-getbpb.md`.
static FORCECHANGE: [AtomicBool; MAX_DRIVES] = [const { AtomicBool::new(false) }; MAX_DRIVES];

/// Is `drive`'s `forcechange` armed? `false` for a drive number outside the
/// table (there is no such `blkdev[]` entry to be armed).
pub fn forcechange(drive: u16) -> bool {
    FORCECHANGE
        .get(drive as usize)
        .is_some_and(|f| f.load(Ordering::Relaxed))
}

/// Arm `drive`'s `forcechange` (`bios/blkdev.c:452-453`). Callers have
/// validated the drive; a number outside the table is ignored.
pub fn arm_forcechange(drive: u16) {
    if let Some(f) = FORCECHANGE.get(drive as usize) {
        f.store(true, Ordering::Relaxed);
    }
}

/// Clear `drive`'s `forcechange` — the reset at the top of `Getbpb`
/// (`bios/blkdev.c:590`). A number outside the table is ignored.
pub fn clear_forcechange(drive: u16) {
    if let Some(f) = FORCECHANGE.get(drive as usize) {
        f.store(false, Ordering::Relaxed);
    }
}

/// Bitmap of present drives (bit `n` = drive `n`), for `Drvmap`.
pub fn drvbits() -> u32 {
    let mut bits = 0u32;
    // SAFETY: single trap path.
    let table = unsafe { &*TABLE.0.get() };
    for (i, p) in table.iter().enumerate() {
        if p.is_some() {
            bits |= 1 << i;
        }
    }
    bits
}
