//! Memory protection per address space — FreeMiNT's `PROT_P/G/S/PR/I` on
//! AArch64 (ARCH03 stage 3b, plan `docs/analysis/arch03-implementation-plan.md`
//! §3.2–§3.4, §3.15, invariants I7–I10, decisions E5, E10, E15, E16).
//!
//! ## Model (FreeMiNT `sys/arch/mprot030.c`)
//!
//! Every allocated pool block carries a **protection mode** — FreeMiNT keeps
//! it in `global_mode_table` per page (`mprot030.c:605-615`), NewTOS in a
//! record per block ([`record`]): `Mxalloc` takes it from the mode word or
//! the caller's `PRGFLAGS` (`sys/dosmem.c:209-221`), `Malloc` from the
//! `PRGFLAGS` (nibble 0), a program's TPA and environment from the program's
//! own `PRGFLAGS` (`sys/memory.c:1403-1407,1511-1513`, `sys/k_exec.c:398`),
//! the kernel's own blocks are `PROT_S` (`sys/memory.c:150-159`).
//!
//! What a process may do with a page follows from the mode and whether it is
//! an **owner** (`mark_region`, `mprot030.c:617-693`; table §3.3 of the
//! plan):
//!
//! | mode | owner | everybody else |
//! |---|---|---|
//! | `PROT_P` | read/write/execute | invalid |
//! | `PROT_G` | read/write/execute | read/write/execute |
//! | `PROT_S` | read/write/execute | invalid (EL1 reaches it through the kernel alias) |
//! | `PROT_PR` | read/write/execute | read/execute |
//! | `PROT_I` | invalid | invalid |
//!
//! Free pool memory is invalid for everybody (I8, FreeMiNT `free_region` →
//! `mark_region(PROT_I)`, `sys/memory.c:932`).
//!
//! **Owners** are the block's EmuTOS tag `m_own` and its further owners
//! ([`crate::bdos::mem::region`]). A process slot counts as owner when one of
//! them is a process of its **memory space** — itself, or, for a `Pexec(4)`
//! child, the space it shares ([`super::Process::space_pd`], FreeMiNT
//! `FORK_SHAREVM`, plan E6) — so a thread and its caller see each other's
//! blocks. In addition:
//!
//! * **E5, not strict (default):** a child started with `Pexec` 0/6 keeps
//!   the rights to the blocks its parent's space owned when it started
//!   (FreeMiNT forks the region list and, unless `MPF_STRICT`, does not
//!   revoke it on exec: `sys/k_exec.c:799-806`). The boot argument
//!   `memprot=strict` switches this off.
//! * **E10:** while a user routine of another process runs in the current
//!   one (a `G_USERDEF` callout of a foreign tree), the current space may
//!   reach the owner's blocks ([`grant_temp`]) — FreeMiNT's `prot_temp`
//!   (`mprot030.c:775-805`).
//! * `Ptermres`: what only the resident program owned and was `PROT_P`
//!   becomes `PROT_G` (`sys/k_exit.c:320-336`).
//!
//! ## Switch
//!
//! The boot argument `memprot=off` (FreeMiNT `MEM_PROT=NO`,
//! `sys/bootmenu.c:207-219`) leaves everything as in stage 3a: the pools are
//! mapped once for everybody, block sizes keep the 16-byte grain, no record
//! is kept. With protection on (the default) the pool grain is one page
//! (E16, FreeMiNT `QUANTUM`, `sys/mint/mem.h:152`), so no page has two
//! owners (I10).
//!
//! ## Tables
//!
//! Rights are **computed, not updated**: an operation only says *where* and
//! *for whom* rights may have changed; [`sync`] recomputes those pages from
//! the pool lists, the owners, the block records and the E10 grants and
//! writes the descriptors — the shared L3 tables get the non-owner rights, a
//! slot whose rights differ in a 2-MiB region gets a private copy of that
//! region's L3 ([`mmu::make_private`]), a slot whose rights no longer differ
//! is pointed back at the shared table ([`mmu::drop_private`]).
//!
//! ### What a change marks (MEMPROT-FAST)
//!
//! A change is a **dirty range** with the slots whose descriptors in it may
//! change. It is kept as small as the change allows — FreeMiNT's
//! `mark_region` shortcut (`references/freemint/sys/arch/mprot040.c:1102-1130`:
//! nothing to do for the processes whose view stays the same):
//!
//! * the **shared** (non-owner) pattern changes only when the non-owner
//!   rights of the pages change: a block of a mode with non-owner access
//!   (`PROT_G`, `PROT_PR`) comes or goes, or a mode change alters them —
//!   then every live slot is recomputed there (its private copy holds the
//!   non-owner pattern for the pages it does not own);
//! * otherwise only the **holders** whose view changes: on allocation the
//!   block's owners; on an owner change the slots that reach the block
//!   *through* the owners that came or went ([`on_owner_change_by`]); on a
//!   mode change the holders if their own rights change (`PROT_P` ↔
//!   `PROT_S` changes nothing at all); on a free or a shrink the slots that
//!   hold a private copy of the region (a slot whose view of those pages
//!   differed from the non-owner pattern must have one — or it ran out of
//!   tables and already sees the non-owner pattern);
//! * nothing at all for a mode change to the same mode, an owner change of a
//!   block whose owners see what everybody sees (`PROT_G`, `PROT_I`), the
//!   free of a `PROT_I` block;
//! * a change of the **process structure** — a root built or released, a
//!   `Pexec(4)` child joining a memory space ([`set_space`]), an E10 grant —
//!   is the difference, per block, between the slots holding it before and
//!   after; only a freshly built root is rebuilt completely.
//!
//! A structure change that did not go through these functions (a slot's
//! root, basepage or space changed behind memprot's back) is caught at the
//! next [`sync`] and answered with a complete rebuild of every live slot —
//! the stage-3b behaviour — and counted ([`stats`]: `fallbacks`).
//!
//! ### When it is entered
//!
//! [`sync`] runs at the end of every mutating BDOS operation, before every
//! root is installed ([`mmu::install_ttbr0`]) and at the end of every trap,
//! and ends with the TLB maintenance the changed entries need
//! ([`mmu::flush_tlb`]; with ASIDs per changed page and slot). A compound
//! kernel operation that changes rights several times — `Pexec` (allocate the
//! environment and the TPA, set the mode, hand them to the child, build the
//! root), `Pterm`'s sweep, an aligned allocation — runs inside a [`batch`]:
//! the end-of-operation syncs inside it are deferred to its end, where one
//! sync enters the net change (a right granted and withdrawn again inside the
//! batch costs nothing). Root installs and the trap end always sync
//! ([`sync_now`]). Nothing inside a batch runs at EL0 or accesses user
//! memory through the user tables.
//!
//! ### Verification mode
//!
//! With the boot argument `memprot=verify` (host tests: [`set_verify`]) every
//! sync is followed by a complete, independent recomputation of the shared
//! tables and every live slot's view of every pool page (the stage-3b
//! algorithm), compared entry by entry with the tables ([`verify_now`]); a
//! difference is reported ([`take_verify_report`], printed by the kernel at
//! the end of the trap) and counted, and panics in the host tests.
//!
//! A private copy comes from the page-table arena. When the arena is empty
//! the slot keeps the shared, non-owner rights in that region — it can lose
//! access, never gain any — and that is counted ([`table_shortfalls`],
//! [`table_shortfalls_of`]) and reported once per process
//! ([`take_shortfall_report`], printed by the kernel at the end of the trap;
//! ARCH03 3c).
//!
//! ## Concurrency
//!
//! Everything here runs under the BDOS giant lock (single CPU, trap / boot
//! context), like the pools and the tables.

use core::cell::UnsafeCell;
use core::ffi::c_void;
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};

use tos_types::{BasePage, Long, ProtMode};

use super::mmu::{self, PageFlags, REGION_SPAN};
use super::{MAX_PROCESSES, PROCESS_TABLE};
use crate::bdos::mem::init::{is_initialised, pmd, pmdalt, stram_range, ttram_range};
use crate::bdos::mem::region::MAX_FURTHER_OWNERS;

// ─── Switch ──────────────────────────────────────────────────────────

static ENABLED: AtomicBool = AtomicBool::new(false);
static STRICT: AtomicBool = AtomicBool::new(false);
static VERIFY: AtomicBool = AtomicBool::new(false);
static VERIFY_PANIC: AtomicBool = AtomicBool::new(false);

/// Is memory protection on? (`false` until [`configure`], and with the boot
/// argument `memprot=off`.)
#[inline]
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Is the parent-to-child rule strict (`memprot=strict`, E5)?
#[inline]
pub fn strict() -> bool {
    STRICT.load(Ordering::Relaxed)
}

/// Set the switch from the boot arguments. Must run before the BDOS pools
/// are seeded and mapped (the pool grain and the pool mapping depend on it).
///
/// # Safety
///
/// Boot CPU, before any pool allocation.
pub unsafe fn configure(enable: bool, strict_mode: bool) {
    ENABLED.store(enable, Ordering::Relaxed);
    STRICT.store(strict_mode, Ordering::Relaxed);
}

/// Switch the verification mode (module docs): after every sync the tables
/// are compared with a complete recomputation. `panic_on_mismatch` (host
/// tests) panics at the first difference; otherwise differences are counted
/// and reported ([`take_verify_report`]). Off by default; the kernel turns it
/// on with the boot argument `memprot=verify`.
pub fn set_verify(on: bool, panic_on_mismatch: bool) {
    VERIFY.store(on, Ordering::Relaxed);
    VERIFY_PANIC.store(panic_on_mismatch, Ordering::Relaxed);
}

