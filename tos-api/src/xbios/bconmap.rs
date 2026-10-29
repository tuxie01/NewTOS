//! XBIOS `Bconmap` (0x2C) — the serial-port mapping table, with the one
//! entry EmuTOS builds on a machine that is neither Falcon nor TT (XBIOS05,
//! user decision 2026-10-28, option 1).
//!
//! Spec: `docs/api/xbios/2c-bconmap.md`, layouts
//! `docs/api/structures/bconmap.md`. EmuTOS: `bios/serport.c:1082-1126`
//! (`bconmap`), `:956-1004` (`init_bconmap`: `maptabsize = 1`,
//! `mapped_device = 6` unless Falcon/ARAnyM, `maptable[0]` = the MFP port),
//! layouts `bios/serport.h:102-121`.
//!
//! # The table (spec "NewTOS implementation")
//!
//! `BCONMAP` and its one `MAPTAB` entry live in the IOREC page
//! ([`super::iorec`], EL0 read-write in every process, never executable), at
//! [`super::iorec::BCONMAP_AREA`], laid out as LP64 C lays out the SDK's
//! `_BCONMAP`/`_MAPTAB` (`sdk/libc/overlay/include/mint/ostruct.h`): 8-byte
//! pointers, `BCONMAP` 16 bytes (`mapped_device` at 10, in the padding of the
//! SDK's two-field `_BCONMAP`), `MAPTAB` 48. Entry 0 is the AUX port: its
//! `iorec` is `Iorec(0)`'s record, as EmuTOS' `maptable_mfp` holds `&iorec1`
//! (`bios/serport.c:98`), which `Iorec(0)` returns (`bios/xbios.c:339-340`).
//!
//! # The five function pointers are not callable (spec, COMPATIBILITY)
//!
//! EmuTOS' `Bconstat`/`Bconin`/`Bcostat`/`Bconout`/`Rsconf` slots hold ROM
//! routines; NewTOS' drivers are kernel code an EL0 program cannot call. Each
//! slot therefore holds its own address in a reserved, never-executable
//! stretch of the IOREC page ([`vector_va`]): a call through it is an
//! instruction abort, the crash report names the region
//! ([`is_vector`], `crate::bios::crash::RegionKind::BconmapVector`) and the
//! program ends with `Pterm(-1)` — loud, never a silent wrong call. The
//! kernel itself never calls through the table either: BIOS handles `>= 6`
//! reach the entry's port in the BIOS device table directly
//! ([`entry_device`], `crate::bios::chardev::device`; BIOS08).
//!
//! # The kernel's state is its own
//!
//! The page is program data. The kernel answers from its own state — the
//! mapped device ([`MAPPED`]), the table size ([`MAPTABSIZE`]) and the entry
//! images — and never reads the page back: a program that writes
//! `maptab`, `maptabsize` or `mapped_device` there changes what it reads, not
//! what `Bconmap` does (EmuTOS reads `bconmap_root` live,
//! `bios/serport.c:1086-1096`). The writes EmuTOS makes, NewTOS makes too:
//! a valid switch stores the old entry back and sets `mapped_device`
//! (`bios/serport.c:1104-1110,1121`).

use core::sync::atomic::{AtomicI16, Ordering};

use super::iorec;
use crate::bdos::process::mmu::IOREC_PAGE_VA;
use crate::trap::TrapArgs;

// ---------------------------------------------------------------------------
// LP64 layout (`docs/api/structures/bconmap.md`; checked against the SDK
// header by `sdk/libc/tests/bconmap-layout.c`).
// ---------------------------------------------------------------------------

/// `sizeof(BCONMAP)` on LP64: pointer, two `WORD`s, rounded to 8.
pub const BCONMAP_SIZE: usize = 16;
/// `BCONMAP.maptab` (`MAPTAB *`).
pub const OFF_MAPTAB: usize = 0;
/// `BCONMAP.maptabsize` (`WORD`).
pub const OFF_MAPTABSIZE: usize = 8;
/// `BCONMAP.mapped_device` (`WORD`).
pub const OFF_MAPPED_DEVICE: usize = 10;

