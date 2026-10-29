//! `Getbpb` — parse a partition's FAT boot sector into a BIOS Parameter
//! Block. Spec: `docs/api/structures/bpb.md`, `docs/api/bios/07-getbpb.md`.
//!
//! Reduced 18-byte BPB (nine `UWORD`s). FAT12/16 only; FAT32 / exFAT
//! (`sectors_per_fat_16 == 0`) return `NULL`.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU16, Ordering};

use tos_types::ProtMode;

use super::drive;
use crate::bdos::process::{self, usermem};

/// BIOS Parameter Block (fixed Atari ABI, 18 bytes).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Bpb {
    /// Sector size in bytes.
    pub recsiz: u16,
    /// Cluster size in sectors.
    pub clsiz: u16,
    /// Cluster size in bytes.
    pub clsizb: u16,
    /// Root-directory length in sectors.
    pub rdlen: u16,
    /// FAT size in sectors (one FAT).
    pub fsiz: u16,
    /// First sector of the default FAT (the 2nd FAT on a 2-FAT disk, the
    /// sole FAT on a 1-FAT disk), partition-relative.
    pub fatrec: u16,
    /// First data sector.
    pub datrec: u16,
    /// Number of data clusters.
    pub numcl: u16,
    /// Flags: `B_16` (0x01) = 16-bit FAT, `B_1FAT` (0x02) = single FAT.
    pub b_flags: u16,
}

impl Bpb {
    const ZERO: Bpb = Bpb {
        recsiz: 0,
        clsiz: 0,
        clsizb: 0,
        rdlen: 0,
        fsiz: 0,
        fatrec: 0,
        datrec: 0,
        numcl: 0,
        b_flags: 0,
    };
}

const B_16: u16 = 0x01;
const B_1FAT: u16 = 0x02;

// FAT-geometry limits, extracted 1:1 from EmuTOS. These are the
// architectural FAT constants used by TOS/HDDRIVER; a boot sector outside
// them is not logged in (see `parse`).
/// The largest logical sector size any disk may declare — the value
/// `pun_info.max_sect_siz` holds **while the boot scan runs**:
/// `references/emutos/bios/blkdev.c:122` sets it to `MAX_LOGSEC_SIZE =
/// MAX_CLUSTER_SIZE / MIN_SECS_PER_CLUS = 32768`
/// (`references/emutos/bios/blkdev.h:30`), "temporarily, for
/// blkdev_getbpb()". See [`scan`] for the value after it.
const MAX_LOGSEC_SIZE: u16 = 32768;
/// The logical sector size GEMDOS handles at least, and the floor of the
/// run-time limit (`CONF_LOGSEC_SIZE`, `references/emutos/include/config.h:1606`;
/// `bios/blkdev.c:159`).
const CONF_LOGSEC_SIZE: u16 = 512;

/// `pun_info.max_sect_siz` — the logical sector size above which `Getbpb`
/// refuses a boot sector (`references/emutos/bios/blkdev.c:642-647`). It has
/// two values over time (`docs/api/bios/07-getbpb.md`, edge case 8): the
/// architectural maximum while [`scan`] runs, then the largest sector size the
/// scan logged in, at least [`CONF_LOGSEC_SIZE`] (BIOS-STOR-004). Until the
/// first scan it holds the architectural maximum.
static MAX_SECT_SIZ: AtomicU16 = AtomicU16::new(MAX_LOGSEC_SIZE);
/// Maximum cluster size in bytes
/// (`MAX_CLUSTER_SIZE`, `references/emutos/bios/blkdev.h:27`).
const MAX_CLUSTER_SIZE: u32 = 32768;
/// FAT16 upper cluster-count bound; more clusters means FAT32 / unsupported
/// (`MAX_FAT16_CLUSTERS`, `references/emutos/bios/blkdev.h:26`).
const MAX_FAT16_CLUSTERS: u32 = 65524;
/// FAT12/FAT16 split by cluster count
/// (`MAX_FAT12_CLUSTERS`, `references/emutos/bios/blkdev.h:25`).
const MAX_FAT12_CLUSTERS: u16 = 4084;
/// Minimum supported FAT-copy count
/// (`MIN_FATS`, `references/emutos/bios/blkdev.h:31`).
const MIN_FATS: u8 = 1;
/// Maximum supported FAT-copy count
/// (`MAX_FATS`, `references/emutos/bios/blkdev.h:32`).
const MAX_FATS: u8 = 2;

