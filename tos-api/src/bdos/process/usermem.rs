//! What user programs can reach — the stage-1 shared address space (ARCH03
//! stage 1, plan `docs/analysis/arch03-implementation-plan.md` Frage 5, §3.1).
//!
//! Classic TOS is flat: a program dereferences whatever address `Malloc`
//! returns, and so does every other program it hands the pointer to. NewTOS
//! runs programs at EL0, and since ARCH03 stage 1 every user address is its
//! own physical address (**VA = PA**, [`super::mmu`]): both BDOS pools are
//! entered into the shared user tables once at boot, so a pool address is
//! valid in every process and `Malloc`/`Mxalloc` hand out the pool block's
//! `m_start` itself — EmuTOS' answer (`bdos/umem.c`). The per-process heap
//! window that ADR-021 put here (a bump cursor, a block table, offset
//! mappings) is gone with it.
//!
//! What remains is the other direction (ADR-023): a syscall that returns a
//! pointer into the **kernel** — BIOS `Getbpb`/`Getmpb`, XBIOS
//! `Physbase`/`Logbase` — must return an address the caller can read.
//! [`expose`] enters such a page into the shared tables at VA = PA under a
//! FreeMiNT protection mode ([`tos_types::ProtMode`],
//! `references/freemint/sys/mint/mem.h:130-134`) and returns the physical
//! address as the pointer. Pool memory needs nothing: it is mapped already.
//! A publication is the same for every process (it lives in the shared
//! tables); pool memory is reached with the rights of each process since
//! ARCH03 stage 3b ([`super::memprot`]), and a pool block that is published
//! takes the publication's mode.
//!
//! [`resolve_kva`] is the reverse path, for kernel code that must follow a
//! user pointer outside its owner's trap (the AES drawing a tree or a window
//! title of another process, XBIOS `Setscreen` checking a screen base): it
//! answers the kernel alias of a user address — `va + VA_OFFSET` — if, and
//! only if, the address is live user memory: an **allocated** pool block, a
//! publication, or the owner's page 0. A freed block resolves to nothing, so
//! a pointer into memory its owner gave back (A96's zombie windows) reads as
//! nothing rather than as whatever the pool put there next.
//!
//! ## Concurrency
//!
//! The publication table lives in a static `UnsafeCell` mutated only under
//! the BDOS giant lock (the single-CPU trap context), the same discipline the
//! pool allocator and the page tables use.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU32, Ordering};

use tos_types::ProtMode;

use super::mmu::{self, PageFlags};
use super::MAX_PROCESSES;

/// User-page size (one 4-KiB physical frame).
pub const PAGE_SIZE: u64 = mmu::PAGE_SIZE;

/// First address a user pointer must stay below: a TOS address travels as a
/// signed 32-bit `LONG`, and the GEMDOS return path sign-extends it into `x0`
/// (`kernel/src/arch/aarch64/exceptions.rs`), so an address at or above
/// `0x8000_0000` reads as a negative error code (ARCH03 plan invariant I2,
/// `docs/COMPATIBILITY.md` — ADR-028, TASK/MEM11).
pub const USER_PTR_END: u64 = 0x8000_0000;

/// Debug-build check that `addr`, about to leave the kernel as the pointer
/// result of the GEMDOS function `func`, is below [`USER_PTR_END`]
/// (TASK/MEM11). `0` (the failure answer) passes.
///
/// Only a check, not the enforcement: pool placement keeps every handed-out
/// address low today; this makes a violation loud in a debug build instead of
/// a program misreading its block as an error code. Release builds compile it
/// away.
#[track_caller]
#[inline]
pub fn debug_assert_user_ptr(func: &str, addr: u64) {
    debug_assert!(
        addr < USER_PTR_END,
        "{func}: user pointer {addr:#x} >= 0x8000_0000 would read as a negative LONG (MEM11)"
    );
}

/// Maximum number of distinct kernel publications. Each is one structure's
/// page span: the BPB table, the `Getmpb` descriptor, the screen memory.
const MAX_PUBLICATIONS: usize = 16;