/// Is the verification mode on?
pub fn verifying() -> bool {
    VERIFY.load(Ordering::Relaxed)
}

/// The allocation grain of the BDOS pools as an `align_mask`: one page with
/// protection on (E16), else the stage-3a 16 bytes.
#[inline]
pub fn pool_align_mask() -> Long {
    if enabled() { mmu::PAGE_SIZE as Long - 1 } else { 15 }
}

// ─── Block records ───────────────────────────────────────────────────
//
// One record per allocated block, in a dense table of pages from the MEM05
// page budget (`docs/analysis/mem05-design-2026-10-13.md` §2.4, decision B):
// entries `0..len` are live, a lookup costs the live records, not a
// capacity. Room for the next record is secured in `traps::mxalloc` before
// the block is allocated ([`reserve`]), so recording a fresh block never
// fails.

use crate::bdos::mem::pages::{PageUse, PagedTable, ScratchRun};

#[derive(Copy, Clone)]
struct Rec {
    /// Block start (`m_start`).
    loc: u64,
    mode: ProtMode,
    /// Slots that inherited access at their start (E5, not strict).
    inherit: u32,
}

struct Records(UnsafeCell<PagedTable<Rec>>);
// SAFETY: BDOS giant lock (single CPU).
unsafe impl Sync for Records {}
static RECS: Records = Records(UnsafeCell::new(PagedTable::new(PageUse::Recs)));

/// # Safety
///
/// BDOS giant lock held; no other borrow of the records alive.
unsafe fn recs() -> &'static mut PagedTable<Rec> {
    // SAFETY: caller contract.
    unsafe { &mut *RECS.0.get() }
}

fn rec_of(r: &mut PagedTable<Rec>, loc: u64) -> Option<&mut Rec> {
    if loc == 0 {
        return None;
    }
    let i = r.position(|e| e.loc == loc)?;
    Some(r.get(i))
}

/// Make sure one more block can be allocated without a table page: room for
/// its record and for one more block in the per-sync snapshot. `false` when
/// no page can be had (MEM05 budget or kernel heap); the caller
/// ([`crate::bdos::mem::traps::mxalloc`]) then refuses the allocation before
/// anything changes. Always `true` with protection off (no tables).
///
/// # Safety
///
/// BDOS giant lock held.
pub unsafe fn reserve() -> bool {
    if !enabled() {
        return true;
    }
    // SAFETY: lock held.
    let r = unsafe { recs() };
    // SAFETY: lock held; no record borrowed.
    if !unsafe { r.reserve(1) } {
        return false;
    }
    // Every allocated block has a record, so the snapshot of the next sync
    // holds at most `records + 1` blocks.
    let need = r.len() + 1;
    // SAFETY: lock held; no snapshot alive outside a sync.
    unsafe { (*SNAP.0.get()).0.ensure(need) }
}

/// Record (or re-record) the mode of the block that starts at `loc`. A stale
/// record of a freed block at the same address is replaced. `false` only if
/// no room could be had, which cannot happen for a block allocated after a
/// successful [`reserve`].
///
/// # Safety
///
/// BDOS giant lock held.
pub unsafe fn record(loc: u64, mode: ProtMode) -> bool {
    // SAFETY: lock held.
    let r = unsafe { recs() };
    if let Some(e) = rec_of(r, loc) {
        *e = Rec { loc, mode, inherit: 0 };
        return true;
    }
    // SAFETY: lock held.
    unsafe { r.push(Rec { loc, mode, inherit: 0 }) }
}

/// Forget the record of the block that starts at `loc`.
///
/// # Safety
///
/// BDOS giant lock held.
unsafe fn unrecord(loc: u64) {
    // SAFETY: lock held.
    let r = unsafe { recs() };
    if let Some(i) = r.position(|e| e.loc == loc) {
        r.swap_remove(i);
    }
}

/// The sizes of the per-block tables (MEM05 diagnostics, host tests).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TableStats {
    /// Live block records.
    pub records: usize,
    /// Records the pages held can take.
    pub record_capacity: usize,
    /// Blocks the snapshot can take.
    pub snapshot_capacity: usize,
    /// Syncs whose snapshot could not hold every block (no page could be
    /// had; the blocks left out lose their rights until the next sync —
    /// fail-closed). Cannot happen while [`reserve`] guards every
    /// allocation.
    pub snapshot_shortfalls: u64,
}

/// The per-block tables now.
pub fn tables() -> TableStats {
    // SAFETY: reads of sizes under the BDOS lock the caller holds (host
    // tests: their lock).
    let (r, snap) = unsafe { (recs(), &(*SNAP.0.get()).0) };
    TableStats {
        records: r.len(),
        record_capacity: r.capacity(),
        snapshot_capacity: snap.capacity(),
        snapshot_shortfalls: SNAP_SHORT.load(Ordering::Relaxed),
    }
}

/// The protection mode of the block that starts at `loc` (`PROT_P` for a
/// block without a record).
///
/// # Safety
///
/// BDOS giant lock held.
pub unsafe fn mode_of(loc: u64) -> ProtMode {
    // SAFETY: lock held.
    rec_of(unsafe { recs() }, loc).map_or(ProtMode::P, |e| e.mode)
}

/// The block that starts at `old` now starts at `new` (the aligned TPA
/// allocation gives its front slack back).
///
/// # Safety
///
/// BDOS giant lock held.
pub unsafe fn rekey(old: u64, new: u64) {
    if !enabled() || old == new {
        return;
    }
    // SAFETY: lock held.
    unsafe { unrecord(new) };
    // SAFETY: lock held.
    if let Some(e) = rec_of(unsafe { recs() }, old) {
        e.loc = new;
    }
}

// ─── Block events ────────────────────────────────────────────────────

/// A block was just allocated at `[loc, loc + len)` with `mode`: record it
/// and enter its rights. The pages were free (invalid for everybody), so the
/// non-owner pattern changes only for a mode with non-owner access, and the
/// owners' view only if owners may touch the mode at all. Nothing with
/// protection off.
///
/// # Safety
///
/// BDOS giant lock held.
pub unsafe fn on_alloc(loc: u64, len: u64, mode: ProtMode) {
    if !enabled() {
        return;
    }
    note_event();
    // SAFETY: lock held.
    unsafe {
        record(loc, mode);
        let shared = other_flags(mode).is_some();
        let slots = if owner_flags(mode).is_some() {
            let st = structure(0);
            block_holders(loc, &st).map_or(0, |(_, _, h)| h)
        } else {
            0
        };
        add_dirty(loc, loc + len, slots, shared, false);
        sync();
    }
}

/// The block that started at `loc` and spanned `len` bytes went back to the
/// pool: forget it, and make its pages invalid for everybody (I8) before
/// anybody can be handed them again.
///
/// # Safety
///
/// BDOS giant lock held.
pub unsafe fn on_freed(loc: u64, len: u64) {
    if !enabled() {
        return;
    }
    note_event();
    // SAFETY: lock held.
    unsafe {
        let mode = mode_of(loc);
        unrecord(loc);
        released(mode, loc, loc + len);
        sync();
    }
}

/// The pages `[lo, hi)` of the block that starts at `block` went back to the
/// pool (the tail `Mshrink` splits off, the slack of an aligned allocation):
/// invalid for everybody again (I8; FreeMiNT `shrink_region` marks them
/// `PROT_I`, `sys/memory.c:1076,1097`). The block's record stays.
///
/// # Safety
///
/// BDOS giant lock held.
pub unsafe fn on_released(block: u64, lo: u64, hi: u64) {
    if !enabled() {
        return;
    }
    note_event();
    // SAFETY: lock held.
    unsafe {
        released(mode_of(block), lo, hi);
        sync();
    }
}

/// Pages of a block in `mode` became free: who may see a difference. The
/// non-owner pattern was `other_flags(mode)` and becomes invalid; a holder's
/// view was `owner_flags(mode)` — where that differed from the non-owner
/// pattern the holder had a private copy of the region (or kept the
/// non-owner rights after a table shortfall), so the private copies there
/// cover every holder.
///
/// # Safety
///
/// BDOS giant lock held.
unsafe fn released(mode: ProtMode, lo: u64, hi: u64) {
    let shared = other_flags(mode).is_some();
    let privates = owner_differs(mode);
    // SAFETY: lock held.
    unsafe { add_dirty(lo, hi, 0, shared, privates) };
}

/// Change the mode of the block that starts at `loc` (`mark_region`).
/// Nothing to recompute when the mode stays (FreeMiNT `mprot040.c:1102-1130`)
/// or when neither the owners' nor the others' rights differ between the two
/// modes (`PROT_P` ↔ `PROT_S`).
///
/// # Safety
///
/// BDOS giant lock held.
pub unsafe fn set_mode(loc: u64, mode: ProtMode) {
    if !enabled() {
        return;
    }
    note_event();
    // SAFETY: lock held.
    unsafe {
        let old = match rec_of(recs(), loc) {
            Some(e) => {
                let old = e.mode;
                e.mode = mode;
                old
            }
            None => {
                record(loc, mode);
                ProtMode::P
            }
        };
        let shared = other_flags(old) != other_flags(mode);
        let owners = owner_flags(old) != owner_flags(mode);
        if shared || owners {
            let st = structure(0);
            if let Some((s, e, h)) = block_holders(loc, &st) {
                add_dirty(s, e, if owners { h } else { 0 }, shared, false);
            }
        }
        sync();
    }
}