/// The per-drive BPB table, in a page of its own.
///
/// `Getbpb` publishes this table **read-only into the calling process's
/// address space** (ADR-023), and the smallest thing an AArch64 translation
/// table can grant is a whole 4-KiB page. Padding the table to exactly one
/// page — and aligning it to one — guarantees that the page an EL0 program
/// can read contains the BPBs and nothing else of the kernel's.
#[repr(C, align(4096))]
struct BpbTable {
    /// One BPB per drive letter A..P.
    entries: [Bpb; 16],
    /// Fill to a whole page so no other kernel datum shares it.
    _pad: [u8; 4096 - 16 * core::mem::size_of::<Bpb>()],
}

const _: () = assert!(core::mem::size_of::<Bpb>() == 18, "BPB ABI is 18 bytes");
const _: () = assert!(
    core::mem::size_of::<BpbTable>() == 4096,
    "the BPB table must own exactly one page — see `getbpb`'s EL0 publish",
);

struct BpbCell(UnsafeCell<BpbTable>);
// SAFETY: written and read only from the single-CPU, IRQ-off trap path.
unsafe impl Sync for BpbCell {}
static TABLE: BpbCell = BpbCell(UnsafeCell::new(BpbTable {
    entries: [Bpb::ZERO; 16],
    _pad: [0; 4096 - 16 * core::mem::size_of::<Bpb>()],
}));

