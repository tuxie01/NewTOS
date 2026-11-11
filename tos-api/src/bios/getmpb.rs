//! `Getmpb` (BIOS 0x00) — report the free ST-RAM region as a Memory
//! Partition Block.
//!
//! Spec: `docs/api/bios/00-getmpb.md`. EmuTOS fills a single static
//! memory descriptor `themd` describing `[membot, memtop)` and points the
//! caller's MPB free list at it (`bios/biosmem.c:134`): `mp_mfl = &themd`,
//! `mp_mal = NULL`, `mp_rover` left untouched; `themd` gets `m_link = NULL`,
//! `m_start = membot`, `m_length = memtop - membot`, `m_own = NULL`.
//!
//! NewTOS seeds its allocator directly from the AArch64 memory map, so
//! `Getmpb` is a **read-only** report of the ST-RAM pool extent (from
//! [`crate::bdos::mem::init::stram_range`]). Unlike EmuTOS it does **not**
//! rewrite the live BDOS free list — the "second call destroys your
//! allocator state" quirk is explicitly rejected (spec "NewTOS
//! implementation notes"), so a second call just re-reports the same
//! extent and is harmless.
//!
//! The descriptor and the caller's block have the **program-visible** layout
//! of the spec's contract (`docs/api/bios/00-getmpb.md` "NewTOS-Vertrag
//! [BIOS06]" V1, user decision O1 = A): the SDK header's LP64 `_MD`
//! (32 bytes) and `_MPB` (24 bytes) — [`PublicMd`] / [`PublicMpb`] — not the
//! BDOS-internal 24-byte [`tos_types::MD`], which is never published.

use core::cell::UnsafeCell;

use core::mem::{offset_of, size_of};

use tos_types::{Long, ProtMode};

use crate::bdos::mem::init::stram_range;
use crate::bdos::process::{self, usermem};
use crate::trap::TrapArgs;
use crate::uaccess::UserFault;

/// The memory descriptor as a program sees it through `Getmpb`: the SDK
/// header's `_MD` under LP64 (`sdk/libc/overlay/include/mint/ostruct.h:293@_md`),
/// EmuTOS' `MD` (`include/bdosdefs.h:108-115`) with every field widened to
/// the AArch64 ABI — spec `docs/api/bios/00-getmpb.md` V1 (O1 = A).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublicMd {
    /// `md_next` (EmuTOS `m_link`): next descriptor, a caller-space
    /// address; always NULL — one descriptor (spec V2).
    pub md_next: u64,
    /// `md_start` (EmuTOS `m_start`): start of the region, a C `long`.
    pub md_start: i64,
    /// `md_length` (EmuTOS `m_length`): region length in bytes, a C `long`.
    pub md_length: i64,
    /// `md_owner` (EmuTOS `m_own`): owning basepage; always NULL (free).
    pub md_owner: u64,
}

/// The memory partition block as a program passes it to `Getmpb`: the SDK
/// header's `_MPB` under LP64 (`sdk/libc/overlay/include/mint/ostruct.h:305@_MPB`),
/// EmuTOS' `MPB` (`include/bdosdefs.h:120-126`) with 8-byte pointers. The
/// kernel only computes the two field offsets it writes from it.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublicMpb {
    /// `mp_free` (EmuTOS `mp_mfl`): free-list head, written.
    pub mp_free: u64,
    /// `mp_used` (EmuTOS `mp_mal`): allocated-list head, written NULL.
    pub mp_used: u64,
    /// `mp_rover` (EmuTOS `mp_rover`): never written.
    pub mp_rover: u64,
}

// The contract's numbers (spec V1 table, V4) — the same ones the SDK header
// asserts with `_Static_assert` (`ostruct.h:312-320`, SDK04).
const _: () = {
    assert!(offset_of!(PublicMd, md_next) == 0);
    assert!(offset_of!(PublicMd, md_start) == 8);
    assert!(offset_of!(PublicMd, md_length) == 16);
    assert!(offset_of!(PublicMd, md_owner) == 24);
    assert!(size_of::<PublicMd>() == 32);
    assert!(offset_of!(PublicMpb, mp_free) == 0);
    assert!(offset_of!(PublicMpb, mp_used) == 8);
    assert!(offset_of!(PublicMpb, mp_rover) == 16);
    assert!(size_of::<PublicMpb>() == 24);
};

/// The single memory descriptor `Getmpb` reports, in a page of its own.
///
/// `Getmpb` publishes it **read-only into the calling process's address
/// space** (ADR-023) so the caller can walk the free list it was handed, and
/// the smallest thing an AArch64 translation table can grant is a whole
/// 4-KiB page. Padding the descriptor to exactly one page — and aligning it
/// to one — guarantees the page an EL0 program can read holds the MD and
/// nothing else of the kernel's.
#[repr(C, align(4096))]
struct ThemdPage {
    /// The descriptor itself, mirroring EmuTOS' `themd`
    /// (`bios/biosmem.c:144`), in the program-visible layout.
    md: PublicMd,
    /// Fill to a whole page so no other kernel datum shares it.
    _pad: [u8; 4096 - size_of::<PublicMd>()],
}

