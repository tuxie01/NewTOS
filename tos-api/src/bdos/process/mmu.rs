//! User page tables — one shared address space, one root per process
//! (ARCH03 stage 1, ADR-025 draft in `docs/analysis/arch03-implementation-plan.md`
//! §1, §3.1, §3.2).
//!
//! ## Model
//!
//! Every user address is its own physical address (**VA = PA**) — the two
//! BDOS pools, the published kernel structures (ADR-023) and the screen
//! memory appear at the same address in every process, so a pointer means the
//! same thing in every process, like on EmuTOS' flat machine and under
//! FreeMiNT with memory protection off. The exceptions are three pages in
//! the slot-private `[0, 2 MiB)` (MEM12, `docs/api/_super-state.md`):
//!
//! * **page 0**, `[0, 0x1000)`, each process its own (the classic system
//!   variables, the OSHEADER, the `$5A0` cookie jar, the `Keytbl` area —
//!   plan E3), backed by a per-slot kernel page ([`page0_kva`]): EL0
//!   read-only in user state, read-write in super state, never executable
//!   ([`set_page0_writable`]);
//! * the **stub page** at [`STUB_PAGE_VA`], one page for all (the upcall
//!   trampoline and the other EL0 stubs): EL0 execute-only;
//! * the **private read-write page** at [`PRIV_PAGE_VA`], each process its
//!   own (the KEYS02 `Kbdvbase` table, the `G_USERDEF` `PARMBLK`): EL0
//!   read-write, never executable;
//! * the **IOREC page** at [`IOREC_PAGE_VA`], one page for all (the
//!   `Iorec` records and their rings, XBIOS01 E1): EL0 read-write, never
//!   executable.
//!
//! FreeMiNT keeps "one of these per process" over one logical address space
//! (`references/freemint/sys/arch/mprot030.c:18,76-80`); stage 1 is its
//! `no_mem_prot` form: every process root shares all tables below it except
//! the ones that lead to page 0.
//!
//! ## Tables
//!
//! ```text
//!   per slot:  L0 ─► L1 ─► L2 (first GiB) ─► L3 for [0, 2 MiB) ─► page 0 (private)
//!                    │       └ entries 1..511: copies of the shared GiB-0 template
//!                    └ entries 1..3: the shared L2 tables of GiB 1..3
//!   shared:    L2 templates/tables for GiB 0..3, the [0, 2 MiB) L3 template,
//!              and a fixed set of L3 tables ([`SHARED_L3_COUNT`]) for 2-MiB
//!              regions that are only partly user memory.
//! ```
//!
//! Only VA `[0, 4 GiB)` is coverable (four L1 entries); every user address is
//! below `0x8000_0000` anyway (plan invariant I2, `TOS_MD_MAX_END`). Entries of
//! the GiB-0 L2 template and of the `[0, 2 MiB)` L3 template are copied into
//! every slot's own tables — at [`init_slot_tables`] and on every later
//! change ([`map_shared`] writes the template and all sixteen copies) — so the
//! per-slot tables never fall behind the shared ones.
//!
//! The pools are entered once at boot ([`map_pools`], plan Frage 2 / E18: the
//! 0b spike and the Pi run confirmed boot-time mapping), with 2-MiB blocks
//! where a whole aligned 2-MiB region is pool and 4-KiB pages at the edges.
//! Later publications ([`map_shared`]) only ever turn **invalid** entries
//! valid, which needs no TLB invalidation (ARM ARM `RXCLRD` D8-7852 /
//! `IWZCBG` D8-7858, plan §3.2) — nothing in stage 1 is ever unmapped.
//!
//! ## Rights per address space (ARCH03 stage 3b)
//!
//! With memory protection on ([`super::memprot`]) the pools are entered as
//! 4-KiB pages only, every entry **invalid** at boot (free memory belongs to
//! nobody, plan invariant I8), in shared L3 tables drawn from a table arena
//! the kernel reserves at boot ([`set_table_arena`]). The shared tables hold
//! the **non-owner** rights of every page (FreeMiNT fills a table "as if
//! you're a non-owner of every page" first, `sys/arch/mprot030.c:1050-1053`);
//! a process whose rights differ somewhere in a 2-MiB region gets its own
//! copy of that region's L3 ([`make_private`], "copy-on-differ", plan §3.2)
//! and its L2 entry points there instead. Every slot has its own L2 for each
//! of the four GiB, a copy of the shared template, so a private L3 is a
//! change of one slot-owned L2 entry on both boards (V0a invariant 4: the
//! virt pool lives in GiB 1).
//!
//! Table-pointer changes in an L2 entry are made break-before-make (V0a
//! C3/C4, ARM ARM `RDDMVT` D8-7859): invalid → `dsb ish` → `tlbi vmalle1is`
//! → `dsb ish` → new entry → `dsb ish`. Leaf changes of a **valid** entry
//! (narrowing, and widening valid → valid) are followed by a TLB invalidation
//! ([`flush_tlb`], ARM ARM `RSQBCS` D8-7852 — 0a correction of plan §3.2).
//!
//! ## ASIDs (stage 4, boot argument `asid=on`; off by default)
//!
//! Off: every root runs with ASID 0 and the kernel drops the whole TLB on
//! every address-space switch (`restore_boot_ttbr0`), as since Phase 06.
//! On ([`configure_asids`], plan Stufe 4 / E12): every user leaf has `nG = 1`,
//! every root is installed with an ASID of its own ([`install_ttbr0`] →
//! [`install_tagged`]; numbers from [`super::asid::Allocator`], one per slot
//! *tenancy*, generations with a full flush on rollover), and a switch drops
//! nothing. A narrowing then owes invalidations by VA: `TLBI VAAE1IS` for a
//! shared table's page, `TLBI VAE1IS` with the owner's ASID for a private one
//! ([`set_leaf`], [`flush_tlb`]); a slot L2 entry's break-before-make takes
//! `TLBI VAE1IS` for the region with the slot's ASID ([`replace_bbm`]); a torn
//! down or rebuilt root retires its ASID (`TLBI ASIDE1IS`,
//! [`flush_tlb_slot_released`], [`init_slot_tables`]).
//!
//! ## Descriptor bits
//!
//! AArch64 stage-1 descriptor layout (ARM ARM DDI 0487 M.d, Figure D8-16 /
//! Table D8-52, verified in V0a): `AP[2:1]` = bits 7:6 (`01` EL1+EL0 RW, `11`
//! EL1+EL0 RO), `SH` = 9:8, `AF` = 10, `AttrIndx` = 4:2, `PXN` = 53, `UXN` =
//! 54; a level-2 entry with bit 1 clear is a 2-MiB block. `AF` is set in every
//! valid entry (the A72 has no hardware access-flag management, V0a C3).
//!
//! ## Concurrency
//!
//! Tables are mutated only under the BDOS giant lock (the single-CPU trap /
//! boot context). The walker is the only concurrent reader; a `dsb ish`
//! orders the writes before it ([`publish_user_mappings`],
//! [`install_ttbr0`]).

use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

use super::MAX_PROCESSES;

// ─── Geometry ────────────────────────────────────────────────────────

/// Bytes in one page (the 4-KiB granule).
pub const PAGE_SIZE: u64 = 4096;

/// VA span of one L3 table / one L2 block (2 MiB).
const L3_SPAN: u64 = 512 * PAGE_SIZE;

/// VA span of one L1 entry / one L2 table (1 GiB).
const L2_SPAN: u64 = 512 * L3_SPAN;

/// Number of 1-GiB regions the user tables cover (`[0, 4 GiB)`).
const GIB_COUNT: usize = 4;

/// One past the highest user VA the tables can map.
pub const USER_VA_LIMIT: u64 = GIB_COUNT as u64 * L2_SPAN;

/// L3 tables available for 2-MiB regions that are only partly user memory —
/// the edges of the pools, the screen memory (not 2-MiB aligned on the Pi,
/// 0b Q5), published kernel pages. Measured need (0b Q2, Plan Frage 14): the
/// pools take at most 2 per pool edge, the largest screen (1920×1080×4, two
/// pages) 9, the kernel publications 1–2. 64 tables = 256 KiB of BSS; the
/// per-process window tables this replaces were 1216 KiB.
pub const SHARED_L3_COUNT: usize = 64;

// ─── Descriptor bits ─────────────────────────────────────────────────

const D_VALID: u64 = 1 << 0;
const D_TABLE: u64 = 1 << 1; // L0/L1/L2 next-level table
const D_PAGE_L3: u64 = 1 << 1; // L3 page (same bit, level-dependent)
const D_AF: u64 = 1 << 10;
const D_AP_USER_RW: u64 = 0b01 << 6; // EL1 + EL0 R/W
const D_AP_USER_RO: u64 = 0b11 << 6; // EL1 + EL0 RO
/// `AP[2:1] = 10`: EL1 read-only, no EL0 data access (ARM ARM Table D8-63).
/// With `UXN = 0`, `PXN = 1` the page is **execute-only** for EL0:
/// `PrivRead, UnprivExecute` (Table D8-65, D8-7760) — the stub page (MEM12).
const D_AP_PRIV_RO: u64 = 0b10 << 6;
const D_SH_INNER: u64 = 0b11 << 8;
/// Non-shareable. Only used with [`D_ATTRIDX_NC`]: Normal Non-cacheable
/// memory is architecturally treated as Outer Shareable whatever the SH
/// field says, so this mirrors the kernel's own Non-Cacheable framebuffer
/// mapping (`kernel::arch::aarch64::mmu::remap_normal_nc`) rather than
/// inventing a second spelling for the same thing.
const D_SH_NON: u64 = 0b00 << 8;
const D_ATTRIDX_NORMAL: u64 = 0 << 2;
/// AttrIdx 2 = Normal **Non-Cacheable** (MAIR `0x44`, programmed once by
/// the boot stub — `kernel::arch::aarch64::boot::MAIR_VALUE`). The
/// attribute the framebuffer carries for memory shared with a DMA master
/// the ARM caches are not coherent with.
const D_ATTRIDX_NC: u64 = 2 << 2;
const D_PXN: u64 = 1 << 53; // EL1 must not execute
const D_UXN: u64 = 1 << 54; // EL0 must not execute
/// Not-global (bit 11, ARM ARM Table D8-52 / `RQGKGF` D8-7854): the leaf
/// applies to the current ASID only. Set on every user leaf when ASIDs are on
/// (stage 4, [`configure_asids`]); with ASIDs off every root runs with ASID 0
/// and each switch drops the whole TLB, so the bit stays clear as before.
const D_NG: u64 = 1 << 11;

/// Output-address field of a 4-KiB page / table descriptor (bits 47:12).
const OA_MASK: u64 = 0x0000_FFFF_FFFF_F000;

/// Phase-05 high-half offset. Mirrors the same constant in
/// `kernel::arch::aarch64::mmu`; the user tables live in the high-half kernel
/// image and their physical addresses go into the descriptors and `TTBR0_EL1`.
const VA_OFFSET: u64 = 0xFFFF_0000_0000_0000;