/// Memory type a published mapping carries — the *cacheability* half of a
/// page descriptor, orthogonal to the [`ProtMode`] permission half.
///
/// Almost everything the kernel publishes is ordinary DRAM that only the CPU
/// touches, so it is mapped Normal Cacheable Write-Back like the rest of the
/// process. The framebuffer is not: its pixels are read out by the VideoCore
/// display engine, a DMA master that scans DRAM from outside the ARM caches
/// (`docs/hardware/mailbox-framebuffer.md` §6, and the mapping note in
/// `kernel/src/drivers/bcm2711/framebuffer.rs`). The kernel keeps its own
/// Write-Back alias and cleans every drawn rectangle to the point of
/// coherency itself; an EL0 program cannot be asked to do that after every
/// store, so the pages it is handed are mapped [`MemAttr::Uncached`], where
/// a plain store lands in DRAM with no maintenance at all.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MemAttr {
    /// Normal Cacheable Write-Back, Inner Shareable (MAIR AttrIdx 0) — the
    /// attribute every other user mapping uses.
    Cacheable,
    /// Normal Non-Cacheable (MAIR AttrIdx 2) — memory shared with a DMA
    /// master the ARM caches are not coherent with.
    Uncached,
}

/// One published kernel range: page-aligned physical (= user) base, the
/// page-rounded span, and the protection and memory type it was entered with.
#[derive(Copy, Clone)]
struct Publication {
    base: u64,
    size: u64,
    prot: ProtMode,
    attr: MemAttr,
}

struct Publications(UnsafeCell<[Option<Publication>; MAX_PUBLICATIONS]>);
// SAFETY: single boot CPU under the implicit BDOS giant lock.
unsafe impl Sync for Publications {}

static PUBLICATIONS: Publications = Publications(UnsafeCell::new([None; MAX_PUBLICATIONS]));

/// Bitmask of the slots that were handed an [`MemAttr::Uncached`]
/// publication — today only the framebuffer, by `Physbase`/`Logbase` — and
/// are still alive.
///
/// The kernel's own drawing path reads it through
/// [`uncached_shared_active`]: while such a holder exists, the same
/// physical pixels are reachable through two aliases with mismatched
/// cacheability, and the cacheable one (the VDI's) must not retain a single
/// line of them across a primitive. It is a plain global because the VDI
/// asks once per drawn primitive.
static UNCACHED_SLOTS: AtomicU32 = AtomicU32::new(0);

// The bitmask must have a bit per process slot.
const _: () = assert!(MAX_PROCESSES <= u32::BITS as usize);

/// Does *any* live process hold an [`MemAttr::Uncached`] published pointer?
/// See [`UNCACHED_SLOTS`].
pub fn uncached_shared_active() -> bool {
    UNCACHED_SLOTS.load(Ordering::Relaxed) != 0
}

/// Forget that `slot` holds an uncached published pointer. Called when the
/// slot is released ([`super::pterm::release_slot`]) and when it is loaded for
/// a new tenant (`loader`): the pointer died with the process that held it.
pub fn clear_uncached(slot: usize) {
    if slot < MAX_PROCESSES {
        UNCACHED_SLOTS.fetch_and(!(1u32 << slot), Ordering::Relaxed);
    }
}

