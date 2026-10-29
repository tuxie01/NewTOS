//! XBIOS05 — `Bconmap` (XBIOS 0x2C): EmuTOS' one-entry table.
//!
//! Spec: `docs/api/xbios/2c-bconmap.md` ("NewTOS implementation"), layouts
//! `docs/api/structures/bconmap.md`; EmuTOS `bios/serport.c:956-1004`
//! (`init_bconmap`), `:1082-1126` (`bconmap`). The table lies in the IOREC
//! page; a program's reads and writes through `Bconmap(-2)`'s pointer land on
//! the page's kernel alias here, as they do through the EL0 mapping in the
//! kernel. The calls in QEMU and against the oracle:
//! `measurements/measurement-2026-10-29-xbios05/`.

use std::sync::{Mutex, MutexGuard};

use tos_api::bdos::process::memprot;
use tos_api::bdos::process::mmu::{self, IOREC_PAGE_VA};
use tos_api::bios::crash::{self, Access, CrashKind, CrashReport, FaultType, RegionKind};
use tos_api::trap::{TrapArgs, TrapClass};
use tos_api::xbios::bconmap;

static LOCK: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn xbios(func: u16, a0: u64) -> i64 {
    let args = TrapArgs {
        regs: [a0, 0, 0, 0, 0, 0, 0, 0],
        func: func as u64,
        svc_imm: TrapClass::Xbios as u16,
        space: tos_api::uaccess::Space::Kernel,
    };
    tos_api::xbios::dispatch(&args)
}

const BCONMAP: u16 = 0x2C;
const IOREC: u16 = 0x0E;

/// `Bconmap(dev)` with `dev` a WORD, as a program passes it.
fn bconmap_call(dev: i16) -> i64 {
    xbios(BCONMAP, dev as u16 as u64)
}

/// The kernel alias of a user address in the IOREC page.
fn user_cell(va: u64) -> *mut u8 {
    assert!((IOREC_PAGE_VA..IOREC_PAGE_VA + 0x1000).contains(&va));
    // SAFETY: inside the static IOREC page.
    unsafe { mmu::iorec_page_kva().add((va - IOREC_PAGE_VA) as usize) }
}
fn rd16(va: u64) -> i16 {
    // SAFETY: an even offset inside the page.
    unsafe { (user_cell(va) as *const i16).read_volatile() }
}
fn wr16(va: u64, v: i16) {
    // SAFETY: as above.
    unsafe { (user_cell(va) as *mut i16).write_volatile(v) }
}
fn rd64(va: u64) -> u64 {
    // SAFETY: an 8-aligned offset inside the page.
    unsafe { (user_cell(va) as *const u64).read_volatile() }
}
fn wr64(va: u64, v: u64) {
    // SAFETY: as above.
    unsafe { (user_cell(va) as *mut u64).write_volatile(v) }
}

/// The six slots of `MAPTAB[0]`.
fn entry0(maptab: u64) -> [u64; 6] {
    core::array::from_fn(|i| rd64(maptab + 8 * i as u64))
}

#[test]
fn query_returns_device_6() {
    let _g = lock();
    // bios/serport.c:966 (not Falcon/ARAnyM), :1089-1090.
    assert_eq!(bconmap_call(-1), 6);
}

#[test]
fn table_pointer_is_a_one_entry_bconmap_in_the_caller_space() {
    let _g = lock();
    let p = bconmap_call(-2) as u64;
    assert_eq!(p, bconmap::BCONMAP_VA);
    assert!((0x400..0x8000_0000).contains(&p) && p.is_multiple_of(2), "PTR_RAM: {p:#x}");
    assert!((IOREC_PAGE_VA..IOREC_PAGE_VA + 0x1000).contains(&p), "in the EL0 IOREC page");
    // {maptab, 1, 6} at the LP64 offsets 0/8/10 (bios/serport.c:964-966).
    let maptab = rd64(p + bconmap::OFF_MAPTAB as u64);
    assert_eq!(rd16(p + bconmap::OFF_MAPTABSIZE as u64), 1);
    assert_eq!(rd16(p + bconmap::OFF_MAPPED_DEVICE as u64), 6);
    assert!((IOREC_PAGE_VA..IOREC_PAGE_VA + 0x1000).contains(&maptab));
    assert!(maptab.is_multiple_of(8));
    // MAPTAB[0]: five distinct non-callable targets, then Iorec(0)'s record
    // (maptable_mfp holds &iorec1, bios/serport.c:98; Iorec(0) returns it).
    let e = entry0(maptab);
    for (i, v) in e[..5].iter().enumerate() {
        assert_eq!(*v, bconmap::vector_va(i));
        assert!(bconmap::is_vector(*v));
        assert_eq!(memprot::region_kind(*v), Some(RegionKind::BconmapVector));
    }
    assert_eq!(e[5] as i64, xbios(IOREC, 0));
    // The same pointer on every call.
    assert_eq!(bconmap_call(-2) as u64, p);
}