#[inline(always)]
const fn va_to_pa(va: u64) -> u64 {
    va.wrapping_sub(VA_OFFSET)
}

// ─── Static tables ───────────────────────────────────────────────────

/// 512-entry, 4-KiB-aligned translation table page.
#[repr(C, align(4096))]
pub struct UserTable([u64; 512]);

impl UserTable {
    const fn zeroed() -> Self {
        UserTable([0; 512])
    }
}

/// One process slot's private tables: its root chain down to the L3 that
/// holds page 0, and its own L2 for every GiB. 28 KiB per slot.
#[repr(C, align(4096))]
struct SlotTables {
    l0: UserTable,
    l1: UserTable,
    /// L2 for VA `[g GiB, g+1 GiB)`, a copy of [`SHARED_L2`]`[g]`; for GiB 0
    /// entry 0 → [`Self::l3`]. An entry differs from the template only where
    /// the slot has a private L3 (stage 3b, [`make_private`]).
    l2: [UserTable; GIB_COUNT],
    /// L3 for VA `[0, 2 MiB)`: entry 0 → the slot's page 0, entries 1..511
    /// copied from [`SHARED_L3_0`].
    l3: UserTable,
}

/// One process slot's page 0 — the data page user VA `[0, 0x1000)` maps to.
#[repr(C, align(4096))]
pub struct Page0(pub [u8; 4096]);

#[unsafe(no_mangle)]
static mut __USER_TABLES: [SlotTables; MAX_PROCESSES] = [const {
    SlotTables {
        l0: UserTable::zeroed(),
        l1: UserTable::zeroed(),
        l2: [const { UserTable::zeroed() }; GIB_COUNT],
        l3: UserTable::zeroed(),
    }
}; MAX_PROCESSES];

/// The per-slot page-0 data pages.
static mut PAGE0: [Page0; MAX_PROCESSES] = [const { Page0([0; 4096]) }; MAX_PROCESSES];

/// The one **stub page** (MEM12, `docs/api/_super-state.md`): the EL0 code
/// the kernel and the programs jump to — upcall trampoline, `G_USERDEF`
/// callout stub, the KEYS02 `kbdvec` stubs, the OSHEADER reset stub. Its
/// content is the same for every process, so one page serves all; every slot
/// maps it at [`STUB_PAGE_VA`], EL0 execute-only.
static mut STUB_PAGE: Page0 = Page0([0; 4096]);

/// The per-slot **private read-write page** (MEM12): what a program may write
/// that used to sit in page 0 — the KEYS02 `Kbdvbase` table and the AES
/// `PARMBLK` scratch. Mapped at [`PRIV_PAGE_VA`], EL0 read-write, never
/// executable.
static mut PRIV_PAGE: [Page0; MAX_PROCESSES] = [const { Page0([0; 4096]) }; MAX_PROCESSES];

/// The one **IOREC page** (XBIOS01, `docs/api/xbios/0e-iorec.md` E1): the
/// records `Iorec(0/1/2)` returns and their ring buffers, the IKBD ring being
/// the live console key queue. Global like EmuTOS' `ikbdiorec`
/// (`bios/aciavecs.S:107`), so one page serves all; every slot maps it at
/// [`IOREC_PAGE_VA`], EL0 read-write (a program flushes the queue by writing
/// `head = tail`), never executable. Its content is program data: the kernel
/// validates it on every use (`crate::xbios::iorec`).
static mut IOREC_PAGE: Page0 = Page0([0; 4096]);

/// EL0 VA of the shared stub page. Inside the slot-private `[0, 2 MiB)` L3,
/// below the Pi kernel's load address (`0x80000`, `link-rpi4.ld`; there is
/// only the pre-MMU boot stack below it, never a published kernel object)
/// and far below the qemu-virt RAM (`0x4000_0000`) — so no VA = PA user
/// mapping can ever want this VA; [`map_shared`] refuses it all the same.
/// Below 4 GiB, so a stub address fits the 32-bit OSHEADER fields.
pub const STUB_PAGE_VA: u64 = 0x7E000;

/// EL0 VA of the slot's private read-write page (see [`STUB_PAGE_VA`]).
pub const PRIV_PAGE_VA: u64 = 0x7F000;

/// EL0 VA of the shared IOREC page (see [`STUB_PAGE_VA`] for why this range
/// is free). Below 4 GiB and even, as `Iorec`'s returned pointer must be
/// (`docs/api/xbios/0e-iorec.md`, "Trap interface").
pub const IOREC_PAGE_VA: u64 = 0x7D000;

/// First VA of the special pages in the slot-private `[0, 2 MiB)` L3 — the
/// IOREC, stub and private read-write pages, `[SPECIAL_PAGES_VA,
/// PRIV_PAGE_VA + PAGE_SIZE)`.
const SPECIAL_PAGES_VA: u64 = IOREC_PAGE_VA;

const STUB_PAGE_IDX: usize = (STUB_PAGE_VA / PAGE_SIZE) as usize;
const PRIV_PAGE_IDX: usize = (PRIV_PAGE_VA / PAGE_SIZE) as usize;
const IOREC_PAGE_IDX: usize = (IOREC_PAGE_VA / PAGE_SIZE) as usize;
const _: () = assert!(STUB_PAGE_IDX > 0 && PRIV_PAGE_IDX > 0 && STUB_PAGE_IDX < 512 && PRIV_PAGE_IDX < 512);
const _: () = assert!(IOREC_PAGE_IDX > 0 && IOREC_PAGE_IDX < 512);
const _: () = assert!(PRIV_PAGE_VA == STUB_PAGE_VA + PAGE_SIZE);
const _: () = assert!(STUB_PAGE_VA == IOREC_PAGE_VA + PAGE_SIZE);
const _: () = assert!(PRIV_PAGE_VA + PAGE_SIZE <= 0x80000);

/// The shared L2 **templates**, one per GiB, that every slot's own L2s copy
/// (GiB 0's entry 0 is unused — it is the slot's own page-0 L3).
static mut SHARED_L2: [UserTable; GIB_COUNT] = [const { UserTable::zeroed() }; GIB_COUNT];

/// Template for every slot's `[0, 2 MiB)` L3 (entry 0 unused — page 0).
static mut SHARED_L3_0: UserTable = UserTable::zeroed();

/// L3 tables for partly-user 2-MiB regions, handed out in order.
static mut SHARED_L3: [UserTable; SHARED_L3_COUNT] =
    [const { UserTable::zeroed() }; SHARED_L3_COUNT];

/// Number of [`SHARED_L3`] tables handed out.
static SHARED_L3_USED: AtomicUsize = AtomicUsize::new(0);

/// Number of shared L3 tables in use, wherever they came from (the static
/// [`SHARED_L3`] set or the arena) — the boot log figure.
static SHARED_L3_TOTAL: AtomicUsize = AtomicUsize::new(0);

// ─── Table arena (stage 3b) ──────────────────────────────────────────

/// Upper bound on the arena's size in tables (128 bitmap words). Room for
/// the shared tables of 2 GiB of pools (1024), two un-shrunk largest-block
/// TPAs' private copies (E9: a TPA is the largest free block until its
/// program shrinks it, 2 × 1024) and the per-process budget besides.
pub const ARENA_MAX_TABLES: usize = 128 * 64;

/// Kernel VA of the first arena table; `0` = no arena (protection off).
static ARENA_KVA: AtomicU64 = AtomicU64::new(0);
/// Number of tables in the arena.
static ARENA_COUNT: AtomicUsize = AtomicUsize::new(0);
/// Tables of the arena in use (bit set = taken).
static ARENA_BITS: [AtomicU64; ARENA_MAX_TABLES / 64] =
    [const { AtomicU64::new(0) }; ARENA_MAX_TABLES / 64];
/// Private (per-slot) tables in use — for the boot log and the tests.
static PRIVATE_USED: AtomicUsize = AtomicUsize::new(0);

/// A **valid** leaf entry of a user table was changed since the last
/// [`flush_tlb`] — the TLB may hold the old translation (ARM ARM `RSQBCS`
/// D8-7852: even permission-fault entries may be cached).
static TLB_DIRTY: AtomicBool = AtomicBool::new(false);

// ─── ASIDs (stage 4) ─────────────────────────────────────────────────

/// ASID numbers per generation in the rollover test mode (boot argument
/// `asid=tiny`): four, three of them usable. Hardware ASIDs are 8 or 16 bits
/// wide (`ID_AA64MMFR0_EL1.ASIDBits`), so a generation limit of 4 is this mode
/// and nothing else — `Ssystem(S_NEWTOS_ASID)` reports it as "test".
pub const ASID_TEST_LIMIT: u32 = 4;

/// ASIDs on (boot argument `asid=on`/`asid=tiny`, [`configure_asids`]).
static ASID_ON: AtomicBool = AtomicBool::new(false);

/// PA of the kernel's empty no-process L0 — what `TTBR0_EL1` holds (with
/// ASID 0) while the whole TLB is dropped on an ASID rollover.
static EMPTY_L0_PA: AtomicU64 = AtomicU64::new(0);

struct AsidState(core::cell::UnsafeCell<(super::asid::Allocator, super::asid::Queue)>);
// SAFETY: touched only under the BDOS giant lock (single CPU, trap / boot
// context), like the tables themselves.
unsafe impl Sync for AsidState {}
static ASIDS: AsidState = AsidState(core::cell::UnsafeCell::new((
    super::asid::Allocator::new(),
    super::asid::Queue::new(),
)));

/// # Safety
///
/// BDOS giant lock held; no other borrow alive.
#[allow(clippy::mut_from_ref)]
unsafe fn asid_state() -> &'static mut (super::asid::Allocator, super::asid::Queue) {
    // SAFETY: caller contract.
    unsafe { &mut *ASIDS.0.get() }
}

/// Are per-process ASIDs on? Off unless the kernel called
/// [`configure_asids`] at boot.
#[inline]
pub fn asids_enabled() -> bool {
    ASID_ON.load(Ordering::Relaxed)
}

/// Switch ASIDs on (ARCH03 stage 4): every user leaf gets `nG = 1`, every
/// root is installed with an ASID of its own and **without** a TLB
/// invalidation, and a page-table change invalidates by VA (and ASID) instead
/// of the whole TLB. `limit` = numbers per generation (2^ASID width;
/// smaller only as a test of the rollover path); `empty_l0_pa` = the kernel's
/// empty root, installed with ASID 0 around a rollover's full flush. The
/// kernel has set `TCR_EL1.AS` to match `limit` before.
///
/// # Safety
///
/// Boot CPU, before any user table is built ([`map_pools`]) and before any
/// root is installed.
pub unsafe fn configure_asids(limit: u32, empty_l0_pa: u64) {
    // SAFETY: boot CPU, no trap yet.
    unsafe { asid_state() }.0.configure(limit);
    EMPTY_L0_PA.store(empty_l0_pa, Ordering::Relaxed);
    ASID_ON.store(true, Ordering::Relaxed);
}

