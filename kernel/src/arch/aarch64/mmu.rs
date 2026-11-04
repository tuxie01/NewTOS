//! AArch64 stage-1 MMU page-table reinstall (Phase 05 D3-bis).
//!
//! # Background
//!
//! Stage 1 of the MMU bring-up runs in `arch::aarch64::boot._start`
//! before any Rust code executes: it installs a tiny 4-GiB
//! 1-GiB-block-granular page table in **both** TTBR0 (identity /
//! low-half) and TTBR1 (high-half), turns the MMU on, and absolutely
//! branches to `kernel_main` at high-half VA. From that point on, the
//! kernel runs entirely in the high-half window
//! `0xFFFF_0000_0000_0000 + phys`.
//!
//! Stage 2 — this module — replaces those Stage-1 tables with a
//! richer mapping: 2-MiB-block granularity per `MemoryMap`
//! classification (DRAM Normal Cacheable, MMIO Device-nGnRnE, stack
//! guard invalid). It then **clears `TTBR0_EL1`** so any subsequent
//! low-half access faults — exactly the contract Phase 06's
//! per-process TTBR0 install will need.
//!
//! Stage 2 does **not** rewrite `MAIR_EL1`, `TCR_EL1`, or
//! `SCTLR_EL1.{M,C,I}`. Those were programmed in Stage 1 with
//! identical values; rewriting them would briefly enter a
//! transient TCR state (e.g. `EPD1=1` while we are running at
//! high-half VA) and crash the kernel.
//!
//! # Granule, levels, coverage
//!
//! 4 KiB granule, 48-bit input VA, three levels:
//!
//! ```text
//!   L0 [bits 47:39]   1 entry  (table)            → L1
//!   L1 [bits 38:30]   ≤4 entries (table)          → L2[i]
//!   L2 [bits 29:21]   512 entries (2 MiB blocks)  → page frame
//! ```
//!
//! Total static coverage: **4 GiB** (`NUM_L2 = 4`, four 1-GiB regions).
//! That fits both boards: QEMU `virt` uses 0..2 GiB, BCM2711 / RPi4
//! uses 0..4 GiB (with VPU reserve at the top). 8-GiB Pi4 SKUs
//! expose ARM-visible RAM above 4 GiB which is **not** covered by
//! Stage 2 yet — to be addressed when 8-GiB RPi4 EC verification
//! lands.
//!
//! Each 2-MiB block is classified once at table build time:
//!   * **DRAM** (`MemoryMap::dram` contains the block start) → Normal
//!     Cacheable, Inner Shareable, AttrIdx 0.
//!   * **MMIO** (any `MemoryMap::mmio[i]` contains the block start) →
//!     Device-nGnRnE, Non-Shareable, AttrIdx 1.
//!   * **Otherwise** → invalid (descriptor = 0).
//!
//! Both boards have MMIO regions whose 2-MiB boundaries do not split
//! a peripheral; that has been verified against `mm::layout::board`.
//!
//! # ARM ARM references
//!
//! * D8.3   "Translation table descriptors" (block/table format)
//! * D8.2.4 "MAIR_EL1" (memory attribute encoding)
//! * D19.2  "TCR_EL1", "SCTLR_EL1", "TTBR0_EL1", "TTBR1_EL1"
//! * D8.7   "TLB maintenance instructions"
//! * K11.5  "MMU programming" (publish/invalidate/barrier sequence)

use crate::bios::machine::layout::{MemoryMap, PhysRange};

// ─────────────────────────────────────────────────────────────────────
// Descriptor bit constants (4 KiB granule, stage 1, EL1)
// ─────────────────────────────────────────────────────────────────────

/// Descriptor[0]: entry is valid.
const VALID: u64 = 1 << 0;
/// Descriptor[1]: at L0/L1/L2, 1 = next-level table, 0 = block.
const TABLE: u64 = 1 << 1;
/// Descriptor[1]: at L2 with 4-KiB granule, 0 = 2-MiB block.
const BLOCK_L2: u64 = 0 << 1;
/// Descriptor[1]: at L3 with 4-KiB granule, 1 = page (and 0 is invalid).
const PAGE_L3: u64 = 1 << 1;
/// Descriptor[4:2]: AttrIndx — index into MAIR_EL1.
const ATTRIDX_NORMAL: u64 = 0 << 2;
const ATTRIDX_DEVICE: u64 = 1 << 2;
/// AttrIdx 2 = Normal Non-Cacheable (MAIR `0x44`, see `boot::MAIR_VALUE`).
const ATTRIDX_NC: u64 = 2 << 2;
/// Mask over the AttrIndx field (descriptor[4:2]).
const ATTRIDX_MASK: u64 = 0b111 << 2;
/// Descriptor[7:6]: AP[2:1] = 00 → R/W at EL1, no EL0 access.
const AP_RW_EL1: u64 = 0b00 << 6;
/// Descriptor[9:8]: SH — shareability for Normal memory.
const SH_INNER: u64 = 0b11 << 8;
const SH_NON: u64 = 0b00 << 8;
/// Descriptor[10]: AF — Access Flag. Set so the CPU never raises an
/// AF fault on first touch.
const AF: u64 = 1 << 10;
/// Descriptor[53]: PXN — Privileged eXecute Never. Set on Device pages.
const PXN: u64 = 1 << 53;
/// Descriptor[54]: UXN — Unprivileged eXecute Never. We are EL1-only,
/// but set this on every block to be future-proof when EL0 mappings
/// arrive in Phase 06.
const UXN: u64 = 1 << 54;

// ─────────────────────────────────────────────────────────────────────
// Static page tables — placed in BSS (zero-initialized at boot)
// ─────────────────────────────────────────────────────────────────────

