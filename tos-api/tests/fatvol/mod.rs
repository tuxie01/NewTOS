//! A FAT16 volume in host memory, wired to the TOS-API layer as its block
//! device, so the GEMDOS file-system calls can be driven from a host test.
//!
//! Shared by the integration tests in `tests/gemdos_*.rs`. Each of those is a
//! separate binary with its own copy of the TOS-API statics, so each installs
//! its own host and owns its own image.

#![allow(dead_code)] // each test binary uses a different part of this

use std::collections::VecDeque;
use std::sync::atomic::{AtomicI32, AtomicU32, AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

use tos_api::host::Host;
use tos_api::trap::{TrapArgs, TrapClass};

// ── FAT16 volume, built in memory ─────────────────────────────────────────

const SECTOR: usize = 512;

/// LBA of the partition inside the image (the MBR's `start_lba`).
const PART_LBA: u32 = 64;

// Geometry of the synthesized volume. Cluster count must exceed
// `MAX_FAT12_CLUSTERS` (4084, `references/emutos/bios/blkdev.h:25`) or the BPB
// parser marks the volume FAT12, which is read-only here.
const CLSIZ: u32 = 1; // sectors per cluster
const RESERVED: u32 = 1;
const NUM_FATS: u32 = 2;
const ROOT_ENTRIES: u32 = 512;
const RDLEN: u32 = ROOT_ENTRIES * 32 / SECTOR as u32; // 32 sectors
const NUMCL: u32 = 5000;
const FSIZ: u32 = ((NUMCL + 2) * 2).div_ceil(SECTOR as u32); // 20 sectors
/// Partition-relative sector of the first FAT copy.
const FAT1: u32 = RESERVED;
/// Partition-relative sector of the root directory.
const ROOT_SEC: u32 = RESERVED + NUM_FATS * FSIZ;
/// Partition-relative sector of cluster 2.
const DATA_SEC: u32 = ROOT_SEC + RDLEN;
const PART_SECTORS: u32 = DATA_SEC + NUMCL * CLSIZ;

/// Build a 32-byte FAT directory entry.
fn dirent(name: &[u8; 11], attr: u8, cluster: u16, size: u32) -> [u8; 32] {
    let mut e = [0u8; 32];
    e[0..11].copy_from_slice(name);
    e[0x0B] = attr;
    e[0x16..0x18].copy_from_slice(&0x8000u16.to_le_bytes()); // 16:00:00
    e[0x18..0x1A].copy_from_slice(&0x5901u16.to_le_bytes()); // 2024-08-01
    e[0x1A..0x1C].copy_from_slice(&cluster.to_le_bytes());
    e[0x1C..0x20].copy_from_slice(&size.to_le_bytes());
    e
}

/// A whole disk image: MBR + one FAT16 partition holding
///
/// ```text
/// C:\HELLO.TXT     5 bytes, cluster 3
/// C:\SUB\          directory, cluster 2
/// C:\SUB\INNER.TXT 3 bytes, cluster 4
/// ```
fn build_image() -> Vec<u8> {
    let total = (PART_LBA + PART_SECTORS) as usize;
    let mut img = vec![0u8; total * SECTOR];

    // ── MBR ──
    {
        let e = 0x1BE;
        img[e + 4] = 0x06; // FAT16 partition type
        img[e + 8..e + 12].copy_from_slice(&PART_LBA.to_le_bytes());
        img[e + 12..e + 16].copy_from_slice(&PART_SECTORS.to_le_bytes());
        img[510] = 0x55;
        img[511] = 0xAA;
    }

    let part = PART_LBA as usize * SECTOR;

    // ── boot sector / BPB ──
    {
        let b = &mut img[part..part + SECTOR];
        b[0x0B..0x0D].copy_from_slice(&(SECTOR as u16).to_le_bytes()); // bytes/sector
        b[0x0D] = CLSIZ as u8; // sectors/cluster
        b[0x0E..0x10].copy_from_slice(&(RESERVED as u16).to_le_bytes());
        b[0x10] = NUM_FATS as u8;
        b[0x11..0x13].copy_from_slice(&(ROOT_ENTRIES as u16).to_le_bytes());
        // total_sectors_16 stays 0 -> the parser uses total_sectors_32
        b[0x16..0x18].copy_from_slice(&(FSIZ as u16).to_le_bytes());
        b[0x20..0x24].copy_from_slice(&PART_SECTORS.to_le_bytes());
        b[510] = 0x55;
        b[511] = 0xAA;
    }

    // ── FAT: clusters 2, 3, 4 are one-cluster chains ──
    for copy in 0..NUM_FATS {
        let fat = part + ((FAT1 + copy * FSIZ) as usize) * SECTOR;
        img[fat..fat + 2].copy_from_slice(&0xFFF8u16.to_le_bytes()); // media
        img[fat + 2..fat + 4].copy_from_slice(&0xFFFFu16.to_le_bytes());
        for cl in 2..5usize {
            let o = fat + cl * 2;
            img[o..o + 2].copy_from_slice(&0xFFFFu16.to_le_bytes());
        }
    }

    // ── root directory ──
    {
        let root = part + ROOT_SEC as usize * SECTOR;
        img[root..root + 32].copy_from_slice(&dirent(b"SUB        ", 0x10, 2, 0));
        img[root + 32..root + 64].copy_from_slice(&dirent(b"HELLO   TXT", 0x20, 3, 5));
    }

    // ── C:\SUB (cluster 2) ──
    {
        let sub = part + (DATA_SEC as usize) * SECTOR; // cluster 2
        img[sub..sub + 32].copy_from_slice(&dirent(b".          ", 0x10, 2, 0));
        img[sub + 32..sub + 64].copy_from_slice(&dirent(b"..         ", 0x10, 0, 0));
        img[sub + 64..sub + 96].copy_from_slice(&dirent(b"INNER   TXT", 0x20, 4, 3));
    }

    // ── file data ──
    {
        let c3 = part + (DATA_SEC as usize + 1) * SECTOR; // cluster 3
        img[c3..c3 + 5].copy_from_slice(b"hello");
        let c4 = part + (DATA_SEC as usize + 2) * SECTOR; // cluster 4
        img[c4..c4 + 3].copy_from_slice(b"in!");
    }

    img
}

/// A whole disk image with **two** FAT16 partitions of the same geometry,
/// `C:` and `D:`, both empty (TASK/FS31): every position a file system can
/// hand out is the same number on both, which is what a test of the keys that
/// name a directory entry needs.
fn build_two_drive_image() -> Vec<u8> {
    let total = (PART_LBA + 2 * PART_SECTORS) as usize;
    let mut img = vec![0u8; total * SECTOR];

    for (i, lba) in [PART_LBA, PART_LBA + PART_SECTORS].into_iter().enumerate() {
        let e = 0x1BE + i * 16;
        img[e + 4] = 0x06; // FAT16 partition type
        img[e + 8..e + 12].copy_from_slice(&lba.to_le_bytes());
        img[e + 12..e + 16].copy_from_slice(&PART_SECTORS.to_le_bytes());

        let part = lba as usize * SECTOR;
        let b = &mut img[part..part + SECTOR];
        b[0x0B..0x0D].copy_from_slice(&(SECTOR as u16).to_le_bytes());
        b[0x0D] = CLSIZ as u8;
        b[0x0E..0x10].copy_from_slice(&(RESERVED as u16).to_le_bytes());
        b[0x10] = NUM_FATS as u8;
        b[0x11..0x13].copy_from_slice(&(ROOT_ENTRIES as u16).to_le_bytes());
        b[0x16..0x18].copy_from_slice(&(FSIZ as u16).to_le_bytes());
        b[0x20..0x24].copy_from_slice(&PART_SECTORS.to_le_bytes());
        b[510] = 0x55;
        b[511] = 0xAA;

        for copy in 0..NUM_FATS {
            let fat = part + ((FAT1 + copy * FSIZ) as usize) * SECTOR;
            img[fat..fat + 2].copy_from_slice(&0xFFF8u16.to_le_bytes()); // media
            img[fat + 2..fat + 4].copy_from_slice(&0xFFFFu16.to_le_bytes());
        }
    }
    img[510] = 0x55;
    img[511] = 0xAA;
    img
}

// ── block device + host ───────────────────────────────────────────────────

pub struct RamDisk {
    img: Mutex<Vec<u8>>,
    pub reads: AtomicU32,
    /// Reads of the partition's boot sector, i.e. drive log-ins.
    pub boot_reads: AtomicU32,
    pub writes: AtomicU32,
    /// Fault injection (HW03): while set, every **write** to this LBA is
    /// refused with the TOS code in [`RamDisk::fail_code`] and the image is
    /// left untouched — the host-side stand-in for the card that answers
    /// `CMD24` with an R1 error bit. `u64::MAX` = no fault armed.
    pub fail_write_lba: AtomicU64,
    /// Same for reads.
    pub fail_read_lba: AtomicU64,
    /// The code the armed fault reports.
    pub fail_code: AtomicI32,
    /// Write attempts the fault injector refused.
    pub refused_writes: AtomicU32,
}

/// "No LBA" for the fault-injection registers.
pub const NO_LBA: u64 = u64::MAX;

impl RamDisk {
    /// Refuse every write to `lba` with `code` until [`RamDisk::disarm`].
    pub fn fail_write_at(&self, lba: u64, code: i32) {
        self.fail_code.store(code, Ordering::Relaxed);
        self.fail_write_lba.store(lba, Ordering::Relaxed);
    }

    /// Refuse every read of `lba` with `code` until [`RamDisk::disarm`].
    pub fn fail_read_at(&self, lba: u64, code: i32) {
        self.fail_code.store(code, Ordering::Relaxed);
        self.fail_read_lba.store(lba, Ordering::Relaxed);
    }

    /// Take the fault injector out again.
    pub fn disarm(&self) {
        self.fail_write_lba.store(NO_LBA, Ordering::Relaxed);
        self.fail_read_lba.store(NO_LBA, Ordering::Relaxed);
    }
}

impl tos_api::bios::block::BlockDev for RamDisk {
    fn block_count(&self) -> u64 {
        (self.img.lock().unwrap().len() / SECTOR) as u64
    }

    fn read_blocks(&self, lba: u64, buf: &mut [u8]) -> Result<(), i32> {
        let n = buf.len() / SECTOR;
        let armed = self.fail_read_lba.load(Ordering::Relaxed);
        if armed != NO_LBA && (lba..lba + n as u64).contains(&armed) {
            return Err(self.fail_code.load(Ordering::Relaxed));
        }
        let img = self.img.lock().unwrap();
        let off = lba as usize * SECTOR;
        if off + n * SECTOR > img.len() {
            return Err(tos_types::error::ESECNF);
        }
        buf.copy_from_slice(&img[off..off + n * SECTOR]);
        self.reads.fetch_add(n as u32, Ordering::Relaxed);
        if (lba as u32) == PART_LBA {
            self.boot_reads.fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    }

    fn write_blocks(&self, lba: u64, buf: &[u8]) -> Result<(), i32> {
        let n = buf.len() / SECTOR;
        let armed = self.fail_write_lba.load(Ordering::Relaxed);
        if armed != NO_LBA && (lba..lba + n as u64).contains(&armed) {
            self.refused_writes.fetch_add(1, Ordering::Relaxed);
            return Err(self.fail_code.load(Ordering::Relaxed));
        }
        let mut img = self.img.lock().unwrap();
        let off = lba as usize * SECTOR;
        if off + n * SECTOR > img.len() {
            return Err(tos_types::error::ESECNF);
        }
        img[off..off + n * SECTOR].copy_from_slice(buf);
        self.writes.fetch_add(n as u32, Ordering::Relaxed);
        Ok(())
    }
}

pub struct DiskHost {
    pub disk: RamDisk,
    /// Console input queue (IKBD key words `scancode << 8 | ascii`), the
    /// stand-in for the keyboard ring (FS11). A blocking read of an empty
    /// queue panics instead of hanging the test.
    pub con_in: Mutex<VecDeque<u16>>,
    /// Every byte written to the console (FS11).
    pub con_out: Mutex<Vec<u8>>,
}

impl DiskHost {
    /// Queue `keys` (ASCII, scancode 0) as console input.
    pub fn type_keys(&self, keys: &[u8]) {
        self.con_in.lock().unwrap_or_else(|e| e.into_inner()).extend(keys.iter().map(|&k| k as u16));
    }

    /// Queue one whole key word.
    pub fn type_word(&self, word: u16) {
        self.con_in.lock().unwrap_or_else(|e| e.into_inner()).push_back(word);
    }

    /// Empty the console input queue and the captured output.
    pub fn reset_console(&self) {
        self.con_in.lock().unwrap_or_else(|e| e.into_inner()).clear();
        self.con_out.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }

    /// Take the captured console output.
    pub fn take_output(&self) -> Vec<u8> {
        core::mem::take(&mut *self.con_out.lock().unwrap_or_else(|e| e.into_inner()))
    }

    /// The key words still queued.
    pub fn pending_keys(&self) -> Vec<u16> {
        self.con_in.lock().unwrap_or_else(|e| e.into_inner()).iter().copied().collect()
    }

    fn pop_blocking(&self) -> u16 {
        let w = self.con_in.lock().unwrap_or_else(|e| e.into_inner()).pop_front();
        w.expect("blocking console read on an empty input queue")
    }
}

// ── raw image access ──────────────────────────────────────────────────────
//
// For tests that have to plant an on-disk fixture the GEMDOS layer cannot
// write itself (long-name slots) and then assert on the raw directory bytes
// a call left behind. Planting must happen *before* the first GEMDOS call of
// the test binary: the BDOS sector cache has no invalidate, so bytes written
// behind its back are only seen while it still holds nothing.

/// Bytes per sector of the synthesized volume.
pub const SECTOR_BYTES: usize = SECTOR;
/// Number of FAT copies the image carries.
pub const FAT_COPIES: u32 = NUM_FATS;

/// Absolute LBA of the partition-relative sector `rel`.
pub fn part_lba(rel: u32) -> u64 {
    (PART_LBA + rel) as u64
}

/// Absolute LBA of the first root-directory sector.
pub fn root_lba() -> u64 {
    part_lba(ROOT_SEC)
}

/// Absolute LBA of the first sector of data cluster `cl`.
pub fn cluster_lba(cl: u32) -> u64 {
    part_lba(DATA_SEC + (cl - 2) * CLSIZ)
}

/// Absolute LBA of the first sector of FAT copy `copy` (0-based).
pub fn fat_lba(copy: u32) -> u64 {
    part_lba(FAT1 + copy * FSIZ)
}

impl DiskHost {
    /// Read one raw sector straight from the image, bypassing the BDOS cache.
    pub fn read_sector(&self, lba: u64) -> [u8; SECTOR] {
        use tos_api::bios::block::BlockDev;
        let mut buf = [0u8; SECTOR];
        self.disk.read_blocks(lba, &mut buf).unwrap_or_else(|e| panic!("read of LBA {lba}: {e}"));
        buf
    }

    /// Write one raw sector straight into the image, bypassing the BDOS cache.
    pub fn write_sector(&self, lba: u64, data: &[u8; SECTOR]) {
        use tos_api::bios::block::BlockDev;
        self.disk.write_blocks(lba, data).unwrap_or_else(|e| panic!("write of LBA {lba}: {e}"));
    }
}

impl Host for DiskHost {
    fn console_write_byte(&self, byte: u8) {
        self.con_out.lock().unwrap_or_else(|e| e.into_inner()).push(byte);
    }
    fn console_try_read(&self) -> Option<u8> {
        self.con_in.lock().unwrap_or_else(|e| e.into_inner()).pop_front().map(|w| w as u8)
    }
    fn console_try_read_key(&self) -> Option<u16> {
        self.con_in.lock().unwrap_or_else(|e| e.into_inner()).pop_front()
    }
    fn console_peek_key(&self) -> Option<u16> {
        self.con_in.lock().unwrap_or_else(|e| e.into_inner()).front().copied()
    }
    fn console_has_input(&self) -> bool {
        !self.con_in.lock().unwrap_or_else(|e| e.into_inner()).is_empty()
    }
    fn console_read_byte_blocking(&self) -> u8 {
        // Skips scancode-only keys like the kernel host (`console::rx_pop`).
        loop {
            let w = self.pop_blocking();
            if w & 0xFF != 0 {
                return w as u8;
            }
        }
    }
    fn console_read_key_blocking(&self) -> u16 {
        self.pop_blocking()
    }
    fn console_read_key_restartable(&self) -> Option<u16> {
        Some(self.pop_blocking())
    }
    fn block(&self) -> Option<&dyn tos_api::bios::block::BlockDev> {
        Some(&self.disk)
    }
}

static HOST: OnceLock<&'static DiskHost> = OnceLock::new();
/// The BDOS file system keeps its state in `UnsafeCell` statics with no
/// locking (it runs on the single-CPU, IRQ-off trap path), so the tests in
/// this binary must not overlap.
static FS_LOCK: Mutex<()> = Mutex::new(());

pub fn fs<'a>() -> (MutexGuard<'a, ()>, &'static DiskHost) {
    fs_on(build_image, 1)
}

/// [`fs`] on the image of [`build_two_drive_image`], `C:` and `D:` (TASK/FS31).
/// A test binary uses either this or [`fs`].
pub fn fs_two_drives<'a>() -> (MutexGuard<'a, ()>, &'static DiskHost) {
    fs_on(build_two_drive_image, 2)
}