/// The owners of the block at `loc` changed in a way the caller cannot name
/// (compatibility path): the slots holding it now plus every private copy in
/// its regions. Prefer [`on_owner_change_by`].
///
/// # Safety
///
/// BDOS giant lock held.
pub unsafe fn on_owner_change(loc: u64) {
    if !enabled() {
        return;
    }
    note_event();
    // SAFETY: lock held.
    unsafe {
        if owner_differs(mode_of(loc)) {
            let st = structure(0);
            if let Some((s, e, h)) = block_holders(loc, &st) {
                add_dirty(s, e, h, false, true);
            }
        }
        sync();
    }
}

/// The owners of the block at `loc` changed, and `who` are the owner cookies
/// that came or went (attach, detach, hand-over, `set_owner`): only the slots
/// that reach the block *through* one of them — the live process behind the
/// cookie and the slots of its memory space or holding an E10 grant for it —
/// can see a difference. Nothing for a block whose owners see what everybody
/// sees (`PROT_G`, `PROT_I`).
///
/// # Safety
///
/// BDOS giant lock held.
pub unsafe fn on_owner_change_by(loc: u64, who: &[*mut c_void]) {
    if !enabled() {
        return;
    }
    note_event();
    // SAFETY: lock held.
    unsafe {
        if owner_differs(mode_of(loc)) {
            let st = structure(0);
            let mut slots = 0u32;
            for &c in who {
                if let Some(t) = st.slot_of(c) {
                    slots |= st.through(t);
                }
            }
            if slots != 0 {
                if let Some((s, e)) = block_at(loc) {
                    add_dirty(s, e, slots, false, false);
                }
            }
        }
        sync();
    }
}

/// `Ptermres`: the block at `loc` now belongs to the resident holder alone;
/// if it was private it becomes global, so every process can call into and
/// read the resident program (FreeMiNT `sys_ptermres`, `sys/k_exit.c:320-336`:
/// `links == 1 && PROT_P → mark_region(PROT_G)`).
///
/// # Safety
///
/// BDOS giant lock held.
pub unsafe fn on_resident(loc: u64) {
    if !enabled() {
        return;
    }
    note_event();
    // SAFETY: lock held.
    unsafe {
        if mode_of(loc) == ProtMode::P {
            set_mode(loc, ProtMode::G);
        } else {
            on_owner_change(loc);
        }
    }
}

// ─── Dirty tracking ──────────────────────────────────────────────────

/// One pending change: the pages `[lo, hi)`, the slots whose view of them
/// may differ, whether the non-owner pattern may differ (`shared`: every
/// live slot), and whether every slot holding a private copy of the region
/// must be looked at (`privates`).
#[derive(Copy, Clone)]
struct Dirty {
    lo: u64,
    hi: u64,
    slots: u32,
    shared: bool,
    privates: bool,
}

const NO_DIRTY: Dirty = Dirty { lo: 0, hi: 0, slots: 0, shared: false, privates: false };

/// Every slot (a conservative mask).
const ALL_SLOTS: u32 = if MAX_PROCESSES >= 32 { u32::MAX } else { (1u32 << MAX_PROCESSES) - 1 };

const MAX_DIRTY: usize = 32;

struct DirtyList(UnsafeCell<([Dirty; MAX_DIRTY], usize)>);
// SAFETY: BDOS giant lock.
unsafe impl Sync for DirtyList {}
static DIRTY: DirtyList = DirtyList(UnsafeCell::new(([NO_DIRTY; MAX_DIRTY], 0)));

/// Slots whose tables need a full rebuild at the next [`sync`].
static REBUILD: AtomicU32 = AtomicU32::new(0);

/// [`sync`] is running (it must not re-enter itself).
static SYNCING: AtomicBool = AtomicBool::new(false);

/// Open [`batch`]es; while > 0, [`sync`] defers to the end of the outermost.
static DEFER: AtomicU32 = AtomicU32::new(0);

/// A structure change went past the announcing functions; the next sync
/// rebuilds every live slot.
static FALLBACK: AtomicBool = AtomicBool::new(false);

/// A rights event happened since the last sync (even one that decided
/// nothing changes): the verification mode checks the next sync even when
/// nothing is pending — a wrongly skipped change shows there.
static EVENT: AtomicBool = AtomicBool::new(false);

fn note_event() {
    EVENT.store(true, Ordering::Relaxed);
}

/// Times a private table could not be had (arena exhausted), all slots.
static NO_TABLE: AtomicUsize = AtomicUsize::new(0);

/// Per slot: times a private table could not be had for it since its process
/// started ([`table_shortfalls_of`]).
static NO_TABLE_SLOT: [AtomicUsize; MAX_PROCESSES] = [const { AtomicUsize::new(0) }; MAX_PROCESSES];

/// Slots with a shortfall the kernel has not reported yet
/// ([`take_shortfall_report`]).
static SHORT_PENDING: AtomicU32 = AtomicU32::new(0);

/// Slots whose process has been reported already — once per process; cleared
/// when the slot is released ([`on_slot_released`]).
static SHORT_REPORTED: AtomicU32 = AtomicU32::new(0);

/// Enter a pending change (page-aligned). A range that overlaps or touches
/// an entry for the same slots and flags is merged into it; when the list is
/// full the two closest entries are merged (slot sets and flags joined — a
/// superset is always correct, only more work).
///
/// # Safety
///
/// BDOS giant lock held.
unsafe fn add_dirty(lo: u64, hi: u64, slots: u32, shared: bool, privates: bool) {
    if !enabled() || hi <= lo || (slots == 0 && !shared && !privates) {
        return;
    }
    let lo = lo & !(mmu::PAGE_SIZE - 1);
    let hi = (hi + mmu::PAGE_SIZE - 1) & !(mmu::PAGE_SIZE - 1);
    // SAFETY: lock held.
    let (list, n) = unsafe { &mut *DIRTY.0.get() };
    for d in list[..*n].iter_mut() {
        // Only entries for the same slots and flags are joined while there
        // is room: joining a block's range with its neighbour's would make
        // each one's slots recompute the other's pages.
        if lo <= d.hi && hi >= d.lo && d.slots == slots && d.shared == shared && d.privates == privates {
            d.lo = d.lo.min(lo);
            d.hi = d.hi.max(hi);
            d.slots |= slots;
            d.shared |= shared;
            d.privates |= privates;
            return;
        }
    }
    if *n == MAX_DIRTY {
        // Full: merge the two entries with the smallest gap between them.
        let (mut bi, mut bj, mut best) = (0usize, 1usize, u64::MAX);
        for i in 0..MAX_DIRTY {
            for j in i + 1..MAX_DIRTY {
                let (a, b) = (&list[i], &list[j]);
                let gap = if a.hi <= b.lo { b.lo - a.hi } else { a.lo.saturating_sub(b.hi) };
                if gap < best {
                    (bi, bj, best) = (i, j, gap);
                }
            }
        }
        let b = list[bj];
        let a = &mut list[bi];
        a.lo = a.lo.min(b.lo);
        a.hi = a.hi.max(b.hi);
        a.slots |= b.slots;
        a.shared |= b.shared;
        a.privates |= b.privates;
        list[bj] = list[MAX_DIRTY - 1];
        *n -= 1;
    }
    list[*n] = Dirty { lo, hi, slots, shared, privates };
    *n += 1;
}

/// Note that the rights of `[lo, hi)` may have changed for anybody (the
/// conservative form: the non-owner pattern and every live slot are
/// recomputed there).
///
/// # Safety
///
/// BDOS giant lock held.
pub unsafe fn touch(lo: u64, hi: u64) {
    // SAFETY: lock held.
    unsafe { add_dirty(lo, hi, ALL_SLOTS, true, true) };
}

/// [`touch`] the whole block that starts at (or contains) `loc`.
///
/// # Safety
///
/// BDOS giant lock held.
pub unsafe fn touch_block(loc: u64) {
    // SAFETY: lock held.
    if let Some((s, e)) = unsafe { block_containing(loc) } {
        // SAFETY: lock held.
        unsafe { touch(s, e) };
    }
}

/// Rebuild `slot`'s tables completely at the next [`sync`].
pub fn mark_rebuild(slot: usize) {
    if enabled() && slot < MAX_PROCESSES {
        REBUILD.fetch_or(1 << slot, Ordering::Relaxed);
    }
}

/// A compound operation whose rights changes are entered together at its
/// end (module docs, "When it is entered"). Dropping the outermost batch
/// syncs.
#[must_use = "the batch ends (and syncs) when this guard is dropped"]
pub struct Batch {
    active: bool,
}

/// Open a [`Batch`]: until the guard is dropped, [`sync`] only collects.
/// Root installs and the trap end still sync ([`sync_now`]).
///
/// # Safety
///
/// BDOS giant lock held for the guard's lifetime; nothing inside the batch
/// runs at EL0 or accesses user memory through the user tables.
pub unsafe fn batch() -> Batch {
    if !enabled() {
        return Batch { active: false };
    }
    DEFER.fetch_add(1, Ordering::Relaxed);
    Batch { active: true }
}

