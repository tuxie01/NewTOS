//! Physical memory map and ST-RAM / TT-RAM pool boundaries.
//!
//! # Background — why two pools on uniform RAM
//!
//! Atari TOS programs call `Mxalloc(size, mode)` (GEMDOS 0x44) with a
//! mode flag that distinguishes ST-RAM (mode 0) from TT-RAM (mode 1),
//! plus two preference modes (2, 3). On the original Atari TT/Falcon
//! the two pools were physically distinct: ST-RAM was the chip-shared
//! memory below 14 MiB, TT-RAM was an extension board with CPU-only
//! access.
//!
//! AArch64 / RPi4 / QEMU `virt` all have uniform DRAM. We could
//! collapse the distinction, but source-ported TOS programs would
//! break:
//!   * Programs that probe with `Mxalloc(-1, 0)` to discover available
//!     ST-RAM would see "all of RAM" and over-allocate.
//!   * Programs that pass `MX_STRAM` for hardware-shared buffers
//!     (sound, video) could end up with addresses outside the
//!     low-memory region they assume.
//!   * Programs that use `MX_TTRAM` for "give me lots, fast" would
//!     get the same pool as `MX_STRAM`, defeating the hint.
//!
//! Decision (ADR 2026-05-03): NewTOS keeps the pools logically
//! separate **and additive**: the ST-RAM slot is a fixed 16 MiB
//! region at the low end of DRAM (matching the Atari address space
//! `$000000–$FFFFFF`), and the TT-RAM slot is an *additional*
//! 256 MiB region above it. The 2 MiB at the top of the ST-RAM slot
//! (`$E00000–$FFFFFF` on Atari, the cartridge / hardware-register
//! area) are excluded from the pool so the pool size matches the
//! 14 MiB maximum a real Atari TT could expose. The total managed
//! DRAM is therefore 272 MiB, not 256 MiB. Both pools are served
//! from the same physical allocator (Phase 03 D4) but are queried
//! separately.
//!
//! # Sources
//!
//! * **QEMU `virt`**: `qemu/hw/arm/virt.c`, `static const MemMapEntry
//!   base_memmap[]`. RAM at 0x4000_0000, GIC at 0x0800_0000, PL011 at
//!   0x0900_0000.
//! * **BCM2711 / Raspberry Pi 4**: BCM2711 ARM Peripherals datasheet
//!   §1.2 "Address map", and Raspberry Pi 4 boot documentation
//!   (kernel8.img loaded at physical 0x0000_8000 in 64-bit mode).
//! * **EmuTOS `Mxalloc`**: `bdos/umem.c` (extracted in
//!   `docs/api/gemdos/memory/44-mxalloc.md`).

#![allow(dead_code)] // consumed by frame allocator and Mxalloc in D4/D6.

/// ST-RAM address-space slot size on the Atari (`$000000–$FFFFFF`,
/// 16 MiB). NewTOS reserves the same low-DRAM window so that the
/// ST-RAM pool sits at "Atari-typical" physical offsets relative to
/// `DRAM_START`.
pub const STRAM_SLOT_SIZE: u64 = 16 * 1024 * 1024;

/// Top-of-slot reserve mirroring the Atari hardware-register window
/// (`$E00000–$FFFFFF`). Excluded from the pool so the maximum pool
/// size matches the 14 MiB a real Atari TT could expose.
pub const STRAM_IO_RESERVE: u64 = 2 * 1024 * 1024;

/// Top of the ST-RAM window: `phystop` / `memtop`
/// (`STRAM_SLOT_SIZE - STRAM_IO_RESERVE` = 14 MiB above `DRAM_START`),
/// matching the Atari TT/Falcon maximum.
pub const STRAM_POOL_MAX: u64 = STRAM_SLOT_SIZE - STRAM_IO_RESERVE;