/// Parse a DOS/FAT boot sector into full [`Geometry`] (the 18-byte ABI
/// [`Bpb`] plus the normalized `reserved`/`num_fats` the FAT write side
/// needs). `max_sect_siz` is the current `pun_info.max_sect_siz`
/// ([`MAX_SECT_SIZ`]).
///
/// Guards, normalizations and field arithmetic follow EmuTOS
/// `blkdev_getbpb` (`references/emutos/bios/blkdev.c:641-756`) 1:1, in the
/// same order, so a malformed / oversized / FAT32 boot record is rejected
/// (`None`) exactly as TOS refuses to log the disk in — never fed to
/// `bdos/fs/fat.rs`, where a wrong `fatrec`/`datrec`/`numcl` corrupts the
/// filesystem. `CONF_WITH_1FAT_SUPPORT` is enabled in our profile (matches
/// the existing `B_1FAT` handling), so a single FAT keeps `fatrec` at the
/// reserved offset rather than skipping a phantom first FAT.
fn parse(sec: &[u8; 512], max_sect_siz: u16) -> Option<Geometry> {
    let recsiz = u16::from_le_bytes([sec[0x0B], sec[0x0C]]);
    let clsiz = sec[0x0D] as u16; // b->spc, sectors per cluster
    let reserved_raw = u16::from_le_bytes([sec[0x0E], sec[0x0F]]); // b->res
    let num_fats_raw = sec[0x10]; // b->fat
    let root_entries = u16::from_le_bytes([sec[0x11], sec[0x12]]); // b->dir
    let tot16 = u16::from_le_bytes([sec[0x13], sec[0x14]]); // b->sec
    let fsiz16 = u16::from_le_bytes([sec[0x16], sec[0x17]]); // b->spf
    let tot32 = u32::from_le_bytes([sec[0x20], sec[0x21], sec[0x22], sec[0x23]]); // b16->sec2

    // M4 — reject a logical sector size too large to log in (blkdev.c:642-647),
    // against the limit of the moment: `MAX_LOGSEC_SIZE` during the boot scan,
    // the scan's result after it (BIOS-STOR-004, [`scan`]).
    if recsiz > max_sect_siz {
        return None;
    }

    // M4 — cluster size in bytes must be valid (blkdev.c:650-656). Computed as
    // a ULONG so `spc * recsiz` cannot wrap before the bound check; this also
    // rejects a zero `recsiz` or `clsiz` (either makes `clsizb == 0`).
    let clsizb32 = clsiz as u32 * recsiz as u32;
    if clsizb32 == 0 || clsizb32 > MAX_CLUSTER_SIZE {
        return None;
    }

    // M4 — FAT count: for TOS compatibility accept 0 and assume 2 FATs, then
    // reject anything outside 1..=2 (blkdev.c:662-670).
    let num_fats: u8 = if num_fats_raw == 0 { 2 } else { num_fats_raw };
    if !(MIN_FATS..=MAX_FATS).contains(&num_fats) {
        return None;
    }

    // clsizb <= MAX_CLUSTER_SIZE now fits a u16 (blkdev.c:675-677).
    let clsizb = clsizb32 as u16;

    // Root-directory length in sectors, rounded up (blkdev.c:685-689).
    // `recsiz != 0` is guaranteed by the `clsizb != 0` guard above.
    let rdlen = (root_entries as u32 * 32).div_ceil(recsiz as u32) as u16;

    let fsiz = fsiz16; // b->spf (blkdev.c:693)

    // M2 — `reserved == 0` should not happen, but Atari TOS assumes 1; do this
    // BEFORE computing fatrec/datrec (blkdev.c:702-704).
    let reserved = if reserved_raw == 0 { 1 } else { reserved_raw };

    // L3 — fatrec = reserved, then with >= 2 FATs skip the first FAT so fatrec
    // points at the (default) 2nd FAT (blkdev.c:705,711-712). M4 has already
    // rejected num_fats > 2, so this only ever adds one fsiz. u16 wrapping
    // matches EmuTOS's UWORD store.
    let mut fatrec = reserved;
    if num_fats >= 2 {
        fatrec = fatrec.wrapping_add(fsiz);
    }
    // First data sector (blkdev.c:713).
    let datrec = fatrec.wrapping_add(fsiz).wrapping_add(rdlen);

    // M3 — cluster count as a ULONG BEFORE narrowing, with the FAT16
    // upper-bound reject (blkdev.c:718-743). Truncating first (as the old code
    // did) would wrap a >65535-cluster disk into a small bogus count.
    // Total sectors: the 16-bit field, else the 32-bit one **whatever its
    // value** — every NewTOS drive is a hard-disk unit (`unit >= NUMFLOPPIES`,
    // blkdev.c:730-731). Both 0 gives a total below `datrec`, i.e. 0 clusters
    // and a logged-in BPB with `numcl` 0 (blkdev.c:732-733) — no fallback to
    // the partition length (BIOS-STOR-005, `docs/api/bios/07-getbpb.md` edge
    // case 9).
    let total = if tot16 != 0 { tot16 as u32 } else { tot32 };
    let clusters = if total < datrec as u32 {
        0
    } else {
        (total - datrec as u32) / clsiz as u32
    };
    // > 65524 clusters (or a zero FAT size) means FAT32 / invalid: EmuTOS marks
    // recsiz=0 for XHDI and returns 0; we discard the whole geometry so the
    // drive is inaccessible, matching the observable "cannot log in" result
    // (blkdev.c:736-742).
    if clusters > MAX_FAT16_CLUSTERS || fsiz == 0 {
        return None;
    }
    let numcl = clusters as u16;

    // FAT12 vs FAT16 purely by cluster count (blkdev.c:750-756).
    let mut b_flags = 0;
    if numcl > MAX_FAT12_CLUSTERS {
        b_flags |= B_16; // FAT16
    }
    if num_fats < 2 {
        b_flags |= B_1FAT; // CONF_WITH_1FAT_SUPPORT
    }

    let bpb = Bpb { recsiz, clsiz, clsizb, rdlen, fsiz, fatrec, datrec, numcl, b_flags };
    Some(Geometry { bpb, reserved, num_fats })
}