impl Drop for Batch {
    fn drop(&mut self) {
        if self.active && DEFER.fetch_sub(1, Ordering::Relaxed) == 1 {
            // SAFETY: the guard's contract (`batch`) — BDOS lock held.
            unsafe { sync() };
        }
    }
}

/// Bring the page tables in line with the recorded rights (see the module
/// docs) and do the TLB maintenance the changes need — unless a [`batch`]
/// is open, which syncs at its end. Cheap when nothing is pending.
///
/// # Safety
///
/// BDOS giant lock held; EL1 (on the target).
pub unsafe fn sync() {
    if DEFER.load(Ordering::Relaxed) != 0 {
        return;
    }
    // SAFETY: caller contract.
    unsafe { sync_now() };
}

/// [`sync`], even inside a [`batch`]: before a root is installed and at the
/// end of every trap.
///
/// # Safety
///
/// BDOS giant lock held; EL1 (on the target).
pub unsafe fn sync_now() {
    if !enabled() || !is_initialised() {
        return;
    }
    check_structure();
    // SAFETY: lock held; read of the pending count only.
    if unsafe { (*DIRTY.0.get()).1 } == 0 && REBUILD.load(Ordering::Relaxed) == 0 {
        // Nothing pending: no table write, no barrier (the trap fast path).
        // The verification mode still checks after a rights event that
        // decided nothing changes.
        if verifying() && EVENT.swap(false, Ordering::Relaxed) && !SYNCING.load(Ordering::Relaxed) {
            // SAFETY: lock held; read-only comparison.
            unsafe { verify_now() };
        }
        return;
    }
    if SYNCING.swap(true, Ordering::Acquire) {
        return;
    }
    EVENT.store(false, Ordering::Relaxed);
    STATS.syncs.fetch_add(1, Ordering::Relaxed);
    let st = structure(0);
    // SAFETY: lock held; the snapshot is ours until the end of this sync.
    let snap = unsafe { snapshot(&st) };
    // SAFETY: lock held.
    let (list, n) = unsafe { &mut *DIRTY.0.get() };
    while *n > 0 {
        *n -= 1;
        let d = list[*n];
        // SAFETY: lock held.
        unsafe { refresh(snap, &st, d, 0) };
    }
    let rebuild = REBUILD.swap(0, Ordering::Relaxed) & st.live;
    if rebuild != 0 {
        STATS.rebuilds.fetch_add(u64::from(rebuild.count_ones()), Ordering::Relaxed);
        for (s, e) in pools() {
            let d = Dirty { lo: s, hi: e, slots: rebuild, shared: false, privates: false };
            // SAFETY: lock held.
            unsafe { refresh(snap, &st, d, rebuild) };
        }
    }
    // SAFETY: EL1, barriers + TLB maintenance only.
    unsafe { mmu::flush_tlb() };
    SYNCING.store(false, Ordering::Release);
    if verifying() {
        // SAFETY: lock held; read-only comparison.
        unsafe { verify_now() };
    }
}

// ─── Rights ──────────────────────────────────────────────────────────

/// E10 grants: `TEMP[s]` = slots whose blocks slot `s` may reach for now.
static TEMP: [AtomicU32; MAX_PROCESSES] = [const { AtomicU32::new(0) }; MAX_PROCESSES];

/// Is `slot` a process with a root (its tables built)?
fn live(slot: usize) -> bool {
    PROCESS_TABLE[slot].l0_pa.load(Ordering::Acquire) != 0
}

/// The memory space of `slot`: its own PD, or the PD of the space it shares
/// as a `Pexec(4)` child.
fn space_root(slot: usize) -> u64 {
    let sp = PROCESS_TABLE[slot].space_pd.load(Ordering::Acquire);
    if sp.is_null() { super::pd_of(slot) as u64 } else { sp as u64 }
}

/// The process structure the rights depend on, besides the blocks: which
/// slots are live (and their basepages, which map owner cookies to slots),
/// which slots share a memory space, and the E10 grants.
#[derive(Copy, Clone)]
struct Structure {
    live: u32,
    kva: [u64; MAX_PROCESSES],
    groups: [u32; MAX_PROCESSES],
    temp: [u32; MAX_PROCESSES],
}

/// The current structure, with the slots in `exclude` counted as not live
/// (the state before a root was built / after it is released).
fn structure(exclude: u32) -> Structure {
    let mut st = Structure {
        live: 0,
        kva: [0; MAX_PROCESSES],
        groups: [0; MAX_PROCESSES],
        temp: [0; MAX_PROCESSES],
    };
    let mut roots = [0u64; MAX_PROCESSES];
    for s in 0..MAX_PROCESSES {
        st.temp[s] = TEMP[s].load(Ordering::Relaxed);
        if live(s) && exclude & (1 << s) == 0 {
            st.live |= 1 << s;
            st.kva[s] = PROCESS_TABLE[s].basepage_kva.load(Ordering::Acquire) as u64;
            roots[s] = space_root(s);
        }
    }
    for s in 0..MAX_PROCESSES {
        if st.live & (1 << s) == 0 || roots[s] == 0 {
            continue;
        }
        for t in 0..MAX_PROCESSES {
            if st.live & (1 << t) != 0 && roots[t] == roots[s] {
                st.groups[s] |= 1 << t;
            }
        }
    }
    st
}

impl Structure {
    /// The live slot behind an owner cookie (a PD pointer).
    fn slot_of(&self, pd: *mut c_void) -> Option<usize> {
        if pd.is_null() {
            return None;
        }
        let kva = super::kalias(pd as u64);
        (0..MAX_PROCESSES).find(|&s| self.live & (1 << s) != 0 && self.kva[s] == kva)
    }

    /// The live slots behind a block's owner cookies.
    fn owners(&self, own: *mut c_void, further: &[*mut c_void]) -> u32 {
        let mut m = 0u32;
        for &o in core::iter::once(&own).chain(further.iter()) {
            if let Some(s) = self.slot_of(o) {
                m |= 1 << s;
            }
        }
        m
    }

    /// Does `slot` hold owner rights on a block with these owners and
    /// inherited slots?
    fn holds(&self, slot: usize, owners: u32, inherit: u32) -> bool {
        owners & self.groups[slot] != 0 || inherit & (1 << slot) != 0 || owners & self.temp[slot] != 0
    }

    /// Every live slot holding owner rights on such a block.
    fn holders(&self, owners: u32, inherit: u32) -> u32 {
        let mut h = 0u32;
        for s in 0..MAX_PROCESSES {
            if self.live & (1 << s) != 0 && self.holds(s, owners, inherit) {
                h |= 1 << s;
            }
        }
        h
    }

    /// The live slots that reach what slot `t` owns: its memory space and
    /// the slots holding an E10 grant for it.
    fn through(&self, t: usize) -> u32 {
        let mut m = 0u32;
        for s in 0..MAX_PROCESSES {
            if self.live & (1 << s) != 0 && (self.groups[s] & (1 << t) != 0 || self.temp[s] & (1 << t) != 0) {
                m |= 1 << s;
            }
        }
        m
    }
}

/// What an owner of a block in `mode` may do with its pages (table §3.3).
fn owner_flags(mode: ProtMode) -> Option<PageFlags> {
    match mode {
        ProtMode::I => None,
        _ => Some(PageFlags::UserRwExec),
    }
}

/// What everybody else may do (table §3.3, FreeMiNT `other_dt/other_s/
/// other_wp`, `mprot030.c:400-402`).
fn other_flags(mode: ProtMode) -> Option<PageFlags> {
    match mode {
        ProtMode::G => Some(PageFlags::UserRwExec),
        ProtMode::Pr => Some(PageFlags::UserRoExec),
        ProtMode::P | ProtMode::S | ProtMode::I => None,
    }
}

/// Does an owner see this mode differently from everybody else?
fn owner_differs(mode: ProtMode) -> bool {
    owner_flags(mode) != other_flags(mode)
}

/// The pool ranges, as `[start, end)`.
fn pools() -> impl Iterator<Item = (u64, u64)> {
    [stram_range(), ttram_range()]
        .into_iter()
        .filter(|&(s, e)| e > s)
        .map(|(s, e)| (s as u64, e as u64))
}

fn in_pool(va: u64) -> bool {
    pools().any(|(s, e)| va >= s && va < e)
}

// ─── Block snapshot ──────────────────────────────────────────────────

/// One allocated block as a sync sees it.
#[derive(Copy, Clone)]
struct Snap {
    start: u64,
    end: u64,
    mode: ProtMode,
    inherit: u32,
    own: *mut c_void,
    further: [*mut c_void; MAX_FURTHER_OWNERS],
    /// Live slots behind the owner cookies ([`Structure::owners`]).
    owners: u32,
}

const NO_SNAP: Snap = Snap {
    start: 0,
    end: 0,
    mode: ProtMode::P,
    inherit: 0,
    own: ptr::null_mut(),
    further: [ptr::null_mut(); MAX_FURTHER_OWNERS],
    owners: 0,
};

/// The snapshot: a run of pages from the MEM05 budget (sorted and searched
/// as one slice, rebuilt on every sync), and the number of blocks in it.
struct SnapBuf(UnsafeCell<(ScratchRun<Snap>, usize)>);
// SAFETY: BDOS giant lock.
unsafe impl Sync for SnapBuf {}
static SNAP: SnapBuf = SnapBuf(UnsafeCell::new((ScratchRun::new(PageUse::Snap), 0)));
static SNAP_SHORT: AtomicU64 = AtomicU64::new(0);