/// Absolute LBA of partition-relative sector `rel` of partition `index`
/// (0 = `C:`, 1 = `D:`) of the image of [`fs_two_drives`].
pub fn two_drive_lba(index: u32, rel: u32) -> u64 {
    (PART_LBA + index * PART_SECTORS + rel) as u64
}

/// Absolute LBA of the first root-directory sector of partition `index` of the
/// image of [`fs_two_drives`].
pub fn two_drive_root_lba(index: u32) -> u64 {
    two_drive_lba(index, ROOT_SEC)
}

/// Absolute LBA of data cluster `cl` of partition `index` of the image of
/// [`fs_two_drives`].
pub fn two_drive_cluster_lba(index: u32, cl: u32) -> u64 {
    two_drive_lba(index, DATA_SEC + (cl - 2) * CLSIZ)
}

/// Clusters of one volume of the image: what `Dfree` reports as total.
pub const VOLUME_CLUSTERS: u32 = NUMCL;

impl DiskHost {
    /// Put the two-drive image back to empty, behind the BDOS' back. The caller
    /// drops the BDOS' state of both drives first.
    pub fn reset_two_drive_image(&self) {
        *self.disk.img.lock().unwrap() = build_two_drive_image();
    }
}

fn fs_on<'a>(build: fn() -> Vec<u8>, drives: usize) -> (MutexGuard<'a, ()>, &'static DiskHost) {
    let guard = FS_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let host = *HOST.get_or_init(|| {
        let host: &'static DiskHost = Box::leak(Box::new(DiskHost {
            disk: RamDisk {
                img: Mutex::new(build()),
                reads: AtomicU32::new(0),
                boot_reads: AtomicU32::new(0),
                writes: AtomicU32::new(0),
                fail_write_lba: AtomicU64::new(NO_LBA),
                fail_read_lba: AtomicU64::new(NO_LBA),
                fail_code: AtomicI32::new(0),
                refused_writes: AtomicU32::new(0),
            },
            con_in: Mutex::new(VecDeque::new()),
            con_out: Mutex::new(Vec::new()),
        }));
        let slot: &'static &'static dyn Host = Box::leak(Box::new(host as &'static dyn Host));
        tos_api::host::install_host(slot);
        // Scan the MBR so `C:` exists. This also logs every drive out again
        // (`tos_api::bios::mount`), which is what makes the read counters
        // below start from a known state.
        assert_eq!(tos_api::bios::mount(), drives, "FAT16 partitions of the image");
        host
    });
    (guard, host)
}