const ENTRIES: usize = 512;
/// 4-KiB translation granule — the L3 page size.
const PAGE_SIZE: u64 = 4096;
const L2_BLOCK_SIZE: u64 = 2 * 1024 * 1024;
const L1_BLOCK_SIZE: u64 = 1024 * 1024 * 1024;
const NUM_L2: usize = 4; // 4 GiB total coverage

#[repr(C, align(4096))]
struct PageTable([u64; ENTRIES]);

// SAFETY: Modified once before MMU enable on the boot CPU. After
// enable, treated as read-only; any future remap must use
// break-before-make per ARM ARM K11.5.
static mut L0: PageTable = PageTable([0; ENTRIES]);
static mut L1: PageTable = PageTable([0; ENTRIES]);
static mut L2: [PageTable; NUM_L2] =
    [const { PageTable([0; ENTRIES]) }; NUM_L2];

/// One L3 table refining the single 2-MiB block that holds the kernel
/// stack and its guard.
///
/// Everything else in the kernel mapping is 2-MiB L2 blocks, and that is
/// what used to force the stack guard to be a whole 2-MiB block: an
/// unmapped hole can be no finer than the descriptor granularity. A
/// 2-MiB guard plus the 2-MiB-aligned stack cost over 4 MiB of the
/// 14-MiB ST-RAM window, which is why the pool could not have a fixed
/// size. Refining exactly one block down to 4-KiB pages makes the guard
/// one page, so the stack fits inside the OS budget below `membot` and
/// the ST-RAM pool becomes a constant 10 MiB.
///
/// One table, 4 KiB of `.bss`. The block is chosen at build time by the
/// linker (`.stack` follows `.bss`), and [`build`] refines whichever
/// block ends up containing [`MemoryMap::stack_guard`].
static mut L3_STACK: PageTable = PageTable([0; ENTRIES]);

/// One extra L2 table for a Device MMIO region **above** the build-time
/// 4-GiB Stage-2 coverage — see [`map_device_high`]. (The QEMU `virt` high
/// PCIe ECAM at `0x40_1000_0000`, PHASE-16.) Covers one 1-GiB L1 slot.
static mut L2_HIGH: PageTable = PageTable([0; ENTRIES]);

/// An all-invalid L0 root for `TTBR0_EL1` whenever no process is active.
/// After the high-half switch (`install_stage2_ttbr1`) TTBR0 points here
/// instead of being left on the Stage-1 boot identity map. That map covers
/// 0..4 GiB with 1-GiB **UXN** L1 blocks; leaving it live let the
/// Cortex-A72 speculatively (re-)cache a UXN block for the low-half, so the
/// first `Pexec`'s eret-to-EL0 instruction fetch hit a stale UXN entry and
/// took an Instruction-Abort permission fault (level 1) on real hardware.
/// With an all-zero root the walker finds only invalid descriptors —
/// nothing executable to cache — and the per-process `install_ttbr0` swaps
/// in the slot's tables cleanly. We cannot instead flush at install time: a
/// `tlbi` on the eret-to-EL0 path wedges QEMU's cortex-a72 (QUIRK-001).
/// This is safe only because the kernel reaches all MMIO through the
/// high-half (see `drivers::mmio_va`), so it never needs the low-half
/// identity. Pointing at a real zero table (not `TTBR0 = 0`, which would
/// walk physical 0 and stall QEMU) keeps both targets happy.
static EMPTY_L0: PageTable = PageTable([0; ENTRIES]);

/// Physical address of [`EMPTY_L0`] for a `TTBR0_EL1` "no process" install.
pub fn empty_l0_pa() -> u64 {
    va_to_pa((&raw const EMPTY_L0) as u64)
}

/// `TCR_EL1.AS` — ASID size: 0 = 8 bit, 1 = 16 bit (ARM ARM D24-9917).
const TCR_AS: u64 = 1 << 36;

/// Size the ASID for ARCH03 stage 4 (per-process ASIDs, boot argument
/// `asid=on`): read `ID_AA64MMFR0_EL1.ASIDBits` ([7:4]; `0b0010` = 16 bits,
/// `0b0000` = 8 bits, ARM ARM D24-9247; Cortex-A72: `0x2`, TRM Table 4-35)
/// and, for 16 bits, set `TCR_EL1.AS` (otherwise the upper 8 ASID bits are
/// treated as 0, D24-9917). Returns `(width in bits, ASIDBits field)`.
///
/// `TCR_EL1.AS` is one of the fields "permitted to be cached in a TLB"
/// (D24.2.193): after the write and its synchronisation the TLB is
/// invalidated (`RVNRFW`, D8-7858) — `msr; isb; tlbi vmalle1; dsb ish; isb`.
/// Only `AS` changes; every other field keeps the boot value, so the running
/// translation is undisturbed. Left alone (8 bits, boot `TCR_EL1`) when
/// ASIDs are off.
///
/// # Safety
///
/// Boot CPU, after [`reinstall`] (`TTBR0_EL1` = the empty root, ASID 0),
/// before any process root is installed.
pub unsafe fn size_asids() -> (u32, u64) {
    let mmfr0: u64;
    // SAFETY: ID_AA64MMFR0_EL1 is readable at EL1.
    unsafe {
        core::arch::asm!("mrs {}, id_aa64mmfr0_el1", out(reg) mmfr0,
            options(nomem, nostack, preserves_flags));
    }
    let field = (mmfr0 >> 4) & 0xF;
    if field != 0b0010 {
        return (8, field);
    }
    // SAFETY: EL1; only TCR_EL1.AS changes (see above), followed by the
    // synchronisation and the invalidation the ARM ARM requires.
    unsafe {
        let tcr: u64;
        core::arch::asm!("mrs {}, tcr_el1", out(reg) tcr, options(nomem, nostack, preserves_flags));
        core::arch::asm!(
            "msr tcr_el1, {t}",
            "isb",
            "tlbi vmalle1",
            "dsb ish",
            "isb",
            t = in(reg) tcr | TCR_AS,
            options(nostack, preserves_flags),
        );
    }
    (16, field)
}