/// Every allocated block of both pools, sorted by start, with its mode,
/// inherited slots and owners under `st`. Its room is secured before every
/// allocation ([`reserve`]); should a block still not fit, the snapshot
/// grows here, and if no page can be had the blocks left out are counted
/// ([`TableStats::snapshot_shortfalls`]).
///
/// # Safety
///
/// BDOS giant lock held; the result is valid until the next call.
unsafe fn snapshot(st: &Structure) -> &'static [Snap] {
    // SAFETY: lock held; exclusive buffer, no earlier snapshot alive.
    let (buf, n) = unsafe { &mut *SNAP.0.get() };
    let mut short = false;
    loop {
        *n = 0;
        let mut total = 0usize;
        for mp in [pmd(), pmdalt()] {
            // SAFETY: lock held; read-only list walk.
            let mut m = unsafe { (*mp).mp_mal };
            while !m.is_null() {
                // SAFETY: list invariant.
                let (start, len, own, next) =
                    unsafe { ((*m).m_start as u32 as u64, (*m).m_length as u32 as u64, (*m).m_own, (*m).m_link) };
                if len > 0 {
                    total += 1;
                    if *n < buf.capacity() {
                        // SAFETY: lock held.
                        let further = unsafe { crate::bdos::mem::region::further_owners(start as Long) };
                        // SAFETY: lock held; `*n < capacity`.
                        unsafe { buf.write(*n, Snap { start, end: start + len, own, further, ..NO_SNAP }) };
                        *n += 1;
                    }
                }
                m = next;
            }
        }
        if total <= *n || short {
            break;
        }
        // SAFETY: lock held; nothing of the run is borrowed.
        if !unsafe { buf.ensure(total) } {
            short = true;
            SNAP_SHORT.fetch_add(1, Ordering::Relaxed);
        }
    }
    // SAFETY: lock held; entries `0..n` written above.
    let blocks = unsafe { buf.filled(*n) };
    blocks.sort_unstable_by_key(|b| b.start);
    // Modes and inherited slots from the records, one pass.
    // SAFETY: lock held.
    let r = unsafe { recs() };
    for i in 0..r.len() {
        let rec = *r.get(i);
        if let Ok(i) = blocks.binary_search_by_key(&rec.loc, |b| b.start) {
            blocks[i].mode = rec.mode;
            blocks[i].inherit = rec.inherit;
        }
    }
    for b in blocks.iter_mut() {
        b.owners = st.owners(b.own, &b.further);
    }
    blocks
}

/// The blocks of `snap` overlapping `[lo, hi)`.
fn overlapping(snap: &[Snap], lo: u64, hi: u64) -> &[Snap] {
    let a = snap.partition_point(|b| b.end <= lo);
    let z = a + snap[a..].partition_point(|b| b.start < hi);
    &snap[a..z]
}

/// The block that starts at `loc`, its owner cookies and its record — read
/// directly from the pool lists (no snapshot).
///
/// # Safety
///
/// BDOS giant lock held.
unsafe fn block_holders(loc: u64, st: &Structure) -> Option<(u64, u64, u32)> {
    for mp in [pmd(), pmdalt()] {
        // SAFETY: lock held; read-only list walk.
        let mut m = unsafe { (*mp).mp_mal };
        while !m.is_null() {
            // SAFETY: list invariant.
            let (start, len, own, next) =
                unsafe { ((*m).m_start as u32 as u64, (*m).m_length as u32 as u64, (*m).m_own, (*m).m_link) };
            if start == loc && len > 0 {
                // SAFETY: lock held.
                let further = unsafe { crate::bdos::mem::region::further_owners(start as Long) };
                // SAFETY: lock held.
                let inherit = rec_of(unsafe { recs() }, start).map_or(0, |r| r.inherit);
                return Some((start, start + len, st.holders(st.owners(own, &further), inherit)));
            }
            m = next;
        }
    }
    None
}

/// The allocated block `[start, end)` that starts at `loc`.
///
/// # Safety
///
/// BDOS giant lock held.
unsafe fn block_at(loc: u64) -> Option<(u64, u64)> {
    // SAFETY: lock held.
    unsafe { block_containing(loc) }.filter(|&(s, _)| s == loc)
}

// ─── Refresh ─────────────────────────────────────────────────────────

/// Recompute the pages `d` names: the shared tables if the non-owner pattern
/// may have changed, and the slots `d` names (every live one for `shared`,
/// plus those with a private copy for `privates`). Slots in `full` get their
/// whole 2-MiB regions written (a rebuild).
///
/// # Safety
///
/// BDOS giant lock held.
unsafe fn refresh(snap: &[Snap], st: &Structure, d: Dirty, full: u32) {
    let pools: [(u64, u64); 2] = [stram_range(), ttram_range()].map(|(s, e)| (s as u64, e as u64));
    let mut region = d.lo & !(REGION_SPAN - 1);
    while region < d.hi {
        let rend = region + REGION_SPAN;
        // SAFETY: lock held.
        let Some(shared) = (unsafe { mmu::shared_l3(region) }) else {
            region = rend;
            continue;
        };
        STATS.regions.fetch_add(1, Ordering::Relaxed);
        let blks = overlapping(snap, region, rend);
        let i_lo = ((d.lo.max(region) - region) / mmu::PAGE_SIZE) as usize;
        let i_hi = ((d.hi.min(rend) - region).div_ceil(mmu::PAGE_SIZE)) as usize;
        let mut visit = if d.shared { st.live } else { d.slots & st.live };
        if d.privates {
            for s in 0..MAX_PROCESSES {
                // SAFETY: lock held.
                if st.live & (1 << s) != 0 && unsafe { mmu::slot_private_l3(s, region) }.is_some() {
                    visit |= 1 << s;
                }
            }
        }
        if d.shared {
            // SAFETY: lock held; `shared` is a live table.
            unsafe { write_pages(shared, None, region, i_lo, i_hi, blks, st, &pools) };
        }
        for s in 0..MAX_PROCESSES {
            if visit & (1 << s) == 0 {
                continue;
            }
            let needs = blks.iter().any(|b| owner_differs(b.mode) && st.holds(s, b.owners, b.inherit));
            if !needs {
                // SAFETY: lock held.
                unsafe { mmu::drop_private(s, region) };
                continue;
            }
            // SAFETY: lock held.
            let had = unsafe { mmu::slot_private_l3(s, region) }.is_some();
            // SAFETY: lock held.
            let p = match unsafe { mmu::make_private(s, region) } {
                Ok(p) => p,
                Err(_) => {
                    // No table left: the slot keeps the shared (non-owner)
                    // rights here — it can lose access, never gain it. Not
                    // silent: counted, and reported once per process.
                    note_shortfall(s);
                    continue;
                }
            };
            let (a, z) = if !had || full & (1 << s) != 0 { (0, 512) } else { (i_lo, i_hi) };
            // SAFETY: lock held; `p` is the slot's private table.
            unsafe { write_pages(p, Some(s), region, a, z, blks, st, &pools) };
        }
        region = rend;
    }
}

/// Write the descriptors of the pool pages `a..z` of the region at `region`
/// into `table` — the non-owner pattern (`owner = None`) or slot `owner`'s
/// view. Non-pool pages (publications) are left alone.
///
/// # Safety
///
/// BDOS giant lock held; `table` is the region's shared L3 (`None`) or
/// `owner`'s private copy of it.
#[allow(clippy::too_many_arguments)]
unsafe fn write_pages(
    table: *mut mmu::UserTable,
    owner: Option<usize>,
    region: u64,
    a: usize,
    z: usize,
    blks: &[Snap],
    st: &Structure,
    pools: &[(u64, u64); 2],
) {
    let mut bi = 0usize;
    let mut n = 0u64;
    for i in a..z {
        let va = region + (i as u64) * mmu::PAGE_SIZE;
        if !pools.iter().any(|&(s, e)| va >= s && va < e) {
            continue;
        }
        while bi < blks.len() && blks[bi].end <= va {
            bi += 1;
        }
        let desc = match blks.get(bi) {
            Some(b) if b.start <= va => {
                let flags = match owner {
                    Some(s) if st.holds(s, b.owners, b.inherit) => owner_flags(b.mode),
                    _ => other_flags(b.mode),
                };
                mmu::leaf_desc(va, flags)
            }
            _ => 0, // free pool memory: nobody's (I8)
        };
        // SAFETY: caller contract.
        unsafe { mmu::set_leaf(table, i, desc, va, owner) };
        n += 1;
    }
    STATS.pages.fetch_add(n, Ordering::Relaxed);
}

/// Times a slot could not get a private table (the arena was exhausted),
/// over all processes since boot.
pub fn table_shortfalls() -> usize {
    NO_TABLE.load(Ordering::Relaxed)
}

/// Times `slot`'s current process could not get a private table.
pub fn table_shortfalls_of(slot: usize) -> usize {
    NO_TABLE_SLOT.get(slot).map_or(0, |c| c.load(Ordering::Relaxed))
}

/// Count a shortfall for `slot` and queue its report if its process has not
/// been reported yet.
fn note_shortfall(slot: usize) {
    NO_TABLE.fetch_add(1, Ordering::Relaxed);
    NO_TABLE_SLOT[slot].fetch_add(1, Ordering::Relaxed);
    if SHORT_REPORTED.load(Ordering::Relaxed) & (1 << slot) == 0 {
        SHORT_PENDING.fetch_or(1 << slot, Ordering::Relaxed);
    }
}