/// ASID statistics `(generation, rollovers, assigned, limit)` — boot log,
/// measurements.
pub fn asid_stats() -> (u32, u32, u64, u32) {
    // SAFETY: reads only, under the caller's single-CPU context.
    let a = unsafe { &(*ASIDS.0.get()).0 };
    (a.generation(), a.rollovers(), a.assigned(), a.limit())
}

/// The ASID `slot`'s root currently carries, if it holds one of the current
/// generation (otherwise the TLB holds nothing of its present tables).
///
/// # Safety
///
/// BDOS giant lock held.
unsafe fn current_asid(slot: usize) -> Option<u16> {
    // SAFETY: caller contract.
    unsafe { asid_state() }.0.current(slot)
}

/// Owe the invalidation of the page at `va` in the table of `owner`
/// (`None` = a shared table, every ASID). Nothing is owed for a slot without
/// a current ASID.
///
/// # Safety
///
/// BDOS giant lock held; ASIDs on.
unsafe fn owe_inval(va: u64, owner: Option<usize>) {
    use super::asid::Inval;
    // SAFETY: caller contract.
    let (alloc, queue) = unsafe { asid_state() };
    match owner {
        None => queue.push(Inval::AllAsids { va }),
        Some(s) => {
            if let Some(asid) = alloc.current(s) {
                queue.push(Inval::Asid { asid, va });
            }
        }
    }
}

/// The slot whose root `l0_pa` is, if any ([`l0_pa_for`] inverted).
fn slot_of_l0(l0_pa: u64) -> Option<usize> {
    // Compare the table-address fields only: `l0_pa` may come from a
    // `TTBR0_EL1` read with the ASID in [63:48] (and on the host the wrapping
    // `va_to_pa` carries bits above 47 that both sides then lose alike).
    let base = l0_pa_for(0) & OA_MASK;
    let size = core::mem::size_of::<SlotTables>() as u64;
    let off = (l0_pa & OA_MASK).wrapping_sub(base);
    (off.is_multiple_of(size) && off / size < MAX_PROCESSES as u64).then_some((off / size) as usize)
}

/// `slot`'s tables are rebuilt for a new tenant or torn down: its ASID is
/// retired (never matched again in this generation) and its entries are
/// dropped (`TLBI ASIDE1IS` — hygiene, the retirement alone keeps them from
/// being used). No-op with ASIDs off.
///
/// # Safety
///
/// BDOS giant lock held; EL1.
unsafe fn retire_asid(slot: usize) {
    if !asids_enabled() {
        return;
    }
    // SAFETY: caller contract.
    if let Some(asid) = unsafe { asid_state() }.0.retire(slot) {
        // SAFETY: EL1, TLB maintenance + barriers.
        unsafe { tlbi_asid(asid) };
    }
}

/// Hand the user page tables a **table arena**: `count` zeroed, 4-KiB-aligned
/// pages at kernel VA `kva` (`count` ≤ [`ARENA_MAX_TABLES`]). Protection
/// (stage 3b) draws the pools' shared L3 tables and every private L3 from
/// it. Called once at boot, before [`map_pools`].
///
/// # Safety
///
/// Boot CPU, before any trap. The pages are memory nobody else uses for as
/// long as the system runs (the kernel takes them from the frame allocator
/// before the BDOS pools claim the rest).
pub unsafe fn set_table_arena(kva: u64, count: usize) {
    ARENA_KVA.store(kva, Ordering::Relaxed);
    ARENA_COUNT.store(count.min(ARENA_MAX_TABLES), Ordering::Relaxed);
    for w in &ARENA_BITS {
        w.store(0, Ordering::Relaxed);
    }
}

/// Take one zeroed table from the arena.
fn arena_alloc() -> Option<&'static mut UserTable> {
    let base = ARENA_KVA.load(Ordering::Relaxed);
    let count = ARENA_COUNT.load(Ordering::Relaxed);
    if base == 0 {
        return None;
    }
    for (wi, w) in ARENA_BITS.iter().enumerate() {
        let bits = w.load(Ordering::Relaxed);
        if bits == u64::MAX {
            continue;
        }
        let bi = (!bits).trailing_zeros() as usize;
        let idx = wi * 64 + bi;
        if idx >= count {
            return None;
        }
        w.store(bits | (1u64 << bi), Ordering::Relaxed);
        let t = (base + (idx as u64) * PAGE_SIZE) as *mut UserTable;
        // SAFETY: `idx < count`: one of the arena's pages, now ours.
        let t = unsafe { &mut *t };
        t.0 = [0; 512];
        return Some(t);
    }
    None
}

/// Give a table back to the arena. A table that is not from the arena (a
/// static one) is never given back.
fn arena_free(t: *const UserTable) {
    let base = ARENA_KVA.load(Ordering::Relaxed);
    let count = ARENA_COUNT.load(Ordering::Relaxed) as u64;
    let a = t as u64;
    if base == 0 || a < base || a >= base + count * PAGE_SIZE {
        return;
    }
    let idx = ((a - base) / PAGE_SIZE) as usize;
    ARENA_BITS[idx / 64].fetch_and(!(1u64 << (idx % 64)), Ordering::Relaxed);
}

/// Tables of the arena still free (boot log / tests).
pub fn arena_free_count() -> usize {
    let count = ARENA_COUNT.load(Ordering::Relaxed);
    let used: usize = ARENA_BITS.iter().map(|w| w.load(Ordering::Relaxed).count_ones() as usize).sum();
    count.saturating_sub(used)
}

/// Private L3 tables in use (boot log / tests).
pub fn private_l3_used() -> usize {
    PRIVATE_USED.load(Ordering::Relaxed)
}

/// The table a valid table descriptor names, through the kernel alias. Our
/// table descriptors carry no upper attribute bits ([63:59] = 0, V0a
/// invariant 1), so everything above the low 12 bits is the address; the
/// same arithmetic undoes [`va_to_pa`] on the host (see [`translate`]).
#[inline]
fn table_at(desc: u64) -> *mut UserTable {
    (desc & !(PAGE_SIZE - 1)).wrapping_add(VA_OFFSET) as *mut UserTable
}

// ─── Public API ─────────────────────────────────────────────────────

/// Page-flag preset — selects what the page descriptor's AP / PXN /
/// UXN / AttrIdx bits resolve to. The cacheable presets share the
/// `VALID | PAGE | AF | SH_INNER | ATTRIDX_NORMAL | PXN` core and differ
/// only in the per-EL0-permission bits; the `*Uncached` presets swap that
/// core for `SH_NON | ATTRIDX_NC` (Normal Non-Cacheable) because the
/// region behind them is shared with a non-coherent DMA master.
///
/// `PXN = 1` is set on every preset because the kernel never
/// executes from user pages — even if a user maps a region readable
/// to EL0, EL1 instruction fetches from that VA fault.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PageFlags {
    /// User Read/Write, EL0 executable — what every pool page carries
    /// (plan §3.2: `AP=01`, `PXN=1`, `UXN=0`; LDG and `Pexec` load code into
    /// ordinary pool blocks). Page 0 no longer does (MEM12).
    UserRwExec,
    /// User Read-Only, EL0 executable.
    UserRoExec,
    /// User Read/Write, **not** executable.
    UserRwNx,
    /// User Read-Only, **not** executable. The AArch64 spelling of
    /// FreeMiNT's [`tos_types::ProtMode::Pr`] (`PROT_PR`, "any read
    /// OK, no write"): what
    /// [`super::usermem::expose`] uses to publish a kernel structure
    /// a program only *reads* (the `Getbpb` BPB, the `Getmpb` memory
    /// descriptor).
    UserRoNx,
    /// User Read/Write, **not** executable, **Normal Non-Cacheable**
    /// (AttrIdx 2). For memory the CPU shares with a DMA master it is not
    /// cache-coherent with — the VideoCore framebuffer, whose display
    /// engine scans DRAM out from beyond the ARM caches
    /// (`docs/hardware/mailbox-framebuffer.md` §6). An EL0 program cannot
    /// be asked to clean the D-cache after every store, so the pages it is
    /// handed by `Physbase`/`Logbase` carry the attribute that makes its
    /// writes reach DRAM unaided ([`super::usermem::MemAttr::Uncached`]).
    UserRwNxUncached,
    /// User Read-Only, **not** executable, **Normal Non-Cacheable**
    /// (AttrIdx 2). The read-only counterpart of
    /// [`Self::UserRwNxUncached`], for a DMA-shared region a program is
    /// only allowed to read.
    UserRoNxUncached,
    /// EL0 **execute-only**: EL1 read, EL0 instruction fetch, no EL0 data
    /// access (`AP=10`, `PXN=1`, `UXN=0` → `PrivRead, UnprivExecute`, ARM
    /// ARM Table D8-65, D8-7760). The shared stub page ([`STUB_PAGE_VA`],
    /// MEM12): code every process jumps to, which no program reads or writes.
    UserExecOnly,
}

impl PageFlags {
    /// The attribute bits of an L3 page descriptor with these flags.
    fn descriptor_bits(self) -> u64 {
        let ng = if asids_enabled() { D_NG } else { 0 };
        let core = D_VALID | D_PAGE_L3 | D_AF | D_SH_INNER | D_ATTRIDX_NORMAL | D_PXN | ng;
        let nc_core = D_VALID | D_PAGE_L3 | D_AF | D_SH_NON | D_ATTRIDX_NC | D_PXN | ng;
        match self {
            Self::UserRwExec => core | D_AP_USER_RW,
            Self::UserRoExec => core | D_AP_USER_RO,
            Self::UserRwNx => core | D_AP_USER_RW | D_UXN,
            Self::UserRoNx => core | D_AP_USER_RO | D_UXN,
            Self::UserRwNxUncached => nc_core | D_AP_USER_RW | D_UXN,
            Self::UserRoNxUncached => nc_core | D_AP_USER_RO | D_UXN,
            Self::UserExecOnly => core | D_AP_PRIV_RO,
        }
    }

    /// The attribute bits of an L2 **block** descriptor with these flags:
    /// the page bits with descriptor bit 1 clear (ARM ARM Figure D8-16).
    fn block_bits(self) -> u64 {
        self.descriptor_bits() & !D_PAGE_L3
    }
}

/// Errors from the user page-table builder.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MapError {
    /// Slot index ≥ [`MAX_PROCESSES`].
    InvalidSlot,
    /// VA is not 4-KiB aligned.
    UnalignedVa,
    /// Phys is not 4-KiB aligned.
    UnalignedPa,
    /// VA is outside the range the call may map: for [`map_page`] the
    /// slot-private `[0, 2 MiB)`, for [`map_shared`] `[0x1000,
    /// USER_VA_LIMIT)` without the IOREC, stub and private pages
    /// (`[IOREC_PAGE_VA, PRIV_PAGE_VA + 0x1000)`).
    VaOutOfRange,
    /// A page or block is already mapped there with different attributes.
    /// Stage 1 never changes a valid entry (that would need a TLB
    /// invalidation, plan §3.2), so the request is refused.
    Conflict,
    /// [`SHARED_L3_COUNT`] tables are all in use.
    NoTable,
}