/// `sizeof(MAPTAB)` on LP64: six pointers.
pub const MAPTAB_SIZE: usize = 48;
/// `MAPTAB.Bconstat`.
pub const OFF_BCONSTAT: usize = 0;
/// `MAPTAB.Bconin`.
pub const OFF_BCONIN: usize = 8;
/// `MAPTAB.Bcostat`.
pub const OFF_BCOSTAT: usize = 16;
/// `MAPTAB.Bconout`.
pub const OFF_BCONOUT: usize = 24;
/// `MAPTAB.Rsconf`.
pub const OFF_RSCONF: usize = 32;
/// `MAPTAB.Iorec` (`EXT_IOREC *`).
pub const OFF_IOREC: usize = 40;

const _: () = assert!(OFF_MAPPED_DEVICE + 2 <= BCONMAP_SIZE && BCONMAP_SIZE.is_multiple_of(8));
const _: () = assert!(OFF_IOREC + 8 == MAPTAB_SIZE);

// ---------------------------------------------------------------------------
// The table (`init_bconmap`, `bios/serport.c:956-1004`).
// ---------------------------------------------------------------------------

/// `BCONMAP_START_HANDLE` (`bios/serport.h:121`): the first mappable device.
pub const BCONMAP_START_HANDLE: i16 = 6;
/// Entries in the table: one, the AUX port — EmuTOS' value for a machine
/// that is neither Falcon/MSTe/ARAnyM nor TT (`bios/serport.c:965`).
pub const MAPTABSIZE: i16 = 1;
/// The five function slots of an entry, in `MAPTAB` order.
pub const VECTOR_COUNT: usize = 5;

/// The port behind each entry, as the BIOS device-table slot whose driver
/// serves it: entry 0 is the AUX port, device 1 (EmuTOS' `maptable_mfp`,
/// `{bconstat1, bconin1, bcostat1, bconout1, ...}`, `bios/serport.c:98,972`
/// — the routines device 1's vectors are taken from, `:996-1003`).
const ENTRY_DEVICES: [u16; MAPTABSIZE as usize] = [crate::bios::chardev::DEV_AUX];

/// The BIOS device-table slot that serves entry `index` (`0 ..
/// MAPTABSIZE`): what a BIOS character call on handle
/// `BCONMAP_START_HANDLE + index` reaches — EmuTOS calls
/// `bconmap_root.maptab[index]`'s routines there (`bios/bios.c:927-928`,
/// `:971-972`, `:1004-1005`, `:1156-1157`). Fixed per entry, independent of
/// the mapped device, as EmuTOS' entries are. Panics on an index outside
/// the table (the caller checks the range first, as EmuTOS does).
pub fn entry_device(index: i16) -> u16 {
    ENTRY_DEVICES[index as usize]
}

/// The device mapped to BIOS device 1 (`bconmap_root.mapped_device`): 6 at
/// start (`bios/serport.c:966`, not Falcon/ARAnyM), changed only by a valid
/// `Bconmap(dev)`.
static MAPPED: AtomicI16 = AtomicI16::new(BCONMAP_START_HANDLE);

// Offsets inside the area (the area's own address: `iorec::BCONMAP_AREA`).
/// The `BCONMAP`.
const AREA_BCONMAP: usize = 0x00;
/// `MAPTAB[0]`, the only entry.
const AREA_MAPTAB: usize = 0x10;
/// The never-executable call targets of the five function slots, 4 bytes
/// each (a slot's value, [`vector_va`]).
const AREA_VECTORS: usize = 0x40;
/// Bytes per call target.
const VECTOR_STRIDE: usize = 4;
/// Bytes the area takes in the IOREC page.
pub const AREA_SIZE: usize = 0x60;

const _: () = assert!(AREA_BCONMAP + BCONMAP_SIZE <= AREA_MAPTAB);
const _: () = assert!(AREA_MAPTAB + MAPTAB_SIZE * MAPTABSIZE as usize <= AREA_VECTORS);
const _: () = assert!(AREA_VECTORS + VECTOR_COUNT * VECTOR_STRIDE <= AREA_SIZE);
const _: () = assert!(AREA_MAPTAB.is_multiple_of(8) && iorec::BCONMAP_AREA.is_multiple_of(16));

/// EL0 address of the `BCONMAP` — what `Bconmap(-2)` returns.
pub const BCONMAP_VA: u64 = IOREC_PAGE_VA + (iorec::BCONMAP_AREA + AREA_BCONMAP) as u64;