/// A page-table shortfall to report on the serial line ([`take_shortfall_report`]).
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ShortfallReport {
    /// The process slot that could not get a private table.
    pub slot: usize,
    /// Its basepage (PD) address.
    pub pd: u64,
    /// Shortfalls of this process so far.
    pub of_process: usize,
    /// Shortfalls of all processes since boot ([`table_shortfalls`]).
    pub total: usize,
}

/// The next process whose page tables could not get a private table and
/// has not been reported yet — the kernel prints one line per process
/// (`kernel/src/arch/aarch64/exceptions.rs`, end of every trap). Marks it
/// reported: the same process is not reported again.
pub fn take_shortfall_report() -> Option<ShortfallReport> {
    let pending = SHORT_PENDING.load(Ordering::Relaxed);
    if pending == 0 {
        return None;
    }
    let slot = pending.trailing_zeros() as usize;
    SHORT_PENDING.fetch_and(!(1 << slot), Ordering::Relaxed);
    SHORT_REPORTED.fetch_or(1 << slot, Ordering::Relaxed);
    Some(ShortfallReport {
        slot,
        pd: super::pd_of(slot) as u64,
        of_process: table_shortfalls_of(slot),
        total: table_shortfalls(),
    })
}

// ─── Process structure ───────────────────────────────────────────────

/// What memprot last saw of a slot: root, basepage, shared space. A slot
/// that differs at a sync changed behind memprot's back ([`check_structure`]).
struct Seen(UnsafeCell<[(u64, u64, u64); MAX_PROCESSES]>);
// SAFETY: BDOS giant lock.
unsafe impl Sync for Seen {}
static SEEN: Seen = Seen(UnsafeCell::new([(0, 0, 0); MAX_PROCESSES]));

/// The current `(root, basepage, space)` of `slot`.
fn seen_now(slot: usize) -> (u64, u64, u64) {
    let p = &PROCESS_TABLE[slot];
    (
        p.l0_pa.load(Ordering::Acquire),
        p.basepage_kva.load(Ordering::Acquire) as u64,
        p.space_pd.load(Ordering::Acquire) as u64,
    )
}

/// # Safety
///
/// BDOS giant lock held.
unsafe fn seen() -> &'static mut [(u64, u64, u64); MAX_PROCESSES] {
    // SAFETY: caller contract.
    unsafe { &mut *SEEN.0.get() }
}

/// Before an announced change of `slot`: if the slot already differs from
/// what memprot last saw, an earlier change went unannounced — fall back.
///
/// # Safety
///
/// BDOS giant lock held.
unsafe fn expect_seen(slot: usize, expected: (u64, u64, u64)) {
    // SAFETY: lock held.
    if unsafe { seen() }[slot] != expected {
        FALLBACK.store(true, Ordering::Relaxed);
    }
}

/// At a sync: any slot that changed without an announcement (a root, a
/// basepage or a shared space set directly) — rebuild every live slot (the
/// stage-3b algorithm) and count it.
fn check_structure() {
    // SAFETY: lock held (sync's contract).
    let seen = unsafe { seen() };
    let mut changed = FALLBACK.swap(false, Ordering::Relaxed);
    for (s, v) in seen.iter_mut().enumerate() {
        let now = seen_now(s);
        if *v != now {
            changed = true;
            *v = now;
        }
    }
    if changed {
        STATS.fallbacks.fetch_add(1, Ordering::Relaxed);
        let mut all = 0u32;
        for s in 0..MAX_PROCESSES {
            if live(s) {
                all |= 1 << s;
            }
        }
        REBUILD.fetch_or(all, Ordering::Relaxed);
    }
}

/// Enter the difference between two structures: per block, the slots whose
/// holding changed. Blocks whose owners see what everybody sees need
/// nothing.
///
/// # Safety
///
/// BDOS giant lock held.
unsafe fn structure_delta(before: &Structure, after: &Structure) {
    // SAFETY: lock held.
    let snap = unsafe { snapshot(after) };
    for b in snap {
        if !owner_differs(b.mode) {
            continue;
        }
        let hb = before.holders(before.owners(b.own, &b.further), b.inherit);
        let ha = after.holders(b.owners, b.inherit);
        if hb != ha {
            // SAFETY: lock held.
            unsafe { add_dirty(b.start, b.end, hb ^ ha, false, false) };
        }
    }
}

/// A root was (re)built for `slot` ([`super::loader`]): enter its rights —
/// all of them for the new root, and for the other slots what the new live
/// process changes (the slots of its memory space reach its blocks now).
///
/// # Safety
///
/// BDOS giant lock held.
pub unsafe fn on_root_built(slot: usize) {
    if !enabled() || slot >= MAX_PROCESSES {
        return;
    }
    note_event();
    // SAFETY: lock held.
    unsafe {
        if seen()[slot] != (0, 0, 0) {
            // A root over a slot memprot still sees as taken: not the
            // fresh-slot path — fall back to a complete rebuild.
            FALLBACK.store(true, Ordering::Relaxed);
        }
        let before = structure(1 << slot);
        let after = structure(0);
        structure_delta(&before, &after);
        seen()[slot] = seen_now(slot);
        mark_rebuild(slot);
        sync();
    }
}

/// `slot` starts running, launched by the process `parent` (`null`: by the
/// kernel) — `Pexec` 0/4/6. A `Pexec(4)` child shares its caller's space
/// (its rights follow from [`super::Process::space_pd`], entered by
/// [`set_space`]); any other child of a process inherits the rights to the
/// blocks its parent's space owns now, unless the mode is strict (E5,
/// FreeMiNT `sys/k_exec.c:799-806`).
///
/// # Safety
///
/// BDOS giant lock held.
pub unsafe fn on_launch(slot: usize, parent: *mut BasePage) {
    if !enabled() || slot >= MAX_PROCESSES {
        return;
    }
    note_event();
    let thread = !PROCESS_TABLE[slot].space_pd.load(Ordering::Acquire).is_null();
    if !thread && !strict() {
        let st = structure(0);
        if let Some(ps) = st.slot_of(parent.cast()) {
            let pg = st.groups[ps] | (1 << ps);
            for mp in [pmd(), pmdalt()] {
                // SAFETY: lock held; read-only list walk.
                let mut m = unsafe { (*mp).mp_mal };
                while !m.is_null() {
                    // SAFETY: list invariant.
                    let (start, len, own, next) = unsafe {
                        ((*m).m_start as u32 as u64, (*m).m_length as u32 as u64, (*m).m_own, (*m).m_link)
                    };
                    // SAFETY: lock held.
                    let further = unsafe { crate::bdos::mem::region::further_owners(start as Long) };
                    if st.owners(own, &further) & pg != 0 {
                        // SAFETY: lock held.
                        let r = unsafe { recs() };
                        if rec_of(r, start).is_none() {
                            // SAFETY: lock held.
                            unsafe { record(start, ProtMode::P) };
                        }
                        if let Some(e) = rec_of(r, start) {
                            if e.inherit & (1 << slot) == 0 {
                                e.inherit |= 1 << slot;
                                if owner_differs(e.mode) {
                                    // SAFETY: lock held.
                                    unsafe { add_dirty(start, start + len, 1 << slot, false, false) };
                                }
                            }
                        }
                    }
                    m = next;
                }
            }
        }
    }
    // SAFETY: lock held.
    unsafe { sync() };
}

/// `slot` now runs in the memory space of `space_pd` (`null`: its own) —
/// the `Pexec(4)` thread of plan E6, FreeMiNT `FORK_SHAREVM`. Stores
/// [`super::Process::space_pd`] and enters what the new grouping changes:
/// the thread reaches its space's blocks and the space reaches the thread's.
///
/// # Safety
///
/// BDOS giant lock held.
pub unsafe fn set_space(slot: usize, space_pd: *mut BasePage) {
    if slot >= MAX_PROCESSES {
        return;
    }
    let p = &PROCESS_TABLE[slot];
    if !enabled() {
        p.space_pd.store(space_pd, Ordering::Release);
        return;
    }
    note_event();
    // SAFETY: lock held.
    unsafe {
        expect_seen(slot, seen_now(slot));
        let before = structure(0);
        p.space_pd.store(space_pd, Ordering::Release);
        let after = structure(0);
        structure_delta(&before, &after);
        seen()[slot] = seen_now(slot);
        sync();
    }
}