// ─────────────────────────────────────────────────────────────────────
// Table construction
// ─────────────────────────────────────────────────────────────────────

/// Classify a single 2-MiB physical block start address.
///
/// Returns `Some(attrs)` with descriptor flags (VALID, AF, AP, AttrIdx,
/// SH, PXN/UXN as appropriate) but **without the address bits**, or
/// `None` if the block is unmapped (either outside DRAM/MMIO or
/// deliberately left invalid as a stack guard, Phase 03 D6).
fn classify_block(map: &MemoryMap, pa: u64) -> Option<u64> {
    // The stack guard is *not* handled here any more: it is a single
    // 4-KiB page, so it is punched out at L3 by [`refine_stack_block`],
    // which replaces this block's descriptor with a table descriptor.
    if range_contains_block(&map.dram, pa, L2_BLOCK_SIZE) {
        // Normal RAM. Code from .text lives here, so do not set PXN.
        Some(VALID | BLOCK_L2 | AF | AP_RW_EL1 | ATTRIDX_NORMAL | SH_INNER | UXN)
    } else if map.mmio.iter().any(|r| range_overlaps(r, pa, L2_BLOCK_SIZE)) {
        // Device memory — never executable.
        Some(VALID | BLOCK_L2 | AF | AP_RW_EL1 | ATTRIDX_DEVICE | SH_NON | PXN | UXN)
    } else {
        None
    }
}

#[inline]
fn range_contains_block(r: &PhysRange, block_start: u64, block_size: u64) -> bool {
    block_start >= r.start && block_start.saturating_add(block_size) <= r.end
}

#[inline]
fn range_overlaps(r: &PhysRange, block_start: u64, block_size: u64) -> bool {
    let block_end = block_start.saturating_add(block_size);
    block_start < r.end && r.start < block_end
}

/// Populate the static L0/L1/L2 tables for the given memory map.
///
/// Runs **before** the MMU is enabled, so it operates on physical
/// addresses (which equal the virtual addresses of these BSS arrays).
//
// `clippy::deref_addrof` would simplify `&*(&raw const X)` to `&X`,
// but the whole reason we go through `&raw` is that `&` on a
// `static mut` triggers the harder `static_mut_refs` warning. The
// `&*(&raw …)` form is the established idiom for getting a tightly-
// scoped reference to a static-mut without producing a long-lived
// `&mut` to it. The lint is a false positive against this pattern.
#[allow(clippy::deref_addrof)]
fn build(map: &MemoryMap) {
    // SAFETY (for every `&mut *(&raw mut …)` / `&*(&raw const …)`
    // below): pre-MMU, single-threaded boot path. No other CPU or
    // exception path can observe these tables until [`enable`] runs.

    // Fill L2 tables: one descriptor per 2-MiB block.
    let l2_mut: &mut [PageTable; NUM_L2] = unsafe { &mut *(&raw mut L2) };
    for (i, table) in l2_mut.iter_mut().enumerate() {
        let l1_base = (i as u64) * L1_BLOCK_SIZE;
        for (j, slot) in table.0.iter_mut().enumerate() {
            let pa = l1_base + (j as u64) * L2_BLOCK_SIZE;
            *slot = match classify_block(map, pa) {
                Some(attrs) => pa | attrs,
                None => 0,
            };
        }
    }

    // Refine the one block holding the stack guard down to 4-KiB pages.
    refine_stack_block(map, l2_mut);

    // Fill L1: one table descriptor per L2 we built. Iterate L2 by
    // reference so the addresses come straight from the slice
    // pointers — no `L2[i]` indexing is needed.
    //
    // **Phys conversion**: `&L2[i]` resolves to the linker-time VA,
    // which after Phase 05 lives in the high-half. Page-table
    // descriptors must hold *physical* addresses; convert via
    // [`va_to_pa`] before stamping.
    let l1_mut: &mut PageTable = unsafe { &mut *(&raw mut L1) };
    let l2_ref: &[PageTable; NUM_L2] = unsafe { &*(&raw const L2) };
    for (l1_slot, l2_table) in l1_mut.0.iter_mut().zip(l2_ref.iter()) {
        let l2_pa = va_to_pa(l2_table as *const PageTable as u64);
        *l1_slot = l2_pa | VALID | TABLE;
    }

    // L0[0] points at L1. Same VA→PA conversion as above.
    let l1_pa = va_to_pa((&raw const L1) as u64);
    let l0_mut: &mut PageTable = unsafe { &mut *(&raw mut L0) };
    l0_mut.0[0] = l1_pa | VALID | TABLE;
}