/// EL0 address of the call target of function slot `i` (0 `Bconstat` … 4
/// `Rsconf`) in an area mapped at `area_va`.
const fn vector_at(area_va: u64, i: usize) -> u64 {
    area_va + (AREA_VECTORS + i * VECTOR_STRIDE) as u64
}

/// EL0 address of the call target of function slot `i` (0 `Bconstat`, 1
/// `Bconin`, 2 `Bcostat`, 3 `Bconout`, 4 `Rsconf`).
pub const fn vector_va(i: usize) -> u64 {
    vector_at(IOREC_PAGE_VA + iorec::BCONMAP_AREA as u64, i)
}

/// Whether `va` is one of the call targets — the crash report's test for "a
/// program called through a `MAPTAB` function pointer".
pub const fn is_vector(va: u64) -> bool {
    va >= vector_va(0) && va < vector_va(VECTOR_COUNT)
}

/// Entry `index`'s six slots for an area mapped at `area_va`: the five call
/// targets, then the device's `EXT_IOREC`. Only entry 0 (AUX, `Iorec(0)`'s
/// record) exists.
fn entry_image(area_va: u64, index: usize) -> [u64; 6] {
    debug_assert!(index < MAPTABSIZE as usize);
    let page_va = area_va - iorec::BCONMAP_AREA as u64;
    [
        vector_at(area_va, 0),
        vector_at(area_va, 1),
        vector_at(area_va, 2),
        vector_at(area_va, 3),
        vector_at(area_va, 4),
        page_va + iorec::AUX_RECORD as u64,
    ]
}

/// The area as the kernel reaches it: its kernel address and the EL0
/// address it is mapped at.
#[derive(Clone, Copy)]
pub(super) struct Area {
    pub(super) base: *mut u8,
    pub(super) va: u64,
}

impl Area {
    fn write_u64(&self, off: usize, v: u64) {
        debug_assert!(off.is_multiple_of(8) && off + 8 <= AREA_SIZE);
        // SAFETY: `base` is the area inside a live page (16-aligned,
        // `AREA_SIZE` bytes); `off` is one of the 8-aligned offsets above.
        unsafe { (self.base.add(off) as *mut u64).write_volatile(v) }
    }

    fn write_u16(&self, off: usize, v: u16) {
        debug_assert!(off.is_multiple_of(2) && off + 2 <= AREA_SIZE);
        // SAFETY: as in `write_u64`, 2-aligned.
        unsafe { (self.base.add(off) as *mut u16).write_volatile(v) }
    }

    fn write_u8(&self, off: usize, v: u8) {
        debug_assert!(off < AREA_SIZE);
        // SAFETY: inside the area.
        unsafe { self.base.add(off).write_volatile(v) }
    }

    /// Store entry `index`'s slots (`bios/serport.c:1104-1110`, the save
    /// step; `init_bconmap`'s `memcpy`, `:972`).
    fn store_entry(&self, index: usize) {
        let at = AREA_MAPTAB + index * MAPTAB_SIZE;
        let image = entry_image(self.va, index);
        for (k, off) in [OFF_BCONSTAT, OFF_BCONIN, OFF_BCOSTAT, OFF_BCONOUT, OFF_RSCONF, OFF_IOREC]
            .into_iter()
            .enumerate()
        {
            self.write_u64(at + off, image[k]);
        }
    }

    /// `init_bconmap` (`bios/serport.c:962-972`): `maptab`, `maptabsize`,
    /// `mapped_device`, the entry; the call targets zero.
    pub(super) fn init(&self) {
        for off in 0..AREA_SIZE {
            self.write_u8(off, 0);
        }
        self.write_u64(AREA_BCONMAP + OFF_MAPTAB, self.va + AREA_MAPTAB as u64);
        self.write_u16(AREA_BCONMAP + OFF_MAPTABSIZE, MAPTABSIZE as u16);
        self.write_u16(AREA_BCONMAP + OFF_MAPPED_DEVICE, MAPPED.load(Ordering::Relaxed) as u16);
        for i in 0..MAPTABSIZE as usize {
            self.store_entry(i);
        }
    }
}