// ── GEMDOS call helpers ───────────────────────────────────────────────────

pub fn call(func: u16, regs: [u64; 8]) -> i32 {
    let args = TrapArgs {
        regs,
        func: func as u64,
        svc_imm: TrapClass::Gemdos as u16,
        space: tos_api::uaccess::Space::Kernel,
    };
    tos_api::gemdos::dispatch(&args)
}

/// [`call`] with the caller's address space given explicitly (ARCH03 3a):
/// [`tos_api::uaccess::Space::User`] models a trap from an EL0 program. On
/// the host both spaces are plain host-memory accesses, so the result must be
/// the same either way.
pub fn call_in(space: tos_api::uaccess::Space, func: u16, regs: [u64; 8]) -> i32 {
    let args = TrapArgs { regs, func: func as u64, svc_imm: TrapClass::Gemdos as u16, space };
    tos_api::gemdos::dispatch(&args)
}

pub fn gemdos1(func: u16, a0: u64) -> i32 {
    call(func, [a0, 0, 0, 0, 0, 0, 0, 0])
}

pub fn gemdos2(func: u16, a0: u64, a1: u64) -> i32 {
    call(func, [a0, a1, 0, 0, 0, 0, 0, 0])
}

pub const FSETDTA: u16 = 0x1A;
pub const FSFIRST: u16 = 0x4E;
pub const FSNEXT: u16 = 0x4F;
pub const FOPEN: u16 = 0x3D;
pub const FREAD: u16 = 0x3F;
pub const FWRITE: u16 = 0x40;
pub const FCLOSE: u16 = 0x3E;
pub const FCREATE: u16 = 0x3C;
pub const FDELETE: u16 = 0x41;
pub const DCREATE: u16 = 0x39;
pub const DDELETE: u16 = 0x3A;
pub const DGETPATH: u16 = 0x47;
pub const DSETDRV: u16 = 0x0E;

pub const E_OK: i32 = 0;
pub const EWRITF: i32 = -10;
pub const EREADF: i32 = -11;
pub const EFILNF: i32 = -33;
pub const EPTHNF: i32 = -34;
pub const ENMFIL: i32 = -49;

/// TeraDesk's `FA_ANY` — every attribute but the volume label
/// (`sdk/apps/teradesk/xfilesys.c:1173`, seen as `attr=0x37` in the trace).
pub const FA_ANY: u64 = 0x37;

/// A 44-byte DTA plus a NUL-terminated path, so a call looks exactly like the
/// application's.
pub struct Dta([u8; 44]);

impl Dta {
    pub fn new() -> Dta {
        Dta([0u8; 44])
    }
    pub fn ptr(&mut self) -> u64 {
        self.0.as_mut_ptr() as u64
    }
    /// `d_fname` — the found entry's name (offset 30, NUL-terminated).
    pub fn name(&self) -> String {
        let n = &self.0[30..44];
        let end = n.iter().position(|&c| c == 0).unwrap_or(n.len());
        String::from_utf8_lossy(&n[..end]).into_owned()
    }
}

pub fn cpath(p: &str) -> Vec<u8> {
    let mut v = p.as_bytes().to_vec();
    v.push(0);
    v
}