/// Replace the L2 block descriptor covering the kernel stack guard with a
/// table descriptor to [`L3_STACK`], and fill that L3 with 4-KiB pages —
/// Normal everywhere except the guard, which stays invalid.
///
/// This is what makes the guard cost one page instead of a whole 2-MiB
/// block, which in turn is what lets the kernel stack fit under the fixed
/// `membot` and the ST-RAM pool have a constant size
/// (`bios::machine::layout::STRAM_OS_RESERVE`).
///
/// Runs pre-MMU from [`build`], on the boot CPU, with exclusive access to
/// the static tables.
#[allow(clippy::deref_addrof)]
fn refine_stack_block(map: &MemoryMap, l2: &mut [PageTable; NUM_L2]) {
    let block = map.stack_guard.start & !(L2_BLOCK_SIZE - 1);
    debug_assert!(
        map.stack_guard.end <= block + L2_BLOCK_SIZE,
        "stack guard must not straddle a 2-MiB block"
    );

    // SAFETY: pre-MMU, single-threaded boot path (see `build`).
    let l3 = unsafe { &mut *(&raw mut L3_STACK) };
    for (k, slot) in l3.0.iter_mut().enumerate() {
        let pa = block + (k as u64) * PAGE_SIZE;
        *slot = if map.stack_guard.contains(pa) {
            // The guard: left invalid so a stack overflow takes a
            // synchronous data abort instead of silently corrupting the
            // memory below the stack (Phase 03 D6).
            0
        } else if range_contains_block(&map.dram, pa, PAGE_SIZE) {
            // Same attributes `classify_block` gives Normal DRAM, at page
            // granularity. PXN stays clear: this block may hold code.
            pa | VALID | PAGE_L3 | AF | AP_RW_EL1 | ATTRIDX_NORMAL | SH_INNER | UXN
        } else {
            0
        };
    }

    let i = (block / L1_BLOCK_SIZE) as usize;
    let j = ((block % L1_BLOCK_SIZE) / L2_BLOCK_SIZE) as usize;
    debug_assert!(i < NUM_L2, "stack guard outside the static L2 coverage");
    let l3_pa = va_to_pa((l3 as *const PageTable) as u64);
    l2[i].0[j] = l3_pa | VALID | TABLE;
}

// ─────────────────────────────────────────────────────────────────────
// MMU enable
// ─────────────────────────────────────────────────────────────────────

// MAIR_EL1 / TCR_EL1 / SCTLR_EL1 are programmed once by the Stage-1
// boot stub (`arch::aarch64::boot::_start`) and not touched again by
// Stage 2. The constants live in `boot.rs` next to the asm that
// consumes them; rewriting them here would create a drift-risk pair.

/// High-half VA offset. Mirrors `VA_OFFSET` in the linker scripts;
/// any drift between this constant and the linker would silently
/// corrupt the phys-address computation below.
pub const VA_OFFSET: u64 = 0xFFFF_0000_0000_0000;

/// Clean (write back) the cache lines covering `[va, va + len)` to the
/// point of coherency, so a separate DMA master (e.g. the display
/// controller) sees writes the CPU made through a cacheable mapping.
///
/// On a Non-Cacheable mapping this is a harmless no-op (no dirty lines),
/// so it is safe to call unconditionally as a robustness net for the
/// framebuffer. ARM ARM B2.4 (cache maintenance). 64-byte line is the
/// architectural minimum and always safe for a clean-by-VA loop.
///
/// # Safety
/// `va`/`len` must describe a mapped, readable region.
pub unsafe fn clean_dcache_range(va: usize, len: usize) {
    if len == 0 {
        return;
    }
    const LINE: usize = 64;
    let mut p = va & !(LINE - 1);
    let end = va + len;
    while p < end {
        // SAFETY: `dc cvac` on a mapped VA is well-defined.
        unsafe { core::arch::asm!("dc cvac, {}", in(reg) p, options(nostack, preserves_flags)) };
        p += LINE;
    }
    // SAFETY: ordering barrier; no memory effect of its own.
    unsafe { core::arch::asm!("dsb sy", options(nostack, preserves_flags)) };
}

/// Clean **and invalidate** the cache lines covering `[va, va + len)` to the
/// point of coherency: afterwards no copy of that physical memory is held in
/// this PE's caches, dirty or clean.
///
/// Used instead of [`clean_dcache_range`] while an EL0 process holds a
/// **Normal Non-Cacheable** alias of the same framebuffer pages
/// (`Physbase`/`Logbase`, ADR-023). Mismatched cacheability attributes are
/// only coherent if software keeps the region out of the caches (ARM ARM,
/// section "Mismatched memory attributes"): a clean-only maintenance leaves
/// the line valid, and a later partial write to it would write the whole
/// line back — silently undoing the stores the program made through its
/// uncached alias in the meantime.
///
/// # Safety
/// `va`/`len` must describe a mapped, readable region.
pub unsafe fn clean_invalidate_dcache_range(va: usize, len: usize) {
    if len == 0 {
        return;
    }
    const LINE: usize = 64;
    let mut p = va & !(LINE - 1);
    let end = va + len;
    while p < end {
        // SAFETY: `dc civac` on a mapped VA is well-defined.
        unsafe { core::arch::asm!("dc civac, {}", in(reg) p, options(nostack, preserves_flags)) };
        p += LINE;
    }
    // SAFETY: ordering barrier; no memory effect of its own.
    unsafe { core::arch::asm!("dsb sy", options(nostack, preserves_flags)) };
}