/// The OS's fixed budget at the bottom of the ST-RAM window: everything
/// below `DRAM_START + STRAM_OS_RESERVE` belongs to the kernel, and the
/// GEMDOS ST-RAM pool starts exactly there. This is `membot` / `end_os`
/// (`references/emutos/bios/biosmem.c:66-68`) — but as a **constant**,
/// not as "wherever the image happens to end".
///
/// Why constant: the pool used to begin at `__kernel_image_end`, so its
/// size moved with the kernel. Two `ALIGN(2M)` in the linker scripts
/// quantised that, and one reachable 2-MiB static was enough to take
/// 2 MiB away from every program (the 5 MiB → 3 MiB drop of 2026-09-04,
/// `docs/analysis/memory-layout-and-reported-free-memory.md` §13). A
/// program cannot build on a number like that.
///
/// The budget holds the pre-MMU boot stack (512 KiB), the kernel image
/// (`.text`/`.rodata`/`.data`/`.bss`) and the kernel stack with its guard
/// page. The `.dma` block and the kernel heap live outside it — in the
/// `$E00000` hardware window and in the top-of-DRAM OS reserve. The
/// linker scripts `ASSERT` that the image fits, so outgrowing the budget
/// is a **link error**, never a quietly smaller ST-RAM.
pub const STRAM_OS_RESERVE: u64 = 4 * 1024 * 1024;

/// Size of the GEMDOS ST-RAM pool. Constant by construction:
/// `memtop - membot` = 14 MiB - 4 MiB = **10 MiB**, on every board and
/// every build.
pub const STRAM_POOL_BYTES: u64 = STRAM_POOL_MAX - STRAM_OS_RESERVE;

/// Largest span of physical memory a single pool can manage.
///
/// The frame allocator keeps one statically sized bitmap per pool
/// (`frames::MAX_FRAMES_PER_POOL` is derived from this constant), so this
/// is one of the two points at which a truthful "there is more RAM"
/// answer stops being usable (the other is [`TOS_MD_MAX_END`]).
/// [`discover`] caps the TT-RAM pool here and records the remainder in
/// [`MemoryMap::dram_unmanaged`] instead of letting `frames::init` abort
/// the boot.
///
/// 1 GiB → 262 144 frames → 32 KiB of bitmap per pool. Raising it costs
/// 32 KiB of `.bss` per additional GiB **per pool**; note that BDOS
/// cannot describe a pool of 2 GiB or more anyway, because a TOS memory
/// descriptor's length is a signed 32-bit `LONG` (EmuTOS limits its own
/// alt-RAM detection the same way, `bios/memory2.c:56-59`).
pub const POOL_MAX_BYTES: u64 = 1024 * 1024 * 1024;

/// Highest exclusive end address a BDOS memory pool may have.
///
/// A TOS memory descriptor stores `m_start` and `m_length` as a signed
/// 32-bit `LONG`, and BDOS computes `m_start + m_length`, so every byte a
/// pool describes must live below `0x8000_0000`
/// (`tos-api/src/bdos/mem/init.rs::seed_md` rejects anything else). This
/// is the ceiling of the API contract, not our limitation: EmuTOS
/// truncates its own alt-RAM detection for the same reason — *"We
/// artificially limit TT-RAM detection to positive addresses to avoid
/// shocking BDOS"* (`references/emutos/bios/memory2.c:56-59`) — and keeps
/// booting rather than failing. [`discover`] does the same.
///
/// 4-KiB-aligned so the truncated pool still ends on a frame boundary.
pub const TOS_MD_MAX_END: u64 = 0x7FFF_F000;

/// Bytes reserved for the **kernel heap**, taken off the top of DRAM
/// before the pools are formed.
///
/// EmuTOS reserves OS buffers the same way: `balloc_stram(size, TRUE)`
/// lowers `memtop` below `phystop` and hands `getmpb` only what is left
/// (`references/emutos/bios/biosmem.c:88-131,134-151`), so the screen
/// buffer and the other BIOS allocations are never part of a pool and
/// never appear in a `Malloc` answer. NewTOS does the same for the two
/// buffers it needs on this machine: the kernel heap and — on
/// `board-qemu-virt`, where the framebuffer is ordinary DRAM — the
/// framebuffer.
///
/// Must be a multiple of `frames::FRAME_SIZE` so the pool below it stays
/// frame-aligned. 2048 frames = 8 MiB (MEM05, decision E2: the MD supply
/// and its tables grow in the kernel heap, at most 2 MiB of it), matching
/// [`crate::bios::machine::heap::HEAP_BYTES`] (asserted there). A whole
/// number of 2-MiB steps, so the edge below it stays 2-MiB aligned.
pub const KERNEL_HEAP_RESERVE: u64 = 8 * 1024 * 1024;

/// Bytes reserved for the framebuffer store on this board. Non-zero only
/// where the framebuffer is ordinary ARM DRAM (`board-qemu-virt`); on the
/// Pi 4 the VideoCore owns it and the reservation is zero.
pub const FRAMEBUFFER_RESERVE: u64 = board::FRAMEBUFFER_RESERVE;