// SAFETY (all accessors below): the statics are mutated only under the BDOS
// giant lock (single CPU, IRQs masked in the trap path / boot CPU before any
// trap), the contract every caller states.

#[allow(clippy::deref_addrof)]
unsafe fn slot_tables(slot: usize) -> &'static mut SlotTables {
    // SAFETY: caller's lock contract; `slot` bounds-checked by the caller.
    unsafe { &mut (*&raw mut __USER_TABLES)[slot] }
}

#[allow(clippy::deref_addrof)]
unsafe fn shared_l2(gib: usize) -> &'static mut UserTable {
    // SAFETY: caller's lock contract; `gib < GIB_COUNT` checked by the caller.
    unsafe { &mut (*&raw mut SHARED_L2)[gib] }
}

#[allow(clippy::deref_addrof)]
unsafe fn shared_l3_0() -> &'static mut UserTable {
    // SAFETY: caller's lock contract.
    unsafe { &mut *&raw mut SHARED_L3_0 }
}

#[inline]
fn table_pa(t: &UserTable) -> u64 {
    va_to_pa(t as *const UserTable as u64)
}

/// Kernel VA of `slot`'s page 0 — the page user VA `[0, 0x1000)` maps to in
/// that slot's address space. The loader writes the system variables, the
/// trampolines and the vector table here; the kernel reads and updates them
/// here (`$4BA` `_hz_200`, `$484` `conterm`, the `kbdvec` table, the userdef
/// scratch). `None` for an out-of-range slot.
pub fn page0_kva(slot: usize) -> Option<*mut u8> {
    if slot >= MAX_PROCESSES {
        return None;
    }
    // Raw address of a `static mut` element: no reference is formed.
    let base = (&raw mut PAGE0) as *mut Page0;
    // SAFETY: `slot < MAX_PROCESSES`, in bounds of the array.
    Some(unsafe { base.add(slot) } as *mut u8)
}

/// [`page0_kva`] as a mutable byte page.
///
/// # Safety
///
/// Caller holds the BDOS giant lock for the borrow's lifetime and does not
/// alias it.
pub unsafe fn page0_bytes(slot: usize) -> Option<&'static mut [u8; 4096]> {
    let p = page0_kva(slot)?;
    // SAFETY: `p` addresses the slot's 4-KiB page; caller's aliasing contract.
    Some(unsafe { &mut *(p as *mut [u8; 4096]) })
}

/// Kernel VA of the shared stub page ([`STUB_PAGE_VA`]).
pub fn stub_page_kva() -> *mut u8 {
    // Raw address of a `static mut`: no reference is formed.
    (&raw mut STUB_PAGE) as *mut u8
}

/// [`stub_page_kva`] as a mutable byte page.
///
/// # Safety
///
/// Caller holds the BDOS giant lock for the borrow's lifetime and does not
/// alias it.
pub unsafe fn stub_page_bytes() -> &'static mut [u8; 4096] {
    // SAFETY: the static page; caller's aliasing contract.
    unsafe { &mut *(stub_page_kva() as *mut [u8; 4096]) }
}

/// Kernel VA of `slot`'s private read-write page ([`PRIV_PAGE_VA`]).
/// `None` for an out-of-range slot.
pub fn priv_page_kva(slot: usize) -> Option<*mut u8> {
    if slot >= MAX_PROCESSES {
        return None;
    }
    let base = (&raw mut PRIV_PAGE) as *mut Page0;
    // SAFETY: `slot < MAX_PROCESSES`, in bounds of the array.
    Some(unsafe { base.add(slot) } as *mut u8)
}

/// Kernel VA of the shared IOREC page ([`IOREC_PAGE_VA`]).
pub fn iorec_page_kva() -> *mut u8 {
    // Raw address of a `static mut`: no reference is formed.
    (&raw mut IOREC_PAGE) as *mut u8
}

/// [`priv_page_kva`] as a mutable byte page.
///
/// # Safety
///
/// Caller holds the BDOS giant lock for the borrow's lifetime and does not
/// alias it.
pub unsafe fn priv_page_bytes(slot: usize) -> Option<&'static mut [u8; 4096]> {
    let p = priv_page_kva(slot)?;
    // SAFETY: `p` addresses the slot's 4-KiB page; caller's aliasing contract.
    Some(unsafe { &mut *(p as *mut [u8; 4096]) })
}

/// The kernel alias of a user VA in one of `slot`'s pages that are not
/// VA = PA — page 0, the private read-write page or the shared IOREC page —
/// or `None` for any other VA (those are VA = PA). The stub page is not
/// listed: no program may read it.
pub fn private_page_alias(slot: usize, va: u64) -> Option<u64> {
    if va < PAGE_SIZE {
        return page0_kva(slot).map(|p| p as u64 + va);
    }
    if (PRIV_PAGE_VA..PRIV_PAGE_VA + PAGE_SIZE).contains(&va) {
        return priv_page_kva(slot).map(|p| p as u64 + (va - PRIV_PAGE_VA));
    }
    if (IOREC_PAGE_VA..IOREC_PAGE_VA + PAGE_SIZE).contains(&va) {
        return Some(iorec_page_kva() as u64 + (va - IOREC_PAGE_VA));
    }
    None
}

/// Is `va` in one of the special pages of the slot-private `[0, 2 MiB)`
/// (page 0, the IOREC page, the stub page, the private read-write page)?
/// Such a page is not contiguous with what lies around it, so a user range
/// may not run across its end.
pub fn in_private_page(va: u64) -> bool {
    va < PAGE_SIZE || (SPECIAL_PAGES_VA..PRIV_PAGE_VA + PAGE_SIZE).contains(&va)
}

/// Make page 0 of `slot` EL0-writable (`on`, the **super state**) or
/// EL0-read-only (user state) — `docs/api/_super-state.md` S2. Never
/// executable. Only the slot's own L3 entry 0 changes.
///
/// TLB maintenance: this changes a **valid** leaf in both directions, and
/// "when a translation table entry associated with a specific VMID or ASID is
/// modified, software is required to invalidate the corresponding TLB entry"
/// (ARM ARM DDI 0487 M.d `RGPPYH`, D8-7858); a read-only entry that faulted a
/// write may itself be cached (`RJVXRH`/`IWJLXV`, D8-7841), so the widening
/// RO → RW needs it as much as the narrowing. [`set_leaf`] records the change
/// and [`flush_tlb`] issues `dsb ish` (the table write visible to the walker
/// before the invalidation), `tlbi vmalle1is` — or with ASIDs on `tlbi
/// vae1is` for VA 0 with the slot's ASID —, `dsb ish` and `isb`: the TLBI is
/// complete after the DSB followed by a context synchronisation event
/// (`IBLDZX`, D8-7875).
///
/// # Safety
///
/// Caller holds the BDOS giant lock; EL1; [`init_slot_tables`] ran for `slot`.
pub unsafe fn set_page0_writable(slot: usize, on: bool) {
    if slot >= MAX_PROCESSES {
        return;
    }
    let Some(p0) = page0_kva(slot) else { return };
    let flags = if on { PageFlags::UserRwNx } else { PageFlags::UserRoNx };
    let desc = va_to_pa(p0 as u64) | flags.descriptor_bits();
    // SAFETY: caller's lock contract; the slot's own L3 (entry 0 = page 0).
    unsafe {
        let t = &raw mut slot_tables(slot).l3;
        set_leaf(t, 0, desc, 0, Some(slot));
        flush_tlb();
    }
}

/// Build `slot`'s root: `L0[0] → L1`, `L1[g]` → the slot's own L2 of GiB `g`
/// (a copy of the shared template), `L2₀[0] → L3`, `L3[0]` = the slot's page
/// 0 (`UserRoNx` — user state; [`set_page0_writable`] widens it in super
/// state), `L3[1..]` = the `[0, 2 MiB)` template, except the stub page
/// (`UserExecOnly`) and the slot's private read-write page (`UserRwNx`) —
/// MEM12, `docs/api/_super-state.md` — and the shared IOREC page
/// (`UserRwNx`, XBIOS01). Private L3 tables
/// a previous tenant of the slot held go back to the arena first.
/// Idempotent; the page-0 *contents* are not touched (the loader writes them).
///
/// # Safety
///
/// Caller holds the BDOS giant lock (or is the boot CPU pre-trap).
/// `slot` must not currently be the active TTBR0 — if it is, the
/// caller must invalidate the TLB before resuming user execution.
pub unsafe fn init_slot_tables(slot: usize) -> Result<(), MapError> {
    if slot >= MAX_PROCESSES {
        return Err(MapError::InvalidSlot);
    }
    let page0_pa = va_to_pa(page0_kva(slot).ok_or(MapError::InvalidSlot)? as u64);
    // Stage 4: the new tenant must not meet the old one's TLB entries — its
    // root gets a fresh ASID at the next install.
    // SAFETY: caller's lock contract; slot bounds-checked.
    unsafe { retire_asid(slot) };
    // SAFETY: caller's lock contract; slot bounds-checked.
    unsafe { release_private_tables(slot) };
    // SAFETY: caller's lock contract; slot bounds-checked.
    let t = unsafe { slot_tables(slot) };

    t.l0.0 = [0; 512];
    t.l0.0[0] = table_pa(&t.l1) | D_VALID | D_TABLE;

    t.l1.0 = [0; 512];
    for gib in 0..GIB_COUNT {
        t.l1.0[gib] = table_pa(&t.l2[gib]) | D_VALID | D_TABLE;
        // SAFETY: caller's lock contract; `gib < GIB_COUNT`.
        t.l2[gib].0 = unsafe { shared_l2(gib) }.0;
    }
    t.l2[0].0[0] = table_pa(&t.l3) | D_VALID | D_TABLE;

    // SAFETY: caller's lock contract.
    t.l3.0 = unsafe { shared_l3_0() }.0;
    t.l3.0[0] = page0_pa | PageFlags::UserRoNx.descriptor_bits();
    t.l3.0[STUB_PAGE_IDX] = va_to_pa(stub_page_kva() as u64) | PageFlags::UserExecOnly.descriptor_bits();
    let priv_pa = va_to_pa(priv_page_kva(slot).ok_or(MapError::InvalidSlot)? as u64);
    t.l3.0[PRIV_PAGE_IDX] = priv_pa | PageFlags::UserRwNx.descriptor_bits();
    t.l3.0[IOREC_PAGE_IDX] = va_to_pa(iorec_page_kva() as u64) | PageFlags::UserRwNx.descriptor_bits();
    Ok(())
}