/// Extended FAT geometry: the 18-byte ABI [`Bpb`] plus the raw
/// `reserved` sector count and `num_fats`, which the write side needs to
/// address the first FAT and update every FAT copy.
pub struct Geometry {
    /// The ABI BPB.
    pub bpb: Bpb,
    /// Reserved sectors before the first FAT (partition-relative).
    /// Normalized like the BPB: a raw value of 0 is treated as 1
    /// (EmuTOS `blkdev.c:702-704`), so it stays consistent with `fatrec`.
    pub reserved: u16,
    /// Number of FAT copies, normalized to match the BPB: a raw value of 0
    /// is treated as 2 FATs (EmuTOS `blkdev.c:662-665`). Always 1 or 2.
    pub num_fats: u8,
}

/// EmuTOS `blkdev_getbpb` (`references/emutos/bios/blkdev.c:569-777`) up to
/// the BPB it builds: validate the drive, **reset its media-change state**,
/// mark its BPB invalid, read its boot sector, parse it, and store the result
/// in the drive's slot of [`TABLE`] (`blkdev[dev].bpb`). Shared by [`geometry`]
/// (the BDOS log-in's `Getbpb`), [`bpb_for`] (the trap's) and [`scan`] (the
/// boot scan's), so all of them leave the same state, as EmuTOS' one `Getbpb`
/// does.
///
/// The reset sits where EmuTOS has it (`blkdev.c:589-590`): after the drive
/// test, before the read and every boot-sector test — so a `Getbpb` whose
/// boot sector is then refused (`None`) has cleared it as well
/// (`docs/api/bios/07-getbpb.md`, "Gaps vs. the contract"). The Pi keeps no
/// latched `mediachange` (the SD is non-removable, `09-mediach.md`), so the
/// state to clear is `forcechange` alone (BIOS-STOR-001).
///
/// Right after it the drive's `recsiz` becomes 0, XHDI's "invalid BPB"
/// (`blkdev.c:601`, a hard-disk unit — every NewTOS drive is one), and only a
/// boot sector that passes every test sets it again: whatever ends a `Getbpb`
/// early — an unreadable sector, a refused sector size, cluster size or FAT
/// count, the FAT32 test (`blkdev.c:736-741`) — leaves it 0, and a logical
/// `Rwabs` on the drive then answers `ESECNF` ([`recsiz`], BIOS-STOR-006).
/// The other eight fields keep their values on a refusal; EmuTOS' FAT32 exit
/// has already overwritten six of them (`blkdev.c:675-713`) — not reproduced,
/// see `07-getbpb.md`.
fn log_in(drive: u16) -> Option<Geometry> {
    if drive as usize >= 16 {
        return None;
    }
    let part = drive::resolve(drive)?;
    drive::clear_forcechange(drive);
    // SAFETY: single trap path; one BPB slot per drive.
    let table = unsafe { &mut *TABLE.0.get() };
    table.entries[drive as usize].recsiz = 0;
    let blk = crate::bios::bcache::device()?;
    let mut sec = [0u8; 512];
    if blk.read_blocks(part.start_lba as u64, &mut sec).is_err() {
        return None;
    }
    let geo = parse(&sec, MAX_SECT_SIZ.load(Ordering::Relaxed))?;
    table.entries[drive as usize] = geo.bpb;
    Some(geo)
}

/// The logical sector size of `drive`'s BPB as the last `Getbpb` left it —
/// `blkdev[dev].bpb.recsiz`, which `Rwabs` tests before anything else of the
/// logical path (step 4 of `docs/api/bios/04-rwabs.md`, `blkdev.c:443-444`).
/// 0 means "invalid BPB": the last `Getbpb` refused the boot sector, or none
/// has run (a drive outside the table).
pub fn recsiz(drive: u16) -> u16 {
    // SAFETY: single trap path; a read of one slot.
    let table = unsafe { &*TABLE.0.get() };
    table.entries.get(drive as usize).map_or(0, |b| b.recsiz)
}