/// Total bytes taken off the top of DRAM for the fixed kernel-owned buffers
/// (heap, framebuffer). The block read cache comes on top of this, sized at
/// boot ([`discover`]'s `block_cache_bytes`).
pub const OS_RESERVE_BYTES: u64 = KERNEL_HEAP_RESERVE + FRAMEBUFFER_RESERVE;

/// Granule of the block-cache reserve (FS09): 2 MiB. The reserve lowers the
/// top of the TT-RAM pool by its size; whole 2-MiB steps keep that edge on the
/// 2-MiB alignment it had before, so no 2-MiB region the user tables cover can
/// straddle pool and cache (`tos-api` `bdos::process::mmu::map_pools`).
pub const BLOCK_CACHE_GRANULE: u64 = 2 * 1024 * 1024;

/// Inclusive-exclusive physical address range.
#[derive(Clone, Copy, Debug)]
pub struct PhysRange {
    /// Inclusive start of the range.
    pub start: u64,
    /// Exclusive end of the range.
    pub end: u64,
}

impl PhysRange {
    /// Construct a half-open `[start, end)` range.
    pub const fn new(start: u64, end: u64) -> Self {
        Self { start, end }
    }
    /// Length in bytes.
    pub const fn len(&self) -> u64 {
        self.end - self.start
    }
    /// `true` if the range covers no bytes.
    pub const fn is_empty(&self) -> bool {
        self.start >= self.end
    }
    /// Half-open containment: `start <= addr < end`.
    pub const fn contains(&self, addr: u64) -> bool {
        addr >= self.start && addr < self.end
    }
}

/// Resolved board memory map. Constructed at boot by [`discover`].
#[derive(Clone, Debug)]
pub struct MemoryMap {
    /// The kernel image (`.text`, `.rodata`, `.data`, `.bss`, plus a
    /// 4-KiB-aligned tail). Never allocatable.
    pub kernel: PhysRange,
    /// Logical ST-RAM pool. Up to `STRAM_POOL_MAX`, immediately above
    /// the kernel image. Empty if total DRAM is too small.
    pub stram: PhysRange,
    /// Logical TT-RAM pool. The DRAM above the ST-RAM slot, up to the
    /// start of [`MemoryMap::os_reserve`] — kernel-owned buffers are
    /// **not** part of it.
    pub ttram: PhysRange,
    /// Kernel-owned buffers, carved off the **top** of DRAM before the
    /// pools are formed, so they are outside ST-RAM and TT-RAM alike.
    /// The NewTOS equivalent of EmuTOS' `balloc_stram(size, TRUE)`
    /// lowering `memtop` below `phystop`
    /// (`references/emutos/bios/biosmem.c:88-131`). Contains
    /// [`MemoryMap::kernel_heap`], [`MemoryMap::framebuffer`] and
    /// [`MemoryMap::block_cache`].
    pub os_reserve: PhysRange,
    /// Arena for the kernel heap, in [`MemoryMap::os_reserve`] above the
    /// block cache.
    pub kernel_heap: PhysRange,
    /// Framebuffer store, the high part of [`MemoryMap::os_reserve`].
    /// Empty on `board-rpi4`, where the VideoCore owns the framebuffer.
    pub framebuffer: PhysRange,
    /// Arena of the block read cache (FS09,
    /// `docs/analysis/fs09-block-cache-2026-10-07.md` §2.4), the **low** part
    /// of [`MemoryMap::os_reserve`], below the heap — so heap and framebuffer
    /// keep the addresses they had without a cache. Empty when no cache was
    /// asked for (`board-qemu-virt`, `bcache=0`).
    pub block_cache: PhysRange,
    /// The full DRAM range the kernel manages. Equal to what
    /// [`MemoryMap::dram_source`] reported, minus any tail the TT-RAM
    /// pool cannot cover (see [`MemoryMap::dram_unmanaged`]).
    pub dram: PhysRange,
    /// Which authority the DRAM extent came from.
    pub dram_source: DramSource,
    /// Bytes of reported DRAM above [`MemoryMap::dram`] that this build
    /// cannot manage, because the TT-RAM pool is bounded by both
    /// [`POOL_MAX_BYTES`] (frame bitmap) and [`TOS_MD_MAX_END`] (TOS
    /// memory-descriptor ABI). Zero on every machine whose report fits.
    /// Reported on the boot console so the loss is never silent.
    pub dram_unmanaged: u64,
    /// Stack guard (Phase 03 D6). One 4-KiB page immediately below the
    /// kernel stack, left invalid in the page tables so a stack overflow
    /// takes a synchronous data abort instead of silently corrupting the
    /// memory below the stack.
    ///
    /// One *page*, not one 2-MiB block: `mmu::refine_stack_block` gives
    /// the containing block an L3 table so the hole can be page-sized.
    /// That is what lets the kernel stack fit inside
    /// [`STRAM_OS_RESERVE`] and the ST-RAM pool have a fixed size.
    pub stack_guard: PhysRange,
    /// Memory-mapped I/O regions. Marked Device-nGnRnE in the page
    /// tables (Phase 03 D2).
    pub mmio: &'static [PhysRange],
}