/// Clean (write back) **every** data/unified cache level of this PE up to the
/// Level of Coherence to the Point of Coherency, by set/way — ARM ARM
/// (DDI 0487 M.d) D7.5.9.16.1 "Example code for cache maintenance
/// instructions", transcribed instruction for instruction (`DC CSW` loop over
/// `CLIDR_EL1` levels, `CCSIDR_EL1` sets/ways in the 32-bit format the
/// Cortex-A72 — no FEAT_CCIDX — implements).
///
/// Used once, right before a whole-machine restart: the counterpart of
/// EmuTOS `_flush_data_cache` in `warm_reset`/`cold_reset`
/// (`bios/panicasm.S:105,133`). The ARM ARM restricts set/way maintenance to
/// such power-down style uses (same section); it is not a coherency tool for
/// a running system. Only the calling PE's caches are touched — the other
/// cores sit in the firmware spin loop and hold no kernel data.
///
/// # Safety
/// EL1. Clobbers `CSSELR_EL1`.
pub unsafe fn clean_dcache_all() {
    // SAFETY: system-register reads/writes and cache maintenance by set/way
    // are permitted at EL1; the loop only computes set/way operands.
    unsafe {
        core::arch::asm!(
            "mrs {clidr}, clidr_el1",
            "and {loc2:w}, {clidr:w}, #0x07000000", // 2 x Level of Coherence
            "lsr {loc2:w}, {loc2:w}, #23",
            "cbz {loc2:w}, 5f",
            "mov {lvl2:w}, #0",                     // 2 x cache level
            "mov {one:w}, #1",
            "1:",
            "add {t:w}, {lvl2:w}, {lvl2:w}, lsr #1", // 3 x cache level
            "lsr {ctype:w}, {clidr:w}, {t:w}",      // cache type for this level
            "and {ctype:w}, {ctype:w}, #0x7",
            "cmp {ctype:w}, #2",
            "b.lt 4f",                              // no data/unified cache here
            "msr csselr_el1, {lvl2}",               // select this level
            "isb",
            "mrs {ccsidr}, ccsidr_el1",
            "and {t:w}, {ccsidr:w}, #7",            // log2(linelen) - 4
            "add {t:w}, {t:w}, #4",                 // log2(linelen)
            "ubfx {maxway:w}, {ccsidr:w}, #3, #10", // max way number
            "clz {wpos:w}, {maxway:w}",             // bit position of the way
            "lsl {way:w}, {maxway:w}, {wpos:w}",    // max way, aligned
            "lsl {wdec:w}, {one:w}, {wpos:w}",      // way decrement
            "2:",
            "ubfx {set:w}, {ccsidr:w}, #13, #15",   // max set number
            "lsl {set:w}, {set:w}, {t:w}",          // aligned
            "lsl {sdec:w}, {one:w}, {t:w}",         // set decrement
            "3:",
            "orr {op:w}, {lvl2:w}, {way:w}",
            "orr {op:w}, {op:w}, {set:w}",
            "dc csw, {op}",                         // clean by set/way
            "subs {set:w}, {set:w}, {sdec:w}",
            "b.ge 3b",
            "subs {way}, {way}, {wdec}",
            "b.ge 2b",
            "4:",
            "add {lvl2:w}, {lvl2:w}, #2",
            "cmp {loc2:w}, {lvl2:w}",
            "dsb sy",
            "b.gt 1b",
            "5:",
            "dsb sy",
            "isb",
            clidr = out(reg) _,
            loc2 = out(reg) _,
            lvl2 = out(reg) _,
            one = out(reg) _,
            t = out(reg) _,
            ctype = out(reg) _,
            ccsidr = out(reg) _,
            maxway = out(reg) _,
            wpos = out(reg) _,
            way = out(reg) _,
            wdec = out(reg) _,
            set = out(reg) _,
            sdec = out(reg) _,
            op = out(reg) _,
            options(nostack),
        );
    }
}

/// Clean (write back) the cache lines covering a **strided rectangle**:
/// `rows` runs of `row_bytes` bytes, the first starting at `va` and each
/// following one `stride_bytes` further on.
///
/// This is [`clean_dcache_range`] for a 2-D region whose rows are *not*
/// contiguous. Treating such a region as the single address span that
/// encloses it cleans the whole `stride_bytes` band per row — for a narrow,
/// tall rectangle that is the row width over and over: a 2-pixel-wide,
/// 401-row primitive on a 1920-pixel line spans 400*7680+8 = 3 072 008 bytes
/// and costs 48 001 `dc cvac`, against at most 802 for the 3 208 bytes it
/// actually wrote (Pi 4, `fb.clean max/call` in
/// `measurements/measurement-2026-09-25-fs04/raw/pi04-FS04.TXT`). Cleaning a line that is
/// not dirty is cheap but not free, and the redundant work grows with the
/// line length.
///
/// One `dsb sy` closes the whole walk, exactly as in [`clean_dcache_range`]:
/// `dc` operations need no barrier *between* them, only one after the last
/// to order them against the following accesses (ARM ARM B2.4).
///
/// # Safety
/// Every row `[va + r * stride_bytes, … + row_bytes)`, `r < rows`, must lie
/// in a mapped, readable region.
pub unsafe fn clean_dcache_rect(va: usize, row_bytes: usize, stride_bytes: usize, rows: usize) {
    if row_bytes == 0 || rows == 0 {
        return;
    }
    const LINE: usize = 64;
    for r in 0..rows {
        let row = va + r * stride_bytes;
        let mut p = row & !(LINE - 1);
        let end = row + row_bytes;
        while p < end {
            // SAFETY: `dc cvac` on a mapped VA is well-defined.
            unsafe { core::arch::asm!("dc cvac, {}", in(reg) p, options(nostack, preserves_flags)) };
            p += LINE;
        }
    }
    // SAFETY: ordering barrier; no memory effect of its own.
    unsafe { core::arch::asm!("dsb sy", options(nostack, preserves_flags)) };
}

/// Clean **and invalidate** the cache lines covering a strided rectangle —
/// [`clean_dcache_rect`] with `dc civac`, for the same reason
/// [`clean_invalidate_dcache_range`] exists: while an EL0 process holds a
/// Normal Non-Cacheable alias of these pages, no line of the region may stay
/// valid in this PE's caches.
///
/// # Safety
/// Same contract as [`clean_dcache_rect`].
pub unsafe fn clean_invalidate_dcache_rect(
    va: usize,
    row_bytes: usize,
    stride_bytes: usize,
    rows: usize,
) {
    if row_bytes == 0 || rows == 0 {
        return;
    }
    const LINE: usize = 64;
    for r in 0..rows {
        let row = va + r * stride_bytes;
        let mut p = row & !(LINE - 1);
        let end = row + row_bytes;
        while p < end {
            // SAFETY: `dc civac` on a mapped VA is well-defined.
            unsafe {
                core::arch::asm!("dc civac, {}", in(reg) p, options(nostack, preserves_flags))
            };
            p += LINE;
        }
    }
    // SAFETY: ordering barrier; no memory effect of its own.
    unsafe { core::arch::asm!("dsb sy", options(nostack, preserves_flags)) };
}