/// Give every private L3 `slot` holds back to the arena and point its L2
/// entries at the shared tables again. No TLB maintenance: the slot is not
/// the active root ([`init_slot_tables`]'s contract) — or, from
/// [`super::memprot`], the caller flushes.
///
/// # Safety
///
/// Caller holds the BDOS giant lock; `slot < MAX_PROCESSES`.
pub unsafe fn release_private_tables(slot: usize) {
    // SAFETY: caller contract.
    let t = unsafe { slot_tables(slot) };
    for gib in 0..GIB_COUNT {
        for idx in 0..512 {
            if gib == 0 && idx == 0 {
                continue; // the slot's page-0 L3
            }
            // SAFETY: caller contract.
            let tmpl = unsafe { shared_l2(gib) }.0[idx];
            let cur = t.l2[gib].0[idx];
            if cur != tmpl && cur & (D_VALID | D_TABLE) == D_VALID | D_TABLE {
                t.l2[gib].0[idx] = tmpl;
                arena_free(table_at(cur));
                PRIVATE_USED.fetch_sub(1, Ordering::Relaxed);
            }
        }
    }
}

/// Install one 4-KiB page in `slot`'s **private** `[0, 2 MiB)` L3 — page 0
/// by default; the EC4 self-test maps its single code page at VA 0 this way.
/// Shared memory goes through [`map_shared`] instead.
///
/// # Safety
///
/// Caller holds the BDOS giant lock; [`init_slot_tables`] ran for the slot.
pub unsafe fn map_page(
    slot: usize,
    va: u64,
    pa: u64,
    flags: PageFlags,
) -> Result<(), MapError> {
    if slot >= MAX_PROCESSES {
        return Err(MapError::InvalidSlot);
    }
    if !va.is_multiple_of(PAGE_SIZE) {
        return Err(MapError::UnalignedVa);
    }
    if !pa.is_multiple_of(PAGE_SIZE) {
        return Err(MapError::UnalignedPa);
    }
    if va >= L3_SPAN {
        return Err(MapError::VaOutOfRange);
    }
    // SAFETY: caller contract.
    let t = unsafe { slot_tables(slot) };
    t.l3.0[(va / PAGE_SIZE) as usize] = (pa & OA_MASK) | flags.descriptor_bits();
    Ok(())
}

/// Write entry `idx` of the GiB-`gib` L2 template and of every slot's copy
/// that still follows the template there (a slot with a private L3 at `idx`
/// keeps it — stage 3b).
///
/// # Safety
///
/// Caller's lock contract; `gib < GIB_COUNT`, `idx < 512`, and `idx != 0`
/// for GiB 0 (that entry is each slot's own).
unsafe fn set_l2(gib: usize, idx: usize, desc: u64) {
    // SAFETY: caller contract.
    unsafe {
        let old = shared_l2(gib).0[idx];
        shared_l2(gib).0[idx] = desc;
        for slot in 0..MAX_PROCESSES {
            let e = &mut slot_tables(slot).l2[gib].0[idx];
            if *e == old {
                *e = desc;
            }
        }
    }
}

/// Write entry `idx` (≥ 1) of the `[0, 2 MiB)` L3: template + every slot.
///
/// # Safety
///
/// Caller's lock contract; `1 <= idx < 512`.
unsafe fn set_l3_0(idx: usize, desc: u64) {
    // SAFETY: caller contract.
    unsafe {
        shared_l3_0().0[idx] = desc;
        for slot in 0..MAX_PROCESSES {
            slot_tables(slot).l3.0[idx] = desc;
        }
    }
}

/// The L3 table covering the 2-MiB region that starts at `region` (≥ 2 MiB),
/// creating it — from the arena if there is one ([`set_table_arena`]),
/// else from [`SHARED_L3`] — if the L2 entry is still invalid.
///
/// # Safety
///
/// Caller's lock contract; `L3_SPAN <= region < USER_VA_LIMIT`, aligned.
#[allow(clippy::deref_addrof)]
unsafe fn shared_l3_for(region: u64) -> Result<&'static mut UserTable, MapError> {
    let gib = (region / L2_SPAN) as usize;
    let idx = ((region % L2_SPAN) / L3_SPAN) as usize;
    // SAFETY: caller contract.
    let cur = unsafe { shared_l2(gib).0[idx] };
    if cur & D_VALID != 0 {
        if cur & D_TABLE == 0 {
            // A block already maps the whole region; a page-level request in
            // it would have to split the block (break-before-make, plan §3.2).
            return Err(MapError::Conflict);
        }
        // SAFETY: a valid table descriptor in our template names one of our
        // tables, reachable through the kernel alias.
        return Ok(unsafe { &mut *table_at(cur) });
    }
    let t = match arena_alloc() {
        Some(t) => t,
        None => {
            let n = SHARED_L3_USED.load(Ordering::Relaxed);
            if n >= SHARED_L3_COUNT {
                return Err(MapError::NoTable);
            }
            SHARED_L3_USED.store(n + 1, Ordering::Relaxed);
            // SAFETY: caller's lock contract; `n` was free.
            let t = unsafe { &mut (*&raw mut SHARED_L3)[n] };
            t.0 = [0; 512];
            t
        }
    };
    SHARED_L3_TOTAL.fetch_add(1, Ordering::Relaxed);
    // SAFETY: caller contract; GiB-0 index 0 is excluded because `region` ≥ 2 MiB.
    unsafe { set_l2(gib, idx, table_pa(t) | D_VALID | D_TABLE) };
    Ok(t)
}

/// Enter `[va, va + len)` into the **shared** user address space at
/// VA = PA, with `flags`. Whole, aligned 2-MiB regions become blocks when
/// `allow_blocks` is set; the rest is 4-KiB pages. Entries that already exist
/// with the same attributes are kept; a different existing mapping is a
/// [`MapError::Conflict`] (stage 1 never rewrites a valid entry). Page 0 is
/// never touched.
///
/// Visible to every process at once — present and future slots — because
/// every slot's root shares these tables.
///
/// # Safety
///
/// Caller holds the BDOS giant lock. `[va, va + len)` must be memory that may
/// be handed to user programs (a BDOS pool, a published kernel page, the
/// screen memory).
pub unsafe fn map_shared(va: u64, len: u64, flags: PageFlags, allow_blocks: bool) -> Result<(), MapError> {
    if !va.is_multiple_of(PAGE_SIZE) {
        return Err(MapError::UnalignedVa);
    }
    let end = va
        .checked_add(len)
        .map(|e| (e + PAGE_SIZE - 1) & !(PAGE_SIZE - 1))
        .ok_or(MapError::VaOutOfRange)?;
    if va < PAGE_SIZE || end > USER_VA_LIMIT {
        return Err(MapError::VaOutOfRange);
    }
    // The IOREC, stub and private read-write pages are each slot's own
    // entries (MEM12, XBIOS01); a VA = PA publication there would shadow them.
    if va < PRIV_PAGE_VA + PAGE_SIZE && end > SPECIAL_PAGES_VA {
        return Err(MapError::VaOutOfRange);
    }
    let page_bits = flags.descriptor_bits();
    let block_bits = flags.block_bits();
    let mut a = va;
    while a < end {
        let region = a & !(L3_SPAN - 1);
        let region_end = region + L3_SPAN;
        if allow_blocks && a == region && end >= region_end && region >= L3_SPAN {
            let gib = (region / L2_SPAN) as usize;
            let idx = ((region % L2_SPAN) / L3_SPAN) as usize;
            let want = region | block_bits;
            // SAFETY: caller's lock contract; indices in range.
            let cur = unsafe { shared_l2(gib).0[idx] };
            if cur & D_VALID == 0 {
                // SAFETY: as above; GiB-0 entry 0 excluded (region ≥ 2 MiB).
                unsafe { set_l2(gib, idx, want) };
                a = region_end;
                continue;
            }
            if cur == want {
                a = region_end;
                continue;
            }
            if cur & D_TABLE == 0 {
                return Err(MapError::Conflict); // a block with other attributes
            }
            // Already an L3 table (part of the region was mapped page by
            // page before): fill in the pages below.
        }
        let stop = end.min(region_end);
        if region == 0 {
            // `[0x1000, 2 MiB)`: the template every slot copies (page 0 is
            // excluded by the `va >= PAGE_SIZE` check above).
            while a < stop {
                let idx = (a / PAGE_SIZE) as usize;
                let want = a | page_bits;
                // SAFETY: caller's lock contract.
                let cur = unsafe { shared_l3_0().0[idx] };
                if cur & D_VALID == 0 {
                    // SAFETY: `1 <= idx < 512`.
                    unsafe { set_l3_0(idx, want) };
                } else if cur != want {
                    return Err(MapError::Conflict);
                }
                a += PAGE_SIZE;
            }
            continue;
        }
        // SAFETY: caller's lock contract; `region` aligned, ≥ 2 MiB, in range.
        let t = unsafe { shared_l3_for(region)? };
        let first = a;
        while a < stop {
            let idx = ((a - region) / PAGE_SIZE) as usize;
            let want = a | page_bits;
            let cur = t.0[idx];
            if cur & D_VALID == 0 {
                t.0[idx] = want;
            } else if cur != want {
                return Err(MapError::Conflict);
            }
            a += PAGE_SIZE;
        }
        // A slot with a private copy of this region (stage 3b) must see the
        // publication as well: it is the same for everybody.
        let lo = ((first - region) / PAGE_SIZE) as usize;
        let hi = ((stop - region) / PAGE_SIZE) as usize;
        for slot in 0..MAX_PROCESSES {
            // SAFETY: caller's lock contract.
            if let Some(p) = unsafe { slot_private_l3(slot, region) } {
                // SAFETY: a private table of ours; only invalid entries of
                // the shared set were just filled, so the copy follows.
                unsafe { (&mut (*p).0)[lo..hi].copy_from_slice(&t.0[lo..hi]) };
            }
        }
    }
    // SAFETY: barrier-only.
    unsafe { publish_user_mappings() };
    Ok(())
}

/// Enter both BDOS pools into the shared address space at VA = PA,
/// `UserRwExec` (plan §3.2: pool entries `AP=01`, `PXN=1`, `UXN=0`), 2-MiB
/// blocks where possible. Called once at boot, right after the pools are
/// seeded (`kernel::bring_up_bdos_pools`).
///
/// # Safety
///
/// Boot CPU, before any trap; the pools are initialised.
pub unsafe fn map_pools() -> Result<(), MapError> {
    let protect = super::memprot::enabled();
    for (start, end) in [
        crate::bdos::mem::init::stram_range(),
        crate::bdos::mem::init::ttram_range(),
    ] {
        if end <= start {
            continue;
        }
        if protect {
            // Stage 3b: one L3 per 2-MiB region the pool touches, every pool
            // entry invalid — nothing is allocated yet, and free memory is
            // nobody's (I8). Rights are entered page by page as blocks are
            // handed out ([`super::memprot`]).
            let mut region = (start as u64) & !(L3_SPAN - 1);
            while region < end as u64 {
                if !(L3_SPAN..USER_VA_LIMIT).contains(&region) {
                    return Err(MapError::VaOutOfRange);
                }
                // SAFETY: boot CPU; in range and aligned.
                unsafe { shared_l3_for(region)? };
                region += L3_SPAN;
            }
        } else {
            // SAFETY: boot CPU; the pool is user memory by definition.
            unsafe {
                map_shared(start as u64, (end - start) as u64, PageFlags::UserRwExec, true)?;
            }
        }
    }
    // SAFETY: barrier-only.
    unsafe { publish_user_mappings() };
    Ok(())
}