impl MemoryMap {
    /// Total bytes available for `Malloc`/`Mxalloc` across both pools.
    pub const fn total_free(&self) -> u64 {
        self.stram.len() + self.ttram.len()
    }
}

// ─────────────────────────────────────────────────────────────────────
// Board-specific physical layouts
// ─────────────────────────────────────────────────────────────────────

#[cfg(feature = "board-qemu-virt")]
mod board {
    use super::{PhysRange, STRAM_SLOT_SIZE};

    /// QEMU `virt` machine DRAM start. Source: `qemu/hw/arm/virt.c`,
    /// `base_memmap[VIRT_MEM]`.
    pub const DRAM_START: u64 = 0x4000_0000;
    /// TT-RAM slot size: 256 MiB, matching the Atari-style
    /// "TT extension board" sitting above the 16 MiB ST-RAM slot.
    pub const TTRAM_SLOT_SIZE: u64 = 256 * 1024 * 1024;
    /// Total DRAM = ST-RAM slot (16 MiB) + TT-RAM slot (256 MiB) =
    /// 272 MiB. xtask passes `-m 272M` to QEMU to match.
    pub const DRAM_SIZE: u64 = STRAM_SLOT_SIZE + TTRAM_SLOT_SIZE;
    pub const DRAM_END: u64 = DRAM_START + DRAM_SIZE;

    /// Bytes reserved for the **ramfb framebuffer**, off the top of DRAM.
    ///
    /// On QEMU `virt` the framebuffer is ordinary DRAM that the device
    /// model scans out (`-device ramfb`), so it has to come from
    /// somewhere. Taking it out of the pool — as this reservation does —
    /// is the only placement that keeps the promise "no framebuffer in
    /// ST-RAM or TT-RAM"; allocating it *from* `Pool::Ttram` (what NewTOS
    /// did until 2026-09-04) left a framebuffer sitting inside the TT-RAM
    /// pool extent and inside the number the boot screen reports.
    ///
    /// Sized for the boot geometry, rounded up to a whole 4-KiB frame.
    /// `ramfb` asserts at compile time that its buffer fits here.
    pub const FRAMEBUFFER_RESERVE: u64 = {
        let bytes = (crate::drivers::BOOT_WIDTH as u64) * (crate::drivers::BOOT_HEIGHT as u64) * 4;
        (bytes + 4095) & !4095
    };

    /// MMIO regions we touch in Phase 02. More entries (RTC, PCIe,
    /// virtio-mmio) will be added in later phases.
    pub static MMIO: &[PhysRange] = &[
        // GICv2 distributor + CPU interface. Source: `virt.c`,
        // `VIRT_GIC_DIST`/`VIRT_GIC_CPU`.
        PhysRange::new(0x0800_0000, 0x0801_0000),
        PhysRange::new(0x0801_0000, 0x0802_0000),
        // PL011 UART. Source: `virt.c`, `VIRT_UART`.
        PhysRange::new(0x0900_0000, 0x0900_1000),
        // PCIe 32-bit MMIO window (xHCI BAR lands here). Source: `virt.c`,
        // `VIRT_PCIE_MMIO` = { 0x10000000, 0x2eff0000 }. The PCIe **ECAM**
        // is *not* here — on QEMU `virt` it is a high address (read from the
        // DTB + mapped with `mmu::map_device_high`, PHASE-16 D1).
        PhysRange::new(0x1000_0000, 0x3eff_0000),
    ];
}

#[cfg(feature = "board-rpi4")]
mod board {
    use super::PhysRange;