/// Convert a high-half kernel VA to the corresponding physical
/// address. Valid only for symbols inside the kernel image (where
/// VA = LMA + VA_OFFSET); do not use for arbitrary mappings.
#[inline(always)]
pub const fn va_to_pa(va: u64) -> u64 {
    va.wrapping_sub(VA_OFFSET)
}

/// Remap the 2-MiB high-half blocks covering `[va, va + len)` from
/// Normal Cacheable to **Normal Non-Cacheable** (AttrIdx 2).
///
/// Used for the VideoCore framebuffer: the display controller is a
/// separate DMA master that reads DRAM *beyond* the ARM caches, so a
/// cached FB would leave drawn pixels stranded in dirty cache lines.
/// Mapping the FB Non-Cacheable removes the coherency problem entirely
/// (no per-frame `dc cvac` needed).
///
/// Whole 2-MiB blocks are converted — the VC-allocated FB region is
/// dedicated, so catching a little neighbouring RAM is harmless. Only
/// blocks that are currently **valid Normal** are touched (Device and
/// unmapped/guard blocks are skipped). Each block uses break-before-make
/// (invalidate → TLB flush → rewrite) per ARM ARM K11.5, so no stale
/// cacheable translation can coexist with the new attribute.
///
/// `va` is a high-half kernel VA (VA = `VA_OFFSET` + PA), matching the
/// Stage-2 identity-offset mapping.
///
/// # Safety
///
/// Boot CPU, single-threaded, after [`reinstall`] has installed the
/// Stage-2 tables. The target blocks must not contain currently-
/// executing code or the active stack (a VC-allocated framebuffer never
/// does).
#[allow(clippy::deref_addrof)]
pub unsafe fn remap_normal_nc(va: u64, len: u64) {
    let attrs = VALID | BLOCK_L2 | AF | AP_RW_EL1 | ATTRIDX_NC | SH_NON | PXN | UXN;
    // SAFETY: forwarded from the caller's contract.
    unsafe { remap_l2_attrs(va, len, attrs) };
}

/// Remap the 2-MiB high-half blocks covering `[va, va + len)` to **Normal
/// Cacheable Write-Back Write-Allocate** (AttrIdx 0, MAIR `0xFF`), Inner
/// Shareable — the same attribute the Stage-2 DRAM blocks carry.
///
/// Used for the VideoCore framebuffer: CPU pixel writes land in the cache
/// (line-fill + write-allocate + prefetch make software blits/fills an order
/// of magnitude faster than a Non-Cacheable mapping, where every access is a
/// single uncached DRAM transaction). The display controller is a separate,
/// read-only DMA master that scans out DRAM *beyond* the ARM caches, so with a
/// cacheable FB **every** region the CPU draws must be cleaned to the point of
/// coherency (`dc cvac` + `dsb`, see [`clean_dcache_range`]) or the scanout
/// shows stale pixels. In NewTOS every framebuffer write routes through the
/// `VdiScreen` raster backend, which cleans each primitive's touched rectangle,
/// so the coherency requirement is already met; the VideoCore never writes the
/// FB, so no invalidate is ever needed.
///
/// Whole 2-MiB blocks are converted (the VC-allocated FB region is dedicated).
/// Only blocks that are currently valid-Normal or invalid are touched; Device
/// (MMIO) blocks are skipped. Each block uses break-before-make per ARM ARM
/// K11.5. `va` is a high-half kernel VA (VA = `VA_OFFSET` + PA).
///
/// # Safety
///
/// Same contract as [`remap_normal_nc`]: boot CPU, single-threaded, after
/// [`reinstall`]; the target blocks hold no executing code or the active stack.
#[allow(clippy::deref_addrof)]
pub unsafe fn remap_normal_wb(va: u64, len: u64) {
    let attrs = VALID | BLOCK_L2 | AF | AP_RW_EL1 | ATTRIDX_NORMAL | SH_INNER | PXN | UXN;
    // SAFETY: forwarded from the caller's contract.
    unsafe { remap_l2_attrs(va, len, attrs) };
}

/// Remap `[va, va + len)` as **Device-nGnRnE** (strongly-ordered, uncached).
///
/// Used for the xHCI DMA block on the real Pi-4: the VL805 PCIe endpoint is
/// not cache-coherent, and — unlike Normal Non-Cacheable — Device memory
/// forbids the CPU's gathering, reordering and speculative reads. That makes
/// the controller's device-context write-backs read back byte-exact, matching
/// circle, which maps its entire xHCI shared region as Device-nGnRnE
/// (`references/circle/lib/memory64.cpp`). Normal-NC + manual cache
/// maintenance corrupted the write-back read (EP0 context appeared at the slot
/// offset), so Address Device "succeeded" yet the slot was never internally
/// Addressed. See `docs/STATUS.md` Phase 16.
///
/// # Safety
///
/// Same contract as [`remap_normal_nc`]: boot CPU, single-threaded, after
/// [`reinstall`]; the target blocks hold no executing code or active stack.
#[allow(clippy::deref_addrof)]
pub unsafe fn remap_device_ngnrne(va: u64, len: u64) {
    let attrs = VALID | BLOCK_L2 | AF | AP_RW_EL1 | ATTRIDX_DEVICE | SH_NON | PXN | UXN;
    // SAFETY: forwarded from the caller's contract.
    unsafe { remap_l2_attrs(va, len, attrs) };
}