#[test]
fn every_other_device_is_zero() {
    let _g = lock();
    // bios/serport.c:1095-1097; oracle bios-r1 185/186, xbios-chardev-r1 45-53.
    for dev in [0i16, 1, 2, 3, 4, 5, 7, 8, 9, 10, 11, 0x7FFF, -3, -32768, -32763, -32762] {
        assert_eq!(bconmap_call(dev), 0, "Bconmap({dev})");
    }
    assert_eq!(bconmap_call(-1), 6, "an invalid device maps nothing");
}

#[test]
fn device_6_returns_6_and_stores_the_entry_back() {
    let _g = lock();
    let p = bconmap_call(-2) as u64;
    let maptab = rd64(p);
    let before = entry0(maptab);
    // A program overwrote MAPTAB[0].Bconstat, .Iorec and mapped_device.
    wr64(maptab, 0x1234);
    wr64(maptab + 40, 0x5678);
    wr16(p + bconmap::OFF_MAPPED_DEVICE as u64, 9);
    // The save step (bios/serport.c:1104-1110) writes the old slot back,
    // :1121 sets mapped_device, :1122 returns the old device.
    assert_eq!(bconmap_call(6), 6);
    assert_eq!(entry0(maptab), before);
    assert_eq!(rd16(p + bconmap::OFF_MAPPED_DEVICE as u64), 6);
    assert_eq!(bconmap_call(-1), 6);
}

#[test]
fn writes_into_the_published_table_are_not_adopted() {
    let _g = lock();
    let p = bconmap_call(-2) as u64;
    let maptab = rd64(p);
    // A program claims four entries and points maptab elsewhere (an unused
    // stretch of the page, marked).
    let elsewhere = IOREC_PAGE_VA + 0x800;
    wr64(elsewhere, 0xA5A5_A5A5_A5A5_A5A5);
    wr16(p + bconmap::OFF_MAPTABSIZE as u64, 4);
    wr64(p, elsewhere);
    assert_eq!(bconmap_call(7), 0, "the kernel's table has one entry");
    assert_eq!(bconmap_call(6), 6);
    // The save step wrote the kernel's own entry, not the program's maptab.
    assert_eq!(rd64(elsewhere), 0xA5A5_A5A5_A5A5_A5A5);
    assert_eq!(rd64(maptab), bconmap::vector_va(0));
    wr64(elsewhere, 0);
    // Restore what the program changed (the page is shared by the tests).
    wr16(p + bconmap::OFF_MAPTABSIZE as u64, 1);
    wr64(p, maptab);
}

/// A call through a `MAPTAB` function pointer from EL0: an instruction abort
/// (lower EL, permission fault level 3 — the IOREC page is never executable)
/// at the slot's value. The crash report names it.
#[test]
fn a_call_through_a_function_slot_is_reported_by_name() {
    let _g = lock();
    let target = bconmap::vector_va(0);
    // ESR: EC 0x20 (IABT from a lower EL), IL, IFSC 0x0F (permission L3).
    let esr = (0x20u64 << 26) | (1 << 25) | 0x0F;
    assert_eq!(crash::classify(esr), CrashKind::BusError);
    let v = crash::mem_violation(esr, memprot::region_kind(target)).expect("a named violation");
    assert_eq!(v.region, RegionKind::BconmapVector);
    assert_eq!(v.fault, FaultType::Permission);
    assert_eq!(v.access, Access::Execute);
    let regs = [0u64; 31];
    let report = CrashReport {
        kind: crash::classify(esr),
        esr,
        far: Some(target),
        pc: target,
        pstate: 0,
        regs: &regs,
        usp: 0,
        program: None,
        violation: Some(v),
    };
    let mut s = String::new();
    crash::write_report(&mut s, &report).unwrap();
    assert!(
        s.contains("Memory violation: Bconmap MAPTAB function (not callable) memory, execute (permission fault, level 3)"),
        "{s}"
    );
    // Neighbours are not the slots.
    assert_ne!(memprot::region_kind(bconmap::BCONMAP_VA), Some(RegionKind::BconmapVector));
    assert_ne!(memprot::region_kind(bconmap::vector_va(5)), Some(RegionKind::BconmapVector));
}