const _: () = assert!(
    size_of::<ThemdPage>() == 4096,
    "the reported MD must own exactly one page — see `getmpb`'s EL0 publish",
);

/// The single static memory descriptor `Getmpb` reports. Re-filled on each
/// call from the boot-time ST-RAM extent; the caller's MPB free list points
/// here.
struct ThemdCell(UnsafeCell<ThemdPage>);

// SAFETY: `THEMD` is only touched by the BIOS dispatcher while filling a
// caller's MPB. BIOS traps run on the boot/trap CPU under the same
// single-owner discipline as the BDOS `MpbCell`; `UnsafeCell` makes the
// auto `Sync` absent, so we assert it explicitly.
unsafe impl Sync for ThemdCell {}

static THEMD: ThemdCell = ThemdCell(UnsafeCell::new(ThemdPage {
    md: PublicMd {
        md_next: 0,
        md_start: 0,
        md_length: 0,
        md_owner: 0,
    },
    _pad: [0; 4096 - size_of::<PublicMd>()],
}));

/// The descriptor's contents for the ST-RAM extent `[start, end)`: one
/// unowned, unlinked region (spec V2.1; EmuTOS `bios/biosmem.c:144-147`).
/// The `Long` bounds are addresses below 2 GiB, widened to the C `long`.
fn describe(start: Long, end: Long) -> PublicMd {
    PublicMd {
        md_next: 0,
        md_start: i64::from(start),
        md_length: i64::from(end) - i64::from(start),
        md_owner: 0,
    }
}

/// `Getmpb` (0x00) — fill the caller-supplied MPB with a one-descriptor
/// free list covering the ST-RAM pool. `getmpb` is `void` in EmuTOS; the
/// result register is undefined, so we return `0` and callers read the
/// populated `*mpb`.
///
/// The free-list head the caller receives has to be a pointer the *caller*
/// can walk (spec `docs/api/bios/00-getmpb.md` §Outputs). For an in-kernel
/// (EL1) caller that is the descriptor's kernel VA; for an **EL0** caller
/// the kernel's high-half pages are unreachable, so the descriptor is
/// published read-only (FreeMiNT `PROT_PR`,
/// `references/freemint/sys/mint/mem.h:133`) into the shared user address
/// space at its own address (VA = PA, ARCH03 stage 1) and `mp_mfl` carries
/// that **user address** (ADR-023). If the publication cannot be made,
/// `mp_mfl` is left NULL — an empty free list, which the contract already
/// defines — rather than a pointer the caller would fault on. A NULL `mpb`
/// is never written (EmuTOS `bios/biosmem.c:149`: bus error, panic; spec V3,
/// O3 decided 2026-11-11): a program's ends it as an unwritable MPB does.
pub fn getmpb(args: &TrapArgs) -> Long {
    let mpb = args.arg_u64(0);
    if mpb == 0 {
        // A program's NULL is refused as a store it cannot make, in super
        // state too (its page 0 would take the store there): the syscall
        // tail ends it with "Bus Error". An in-kernel caller's pointers are
        // kernel addresses; its NULL is a kernel bug and is only refused.
        if args.space.is_user() {
            let _ = crate::uaccess::refuse_user_write(mpb);
        }
        return crate::uaccess::FAULTED;
    }
    let (start, end) = stram_range();
    let page = THEMD.0.get();
    // SAFETY: `page` is our stable static and the single boot/trap CPU is
    // the only accessor; taking the address of the `md` field is in-bounds.
    let md = unsafe { &raw mut (*page).md };

    // SAFETY: `md` is our stable static. Single boot/trap CPU, so no
    // concurrent access to `THEMD`.
    unsafe { md.write(describe(start, end)) };

    // The free-list head, expressed in the caller's address space.
    let head = match process::slot_for(process::current()) {
        // EL1 caller: the kernel VA is readable.
        None => md as u64,
        // A failed publication leaves the list empty (NULL head), never a
        // pointer the caller would fault on (spec V2.4).
        Some(slot) => {
            // SAFETY: BDOS lock held (single-CPU SVC context); `THEMD` is a
            // `'static` that owns its page, so publishing it read-only
            // exposes nothing else and it outlives every process holding
            // the pointer.
            unsafe { usermem::expose(slot, md.cast(), size_of::<PublicMd>(), ProtMode::Pr) }
                .unwrap_or(0)
        }
    };

    // `mpb` is the caller's MPB, written in the caller's space (ARCH03 3a,
    // `crate::uaccess`): a program's MPB with its rights — one it cannot
    // write ends the call and the program (EmuTOS: bus error in `getmpb`) —
    // an in-kernel caller's as the kernel address it is.
    fill_mpb(mpb, head, |va, v| args.space.put_u64(va, v))
}