    /// RPi4 DRAM starts at physical 0. Source: BCM2711 ARM Peripherals
    /// §1.2.
    pub const DRAM_START: u64 = 0x0000_0000;
    /// We treat 1 GiB as the lower bound for usable kernel RAM on any
    /// RPi4 SKU. Real boards have 2/4/8 GiB; the allocator (D4) will
    /// trim or extend at runtime once it queries the actual size from
    /// the firmware. For D1 we hard-cap conservatively.
    pub const DRAM_SIZE: u64 = 1024 * 1024 * 1024;
    pub const DRAM_END: u64 = DRAM_START + DRAM_SIZE;

    /// No framebuffer reservation on the Pi 4: the VideoCore owns the
    /// framebuffer and allocates it from **its** share of SDRAM, which is
    /// mapped in from `0x0_4000_0000` downwards and sized by `gpu_mem`
    /// (BCM2711 ARM Peripherals §1.2.3). The firmware's "Get ARM memory"
    /// answer already ends below it, so the framebuffer is outside
    /// `dram` — and therefore outside both pools — by construction.
    pub const FRAMEBUFFER_RESERVE: u64 = 0;

    /// MMIO regions. Sources: BCM2711 ARM Peripherals §1.2 (low
    /// peripherals at 0xFE00_0000) and §6 (ARM-local + GIC at
    /// 0xFF80_0000).
    pub static MMIO: &[PhysRange] = &[
        // BCM2711 PCIe root-complex registers (brcmstb @ 0xFD50_0000, ~0x9310
        // long). Needed for the VL805 xHCI bring-up (PHASE-16 D7,
        // `docs/api/usb/pcie-rpi4.md`). The VL805 xHCI BAR itself is in the
        // PCIe outbound window at 0x6_0000_0000 — mapped dynamically via
        // `mmu::map_device_high` (a >4 GiB address), like the QEMU ECAM.
        PhysRange::new(0xFD50_0000, 0xFD51_0000),
        // Low peripherals (GPIO, UART, AUX, etc.).
        PhysRange::new(0xFE00_0000, 0xFF00_0000),
        // ARM-local + GICv2 distributor/CPU interface.
        PhysRange::new(0xFF80_0000, 0x1_0000_0000),
    ];
}

// Compile-time guard: exactly one board feature must be active.
#[cfg(not(any(feature = "board-qemu-virt", feature = "board-rpi4")))]
compile_error!("kernel/mm/layout: enable exactly one board feature");

#[cfg(all(feature = "board-qemu-virt", feature = "board-rpi4"))]
compile_error!("kernel/mm/layout: features board-qemu-virt and board-rpi4 are mutually exclusive");

// ─────────────────────────────────────────────────────────────────────
// User addresses below 2 GiB (TASK/MEM11, user decision 2026-10-31)
// ─────────────────────────────────────────────────────────────────────

/// Exclusive end of the DRAM [`discover`] manages, for a TT-RAM slot that
/// starts at `ttram_start`: the lower of the two ceilings above a truthful
/// "there is more RAM" answer — the frame bitmap ([`POOL_MAX_BYTES`]) and the
/// TOS memory-descriptor ABI ([`TOS_MD_MAX_END`]).
///
/// Every address the kernel can hand a program from DRAM lies below it: both
/// pools, and everything `usermem::expose` can publish (the kernel image and
/// its statics, `.dma`, the kernel heap, the block cache and — on
/// `board-qemu-virt` — the framebuffer all lie inside the capped `dram`).
const fn ttram_cap_end_for(ttram_start: u64) -> u64 {
    let by_bitmap = ttram_start.saturating_add(POOL_MAX_BYTES);
    if by_bitmap < TOS_MD_MAX_END { by_bitmap } else { TOS_MD_MAX_END }
}

// The 2-GiB contract is held by placement only — the user tables could map
// up to 4 GiB (`tos-api` `bdos::process::mmu::USER_VA_LIMIT`), and the
// enforcement at the mapping boundary is parked (TASK/MEM11). A layout
// change that would let a user address reach `USER_PTR_END` (a TOS address is
// a signed 32-bit `LONG`; at or above `0x8000_0000` it reads as a negative
// error code) must fail the build on both boards instead of reopening the gap
// silently. The host-side half: `tos-api/tests/mem11_layout_below_2gib.rs`.
const _: () = assert!(
    TOS_MD_MAX_END <= tos_api::bdos::process::usermem::USER_PTR_END,
    "MEM11: TOS_MD_MAX_END lets a pool reach 0x8000_0000"
);
const _: () = assert!(
    ttram_cap_end_for(board::DRAM_START + STRAM_SLOT_SIZE)
        <= tos_api::bdos::process::usermem::USER_PTR_END,
    "MEM11: the managed DRAM can reach 0x8000_0000"
);