/// Shared break-before-make remap of the 2-MiB L2 blocks covering
/// `[va, va + len)` to the descriptor attributes `attrs`. Only blocks that are
/// currently valid-Normal or invalid are touched; existing Device (MMIO)
/// blocks are skipped, so this never rewrites peripheral mappings.
///
/// # Safety
///
/// Boot CPU, single-threaded, after [`reinstall`]; the target blocks hold no
/// executing code or the active stack.
#[allow(clippy::deref_addrof)]
unsafe fn remap_l2_attrs(va: u64, len: u64, attrs: u64) {
    if len == 0 {
        return;
    }
    let pa_first = va_to_pa(va) & !(L2_BLOCK_SIZE - 1);
    let pa_last = va_to_pa(va + len - 1) & !(L2_BLOCK_SIZE - 1);

    // SAFETY: single-CPU boot path; exclusive access to the static tables.
    let l2 = unsafe { &mut *(&raw mut L2) };

    let mut pa = pa_first;
    loop {
        let i = (pa / L1_BLOCK_SIZE) as usize;
        let j = ((pa % L1_BLOCK_SIZE) / L2_BLOCK_SIZE) as usize;
        if i < NUM_L2 {
            let slot = &mut l2[i].0[j];
            let old = *slot;
            // Re-attribute this block. Two cases the callers hit:
            //   * the block was Normal DRAM (framebuffer / `.dma` carved out
            //     of the kernel's mapped RAM) — re-attribute it; or
            //   * the block was *invalid* — a framebuffer above `dram.end`,
            //     which Stage-2 left unmapped. Create the mapping.
            // Never touch a Device block (MMIO), which is never a target.
            let is_device =
                old & VALID != 0 && (old & ATTRIDX_MASK) == ATTRIDX_DEVICE;
            // A *table* descriptor (the one block refined to 4-KiB pages
            // for the stack guard) must never be overwritten with a block
            // descriptor — that would unmap the kernel stack. No caller
            // targets it; the check makes that structural.
            let is_table = old & VALID != 0 && old & TABLE != 0;
            debug_assert!(!is_table, "remap_l2_attrs would clobber the stack L3 table");
            if !is_device && !is_table {
                // Break-before-make for this block (ARM ARM K11.5). For a
                // previously-invalid block the zero-write is a harmless
                // no-op; the TLBI drops any stale (or speculatively cached
                // invalid) entry before the new descriptor goes live.
                *slot = 0;
                // SAFETY: publish the break, drop stale TLB, then make.
                unsafe {
                    core::arch::asm!(
                        "dsb ishst",
                        "tlbi vmalle1",
                        "dsb ish",
                        options(nostack, preserves_flags),
                    );
                }
                *slot = pa | attrs;
                // SAFETY: publish the new descriptor before any access.
                unsafe {
                    core::arch::asm!(
                        "dsb ishst",
                        "isb",
                        options(nostack, preserves_flags),
                    );
                }
            }
        }
        if pa == pa_last {
            break;
        }
        pa += L2_BLOCK_SIZE;
    }
}

/// Map a Device MMIO region `[pa, pa + len)` into the high-half VA
/// (`pa + VA_OFFSET`) for MMIO **above** the build-time 4-GiB Stage-2
/// coverage. The region must fit within a single 1-GiB L1 slot — true for
/// the QEMU `virt` high PCIe ECAM (`0x40_1000_0000`, 256 MiB) that PHASE-16
/// reads from the DTB. Runs after the MMU is live, so it publishes the
/// writes and flushes the TLB (EL1 context — no eret-to-EL0, so the TLBI is
/// safe, unlike the per-process install path / QUIRK-001).
///
/// # Safety
///
/// Single-CPU; `pa`/`len` describe a real Device region; caller maps it once.
pub unsafe fn map_device_high(pa: u64, len: u64) {
    if len == 0 {
        return;
    }
    let dev = VALID | BLOCK_L2 | AF | AP_RW_EL1 | ATTRIDX_DEVICE | SH_NON | PXN | UXN;
    let l1_idx = (pa >> 30) as usize; // 1 GiB per L1 entry
    let l1_base = (l1_idx as u64) << 30;

    // SAFETY: single-CPU; exclusive access to the static tables. Pick the L2
    // table for this 1-GiB slot: an existing one if the slot is within the
    // build-time coverage (don't repoint its L1 entry), else the dedicated
    // high table.
    let l2h: &mut PageTable = if l1_idx < NUM_L2 {
        unsafe { &mut (*&raw mut L2)[l1_idx] }
    } else {
        let l1 = unsafe { &mut *(&raw mut L1) };
        let l2h_pa = va_to_pa((&raw const L2_HIGH) as u64);
        l1.0[l1_idx] = l2h_pa | VALID | TABLE;
        unsafe { &mut *(&raw mut L2_HIGH) }
    };

    // Fill the 2-MiB Device blocks covering [pa, pa + len).
    let first = pa & !(L2_BLOCK_SIZE - 1);
    let last = (pa + len - 1) & !(L2_BLOCK_SIZE - 1);
    let mut b = first;
    while b <= last {
        let j = ((b - l1_base) / L2_BLOCK_SIZE) as usize;
        if j < ENTRIES {
            l2h.0[j] = b | dev;
        }
        b += L2_BLOCK_SIZE;
    }

    // SAFETY: publish the table writes, drop stale TLB, synchronise.
    unsafe {
        core::arch::asm!(
            "dsb ishst",
            "tlbi vmalle1",
            "dsb ish",
            "isb",
            options(nostack, preserves_flags),
        );
    }
}