/// Round `v` up to the next multiple of [`PAGE_SIZE`].
#[inline]
fn page_round_up(v: u64) -> u64 {
    (v + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}

/// Is `[pa, pa + len)` inside one of the two BDOS pools? Pool memory is
/// mapped for every process from boot on ([`mmu::map_pools`]).
fn in_pool(pa: u64, len: u64) -> bool {
    let Some(end) = pa.checked_add(len) else {
        return false;
    };
    [crate::bdos::mem::init::stram_range(), crate::bdos::mem::init::ttram_range()]
        .iter()
        .any(|&(s, e)| e > s && pa >= s as u64 && end <= e as u64)
}

/// Publish a **kernel** structure to user programs and return the user
/// address that names it (ADR-023), which under stage 1 is its physical
/// address (VA = PA).
///
/// A syscall that hands an application a pointer to one of the kernel's own
/// structures (BIOS `Getbpb` → the drive's BPB, `Getmpb` → the memory
/// descriptor) must hand back an address the *caller* can read. The kernel is
/// linked high-half (`0xFFFF_0000_0000_0000 + phys`) and EL1-only, so the
/// raw `&'static` is useless at EL0.
///
/// * `kernel_va` — address of the structure in the kernel's high-half
///   mapping. Its physical page(s) are entered at VA = PA; the intra-page
///   offset is preserved.
/// * `len` — bytes of the structure that must be reachable.
/// * `prot` — FreeMiNT protection mode
///   (`references/freemint/sys/mint/mem.h:130-134`): [`ProtMode::Pr`]
///   (`PROT_PR`, read-only) and [`ProtMode::G`] (`PROT_G`, read/write)
///   produce a user mapping; [`ProtMode::P`], [`ProtMode::S`] and
///   [`ProtMode::I`] mean "not reachable from the user" and yield `None`.
///
/// `slot` is the calling process; stage 1 has no per-process rights, so it
/// matters only for the [`MemAttr::Uncached`] bookkeeping.
///
/// **Idempotent.** Pool memory is already mapped and is returned as is;
/// a range published before with the same protection and memory type is
/// returned again. A request that would need a *different* mapping of pages
/// already published (or mapped) — a change of a valid entry, which stage 1
/// never makes — yields `None`, as does an exhausted table.
///
/// # Safety
///
/// Caller holds the BDOS giant lock. `kernel_va .. kernel_va + len` must lie
/// inside the kernel's high-half mapping of DRAM or of a permanently mapped
/// device buffer (the VideoCore framebuffer) and stay valid for as long as a
/// process may hold the returned pointer — a `'static` kernel object. The
/// whole physical page(s) covering it become readable (and, with
/// [`ProtMode::G`], writable) to every process, so the object must own its
/// page(s).
pub unsafe fn expose(
    slot: usize,
    kernel_va: *const u8,
    len: usize,
    prot: ProtMode,
) -> Option<u64> {
    // SAFETY: the caller's contract is exactly `expose_attr`'s.
    unsafe { expose_attr(slot, kernel_va, len, prot, MemAttr::Cacheable) }
}

/// [`expose`], plus an explicit choice of memory type ([`MemAttr`]).
///
/// Publishing a region as [`MemAttr::Uncached`] while the kernel keeps its
/// own Write-Back alias of the same physical pages is a **mismatched-
/// attribute** situation in the ARM ARM's sense. The architecture permits a
/// loss of coherency there unless software guarantees that no cached copy of
/// the region survives, so whenever the first live holder of such a pointer
/// appears, this function cleans **and invalidates** the published span
/// through the kernel alias before it returns: no dirty line can later be
/// written back over what the program stores, and no clean line can satisfy
/// a stale kernel read of what the program stored. From then on the VDI
/// keeps the region out of the caches itself ([`uncached_shared_active`]).
///
/// # Safety
///
/// As [`expose`]. Additionally, for [`MemAttr::Uncached`] the caller must
/// accept that the kernel's cached copies of `[kernel_va, kernel_va + len)`
/// are discarded — the region must not be holding un-written-back state the
/// kernel still needs beyond what this call writes back.
pub unsafe fn expose_attr(
    slot: usize,
    kernel_va: *const u8,
    len: usize,
    prot: ProtMode,
    attr: MemAttr,
) -> Option<u64> {
    if slot >= MAX_PROCESSES || len == 0 {
        return None;
    }
    // PROT_P / PROT_S / PROT_I give the user no access at all — there is
    // nothing to map, which is a legitimate answer, not a failure.
    let flags = match (prot, attr) {
        (ProtMode::Pr, MemAttr::Cacheable) => PageFlags::UserRoNx,
        (ProtMode::G, MemAttr::Cacheable) => PageFlags::UserRwNx,
        (ProtMode::Pr, MemAttr::Uncached) => PageFlags::UserRoNxUncached,
        (ProtMode::G, MemAttr::Uncached) => PageFlags::UserRwNxUncached,
        (ProtMode::P | ProtMode::S | ProtMode::I, _) => return None,
    };

    let pa = super::kva_to_user_va(kernel_va as u64);
    // Pool memory (the AES screen buffer `WF_SCREEN` hands out, for one) is
    // in every process's tables already, read/write — a publication would
    // only try to remap it. With memory protection on (ARCH03 3b) the block
    // it lies in takes the publication's mode instead, so everybody gets the
    // access `prot` grants (FreeMiNT `mark_region`).
    if attr == MemAttr::Cacheable && in_pool(pa, len as u64) {
        if super::memprot::enabled() {
            // SAFETY: caller's lock contract.
            unsafe {
                if let Some((start, _)) = super::memprot::block_containing(pa) {
                    super::memprot::set_mode(start, prot);
                }
            }
        }
        return Some(pa);
    }

    let base = pa & !(PAGE_SIZE - 1);
    let span = page_round_up(pa - base + len as u64);

    // SAFETY: caller's lock contract; exclusive access to the table.
    let pubs = unsafe { &mut *PUBLICATIONS.0.get() };
    let known = pubs.iter().flatten().any(|p| {
        p.base <= base && base + span <= p.base + p.size && p.prot == prot && p.attr == attr
    });
    if !known {
        let free = pubs.iter().position(Option::is_none)?;
        // SAFETY: caller's lock contract; the range is a kernel-owned
        // `'static` region the caller vouches may be handed to programs.
        unsafe { mmu::map_shared(base, span, flags, false) }.ok()?;
        pubs[free] = Some(Publication { base, size: span, prot, attr });
    }

    if attr == MemAttr::Uncached {
        let before = UNCACHED_SLOTS.fetch_or(1u32 << slot, Ordering::Relaxed);
        if before == 0 {
            // The first live holder: drop every cached copy of the span so
            // neither a dirty line can be written back over the program's
            // stores nor a clean line satisfy a stale read of them (ARM ARM,
            // "Mismatched memory attributes").
            // SAFETY: the caller guarantees `[kernel_va, +len)` is mapped and
            // readable at EL1; maintenance is by VA over exactly that span.
            unsafe { mmu::clean_invalidate_dcache_range(kernel_va as usize, len) };
        }
    }
    Some(pa)
}

/// Is user VA `va` inside a published range?
fn in_publication(va: u64) -> bool {
    // SAFETY: single-CPU under the BDOS lock; shared read.
    let pubs = unsafe { &*PUBLICATIONS.0.get() };
    pubs.iter().flatten().any(|p| va >= p.base && va - p.base < p.size)
}

/// The **kernel high-half alias** of user address `va` as process `slot`
/// sees it, or `None` if it names no live user memory.
///
/// [`expose`] answers "where can a program reach this kernel object"; this
/// answers the other direction, which kernel code needs when it follows a
/// pointer a program handed it *outside that program's trap* (the AES
/// drawing another process's tree or window title, A83/A97) or must check one
/// (XBIOS `Setscreen`'s screen base):
///
/// * `va` in page 0 (`< 0x1000`) or in the private read-write page — the
///   slot's own frame ([`mmu::private_page_alias`]); the EL0 execute-only stub
///   page names no data (`None`);
/// * `va` inside an **allocated** BDOS pool block, or inside a publication —
///   `va + VA_OFFSET` (VA = PA);
/// * anything else — free pool memory, unmapped addresses — `None`.
///
/// The result is directly comparable with the addresses
/// [`crate::host::ScreenInfo`] reports, because the whole of DRAM (and the
/// framebuffer) is mapped for EL1 at `phys + VA_OFFSET`.
///
/// **On the host** (unit and integration tests) user memory is host memory
/// and [`super::kalias`] is the identity, so an address outside the BDOS
/// pools — a test's own buffer — is taken as live; inside a pool (a test
/// pool at addresses nobody dereferences) the allocated-block rule applies
/// exactly as on the target.
pub fn resolve_kva(slot: usize, va: u64) -> Option<u64> {
    if slot >= MAX_PROCESSES {
        return None;
    }
    if mmu::in_private_page(va) {
        // Page 0 and the private read-write page through the slot's own
        // frames; the EL0 execute-only stub page is no data (MEM12).
        return mmu::private_page_alias(slot, va);
    }
    // SAFETY: single-CPU under the BDOS lock; read-only list walk.
    let allocated = unsafe { crate::bdos::mem::traps::is_allocated(va) };
    let live = allocated || in_publication(va) || host_memory(va);
    live.then(|| super::kalias(va))
}

/// On the host, an address outside both BDOS pools is ordinary host memory
/// (see [`resolve_kva`]); on the target there is no such memory.
#[inline]
fn host_memory(va: u64) -> bool {
    #[cfg(all(target_arch = "aarch64", target_os = "none"))]
    {
        let _ = va;
        false
    }
    #[cfg(not(all(target_arch = "aarch64", target_os = "none")))]
    {
        !in_pool(va, 1)
    }
}