// ─────────────────────────────────────────────────────────────────────
// Linker-provided kernel image symbols
// ─────────────────────────────────────────────────────────────────────

unsafe extern "C" {
    static __kernel_image_start: u8;
    static __kernel_image_end: u8;
    static __stack_guard_start: u8;
    static __stack_bottom_inclusive: u8;
    static __stack_top_exclusive: u8;
    static __emerg_stack_top_exclusive: u8;
}

/// Phase 05 high-half offset. The kernel image is linked at
/// `VA_OFFSET + phys`; addresses we hand to the page-table builder
/// and the frame allocator must be **physical**, so every linker
/// symbol pulled into a [`MemoryMap`] field gets converted via
/// [`va_to_pa`] below.
///
/// Mirrors the `VA_OFFSET` constant in
/// [`crate::arch::aarch64::mmu`] and in the linker scripts; any
/// drift would silently mis-classify blocks (kernel image landing
/// outside DRAM, stack guard outside its 2-MiB block, etc.).
const VA_OFFSET: u64 = 0xFFFF_0000_0000_0000;

#[inline(always)]
const fn va_to_pa(va: u64) -> u64 {
    va.wrapping_sub(VA_OFFSET)
}

#[inline]
fn kernel_image_start() -> u64 {
    va_to_pa((&raw const __kernel_image_start) as u64)
}
#[inline]
fn kernel_image_end() -> u64 {
    va_to_pa((&raw const __kernel_image_end) as u64)
}

/// Physical address of the lowest byte of the stack-guard region.
/// Equal to the start of the 2-MiB block left invalid in the page
/// tables. Exposed for `mmu::build` and the data-abort handler.
#[inline]
pub fn stack_guard_start() -> u64 {
    va_to_pa((&raw const __stack_guard_start) as u64)
}

/// Physical address one past the stack guard (= stack bottom).
#[inline]
pub fn stack_guard_end() -> u64 {
    va_to_pa((&raw const __stack_bottom_inclusive) as u64)
}

/// Top (exclusive) of the main kernel stack used in EL1 with SPSel=0.
#[inline]
pub fn stack_top_exclusive() -> u64 {
    (&raw const __stack_top_exclusive) as u64
}

/// Top (exclusive) of the emergency exception stack used as SP_EL1
/// during exception handling. Sized to hold one nested handler frame
/// plus generous diagnostic state.
#[inline]
pub fn emerg_stack_top_exclusive() -> u64 {
    (&raw const __emerg_stack_top_exclusive) as u64
}

// ─────────────────────────────────────────────────────────────────────
// Discovery
// ─────────────────────────────────────────────────────────────────────

/// Where the DRAM extent in a [`MemoryMap`] came from. Reported on the
/// boot console so a serial log always says which authority was believed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DramSource {
    /// The board firmware answered a direct query (RPi4: VideoCore
    /// mailbox property tag `0x0001_0005`, "Get ARM memory").
    Firmware,
    /// The `/memory@…` node of the device-tree blob the boot loader
    /// handed us.
    DeviceTree,
    /// Neither authority was available or plausible — the compile-time
    /// `board::DRAM_SIZE` was used.
    BoardDefault,
}

impl DramSource {
    /// Short label for the boot log.
    pub const fn label(self) -> &'static str {
        match self {
            DramSource::Firmware => "firmware",
            DramSource::DeviceTree => "device tree",
            DramSource::BoardDefault => "compiled-in board default",
        }
    }
}

/// A DRAM extent as reported by an outside authority (firmware query or
/// device tree), before validation against the board's address map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DramReport {
    /// Physical base address of the reported memory.
    pub start: u64,
    /// Reported size in bytes.
    pub size: u64,
}

/// Accept a reported extent only if it starts where this board's DRAM
/// starts and is big enough to hold the ST-RAM slot; anything else would
/// build an inconsistent map (the kernel image, the pools and the MMU
/// block classification all assume `board::DRAM_START`).
fn plausible(r: &DramReport) -> bool {
    // Room for the ST-RAM slot *and* the kernel-owned buffers taken off
    // the top ([`OS_RESERVE_BYTES`]) — otherwise the reported extent
    // cannot host a memory map at all.
    r.start == board::DRAM_START && r.size >= STRAM_SLOT_SIZE + OS_RESERVE_BYTES
}