/// Store the free-list head `head` and an empty allocated list into the MPB
/// at `mpb` through `put` (a store in the caller's space). Writes only
/// mp_free, then mp_used; mp_rover is preserved, exactly as EmuTOS leaves the
/// caller's value (`bios/biosmem.c:149-150`; spec §Outputs, V2.6). The first
/// store that faults ends the call with [`crate::uaccess::FAULTED`]; nothing
/// after it is written.
fn fill_mpb(mpb: u64, head: u64, mut put: impl FnMut(u64, u64) -> Result<(), UserFault>) -> Long {
    let free = mpb + offset_of!(PublicMpb, mp_free) as u64;
    let used = mpb + offset_of!(PublicMpb, mp_used) as u64;
    if put(free, head).is_err() || put(used, 0).is_err() {
        return crate::uaccess::FAULTED;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describe_is_one_unlinked_unowned_region() {
        // The measured NewTOS extent (probe G00: membot 0x400000, memtop
        // 0xE00000) and EmuTOS' TT one (membot 0x23BA0, memtop 0x3DA700).
        assert_eq!(
            describe(0x0040_0000, 0x00E0_0000),
            PublicMd { md_next: 0, md_start: 0x0040_0000, md_length: 0x00A0_0000, md_owner: 0 }
        );
        assert_eq!(describe(0x0002_3BA0, 0x003D_A700).md_length, 0x003B_6B60);
    }

    #[test]
    fn descriptor_bytes_follow_the_lp64_header_layout() {
        // What a program reads through `_MD *`: md_start at 8, md_length
        // at 16, 8 bytes each, in the machine's byte order.
        let md = describe(0x0040_0000, 0x00E0_0000);
        // SAFETY: `PublicMd` is `repr(C)` with four 8-byte fields, no padding.
        let bytes: [u8; 32] = unsafe { core::mem::transmute(md) };
        assert_eq!(&bytes[0..8], &[0; 8]);
        assert_eq!(u64::from_ne_bytes(bytes[8..16].try_into().unwrap()), 0x0040_0000);
        assert_eq!(u64::from_ne_bytes(bytes[16..24].try_into().unwrap()), 0x00A0_0000);
        assert_eq!(&bytes[24..32], &[0; 8]);
    }

    /// A store in the caller's space as the MMU presents it to an EL0
    /// program in user state: page 0 `[0, 0x1000)` is read-only (MEM12,
    /// `UserRoNx`), so a store there faults; every other address of the fake
    /// is writable. Records each store that succeeds.
    fn user_state_put(
        log: &mut Vec<(u64, u64)>,
    ) -> impl FnMut(u64, u64) -> Result<(), UserFault> + '_ {
        move |va, v| {
            if va < 0x1000 {
                return Err(UserFault);
            }
            log.push((va, v));
            Ok(())
        }
    }

    #[test]
    fn unwritable_mpb_faults_and_writes_nothing() {
        // An MPB in page 0 of a program in user state: the first store
        // (mp_free) faults and the call returns the fault result, after which
        // the syscall tail ends the program.
        let mut log = Vec::new();
        assert_eq!(fill_mpb(0x100, 0x0007_D000, user_state_put(&mut log)), crate::uaccess::FAULTED);
        assert!(log.is_empty());
    }

    #[test]
    fn null_mpb_from_a_program_is_refused_before_any_store_in_any_state() {
        // O3 (owner 2026-11-11): a program's NULL is refused up front, in
        // super state too, where its page 0 would take the store. The check
        // does not consult the super state; on the host a store to address 0
        // would crash the test, so returning shows nothing was written. The
        // recorded fault is the one a faulting `sttr` to address 0 leaves.
        let _serial =
            crate::uaccess::FAULT_RECORD_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _ = crate::uaccess::take_pending_fault();
        let args = TrapArgs { space: crate::uaccess::Space::User, ..TrapArgs::kernel([0; 8], 0x00, 13) };
        assert_eq!(getmpb(&args), crate::uaccess::FAULTED);
        let rec = crate::uaccess::take_pending_fault().expect("a recorded fault ends the program");
        assert_eq!(rec, crate::uaccess::refused_write_record(0));
        assert_eq!(rec.far, 0);
        assert_eq!(crate::bios::crash::classify(rec.esr), crate::bios::crash::CrashKind::BusError);
        assert_eq!(rec.esr & 0x7F, 0x4F, "WnR + permission fault, level 3");
    }

    #[test]
    fn writable_mpb_gets_free_then_used_only() {
        let mut log = Vec::new();
        assert_eq!(fill_mpb(0x0040_0100, 0x0007_D000, user_state_put(&mut log)), 0);
        // mp_free at +0, then mp_used at +8; mp_rover (+16) untouched.
        assert_eq!(log, [(0x0040_0100, 0x0007_D000), (0x0040_0108, 0)]);
    }

    #[test]
    fn null_mpb_from_the_kernel_is_refused_before_any_store() {
        // An in-kernel caller's NULL is a kernel bug: refused, never stored
        // through as a kernel address, and no fault is recorded (there is no
        // program to end).
        let _serial =
            crate::uaccess::FAULT_RECORD_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _ = crate::uaccess::take_pending_fault();
        let args = TrapArgs::kernel([0; 8], 0x00, 13);
        assert_eq!(getmpb(&args), crate::uaccess::FAULTED);
        assert_eq!(crate::uaccess::take_pending_fault(), None);
    }
}