/// `Bconmap(dev)` (XBIOS 0x2C), `bios/serport.c:1082-1122`:
///
/// - `-1`: the mapped device;
/// - `-2`: the address of the `BCONMAP` (the table written before it is
///   handed out);
/// - a device `6 .. 6 + maptabsize - 1`: the old device's entry is stored
///   back, the new one becomes the mapped device, the old one is returned;
/// - anything else: `0`.
///
/// With one entry the only valid device is 6, the one already mapped, so a
/// switch changes no driver: device 1 stays the AUX port.
pub fn bconmap(args: &TrapArgs) -> i64 {
    let dev = args.arg_i16(0);
    let old = MAPPED.load(Ordering::Relaxed);
    match dev {
        -1 => return old as i64,
        -2 => return iorec::with_bconmap_area(|_| BCONMAP_VA) as i64,
        _ => {}
    }
    // WORD arithmetic as in EmuTOS (`map_index = dev - BCONMAP_START_HANDLE`
    // in a WORD): for dev < -32762 it wraps to a large positive index, which
    // is past the table like every other out-of-range value.
    let map_index = dev.wrapping_sub(BCONMAP_START_HANDLE);
    if !(0..MAPTABSIZE).contains(&map_index) {
        return 0;
    }
    iorec::with_bconmap_area(|a| {
        a.store_entry((old - BCONMAP_START_HANDLE) as usize);
        MAPPED.store(dev, Ordering::Relaxed);
        a.write_u16(AREA_BCONMAP + OFF_MAPPED_DEVICE, dev as u16);
    });
    old as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[repr(C, align(16))]
    struct TestArea([u8; AREA_SIZE]);

    const PAGE: u64 = 0x7D000;

    fn fresh() -> (Box<TestArea>, Area) {
        let mut b = Box::new(TestArea([0xA5; AREA_SIZE]));
        let a = Area { base: b.0.as_mut_ptr(), va: PAGE + iorec::BCONMAP_AREA as u64 };
        a.init();
        (b, a)
    }

    fn rd16(b: &TestArea, off: usize) -> u16 {
        u16::from_ne_bytes([b.0[off], b.0[off + 1]])
    }
    fn rd64(b: &TestArea, off: usize) -> u64 {
        u64::from_ne_bytes(b.0[off..off + 8].try_into().unwrap())
    }

    #[test]
    fn the_table_is_emutos_one_entry_table() {
        let (b, a) = fresh();
        // BCONMAP {maptab, 1, 6} (bios/serport.c:964-966).
        assert_eq!(rd64(&b, AREA_BCONMAP + OFF_MAPTAB), a.va + AREA_MAPTAB as u64);
        assert_eq!(rd16(&b, AREA_BCONMAP + OFF_MAPTABSIZE), 1);
        assert_eq!(rd16(&b, AREA_BCONMAP + OFF_MAPPED_DEVICE), 6);
        // MAPTAB[0]: five distinct call targets inside the area, then
        // Iorec(0)'s record.
        let e = AREA_MAPTAB;
        let slots: [u64; 5] = core::array::from_fn(|i| rd64(&b, e + 8 * i));
        for (i, s) in slots.iter().enumerate() {
            assert_eq!(*s, a.va + (AREA_VECTORS + 4 * i) as u64);
            assert_eq!(b.0[AREA_VECTORS + 4 * i], 0);
        }
        assert_eq!(rd64(&b, e + OFF_IOREC), PAGE + iorec::AUX_RECORD as u64);
    }

    #[test]
    fn the_real_call_targets_are_recognised() {
        assert!(is_vector(vector_va(0)));
        assert!(is_vector(vector_va(4)));
        assert!(is_vector(vector_va(4) + 3));
        assert!(!is_vector(vector_va(0) - 1));
        assert!(!is_vector(vector_va(VECTOR_COUNT)));
        assert!(!is_vector(BCONMAP_VA));
    }

    #[test]
    fn a_switch_stores_the_old_entry_back() {
        let (mut b, a) = fresh();
        // A program overwrote MAPTAB[0].Bconstat and mapped_device.
        b.0[AREA_MAPTAB..AREA_MAPTAB + 8].copy_from_slice(&0x1234u64.to_ne_bytes());
        b.0[AREA_BCONMAP + OFF_MAPPED_DEVICE..][..2].copy_from_slice(&9u16.to_ne_bytes());
        a.store_entry(0);
        a.write_u16(AREA_BCONMAP + OFF_MAPPED_DEVICE, 6);
        assert_eq!(rd64(&b, AREA_MAPTAB), a.va + AREA_VECTORS as u64);
        assert_eq!(rd16(&b, AREA_BCONMAP + OFF_MAPPED_DEVICE), 6);
    }
}