/// Build the [`MemoryMap`] for the running board.
///
/// The DRAM extent is taken from the first authority that answers with a
/// plausible extent, in this order:
///
/// 1. `fw` — the board firmware's own answer, obtained by the caller
///    before the memory map exists (RPi4: VideoCore mailbox, see
///    [`crate::drivers::bcm2711::mailbox::get_memory`]). This is the only
///    source that knows where the *ARM-visible* memory ends, i.e. where
///    the GPU's `gpu_mem` reservation begins — allocating past that point
///    would hand VideoCore-owned DRAM to `Malloc`.
/// 2. `dtb_phys` — the `/memory@…` node of the device-tree blob the boot
///    loader supplied, or `None` when there is none (e.g. host-side unit
///    tests). Its first bank is used (Phase 03 D8).
/// 3. The compile-time `board::DRAM_SIZE` constant.
///
/// The chosen source is recorded in [`MemoryMap::dram_source`] so the
/// boot console can report it.
///
/// Pure function of board constants, linker symbols, the parsed DTB and
/// the caller-supplied firmware report. No allocation, no MMIO.
///
/// `block_cache_bytes` is the size the block read cache asks for (FS09); it is
/// rounded **up** to [`BLOCK_CACHE_GRANULE`] and capped at half of what would
/// otherwise be TT-RAM, so a large request can never take the pool away. The
/// size actually reserved is [`MemoryMap::block_cache`].
pub fn discover(dtb_phys: Option<usize>, fw: Option<DramReport>, block_cache_bytes: u64) -> MemoryMap {
    // ---- DRAM extent: firmware, then DTB, then board default. ----
    let mut dram = PhysRange::new(board::DRAM_START, board::DRAM_END);
    let mut dram_source = DramSource::BoardDefault;

    let dtb_report = dtb_phys.and_then(|phys| {
        // SAFETY: caller passes the firmware-supplied DTB pointer; the
        // parser bounds-checks the header before any further reads.
        unsafe { crate::arch::aarch64::dtb::parse_memory(phys) }
            .map(|m| DramReport { start: m.start, size: m.size })
    });

    if let Some(r) = fw.filter(plausible) {
        dram = PhysRange::new(r.start, r.start + r.size);
        dram_source = DramSource::Firmware;
    } else if let Some(r) = dtb_report.filter(plausible) {
        dram = PhysRange::new(r.start, r.start + r.size);
        dram_source = DramSource::DeviceTree;
    }

    // ---- Cap the TT-RAM pool at what we can actually manage. ----
    // Two ceilings sit above a truthful "there is more RAM" answer, and
    // neither may turn it into a boot failure:
    //   * the frame allocator's per-pool bitmap is statically sized for
    //     `POOL_MAX_BYTES` (`frames::MAX_FRAMES_PER_POOL`); and
    //   * a TOS memory descriptor cannot describe a byte at or above
    //     `TOS_MD_MAX_END` (signed 32-bit `LONG`).
    // We manage up to the lower ceiling and report the remainder as
    // unmanaged, instead of aborting in `frames::init` or
    // `bdos::mem::init`.
    let ttram_start = dram.start + STRAM_SLOT_SIZE;
    let ttram_cap_end = ttram_cap_end_for(ttram_start);
    let dram_unmanaged = dram.end.saturating_sub(ttram_cap_end);
    if dram_unmanaged > 0 {
        dram = PhysRange::new(dram.start, ttram_cap_end);
    }

    let img_start = kernel_image_start();
    let img_end = kernel_image_end();
    let kernel = PhysRange::new(img_start, img_end);

    // Sanity: kernel image must lie inside DRAM and be 4-KiB-aligned
    // at the upper edge (linker script guarantees this).
    debug_assert!(dram.contains(img_start));
    debug_assert!(img_end <= dram.end);
    debug_assert!(img_end.is_multiple_of(4096));

    // ST-RAM slot: fixed 16 MiB at DRAM_START. The pool is
    // [membot, memtop) = [DRAM_START + STRAM_OS_RESERVE,
    // DRAM_START + STRAM_POOL_MAX) — the Atari `$000000–$DFFFFF` window
    // minus the OS's fixed budget at the bottom. The top 2 MiB of the
    // slot stay out (Atari hardware-register reserve, `$E00000–$FFFFFF`),
    // and that is where `.dma` lives.
    //
    // Both edges are constants, so the pool is exactly `STRAM_POOL_BYTES`
    // on every build. The linker scripts `ASSERT` that the image and the
    // kernel stack fit below `membot`; the check here is the runtime
    // mirror of that link-time guarantee.
    let stram_slot_end = dram.start + STRAM_SLOT_SIZE;
    let stram_pool_end = dram.start + STRAM_POOL_MAX;
    let membot = dram.start + STRAM_OS_RESERVE;
    debug_assert!(
        img_end <= membot,
        "kernel image + stack overruns the ST-RAM OS reserve"
    );
    let stram = PhysRange::new(membot, stram_pool_end);
    debug_assert!(stram.len() == STRAM_POOL_BYTES);

    // Kernel-owned buffers, off the **top** of DRAM, before either pool
    // is formed — EmuTOS' `balloc_stram(size, TRUE)`, which lowers
    // `memtop` below `phystop` so `getmpb` never sees those bytes
    // (`references/emutos/bios/biosmem.c:88-131,134-151`). They stay
    // inside `dram`, so the MMU still maps them Normal-Cacheable; they
    // are simply not part of a pool, not reachable through `Malloc`, and
    // not counted in the ST-RAM/TT-RAM figures the boot screen reports.
    //
    // `dram.end` is frame-aligned (every source is at worst MiB-granular,
    // and `TOS_MD_MAX_END` is 4-KiB-aligned) and `OS_RESERVE_BYTES` is a
    // whole number of frames, so `pool_top` is frame-aligned too.
    debug_assert!(dram.end.is_multiple_of(4096));
    debug_assert!(OS_RESERVE_BYTES.is_multiple_of(4096));
    let fixed_bottom = dram.end.saturating_sub(OS_RESERVE_BYTES).max(stram_slot_end);
    // FS09 block read cache: below the fixed buffers, in 2-MiB steps, at most
    // half of what TT-RAM would otherwise be (rounded down to the granule).
    let cache_max =
        (fixed_bottom.saturating_sub(stram_slot_end) / 2) / BLOCK_CACHE_GRANULE * BLOCK_CACHE_GRANULE;
    let cache_bytes = block_cache_bytes
        .div_ceil(BLOCK_CACHE_GRANULE)
        .saturating_mul(BLOCK_CACHE_GRANULE)
        .min(cache_max);
    let pool_top = fixed_bottom - cache_bytes;
    let os_reserve = PhysRange::new(pool_top, dram.end);
    let block_cache = PhysRange::new(pool_top, fixed_bottom);
    let kernel_heap =
        PhysRange::new(fixed_bottom, fixed_bottom.saturating_add(KERNEL_HEAP_RESERVE));
    let framebuffer = PhysRange::new(
        kernel_heap.end,
        kernel_heap.end.saturating_add(FRAMEBUFFER_RESERVE),
    );
    debug_assert!(framebuffer.end <= dram.end, "OS reserve overruns DRAM");

    // TT-RAM slot: from the end of the ST-RAM slot up to the OS reserve.
    let ttram = PhysRange::new(stram_slot_end, pool_top);

    // Stack guard: one 2-MiB block whose start address is provided by
    // the linker. Lives inside the kernel image, so it sits in DRAM
    // and would otherwise be classified as Normal Cacheable; the MMU
    // builder consults `MemoryMap::stack_guard` and forces the block
    // to invalid.
    let guard_start = stack_guard_start();
    let guard_end = stack_guard_end();
    debug_assert!(guard_end - guard_start == 4096, "stack guard is one page");
    debug_assert!(guard_start.is_multiple_of(4096));
    let stack_guard = PhysRange::new(guard_start, guard_end);

    MemoryMap {
        kernel,
        stram,
        ttram,
        os_reserve,
        kernel_heap,
        framebuffer,
        block_cache,
        dram,
        dram_source,
        dram_unmanaged,
        stack_guard,
        mmio: board::MMIO,
    }
}

// ─────────────────────────────────────────────────────────────────────
// Tests (host-runnable: pure-logic unit tests for PhysRange).
// ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_contains_is_half_open() {
        let r = PhysRange::new(10, 20);
        assert!(r.contains(10));
        assert!(r.contains(19));
        assert!(!r.contains(20));
        assert!(!r.contains(9));
    }

    #[test]
    fn empty_range() {
        let r = PhysRange::new(42, 42);
        assert!(r.is_empty());
        assert_eq!(r.len(), 0);
    }
}