/// `slot` is released: its private tables go back to the arena, its
/// inherited and temporary rights are forgotten, and the slots that reached
/// its blocks through it (its memory space, E10 grants) lose that.
///
/// # Safety
///
/// BDOS giant lock held. The caller clears the slot's root, basepage and
/// space right after (`free_slot`).
pub unsafe fn on_slot_released(slot: usize) {
    if !enabled() || slot >= MAX_PROCESSES {
        return;
    }
    note_event();
    // SAFETY: lock held.
    unsafe {
        expect_seen(slot, seen_now(slot));
        let before = structure(0);
        recs().for_each_mut(|e| e.inherit &= !(1 << slot));
        TEMP[slot].store(0, Ordering::Relaxed);
        for t in &TEMP {
            t.fetch_and(!(1 << slot), Ordering::Relaxed);
        }
        let after = structure(1 << slot);
        structure_delta(&before, &after);
        // What `free_slot` leaves: no root, no basepage, no space.
        seen()[slot] = (0, 0, 0);
    }
    // The next process in this slot is reported on its own.
    NO_TABLE_SLOT[slot].store(0, Ordering::Relaxed);
    SHORT_PENDING.fetch_and(!(1 << slot), Ordering::Relaxed);
    SHORT_REPORTED.fetch_and(!(1 << slot), Ordering::Relaxed);
    REBUILD.fetch_and(!(1 << slot), Ordering::Relaxed);
    VERIFY_STATS_DUE.store(true, Ordering::Relaxed);
    // SAFETY: lock held.
    unsafe {
        mmu::release_private_tables(slot);
        // The released root may still be the installed one until the next
        // switch: drop what the TLB holds of it (stage 4: retire its ASID).
        mmu::flush_tlb_slot_released(slot);
    }
}

/// E10: let `slot` reach the blocks of `owner`'s memory space while a user
/// routine of `owner` runs in `slot` (a `G_USERDEF` callout of a foreign
/// tree) — FreeMiNT `prot_temp(loc, len, -1)` (`mprot030.c:775-805`) for
/// the owner's regions. Revoked by [`revoke_temp`].
///
/// # Safety
///
/// BDOS giant lock held.
pub unsafe fn grant_temp(slot: usize, owner: usize) {
    if !enabled() || slot >= MAX_PROCESSES || owner >= MAX_PROCESSES {
        return;
    }
    note_event();
    let before = structure(0);
    let g = before.groups[owner] | (1 << owner);
    TEMP[slot].fetch_or(g, Ordering::Relaxed);
    let after = structure(0);
    // SAFETY: lock held.
    unsafe {
        structure_delta(&before, &after);
        sync();
    }
}

/// End every temporary right of `slot` ([`grant_temp`]).
///
/// # Safety
///
/// BDOS giant lock held.
pub unsafe fn revoke_temp(slot: usize) {
    if !enabled() || slot >= MAX_PROCESSES {
        return;
    }
    note_event();
    let before = structure(0);
    if TEMP[slot].swap(0, Ordering::Relaxed) != 0 {
        let after = structure(0);
        // SAFETY: lock held.
        unsafe {
            structure_delta(&before, &after);
            sync();
        }
    }
}

// ─── Verification ────────────────────────────────────────────────────

/// Counters of the table work and of the verification mode ([`stats`]).
struct Counters {
    syncs: AtomicU64,
    regions: AtomicU64,
    pages: AtomicU64,
    rebuilds: AtomicU64,
    fallbacks: AtomicU64,
    verified: AtomicU64,
    verified_pages: AtomicU64,
    mismatches: AtomicU64,
    tolerated: AtomicU64,
}

static STATS: Counters = Counters {
    syncs: AtomicU64::new(0),
    regions: AtomicU64::new(0),
    pages: AtomicU64::new(0),
    rebuilds: AtomicU64::new(0),
    fallbacks: AtomicU64::new(0),
    verified: AtomicU64::new(0),
    verified_pages: AtomicU64::new(0),
    mismatches: AtomicU64::new(0),
    tolerated: AtomicU64::new(0),
};

/// The table work since boot, and the verification mode's results.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct Stats {
    /// Syncs that had something to do.
    pub syncs: u64,
    /// 2-MiB regions looked at.
    pub regions: u64,
    /// Leaf descriptors computed (each compared, written if different).
    pub pages: u64,
    /// Complete slot rebuilds (new roots, fallbacks).
    pub rebuilds: u64,
    /// Structure changes that went past the announcing functions.
    pub fallbacks: u64,
    /// Verifications run.
    pub verified: u64,
    /// Pool pages compared by them (shared tables + private copies).
    pub verified_pages: u64,
    /// Differences found.
    pub mismatches: u64,
    /// Regions where a slot keeps the non-owner rights after a table
    /// shortfall (not a difference: the documented 3c behaviour).
    pub tolerated: u64,
}

/// The counters since boot.
pub fn stats() -> Stats {
    let l = |a: &AtomicU64| a.load(Ordering::Relaxed);
    Stats {
        syncs: l(&STATS.syncs),
        regions: l(&STATS.regions),
        pages: l(&STATS.pages),
        rebuilds: l(&STATS.rebuilds),
        fallbacks: l(&STATS.fallbacks),
        verified: l(&STATS.verified),
        verified_pages: l(&STATS.verified_pages),
        mismatches: l(&STATS.mismatches),
        tolerated: l(&STATS.tolerated),
    }
}

/// What kind of difference the verification found.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MismatchKind {
    /// A leaf descriptor differs from the recomputed one.
    Descriptor,
    /// The slot's rights differ in the region but it has no private copy
    /// (and no table shortfall explains it).
    MissingPrivate,
    /// The slot has a private copy of a region where its rights equal
    /// everybody else's.
    StalePrivate,
    /// Changes were still pending after the sync.
    Pending,
}

/// One difference ([`take_verify_report`]).
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Mismatch {
    /// What differs.
    pub kind: MismatchKind,
    /// The slot whose table differs; `None` = the shared tables.
    pub slot: Option<usize>,
    /// The page (or region) address.
    pub va: u64,
    /// The recomputed descriptor.
    pub expected: u64,
    /// The descriptor in the table.
    pub actual: u64,
}

/// A line the kernel prints for the verification mode.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum VerifyReport {
    /// A difference (the first few per sync are kept).
    Mismatch(Mismatch),
    /// The counters (after a process ended, and every 256 verifications).
    Stats(Stats),
}

const MAX_PENDING_MISMATCHES: usize = 8;

struct MismatchQueue(UnsafeCell<([Option<Mismatch>; MAX_PENDING_MISMATCHES], usize)>);
// SAFETY: BDOS giant lock.
unsafe impl Sync for MismatchQueue {}
static MISMATCHES: MismatchQueue = MismatchQueue(UnsafeCell::new(([None; MAX_PENDING_MISMATCHES], 0)));

/// A counters line is due ([`take_verify_report`]).
static VERIFY_STATS_DUE: AtomicBool = AtomicBool::new(false);

/// The next line of the verification mode to print: a queued difference,
/// else the counters when due. `None` with the mode off.
pub fn take_verify_report() -> Option<VerifyReport> {
    if !verifying() || !enabled() {
        return None;
    }
    // SAFETY: called under the BDOS lock (end of trap).
    let (q, n) = unsafe { &mut *MISMATCHES.0.get() };
    if *n > 0 {
        let m = q[0];
        q.copy_within(1.., 0);
        *n -= 1;
        q[*n] = None;
        return m.map(VerifyReport::Mismatch);
    }
    VERIFY_STATS_DUE.swap(false, Ordering::Relaxed).then(|| VerifyReport::Stats(stats()))
}

fn note_mismatch(m: Mismatch) {
    STATS.mismatches.fetch_add(1, Ordering::Relaxed);
    if VERIFY_PANIC.load(Ordering::Relaxed) {
        panic!("memprot verify: {m:?}");
    }
    // SAFETY: under the BDOS lock (sync's contract).
    let (q, n) = unsafe { &mut *MISMATCHES.0.get() };
    if *n < MAX_PENDING_MISMATCHES {
        q[*n] = Some(m);
        *n += 1;
    }
}

/// One allocated block overlapping the region being verified.
#[derive(Copy, Clone)]
struct VBlk {
    mode: ProtMode,
    owners: u32,
    inherit: u32,
}

const NO_VBLK: VBlk = VBlk { mode: ProtMode::P, owners: 0, inherit: 0 };

/// Page → block index + 1 in the region (`0` = free pool page,
/// [`NOT_POOL`] = not pool memory — not memprot's).
const NOT_POOL: u16 = u16::MAX;

/// Blocks one 2-MiB region can overlap: with protection on the pool grain
/// is one page, so at most one block per page of the region (MEM05: fixed,
/// no longer tied to the MD supply).
const VBLKS_PER_REGION: usize = (REGION_SPAN / mmu::PAGE_SIZE) as usize;

struct VScratch(UnsafeCell<([VBlk; VBLKS_PER_REGION], [u16; VBLKS_PER_REGION])>);
// SAFETY: BDOS giant lock.
unsafe impl Sync for VScratch {}
static VSCRATCH: VScratch = VScratch(UnsafeCell::new(([NO_VBLK; VBLKS_PER_REGION], [0; VBLKS_PER_REGION])));

/// The live slot (one with a root) whose PD pointer is `pd` — the
/// verification's own lookup, independent of [`Structure`].
fn live_slot_of(pd: *mut c_void) -> Option<usize> {
    if pd.is_null() {
        return None;
    }
    let kva = super::kalias(pd as u64) as *mut BasePage;
    (0..MAX_PROCESSES).find(|&s| live(s) && PROCESS_TABLE[s].basepage_kva.load(Ordering::Acquire) == kva)
}

/// For every live slot, the set of slots in its memory space.
fn group_masks() -> [u32; MAX_PROCESSES] {
    let mut roots = [0u64; MAX_PROCESSES];
    let mut alive = 0u32;
    for (s, root) in roots.iter_mut().enumerate() {
        if live(s) {
            *root = space_root(s);
            alive |= 1 << s;
        }
    }
    let mut g = [0u32; MAX_PROCESSES];
    for s in 0..MAX_PROCESSES {
        if alive & (1 << s) == 0 || roots[s] == 0 {
            continue;
        }
        for t in 0..MAX_PROCESSES {
            if alive & (1 << t) != 0 && roots[t] == roots[s] {
                g[s] |= 1 << t;
            }
        }
    }
    g
}