/// The boot scan of EmuTOS `pun_info_setup`
/// (`references/emutos/bios/blkdev.c:112-163`), the part that has an effect
/// on the Pi: a `Getbpb` of every drive the partition scan found
/// (`:145`, "force boot sector scan & BPB rebuild") with the limit at the
/// architectural maximum (`:122`), then `pun_info.max_sect_siz` = the largest
/// `recsiz` logged in, at least `CONF_LOGSEC_SIZE` (`:134,147-150,159`).
/// From then on `Getbpb` refuses a boot sector with a larger sector size
/// (`:642`, BIOS-STOR-004): on a card whose partitions all have 512-byte
/// sectors, everything above 512.
///
/// Called by `bios::mount` right after the partition scan, as EmuTOS'
/// `blkdev_init` runs `pun_info_setup` right after `disk_init_all`
/// (`:190-195`). Not reproduced (no NewTOS counterpart): the AHDI
/// `PUN_INFO` structure itself and `pun_ptr`; the `GETBPB_ALLOWED` filter on
/// the maximum (`:148`) — NewTOS logs in every partition regardless of its
/// MBR type (`drive.rs`).
pub fn scan() {
    MAX_SECT_SIZ.store(MAX_LOGSEC_SIZE, Ordering::Relaxed);
    let mut max_size = CONF_LOGSEC_SIZE;
    for d in 0..16u16 {
        if drive::resolve(d).is_none() {
            continue;
        }
        if let Some(geo) = log_in(d) {
            max_size = max_size.max(geo.bpb.recsiz);
        }
    }
    MAX_SECT_SIZ.store(max_size, Ordering::Relaxed);
}

/// Read and parse a drive's boot sector into full geometry — the BDOS
/// log-in's `Getbpb` (EmuTOS `ckdrv`, `bdos/fsdrive.c:96`), so it resets the
/// drive's media-change state and rebuilds its BPB slot like the trap does
/// ([`log_in`]).
pub fn geometry(drive: u16) -> Option<Geometry> {
    log_in(drive)
}

/// Read a drive's boot sector, parse its BPB, and cache it in the
/// per-drive static. Returns the parsed value (`None` on failure).
/// Used by [`getbpb`] (the BIOS trap).
pub fn bpb_for(drive: u16) -> Option<Bpb> {
    log_in(drive).map(|g| g.bpb)
}

/// `Getbpb` (0x07) — read the drive's boot sector, parse its BPB into
/// the per-drive static, and return a pointer to it (`0` on failure).
/// The return is a **native 64-bit pointer** (the BIOS dispatch carries
/// `i64`).
///
/// The contract is "a pointer to the drive's BPB", and the *caller* reads
/// it — so the address has to be one the caller can dereference (spec
/// `docs/api/bios/07-getbpb.md` §Outputs):
///
/// * **In-kernel (EL1) caller** — the FAT mount, EmuCON, the boot
///   self-tests: the kernel VA of the table entry is exactly that address.
/// * **EL0 caller** — the kernel is linked high-half and those pages are
///   EL1-only, so the table is published read-only (FreeMiNT `PROT_PR`,
///   `references/freemint/sys/mint/mem.h:133`) into the shared user address
///   space at its own address (VA = PA, ARCH03 stage 1) and that **user
///   address** is returned (ADR-023). All 16 entries share one page, so it
///   is one publication for every drive and every process. If the
///   publication cannot be made the answer is `0` — the contract's "no BPB
///   for this drive" — because a pointer the caller would fault on is not an
///   answer.
pub fn getbpb(drive: u16) -> i64 {
    if bpb_for(drive).is_none() {
        return 0;
    }
    // SAFETY: single trap path; the slot was just written by `bpb_for`.
    let table = unsafe { &*TABLE.0.get() };
    let kva = &table.entries[drive as usize] as *const Bpb;

    let Some(slot) = process::slot_for(process::current()) else {
        return kva as i64; // EL1 caller: the kernel VA is readable.
    };
    // SAFETY: BDOS lock held (single-CPU SVC context); `TABLE` is a
    // `'static` that owns its page, so publishing it read-only exposes
    // nothing else and it outlives every process that holds the pointer.
    match unsafe {
        usermem::expose(slot, kva.cast(), core::mem::size_of::<Bpb>(), ProtMode::Pr)
    } {
        Some(user_va) => user_va as i64,
        None => 0,
    }
}