/// Install the Stage-2 high-half tables into `TTBR1_EL1` and flush
/// the TLB.
///
/// Sequence (per ARM ARM K11.5 "MMU programming"):
/// 1. `dsb ishst` — publish the populated `L0`/`L1`/`L2` writes
///    that [`build`] just performed.
/// 2. `msr ttbr0_el1, <empty>; isb` — drop the boot identity low half
///    (the empty root, [`EMPTY_L0`]) and make that effective before the
///    invalidation (TASK/PROC08; comment in the body).
/// 3. `msr ttbr1_el1, <new_l0_pa>` — install the new high-half root.
/// 4. `tlbi vmalle1` — drop any stale Stage-1 translations.
/// 5. `dsb ish` — gate the TLBI.
/// 6. `isb` — refetch instructions under the new translation regime.
///
/// `MAIR_EL1` / `TCR_EL1` / `SCTLR_EL1` are **deliberately
/// untouched**: Stage 1 already programmed them with the values
/// Stage 2 wants, and rewriting `TCR_EL1` mid-flight would introduce
/// a race window where the running PC's high-half VA is unmapped.
///
/// `TTBR0_EL1` is left on the empty root (see comment at end of
/// function); Phase 06 takes over the low-half on the first `Pexec`.
///
/// # Safety
///
/// Caller holds the BDOS giant lock or equivalent; the static
/// `L0`/`L1`/`L2` tables must have been populated by [`build`]
/// immediately before this call.
unsafe fn install_stage2_ttbr1() {
    // VA → PA for the kernel's high-half-resident L0 root. The
    // `L0` static lives in `.bss` (high-half VA), so subtracting
    // `VA_OFFSET` yields its physical load address — which is what
    // `TTBR1_EL1` consumes.
    let new_l0_pa: u64 = va_to_pa((&raw const L0) as u64);
    // Replace the Stage-1 boot identity map in TTBR0 with an all-invalid
    // root. Its 1-GiB UXN L1 blocks could otherwise be speculatively
    // (re-)cached for the low-half, and the first per-process eret-to-EL0
    // fetch would hit the stale UXN entry (real-HW Instruction-Abort
    // permission fault, level 1). A `tlbi` at install time can't fix it —
    // it wedges QEMU before eret-to-EL0 (QUIRK-001) — so we drop the
    // offending mapping here, in pure EL1 boot context. Safe because the
    // kernel now reaches all MMIO through the high-half (drivers::mmio_va),
    // so it never touches the low-half identity. See `EMPTY_L0`.
    let empty_pa: u64 = empty_l0_pa();

    // Stage-2 high-half install + low-half drop + TLB sync. Writing TTBR0
    // only affects low-half translation; the PC runs in the high-half
    // (TTBR1), so this is race-free.
    //
    // The empty TTBR0 is made effective (`isb`) before the `tlbi`
    // (TASK/PROC08): a TTBR0_EL1 write is not observed by translation
    // before a Context synchronization event (ARM ARM DDI 0487 M.d
    // `IRBMTM`, D8-7700), so without it a speculative walk after the `tlbi`
    // could still start from the boot identity root and cache its 1-GiB UXN
    // blocks (`RSQBCS`, D8-7852) — the very entries this switch drops, and
    // nothing invalidates them before the first kernel launch's
    // `install_ttbr0` (no TLBI). The `tlbi` completes after `dsb` + Context
    // synchronization event (`IBLDZX`, D8-7875). The kernel keeps running
    // on the boot TTBR1 across that first `isb`; the TTBR1 write follows
    // in the old order (`msr` then `tlbi`), unchanged.
    unsafe {
        core::arch::asm!(
            "dsb ishst",
            "msr ttbr0_el1, {ttbr0}",
            "isb",
            "msr ttbr1_el1, {ttbr1}",
            "tlbi vmalle1",
            "dsb ish",
            "isb",
            ttbr1 = in(reg) new_l0_pa,
            ttbr0 = in(reg) empty_pa,
            options(nostack, preserves_flags),
        );
    }
    // TTBR0_EL1 now maps nothing: low-half VAs raise a translation fault
    // until Phase 06's per-process `install_ttbr0` swaps in a slot's
    // tables. The kernel runs wholly in the high-half, so it never touches
    // the low-half in between. (We point at a real zero table rather than
    // `TTBR0 = 0`, which would walk physical 0 and stall QEMU.)
}

// ─────────────────────────────────────────────────────────────────────
// Public entry
// ─────────────────────────────────────────────────────────────────────

/// Build the full Stage-2 page tables from `map` and atomically
/// install them into `TTBR1_EL1`, clearing `TTBR0_EL1`.
///
/// Must be called exactly once, on the boot CPU, while Stage-1 boot
/// tables are still active (i.e. directly after the `kernel_main`
/// entry shim hands control to Rust). After this returns, kernel
/// code continues to execute at the same high-half VAs but those
/// VAs now resolve through the richer 2-MiB classification (DRAM
/// Normal Cacheable, MMIO Device-nGnRnE, stack guard invalid).
///
/// # Safety
///
/// Caller must guarantee single-CPU, post-Stage-1 state: this
/// function mutates the static page tables and rewrites
/// `TTBR0_EL1` / `TTBR1_EL1`. Concurrent observation by another CPU
/// or an exception handler is undefined behaviour.
pub unsafe fn reinstall(map: &MemoryMap) {
    build(map);
    // SAFETY: build() has just populated the static tables; we are
    // the sole CPU running at this point.
    unsafe { install_stage2_ttbr1() };
}