// ─── Per-slot rights (stage 3b) ──────────────────────────────────────

/// Span of one L3 table / 2-MiB region — the granule of [`make_private`].
pub const REGION_SPAN: u64 = L3_SPAN;

/// The shared L3 of the 2-MiB region at `region`, or `None` if the region
/// has none (not user memory, or mapped by a block).
///
/// # Safety
///
/// Caller holds the BDOS giant lock; `region` is 2-MiB aligned.
pub unsafe fn shared_l3(region: u64) -> Option<*mut UserTable> {
    if !(L3_SPAN..USER_VA_LIMIT).contains(&region) {
        return None;
    }
    let gib = (region / L2_SPAN) as usize;
    let idx = ((region % L2_SPAN) / L3_SPAN) as usize;
    // SAFETY: caller contract.
    let cur = unsafe { shared_l2(gib).0[idx] };
    (cur & (D_VALID | D_TABLE) == D_VALID | D_TABLE).then(|| table_at(cur))
}

/// `slot`'s private L3 of the region at `region`, if it has one.
///
/// # Safety
///
/// Caller holds the BDOS giant lock; `region` is 2-MiB aligned.
pub unsafe fn slot_private_l3(slot: usize, region: u64) -> Option<*mut UserTable> {
    if slot >= MAX_PROCESSES || !(L3_SPAN..USER_VA_LIMIT).contains(&region) {
        return None;
    }
    let gib = (region / L2_SPAN) as usize;
    let idx = ((region % L2_SPAN) / L3_SPAN) as usize;
    // SAFETY: caller contract.
    let (tmpl, cur) = unsafe { (shared_l2(gib).0[idx], slot_tables(slot).l2[gib].0[idx]) };
    (cur != tmpl && cur & (D_VALID | D_TABLE) == D_VALID | D_TABLE).then(|| table_at(cur))
}

/// The L2 entry `slot` uses for `region`, through which an entry change is
/// made break-before-make.
///
/// # Safety
///
/// Caller holds the BDOS giant lock; `slot`/`region` in range.
unsafe fn slot_l2_entry(slot: usize, region: u64) -> *mut u64 {
    let gib = (region / L2_SPAN) as usize;
    let idx = ((region % L2_SPAN) / L3_SPAN) as usize;
    // SAFETY: caller contract.
    unsafe { &raw mut slot_tables(slot).l2[gib].0[idx] }
}

/// Replace a valid table descriptor with another **break-before-make**
/// (V0a C3/C4; ARM ARM `RDDMVT` D8-7859; a table-attribute change is BBM by
/// `IYQWJF` D8-7757): write invalid, `dsb ish`, `tlbi vmalle1is`, `dsb ish`,
/// write the new descriptor, `dsb ish`, `isb`.
///
/// With ASIDs on (stage 4) the entry is `slot`'s own, so only `slot`'s
/// translations can hold it: the invalidation is `TLBI VAE1IS` for the
/// region's address with the slot's ASID — non-global entries **of any
/// lookup level** for that address (`RVMSWQ`, D8-7868), which includes the
/// cached L2 descriptor that covers the whole region; not a `…L` variant
/// (`RDRCZZ`). A slot without a current ASID has nothing in the TLB.
///
/// # Safety
///
/// Caller holds the BDOS giant lock; `entry` is `slot`'s L2 entry for the
/// 2-MiB region at `region`.
unsafe fn replace_bbm(entry: *mut u64, new: u64, slot: usize, region: u64) {
    if asids_enabled() {
        // SAFETY: caller contract — a live entry of `slot`'s table.
        unsafe {
            core::ptr::write_volatile(entry, 0);
            if let Some(asid) = current_asid(slot) {
                tlbi_va_asid_barriered(super::asid::Inval::Asid { asid, va: region }.operand());
            } else {
                publish_user_mappings();
            }
            core::ptr::write_volatile(entry, new);
            publish_user_mappings();
        }
        return;
    }
    // SAFETY: caller contract — a live entry of a user table.
    unsafe {
        core::ptr::write_volatile(entry, 0);
        tlbi_all_barriered();
        core::ptr::write_volatile(entry, new);
        publish_user_mappings();
    }
}

/// Give `slot` its own copy of the shared L3 of `region` (copy-on-differ,
/// plan §3.2) and return it; an existing private copy is returned as is.
/// The caller then writes the slot's own rights into it.
///
/// # Errors
///
/// [`MapError::NoTable`] — the arena is exhausted; [`MapError::VaOutOfRange`]
/// — the region has no shared L3.
///
/// # Safety
///
/// Caller holds the BDOS giant lock; `region` 2-MiB aligned.
pub unsafe fn make_private(slot: usize, region: u64) -> Result<*mut UserTable, MapError> {
    if slot >= MAX_PROCESSES {
        return Err(MapError::InvalidSlot);
    }
    // SAFETY: caller contract.
    if let Some(p) = unsafe { slot_private_l3(slot, region) } {
        return Ok(p);
    }
    // SAFETY: caller contract.
    let shared = unsafe { shared_l3(region) }.ok_or(MapError::VaOutOfRange)?;
    let t = arena_alloc().ok_or(MapError::NoTable)?;
    // SAFETY: `shared` is a live table of ours.
    t.0 = unsafe { (*shared).0 };
    PRIVATE_USED.fetch_add(1, Ordering::Relaxed);
    let new = table_pa(t) | D_VALID | D_TABLE;
    // SAFETY: caller contract; the slot's own L2 entry.
    unsafe {
        let e = slot_l2_entry(slot, region);
        // The copy must be visible to the walker before the entry names it.
        core::arch::asm!("dsb ish", options(nostack, preserves_flags));
        if core::ptr::read_volatile(e) & D_VALID != 0 {
            replace_bbm(e, new, slot, region);
        } else {
            core::ptr::write_volatile(e, new);
            publish_user_mappings();
        }
    }
    Ok(t as *mut UserTable)
}

/// Point `slot` back at the shared L3 of `region` and free its private copy
/// (break-before-make). Nothing to do if it has none.
///
/// # Safety
///
/// Caller holds the BDOS giant lock; `region` 2-MiB aligned.
pub unsafe fn drop_private(slot: usize, region: u64) {
    // SAFETY: caller contract.
    let Some(p) = (unsafe { slot_private_l3(slot, region) }) else {
        return;
    };
    let gib = (region / L2_SPAN) as usize;
    let idx = ((region % L2_SPAN) / L3_SPAN) as usize;
    // SAFETY: caller contract.
    unsafe {
        let tmpl = shared_l2(gib).0[idx];
        if asids_enabled() && current_asid(slot).is_some() {
            // Stage 4: the leaves the slot cached from its private copy stay
            // tagged with its ASID after the region points at the shared
            // table again; owe their invalidation where the two differ
            // (the BBM below only drops the region's table descriptor).
            let shared = (tmpl & (D_VALID | D_TABLE) == D_VALID | D_TABLE).then(|| table_at(tmpl));
            for i in 0..512 {
                let mine = core::ptr::read_volatile(&raw const (*p).0[i]);
                let theirs = shared.map_or(0, |t| core::ptr::read_volatile(&raw const (*t).0[i]));
                if mine & D_VALID != 0 && mine != theirs {
                    owe_inval(region + (i as u64) * PAGE_SIZE, Some(slot));
                }
            }
        }
        replace_bbm(slot_l2_entry(slot, region), tmpl, slot, region);
    }
    arena_free(p);
    PRIVATE_USED.fetch_sub(1, Ordering::Relaxed);
}

/// Write leaf entry `idx` of `table` — the page at user VA `va` — and
/// remember whether a **valid** entry changed (then the TLB may hold the old
/// one, [`flush_tlb`]). `owner` names the slot whose private table it is;
/// `None` = a shared table, walked by every root without a private copy of
/// the region. With ASIDs off any such change makes the next [`flush_tlb`]
/// drop the whole TLB; with ASIDs on (stage 4) it owes the invalidation of
/// just that page — for every ASID (shared) or for the owner's ASID.
///
/// # Safety
///
/// Caller holds the BDOS giant lock; `table` is a live user L3 (`owner`'s
/// private one, or a shared one for `None`); `idx < 512`.
pub unsafe fn set_leaf(table: *mut UserTable, idx: usize, desc: u64, va: u64, owner: Option<usize>) {
    // SAFETY: caller contract.
    unsafe {
        let e = &raw mut (*table).0[idx];
        let old = core::ptr::read_volatile(e);
        if old == desc {
            return;
        }
        if old & D_VALID != 0 {
            if asids_enabled() {
                owe_inval(va, owner);
            } else {
                TLB_DIRTY.store(true, Ordering::Relaxed);
            }
        }
        core::ptr::write_volatile(e, desc);
    }
}

/// Read leaf entry `idx` of `table`.
///
/// # Safety
///
/// As [`set_leaf`].
pub unsafe fn leaf(table: *const UserTable, idx: usize) -> u64 {
    // SAFETY: caller contract.
    unsafe { core::ptr::read_volatile(&raw const (*table).0[idx]) }
}

/// The leaf descriptor of the 4-KiB user page at `va` (VA = PA) with
/// `flags`; `0` (invalid) for `None`.
pub fn leaf_desc(va: u64, flags: Option<PageFlags>) -> u64 {
    match flags {
        Some(f) => (va & OA_MASK) | f.descriptor_bits(),
        None => 0,
    }
}

/// Make all table writes visible and, if a valid leaf entry changed since
/// the last call, drop every stage-1 EL1&0 translation:
/// `dsb ish; tlbi vmalle1is; dsb ish; isb` (ARM ARM `RSQBCS` D8-7852 — a
/// narrowing or a valid→valid widening needs the TLBI; an invalid→valid
/// change needs only the barrier, `RXCLRD`). The trailing `isb` (or the
/// `eret` that follows) is the context synchronisation. With ASIDs off all
/// processes run with ASID 0 and every root switch invalidates the whole TLB
/// already ([`crate::bdos::process::mmu::switch_ttbr0_kernel`], the kernel's
/// `restore_boot_ttbr0`), so `vmalle1is` covers the active root and nothing
/// else can be cached.
///
/// With ASIDs on (stage 4) other roots' entries survive a switch, so the
/// owed invalidations are issued by VA: `TLBI VAAE1IS` for a shared table's
/// page, `TLBI VAE1IS` with the owner's ASID for a private one (any lookup
/// level, `RZSXRQ`/`RVMSWQ` D8-7868/7869), then `dsb ish; isb`. More than
/// [`super::asid::QUEUE_CAP`] owed → `vmalle1is`.
///
/// # Safety
///
/// Caller is at EL1 (barrier + TLB maintenance only).
pub unsafe fn flush_tlb() {
    if asids_enabled() {
        // SAFETY: the caller's lock contract (BDOS, single CPU).
        let (ops, overflow) = unsafe { asid_state() }.1.take();
        if overflow {
            // SAFETY: caller contract.
            unsafe { tlbi_all_barriered() };
        } else if ops.as_slice().is_empty() {
            // SAFETY: barrier-only.
            unsafe { publish_user_mappings() };
        } else {
            // SAFETY: caller contract.
            unsafe { tlbi_ops_barriered(ops.as_slice()) };
        }
        return;
    }
    if TLB_DIRTY.swap(false, Ordering::Relaxed) {
        // SAFETY: caller contract.
        unsafe { tlbi_all_barriered() };
    } else {
        // SAFETY: barrier-only.
        unsafe { publish_user_mappings() };
    }
}