fn v_has_rights(slot: usize, b: &VBlk, groups: &[u32; MAX_PROCESSES]) -> bool {
    b.owners & groups[slot] != 0
        || b.inherit & (1 << slot) != 0
        || b.owners & TEMP[slot].load(Ordering::Relaxed) != 0
}

fn v_desc(va: u64, bi: u16, blks: &[VBlk], slot: Option<(usize, &[u32; MAX_PROCESSES])>) -> u64 {
    if bi == 0 {
        return 0;
    }
    let b = &blks[(bi - 1) as usize];
    let flags = match slot {
        Some((s, groups)) if v_has_rights(s, b, groups) => owner_flags(b.mode),
        _ => other_flags(b.mode),
    };
    mmu::leaf_desc(va, flags)
}

/// Compare the tables with a complete recomputation (the stage-3b
/// algorithm, written independently of the incremental path): every pool
/// page of the shared tables, and for every live slot whether it has a
/// private copy exactly where its rights differ and every pool page of that
/// copy. Returns the number of differences (also counted and reported).
///
/// # Safety
///
/// BDOS giant lock held.
pub unsafe fn verify_now() -> u64 {
    if !enabled() || !is_initialised() {
        return 0;
    }
    STATS.verified.fetch_add(1, Ordering::Relaxed);
    let before = STATS.mismatches.load(Ordering::Relaxed);
    // SAFETY: lock held.
    let pending = unsafe { (*DIRTY.0.get()).1 } != 0 || REBUILD.load(Ordering::Relaxed) != 0;
    if pending && DEFER.load(Ordering::Relaxed) == 0 {
        note_mismatch(Mismatch { kind: MismatchKind::Pending, slot: None, va: 0, expected: 0, actual: 0 });
    }
    let groups = group_masks();
    let mut pages = 0u64;
    // SAFETY: lock held; exclusive scratch.
    let (blks, pageb) = unsafe { &mut *VSCRATCH.0.get() };
    for (ps, pe) in pools() {
        let mut region = ps & !(REGION_SPAN - 1);
        while region < pe {
            let rend = region + REGION_SPAN;
            // SAFETY: lock held.
            let Some(shared) = (unsafe { mmu::shared_l3(region) }) else {
                region = rend;
                continue;
            };
            let mut nb = 0usize;
            for (i, p) in pageb.iter_mut().enumerate() {
                *p = if in_pool(region + (i as u64) * mmu::PAGE_SIZE) { 0 } else { NOT_POOL };
            }
            for mp in [pmd(), pmdalt()] {
                // SAFETY: lock held; read-only list walk.
                let mut m = unsafe { (*mp).mp_mal };
                while !m.is_null() && nb < blks.len() {
                    // SAFETY: list invariant.
                    let (start, len, own, next) = unsafe {
                        ((*m).m_start as u32 as u64, (*m).m_length as u32 as u64, (*m).m_own, (*m).m_link)
                    };
                    let end = start + len;
                    if start < rend && end > region && len > 0 {
                        // SAFETY: lock held.
                        let further = unsafe { crate::bdos::mem::region::further_owners(start as Long) };
                        // SAFETY: lock held.
                        let (mode, inherit) = match rec_of(unsafe { recs() }, start) {
                            Some(r) => (r.mode, r.inherit),
                            None => (ProtMode::P, 0),
                        };
                        let mut owners = 0u32;
                        for &o in core::iter::once(&own).chain(further.iter()) {
                            if let Some(s) = live_slot_of(o) {
                                owners |= 1 << s;
                            }
                        }
                        blks[nb] = VBlk { mode, owners, inherit };
                        nb += 1;
                        let mut va = start.max(region) & !(mmu::PAGE_SIZE - 1);
                        while va < end.min(rend) {
                            let i = ((va - region) / mmu::PAGE_SIZE) as usize;
                            if pageb[i] != NOT_POOL {
                                pageb[i] = nb as u16;
                            }
                            va += mmu::PAGE_SIZE;
                        }
                    }
                    m = next;
                }
            }
            let blks = &blks[..nb];
            for (i, &bi) in pageb.iter().enumerate() {
                if bi == NOT_POOL {
                    continue;
                }
                let va = region + (i as u64) * mmu::PAGE_SIZE;
                let expected = v_desc(va, bi, blks, None);
                // SAFETY: lock held; live table.
                let actual = unsafe { mmu::leaf(shared, i) };
                pages += 1;
                if expected != actual {
                    note_mismatch(Mismatch { kind: MismatchKind::Descriptor, slot: None, va, expected, actual });
                }
            }
            for s in 0..MAX_PROCESSES {
                if !live(s) {
                    continue;
                }
                let needs = blks.iter().any(|b| owner_differs(b.mode) && v_has_rights(s, b, &groups));
                // SAFETY: lock held.
                let private = unsafe { mmu::slot_private_l3(s, region) };
                match (needs, private) {
                    (true, Some(p)) => {
                        for (i, &bi) in pageb.iter().enumerate() {
                            if bi == NOT_POOL {
                                continue;
                            }
                            let va = region + (i as u64) * mmu::PAGE_SIZE;
                            let expected = v_desc(va, bi, blks, Some((s, &groups)));
                            // SAFETY: lock held; the slot's private table.
                            let actual = unsafe { mmu::leaf(p, i) };
                            pages += 1;
                            if expected != actual {
                                note_mismatch(Mismatch {
                                    kind: MismatchKind::Descriptor,
                                    slot: Some(s),
                                    va,
                                    expected,
                                    actual,
                                });
                            }
                        }
                    }
                    (true, None) => {
                        if table_shortfalls_of(s) > 0 {
                            STATS.tolerated.fetch_add(1, Ordering::Relaxed);
                        } else {
                            note_mismatch(Mismatch {
                                kind: MismatchKind::MissingPrivate,
                                slot: Some(s),
                                va: region,
                                expected: 1,
                                actual: 0,
                            });
                        }
                    }
                    (false, Some(_)) => note_mismatch(Mismatch {
                        kind: MismatchKind::StalePrivate,
                        slot: Some(s),
                        va: region,
                        expected: 0,
                        actual: 1,
                    }),
                    (false, None) => {}
                }
            }
            region = rend;
        }
    }
    STATS.verified_pages.fetch_add(pages, Ordering::Relaxed);
    if STATS.verified.load(Ordering::Relaxed).is_multiple_of(256) {
        VERIFY_STATS_DUE.store(true, Ordering::Relaxed);
    }
    STATS.mismatches.load(Ordering::Relaxed) - before
}

// ─── Lookups ─────────────────────────────────────────────────────────

/// The allocated block `[start, end)` that contains `addr`, on either pool.
///
/// # Safety
///
/// BDOS giant lock held.
pub unsafe fn block_containing(addr: u64) -> Option<(u64, u64)> {
    if !is_initialised() {
        return None;
    }
    for mp in [pmd(), pmdalt()] {
        // SAFETY: lock held; read-only list walk.
        let mut m = unsafe { (*mp).mp_mal };
        while !m.is_null() {
            // SAFETY: list invariant.
            let (start, len, next) =
                unsafe { ((*m).m_start as u32 as u64, (*m).m_length as u32 as u64, (*m).m_link) };
            if addr >= start && addr < start + len {
                return Some((start, start + len));
            }
            m = next;
        }
    }
    None
}

/// What a faulting address was, for the crash report — FreeMiNT
/// `report_buserr` (`sys/arch/mprot.x:71-124`): the mode of the block it
/// lies in ("private", "super", "readable"; a global block is a hardware
/// error), "free" pool memory, or memory protection does not control
/// ("hardware"). `None` with protection off (FreeMiNT reports nothing then,
/// `mprot.x:96-97`) — except for page 0 and the stub page, which are named in
/// every mode: their rights are the super state's, not memory protection's
/// (MEM12, `docs/api/_super-state.md` S9); and the call targets of the
/// `Bconmap` `MAPTAB` function slots, which no program may call (XBIOS05).
pub fn region_kind(far: u64) -> Option<crate::bios::crash::RegionKind> {
    use crate::bios::crash::RegionKind;
    if crate::xbios::bconmap::is_vector(far) {
        return Some(RegionKind::BconmapVector);
    }
    if far < super::mmu::PAGE_SIZE {
        return Some(RegionKind::SystemVariables);
    }
    if (super::mmu::STUB_PAGE_VA..super::mmu::STUB_PAGE_VA + super::mmu::PAGE_SIZE).contains(&far) {
        return Some(RegionKind::StubPage);
    }
    if !enabled() {
        return None;
    }
    if !in_pool(far) {
        return Some(RegionKind::Hardware);
    }
    // SAFETY: the crash path runs in the trap of the faulting program, under
    // the BDOS lock; read-only walks.
    unsafe {
        match block_containing(far) {
            None => Some(RegionKind::Free),
            Some((start, _)) => Some(match mode_of(start) {
                ProtMode::P => RegionKind::Private,
                ProtMode::G => RegionKind::Hardware,
                ProtMode::S => RegionKind::Super,
                ProtMode::Pr => RegionKind::Readable,
                ProtMode::I => RegionKind::Free,
            }),
        }
    }
}
