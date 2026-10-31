//! MEM11 — the memory layout keeps every user address below 2 GiB
//! (`TASK/MEM11.md`, user decision 2026-10-31).
//!
//! A TOS address is a signed 32-bit `LONG`: a user address at or above
//! `0x8000_0000` sign-extends on the GEMDOS return path and reads as a
//! negative error code (`Malloc`/`Mxalloc`, `Fgetdta`, the MD fields of
//! `Getmpb`, `Physbase`/`Logbase` in 32-bit fields). The user tables could map
//! up to 4 GiB ([`USER_VA_LIMIT`]); the enforcement at the mapping boundary is
//! parked, so today the contract holds by placement only. These tests pin that
//! placement where `tos-api` owns it, so a layout change fails loudly instead
//! of reopening the gap silently:
//!
//! * the limit itself is the positive `LONG` range;
//! * the fixed user pages (page 0, IOREC, stub, private read-write page) and
//!   every fixed pointer the kernel hands out from them lie below it;
//! * BDOS refuses to seed a pool that reaches it (`seed_md`), and accepts the
//!   last one that ends a page below — the kernel's `TOS_MD_MAX_END`.
//!
//! The kernel-owned half (the DRAM cap of `layout::discover`, which bounds the
//! pools and everything `usermem::expose` publishes, and the Pi framebuffer's
//! bus-address mask) is checked at compile time in the kernel on both boards
//! (`kernel/src/bios/machine/layout.rs`, `ttram_cap_end_for`;
//! `kernel/src/drivers/bcm2711/framebuffer.rs`, `BUS_TO_PHYS`): the kernel
//! cannot be linked for the host.

use tos_api::bdos::mem::init::{self, InitError};
use tos_api::bdos::process::loader::{
    KBDVECS_AREA_SIZE, KBDVECS_AREA_VA, KBDVEC_STUB_AREA_SIZE, KBDVEC_STUB_AREA_VA, KEYTBL_AREA_SIZE,
    KEYTBL_AREA_VA, OS_RUN_CELL_VA, PARMBLK_SCRATCH_VA, RESET_STUB_AREA_SIZE, RESET_STUB_AREA_VA,
    TRAMPOLINE_VA, USERDEF_TRAMPOLINE_VA, USER_VA_BASE,
};
use tos_api::bdos::process::mmu::{IOREC_PAGE_VA, PAGE_SIZE, PRIV_PAGE_VA, STUB_PAGE_VA, USER_VA_LIMIT};
use tos_api::bdos::process::usermem::USER_PTR_END;
use tos_api::bios::reset::RESET_ENTRY_VA;
use tos_api::xbios::bconmap::BCONMAP_VA;
use tos_api::xbios::iorec::{record_va, IKBD_IOREC_VA};
use tos_api::xbios::kbdvecs::{KBDVEC_DEFAULT_VA, RET_STUB_VA};
use tos_types::Long;

/// The limit is the first address that no longer fits a positive `LONG`, and
/// the user tables reach at least that far (the 2..4 GiB beyond it is the
/// parked gap, not a user range).
#[test]
fn limit_is_the_positive_long_range() {
    assert_eq!(USER_PTR_END, Long::MAX as u64 + 1);
    const { assert!(USER_VA_LIMIT >= USER_PTR_END) };
}

/// Every fixed user region and every fixed pointer handed out of one ends
/// at or below the limit.
#[test]
fn fixed_user_pages_and_pointers_lie_below_2gib() {
    // (name, start, length) — the whole region must lie below the limit.
    let regions: [(&str, u64, u64); 8] = [
        ("page 0", USER_VA_BASE, PAGE_SIZE),
        ("IOREC page", IOREC_PAGE_VA, PAGE_SIZE),
        ("stub page", STUB_PAGE_VA, PAGE_SIZE),
        ("private page", PRIV_PAGE_VA, PAGE_SIZE),
        ("Keytbl area", KEYTBL_AREA_VA, KEYTBL_AREA_SIZE),
        ("Kbdvbase table", KBDVECS_AREA_VA, KBDVECS_AREA_SIZE),
        ("kbdvec stubs", KBDVEC_STUB_AREA_VA, KBDVEC_STUB_AREA_SIZE),
        ("reset stub", RESET_STUB_AREA_VA, RESET_STUB_AREA_SIZE),
    ];
    for (name, start, len) in regions {
        let end = start.checked_add(len).expect("region end overflows");
        assert!(end <= USER_PTR_END, "{name} [{start:#x}, {end:#x}) reaches 0x8000_0000");
    }

    // Pointers a program receives: Iorec, Bconmap, Kbdvbase's stubs, the
    // OSHEADER reset entry, the upcall/G_USERDEF trampolines, the PARMBLK
    // scratch, the os_run cell.
    let mut pointers = vec![
        ("IKBD Iorec", IKBD_IOREC_VA),
        ("Bconmap", BCONMAP_VA),
        ("ret stub", RET_STUB_VA),
        ("kbdvec default", KBDVEC_DEFAULT_VA),
        ("reset entry", RESET_ENTRY_VA),
        ("upcall trampoline", TRAMPOLINE_VA),
        ("G_USERDEF trampoline", USERDEF_TRAMPOLINE_VA),
        ("PARMBLK scratch", PARMBLK_SCRATCH_VA),
        ("os_run cell", OS_RUN_CELL_VA),
    ];
    for devno in 0..=2 {
        pointers.push(("Iorec record", record_va(devno).expect("devices 0..=2 have a record")));
    }
    for (name, va) in pointers {
        assert!(va < USER_PTR_END, "{name} at {va:#x} reads as a negative LONG");
    }
}

/// BDOS seeds a pool only if `m_start + m_length` stays a positive `LONG`:
/// a pool ending one page below the limit (the kernel's `TOS_MD_MAX_END`) is
/// accepted, one ending at the limit is refused. One `init` call per test
/// binary (it is once-only), so the accepted pool is the ST-RAM one, seeded
/// before the refused TT-RAM one.
#[test]
fn pool_seed_refuses_a_pool_reaching_2gib() {
    let last_ok_end = USER_PTR_END - PAGE_SIZE;
    let st = (last_ok_end - 16 * 1024 * 1024, 16 * 1024 * 1024);
    let tt = (last_ok_end, PAGE_SIZE);
    // SAFETY: the only `init` in this binary; nothing allocates from the pools.
    let r = unsafe { init::init(st.0, st.1, tt.0, tt.1) };
    assert_eq!(r, Err(InitError::AddressOutOfRange));
    assert_eq!(init::stram_range(), (st.0 as Long, last_ok_end as Long));
}