/// [`flush_tlb`], invalidating unconditionally (a root was torn down).
///
/// # Safety
///
/// EL1.
pub unsafe fn flush_tlb_always() {
    TLB_DIRTY.store(true, Ordering::Relaxed);
    // SAFETY: caller contract.
    unsafe { flush_tlb() };
}

/// `slot`'s root was torn down (its private tables went back to the arena,
/// [`release_private_tables`]): drop what the TLB holds of it. ASIDs off: the
/// whole TLB ([`flush_tlb_always`], as before). ASIDs on: the slot's ASID is
/// retired — no root carries it again in this generation — and its entries
/// dropped (`TLBI ASIDE1IS`), then the owed invalidations are issued.
///
/// # Safety
///
/// Caller holds the BDOS giant lock; EL1.
pub unsafe fn flush_tlb_slot_released(slot: usize) {
    if !asids_enabled() {
        // SAFETY: caller contract.
        unsafe { flush_tlb_always() };
        return;
    }
    // SAFETY: caller contract.
    unsafe {
        retire_asid(slot);
        flush_tlb();
    }
}

/// `dsb ish; tlbi vmalle1is; dsb ish; isb` (on the host: nothing).
///
/// # Safety
///
/// EL1.
unsafe fn tlbi_all_barriered() {
    #[cfg(all(target_arch = "aarch64", target_os = "none"))]
    // SAFETY: TLB maintenance + barriers are permitted at EL1.
    unsafe {
        core::arch::asm!(
            "dsb ish",
            "tlbi vmalle1is",
            "dsb ish",
            "isb",
            options(nostack, preserves_flags),
        );
    }
}

/// `dsb ish; tlbi vae1is|vaae1is …; dsb ish; isb` for the owed
/// invalidations (on the host: nothing).
///
/// # Safety
///
/// EL1.
unsafe fn tlbi_ops_barriered(ops: &[super::asid::Inval]) {
    #[cfg(all(target_arch = "aarch64", target_os = "none"))]
    // SAFETY: TLB maintenance + barriers are permitted at EL1.
    unsafe {
        core::arch::asm!("dsb ish", options(nostack, preserves_flags));
        for op in ops {
            let xt = op.operand();
            match op {
                super::asid::Inval::AllAsids { .. } => {
                    core::arch::asm!("tlbi vaae1is, {}", in(reg) xt, options(nostack, preserves_flags));
                }
                super::asid::Inval::Asid { .. } => {
                    core::arch::asm!("tlbi vae1is, {}", in(reg) xt, options(nostack, preserves_flags));
                }
            }
        }
        core::arch::asm!("dsb ish", "isb", options(nostack, preserves_flags));
    }
    #[cfg(not(all(target_arch = "aarch64", target_os = "none")))]
    let _ = ops;
}

/// `dsb ish; tlbi vae1is, xt; dsb ish; isb` (on the host: nothing).
///
/// # Safety
///
/// EL1.
unsafe fn tlbi_va_asid_barriered(xt: u64) {
    #[cfg(all(target_arch = "aarch64", target_os = "none"))]
    // SAFETY: TLB maintenance + barriers are permitted at EL1.
    unsafe {
        core::arch::asm!(
            "dsb ish",
            "tlbi vae1is, {xt}",
            "dsb ish",
            "isb",
            xt = in(reg) xt,
            options(nostack, preserves_flags),
        );
    }
    #[cfg(not(all(target_arch = "aarch64", target_os = "none")))]
    let _ = xt;
}

/// `dsb ish; tlbi aside1is, xt; dsb ish; isb` — the non-global final-level
/// entries of `asid` (`RFKSVM`, D8-7868) (on the host: nothing).
///
/// # Safety
///
/// EL1.
unsafe fn tlbi_asid(asid: u16) {
    #[cfg(all(target_arch = "aarch64", target_os = "none"))]
    // SAFETY: TLB maintenance + barriers are permitted at EL1.
    unsafe {
        core::arch::asm!(
            "dsb ish",
            "tlbi aside1is, {xt}",
            "dsb ish",
            "isb",
            xt = in(reg) (asid as u64) << 48,
            options(nostack, preserves_flags),
        );
    }
    #[cfg(not(all(target_arch = "aarch64", target_os = "none")))]
    let _ = asid;
}

/// Write `TTBR0_EL1` = `l0_pa` tagged with the ASID of the slot it belongs to
/// (ASID 0 for any other root, e.g. the empty one) — stage 4's switch: no TLB
/// invalidation, because the kernel's own (TTBR1) leaves are global and every
/// other root's entries carry another ASID (ARM ARM Example D8-1, D8-7700;
/// V0a B8). `dsb ish` first so the walker sees the table writes, `isb` after
/// (context synchronisation). If the slot needs a new ASID and the numbers
/// ran out (a new generation), the whole TLB is dropped first — with the
/// empty root and ASID 0 in `TTBR0_EL1`, so that no entry of a stale-generation
/// ASID can be walked in again between the flush and the switch.
///
/// # Safety
///
/// EL1, BDOS giant lock held; ASIDs on; `l0_pa` a fully built root.
unsafe fn install_tagged(l0_pa: u64) {
    let asid = match slot_of_l0(l0_pa) {
        Some(slot) => {
            // SAFETY: caller contract.
            let (asid, rolled) = unsafe { asid_state() }.0.assign(slot);
            if rolled {
                // SAFETY: EL1; the empty root maps nothing.
                unsafe { rollover_flush() };
            }
            asid
        }
        None => super::asid::NO_ASID,
    };
    let ttbr0 = (l0_pa & OA_MASK) | ((asid as u64) << 48);
    #[cfg(all(target_arch = "aarch64", target_os = "none"))]
    // SAFETY: writing TTBR0_EL1 is permitted at EL1 (caller contract).
    unsafe {
        core::arch::asm!(
            "dsb ish",
            "msr ttbr0_el1, {ttbr0}",
            "isb",
            ttbr0 = in(reg) ttbr0,
            options(nostack, preserves_flags),
        );
    }
    #[cfg(not(all(target_arch = "aarch64", target_os = "none")))]
    let _ = ttbr0;
}

/// A new ASID generation: park `TTBR0_EL1` on the empty root with ASID 0,
/// then `tlbi vmalle1is` (every ASID, every level — `RPMNQC` D8-7867/7868; not
/// `ASIDE1`, V0a C6), `dsb ish`, `isb`. The owed per-VA invalidations are
/// covered by it and dropped.
///
/// # Safety
///
/// EL1, BDOS giant lock held.
unsafe fn rollover_flush() {
    let empty = EMPTY_L0_PA.load(Ordering::Relaxed);
    // SAFETY: caller contract; the queue is ours under the lock.
    let _ = unsafe { asid_state() }.1.take();
    #[cfg(all(target_arch = "aarch64", target_os = "none"))]
    // SAFETY: writing TTBR0_EL1 + TLB maintenance are permitted at EL1.
    unsafe {
        core::arch::asm!(
            "dsb ish",
            "msr ttbr0_el1, {empty}",
            "isb",
            "tlbi vmalle1is",
            "dsb ish",
            "isb",
            empty = in(reg) empty,
            options(nostack, preserves_flags),
        );
    }
    #[cfg(not(all(target_arch = "aarch64", target_os = "none")))]
    let _ = empty;
    TLB_DIRTY.store(false, Ordering::Relaxed);
}

/// Walk `slot`'s tables in software and return the output address and the
/// leaf descriptor that maps user VA `va`, or `None` if `va` is not mapped
/// there. What the MMU would do for the process — for the boot log, the
/// self-tests and the host tests of the shared-space invariants (plan §3.1
/// I1/I3). Reads only.
pub fn translate(slot: usize, va: u64) -> Option<(u64, u64)> {
    if slot >= MAX_PROCESSES || va >= USER_VA_LIMIT {
        return None;
    }
    // A table descriptor's output address is a physical address; the tables
    // are kernel statics reachable at `pa + VA_OFFSET`. Our table descriptors
    // carry no upper attribute bits ([63:59] = 0, plan §3.2 invariant 1), so
    // everything above the low 12 bits is the address — which also lets the
    // same arithmetic undo `va_to_pa` on the host, where a "physical" table
    // address does not fit the 48-bit field.
    let table = |desc: u64| (desc & !(PAGE_SIZE - 1)).wrapping_add(VA_OFFSET) as *const u64;
    // SAFETY: read-only walk of the static tables under the BDOS lock (the
    // caller's single-CPU context); every pointer followed is a table this
    // module wrote.
    unsafe {
        // `l0` is the first field of the `#[repr(C)]` `SlotTables`.
        let l0 = ((&raw const __USER_TABLES) as *const SlotTables).add(slot) as *const u64;
        let d0 = l0.read();
        if d0 & D_VALID == 0 {
            return None;
        }
        let d1 = table(d0).add(((va >> 30) & 0x1FF) as usize).read();
        if d1 & D_VALID == 0 {
            return None;
        }
        let d2 = table(d1).add(((va >> 21) & 0x1FF) as usize).read();
        if d2 & D_VALID == 0 {
            return None;
        }
        if d2 & D_TABLE == 0 {
            return Some(((d2 & OA_MASK & !(L3_SPAN - 1)) | (va & (L3_SPAN - 1)), d2));
        }
        let d3 = table(d2).add(((va >> 12) & 0x1FF) as usize).read();
        if d3 & D_VALID == 0 {
            return None;
        }
        Some(((d3 & OA_MASK) | (va & (PAGE_SIZE - 1)), d3))
    }
}

/// The descriptor attribute bits `flags` maps a 4-KiB page with (tests).
pub fn page_bits(flags: PageFlags) -> u64 {
    flags.descriptor_bits()
}

/// Number of shared L3 tables in use (boot log / tests).
pub fn shared_l3_used() -> usize {
    SHARED_L3_TOTAL.load(Ordering::Relaxed)
}

/// Phys address of `slot`'s L0 table — the value written to `TTBR0_EL1` via
/// [`install_ttbr0`].
///
/// Returns `0` for an out-of-range slot index (defensive — callers
/// would normally check the slot before reaching here).
pub fn l0_pa_for(slot: usize) -> u64 {
    if slot >= MAX_PROCESSES {
        return 0;
    }
    // Raw address-of a `static mut`: no reference, read-only arithmetic.
    // The manual offset keeps the computation unambiguous (`&raw const
    // tables.l0` produced surprising results during Phase-06 D3 bring-up).
    let base = (&raw const __USER_TABLES) as *const SlotTables as u64;
    let offset = (slot as u64) * (core::mem::size_of::<SlotTables>() as u64);
    va_to_pa(base + offset)
}

/// Install `l0_pa` into `TTBR0_EL1` and synchronise the pipeline.
///
/// **No TLBI on the install path.** `tlbi vmalle1` immediately
/// before an `eret`-to-EL0 wedges QEMU `aarch64-virt cortex-a72`
/// (verified empirically in Phase 05 EC4 spike, reproduced again
/// in Phase 06 D6 bring-up). Callers that need to drop stale
/// translations from a *previous* slot must invalidate the TLB
/// **before** calling this function — typically by routing through
/// [`crate::arch::aarch64::user_entry::restore_boot_ttbr0`] (which
/// does include the TLBI safely, since it executes wholly at EL1
/// without an immediately-following `eret`-to-EL0). The Pterm
/// handler does exactly that on every process exit, so any later
/// `Pexec` install starts from an empty EL0 TLB.
///
/// **ASIDs off (the default): all processes run with ASID 0.** QEMU
/// 11.0.0's cortex-a72 TCG model wedged on the ASID-tagged TTBR0 write
/// (QUIRK-001); 11.0.2 and the real Pi 4 do not (ARCH03 0b, S0b §4).
///
/// **ASIDs on (ARCH03 stage 4, boot argument `asid=on`):** the root is
/// installed with the slot's own ASID and the switch needs no invalidation
/// at all ([`install_tagged`]); the kernel's `restore_boot_ttbr0` then
/// skips its TLBI too.
///
/// # Safety
///
/// Caller is the boot CPU and is about to either `eret` to EL0
/// (Pexec) or fall through to kernel code that does **not**
/// touch low-half VAs.
pub unsafe fn install_ttbr0(l0_pa: u64) {
    // Every root goes live with its rights up to date (ARCH03 3b): pending
    // protection changes are entered before the switch.
    // SAFETY: EL1 under the BDOS lock (the caller's single-CPU context).
    unsafe { super::memprot::sync_now() };
    if asids_enabled() {
        // Stage 4: the root's own ASID, no invalidation (see `install_tagged`).
        // SAFETY: caller contract.
        unsafe { install_tagged(l0_pa) };
        return;
    }
    // SAFETY: writing TTBR0_EL1 is permitted at EL1. The `dsb ish`
    // before the install is mandatory (ARM ARM D5.10.1): the slot's
    // page-table entries were just written through cacheable kernel
    // memory, and a translation-table walk may only rely on them after
    // a DSB makes the writes observable in the Inner-Shareable domain.
    // Without it, real hardware's walker reads stale/unordered table
    // memory and faults (QEMU's coherent walker tolerated the absence —
    // the real-Pi4 Instruction-Abort permission fault at vaddr 0x100).
    // ISB is the ARM ARM K11.5 context-synchronization event after the
    // TTBR write. No TLBI here (see the function docs — it wedges QEMU
    // before an eret-to-EL0; the TLB is flushed earlier in EL1 context).
    unsafe {
        core::arch::asm!(
            "dsb ish",
            "msr ttbr0_el1, {ttbr0}",
            "isb",
            ttbr0 = in(reg) l0_pa,
            options(nostack, preserves_flags),
        );
    }
}

/// Make freshly-installed user page-table entries (and the data they
/// map) observable to a subsequent EL0 access, **without** a TLB
/// invalidate.
///
/// The AES resource loader calls this after installing a per-process
/// `.RSC` mapping while the owning process's `TTBR0_EL1` is already
/// active (the app trapped via `svc`). The target VAs were previously
/// **invalid** (unmapped spare-window pages), so no stale TLB entry can
/// exist for them and no TLBI is required — which also avoids the
/// QUIRK-001 QEMU wedge that a TLBI immediately before an `eret`-to-EL0
/// triggers (see [`install_ttbr0`]). A `dsb ish` orders the cacheable
/// page-table and blob writes before the translation-table walker and
/// EL0 observe them; the trailing `isb` is the context-synchronisation
/// event so the handler's own EL1 view is consistent.
///
/// # Safety
///
/// Barrier-only; always permitted at EL1.
pub unsafe fn publish_user_mappings() {
    // SAFETY: DSB/ISB are unprivileged-safe barriers, permitted at EL1.
    unsafe {
        core::arch::asm!("dsb ish", "isb", options(nostack, preserves_flags));
    }
}

/// Make freshly-written program bytes at `[va, va + len)` observable to
/// instruction fetch: clean each D-cache line to the Point of Unification,
/// then invalidate the matching I-cache lines, with the barriers ARM ARM
/// B2.4.4 mandates for "data written to be executed". Required on real
/// hardware after the loader copies code into a page; QEMU has no caches,
/// so it is effectively a no-op there.
///
/// `va` is a currently-mapped (kernel high-half) VA aliasing the program
/// page's physical address. The Cortex-A72 caches are PIPT, so by-VA
/// maintenance via any alias of the same PA affects the correct physical
/// lines — the program later fetches the same PA through its EL0 VA.
///
/// # Safety
///
/// `[va, va + len)` must be a mapped, readable range in the active EL1
/// translation regime.
pub unsafe fn sync_code_cache(va: usize, len: usize) {
    if len == 0 {
        return;
    }
    // D/I-cache minimum line sizes from CTR_EL0: the fields are log2 of the
    // line size in 32-bit words, so bytes = 4 << field (ARM ARM CTR_EL0).
    let ctr: u64;
    // SAFETY: CTR_EL0 is read-only and readable at EL1.
    unsafe {
        core::arch::asm!("mrs {}, ctr_el0", out(reg) ctr,
            options(nomem, nostack, preserves_flags));
    }
    let dline = 4usize << ((ctr >> 16) & 0xF); // DminLine
    let iline = 4usize << (ctr & 0xF); // IminLine
    let end = va + len;

    // Clean the D-cache to PoU so the writes reach the point instruction
    // fetch observes, then order before the I-cache maintenance.
    let mut a = va & !(dline - 1);
    while a < end {
        // SAFETY: `a` is within the mapped range; DC CVAU is permitted at EL1.
        unsafe {
            core::arch::asm!("dc cvau, {}", in(reg) a, options(nostack, preserves_flags));
        }
        a += dline;
    }
    // SAFETY: barrier-only.
    unsafe {
        core::arch::asm!("dsb ish", options(nostack, preserves_flags));
    }

    // Invalidate the I-cache to PoU so stale lines cannot be fetched.
    let mut a = va & !(iline - 1);
    while a < end {
        // SAFETY: `a` is within the mapped range; IC IVAU is permitted at EL1.
        unsafe {
            core::arch::asm!("ic ivau, {}", in(reg) a, options(nostack, preserves_flags));
        }
        a += iline;
    }
    // SAFETY: barrier + context synchronisation after I-cache maintenance.
    unsafe {
        core::arch::asm!("dsb ish", "isb", options(nostack, preserves_flags));
    }
}

/// Clean **and invalidate** the D-cache lines covering `[va, va + len)` to
/// the point of coherency, so that afterwards no cached copy of that
/// physical memory exists in this PE's caches — neither a dirty one that
/// could later be written back over somebody else's data, nor a clean one
/// that could satisfy a stale read.
///
/// This is the maintenance the ARM ARM requires before the same physical
/// memory is also accessed through an alias with **mismatched cacheability**
/// attributes (ARM ARM, section "Mismatched memory attributes"): the loss
/// of coherency such a mismatch permits is avoided by ensuring the region
/// is not held in any cache across the mismatched accesses.
///
/// [`super::usermem::expose_attr`] calls it when it publishes a region as
/// Normal Non-Cacheable to EL0 while the kernel keeps a Write-Back alias of
/// the same pages — the framebuffer.
///
/// `va` is a currently-mapped VA aliasing the region's physical address; the
/// Cortex-A72 caches are PIPT, so by-VA maintenance through any alias hits
/// the correct physical lines.
///
/// # Safety
///
/// `[va, va + len)` must be a mapped range in the active EL1 translation
/// regime.
pub unsafe fn clean_invalidate_dcache_range(va: usize, len: usize) {
    if len == 0 {
        return;
    }
    // D-cache minimum line size from CTR_EL0.DminLine: log2 of the line
    // size in 32-bit words, so bytes = 4 << field (ARM ARM CTR_EL0).
    let ctr: u64;
    // SAFETY: CTR_EL0 is read-only and readable at EL1.
    unsafe {
        core::arch::asm!("mrs {}, ctr_el0", out(reg) ctr,
            options(nomem, nostack, preserves_flags));
    }
    let line = 4usize << ((ctr >> 16) & 0xF);
    let end = va + len;
    let mut a = va & !(line - 1);
    while a < end {
        // SAFETY: `a` is within the mapped range; DC CIVAC is permitted at EL1.
        unsafe {
            core::arch::asm!("dc civac, {}", in(reg) a, options(nostack, preserves_flags));
        }
        a += line;
    }
    // SAFETY: barrier-only. `sy` because the other observer is a DMA master
    // outside the Inner-Shareable domain, not just another PE.
    unsafe {
        core::arch::asm!("dsb sy", options(nostack, preserves_flags));
    }
}

/// Switch `TTBR0_EL1` to `l0_pa` **with** a TLB invalidate, for
/// callers that run wholly at EL1 and are *not* about to `eret`-to-
/// EL0 immediately. The Phase-05 EC4 spike found that pairing
/// `tlbi vmalle1` with an immediately-following `eret-to-EL0` wedges
/// QEMU `aarch64-virt cortex-a72`; the workaround is to keep the
/// TLBI in kernel context where the wedge does not occur.
///
/// Use this from the Pterm SVC handler when restoring a real
/// parent's address space (the resume-`eret` lands at EL1, not
/// EL0, so the wedge condition does not apply). For the
/// `eret-to-EL0` install path, [`install_ttbr0`] is the right
/// primitive — but the caller must have flushed the TLB **earlier**
/// in kernel context (e.g. via [`crate::bdos::process::pterm`]'s
/// boot-table restore or via Phase 07 `Pexec`'s pre-install flush
/// step).
///
/// # Safety
///
/// Caller is at EL1, single-CPU. Caller must not be holding a
/// low-half VA reference whose translation depends on the *old*
/// TTBR0 — those become invalid the instant the new TTBR0 is
/// observed.
pub unsafe fn switch_ttbr0_kernel(l0_pa: u64) {
    if asids_enabled() {
        // Stage 4: the root's own ASID makes the invalidation unnecessary.
        // SAFETY: caller contract (EL1, single CPU).
        unsafe { install_tagged(l0_pa) };
        return;
    }
    // SAFETY: writing TTBR0_EL1 + TLBI is permitted at EL1; the
    // DSB pair brackets the invalidate per ARM ARM B5.2.
    unsafe {
        core::arch::asm!(
            "dsb ishst",
            "msr ttbr0_el1, {ttbr0}",
            "tlbi vmalle1",
            "dsb ish",
            "isb",
            ttbr0 = in(reg) l0_pa,
            options(nostack, preserves_flags),
        );
    }
}
