//! GICv2 driver — Distributor + CPU Interface.
//!
//! Both target boards use a GICv2-compatible interrupt controller:
//! * QEMU `aarch64-virt`: GICv2 emulation
//! * Raspberry Pi 4 (BCM2711): ARM GIC-400 (GICv2 implementation)
//!
//! Phase 02 D5 needs only the bare minimum to wire PL011 RX through:
//!   * unmask one SPI,
//!   * route it to CPU 0,
//!   * acknowledge / EOI in the IRQ handler.
//!
//! References:
//!   - ARM IHI 0048B "ARM Generic Interrupt Controller Architecture
//!     Specification, version 2.0"
//!   - ARM DDI 0471B "ARM GIC-400 Technical Reference Manual"
//!
//! Out of scope for PHASE-02: priority groups, banked registers,
//! security extensions, SGI / PPI handling, SMP affinity routing.
//! Those land in PHASE-03+.

// ---------------------------------------------------------------------------
// Distributor (GICD) register offsets — ARM IHI 0048B §4.3
// ---------------------------------------------------------------------------

const GICD_CTLR: usize = 0x000;
const GICD_ISENABLER: usize = 0x100;
const GICD_ICENABLER: usize = 0x180;
const GICD_IPRIORITYR: usize = 0x400;
const GICD_ITARGETSR: usize = 0x800;
const GICD_ICFGR: usize = 0xC00;

// ---------------------------------------------------------------------------
// CPU Interface (GICC) register offsets — ARM IHI 0048B §4.4
// ---------------------------------------------------------------------------

const GICC_CTLR: usize = 0x000;
const GICC_PMR: usize = 0x004;
const GICC_IAR: usize = 0x00C;
const GICC_EOIR: usize = 0x010;

/// `GICC_IAR.INTID` mask. Bits [9:0] hold the interrupt ID; bits [12:10]
/// the source CPU ID for SGIs (we ignore that here).
pub const IAR_INTID_MASK: u32 = 0x3FF;
/// Spurious interrupt ID returned when no IRQ is pending.
pub const SPURIOUS_INTID: u32 = 1023;

/// GICv2 driver instance. Holds the two MMIO base addresses; safe to share
/// across cores (volatile-only access).
pub struct GicV2 {
    gicd: usize,
    gicc: usize,
}

impl GicV2 {
    /// Construct a driver bound to the given Distributor / CPU-interface bases.
    pub const fn new(gicd_base: usize, gicc_base: usize) -> Self {
        Self {
            gicd: gicd_base,
            gicc: gicc_base,
        }
    }

    #[inline(always)]
    fn d_reg(&self, off: usize) -> *mut u32 {
        (self.gicd + off) as *mut u32
    }

    #[inline(always)]
    fn c_reg(&self, off: usize) -> *mut u32 {
        (self.gicc + off) as *mut u32
    }

    /// Initialize the GIC: disable everything, then bring up Distributor
    /// and CPU interface with priority mask wide open.
    pub fn init(&self) {
        // SAFETY: GIC MMIO is valid on both target boards once the firmware
        // has handed control to the kernel.
        unsafe {
            // Disable both halves while we configure them.
            self.d_reg(GICD_CTLR).write_volatile(0);
            self.c_reg(GICC_CTLR).write_volatile(0);

            // PMR = 0xFF accepts every priority (lowest threshold).
            self.c_reg(GICC_PMR).write_volatile(0xFF);

            // Re-enable.
            self.d_reg(GICD_CTLR).write_volatile(1);
            self.c_reg(GICC_CTLR).write_volatile(1);
        }
    }

    /// Enable a Shared Peripheral Interrupt (SPI), set its priority,
    /// route it to CPU 0, and configure it as level-sensitive (PL011 RX
    /// is level-triggered).
    pub fn enable_spi(&self, intid: u32) {
        let bit = intid & 0x1F;
        let reg = (intid >> 5) as usize;
        let byte = (intid & 0x3) as usize;
        let pri_reg = (intid >> 2) as usize;
        let cfg_bit = ((intid & 0xF) * 2) as usize;
        let cfg_reg = (intid >> 4) as usize;

        // SAFETY: see `init`.
        unsafe {
            // Priority 0xA0 — middle of the range, well below the PMR.
            let p_addr = (self.gicd + GICD_IPRIORITYR + pri_reg * 4) as *mut u32;
            let mut p = p_addr.read_volatile();
            p &= !(0xFFu32 << (byte * 8));
            p |= 0xA0u32 << (byte * 8);
            p_addr.write_volatile(p);

            // Target = CPU 0 (bit 0 of the relevant byte).
            let t_addr = (self.gicd + GICD_ITARGETSR + pri_reg * 4) as *mut u32;
            let mut t = t_addr.read_volatile();
            t &= !(0xFFu32 << (byte * 8));
            t |= 0x01u32 << (byte * 8);
            t_addr.write_volatile(t);

            // Configure level-sensitive: ICFGR uses 2 bits per IRQ;
            // 0b00 = level, 0b10 = edge.
            let cfg_addr = (self.gicd + GICD_ICFGR + cfg_reg * 4) as *mut u32;
            let mut c = cfg_addr.read_volatile();
            c &= !(0b11u32 << cfg_bit);
            cfg_addr.write_volatile(c);

            // Enable.
            let en_addr = (self.gicd + GICD_ISENABLER + reg * 4) as *mut u32;
            en_addr.write_volatile(1u32 << bit);
        }
    }

    /// Enable a Private Peripheral Interrupt (PPI, INTID 16..=31) on this
    /// CPU interface and give it the same priority as the SPIs above.
    ///
    /// For INTIDs 0..31 the Distributor's `GICD_ISENABLER0` and
    /// `GICD_IPRIORITYR` are banked per CPU interface (ARM IHI 0048B §4.3),
    /// so writing them here configures the calling core — which is what a
    /// *private* peripheral interrupt needs. Unlike [`Self::enable_spi`]
    /// this writes neither `GICD_ITARGETSR` (a PPI is by definition wired to
    /// its own core; there is nothing to route) nor `GICD_ICFGR` (the
    /// generic timer asserts a level, and the PPI configuration is fixed by
    /// the implementation).
    pub fn enable_ppi(&self, intid: u32) {
        let byte = (intid & 0x3) as usize;
        let pri_reg = (intid >> 2) as usize;

        // SAFETY: see `init`.
        unsafe {
            // Priority 0xA0, as for the SPIs — well below the PMR (0xFF).
            let p_addr = (self.gicd + GICD_IPRIORITYR + pri_reg * 4) as *mut u32;
            let mut p = p_addr.read_volatile();
            p &= !(0xFFu32 << (byte * 8));
            p |= 0xA0u32 << (byte * 8);
            p_addr.write_volatile(p);

            // ISENABLER is write-one-to-set: no read-modify-write.
            let en_addr = (self.gicd + GICD_ISENABLER) as *mut u32;
            en_addr.write_volatile(1u32 << (intid & 0x1F));
        }
    }

    /// Disable a Shared Peripheral Interrupt.
    #[allow(dead_code)]
    pub fn disable_spi(&self, intid: u32) {
        let bit = intid & 0x1F;
        let reg = (intid >> 5) as usize;
        // SAFETY: see `init`.
        unsafe {
            let dis_addr = (self.gicd + GICD_ICENABLER + reg * 4) as *mut u32;
            dis_addr.write_volatile(1u32 << bit);
        }
    }

    /// Acknowledge the highest-priority pending interrupt and return its
    /// ID. Returns [`SPURIOUS_INTID`] if nothing is pending.
    pub fn acknowledge(&self) -> u32 {
        // SAFETY: see `init`.
        unsafe { self.c_reg(GICC_IAR).read_volatile() & IAR_INTID_MASK }
    }

    /// Signal end-of-interrupt for `intid` (must be exactly the value
    /// returned by [`Self::acknowledge`]).
    pub fn end_of_interrupt(&self, intid: u32) {
        // SAFETY: see `init`.
        unsafe { self.c_reg(GICC_EOIR).write_volatile(intid) };
    }
}

// ---------------------------------------------------------------------------
// Top-level IRQ entry — called from the AArch64 IRQ exception vector.
// ---------------------------------------------------------------------------

/// IRQ entry point. Acknowledges the IRQ, dispatches to the board's
/// peripheral handler, then EOIs.
///
/// After a timer tick it runs the INPUT01 input pump
/// ([`crate::host_env::irq_input_pump`]) — which itself pumps only when the
/// tick interrupted a program at EL0 — after the EOI, still with IRQs masked
/// (the exception entry masked them; nothing on the pump path opens them).
#[unsafe(no_mangle)]
pub extern "C" fn rust_irq_handler(frame: &mut crate::arch::aarch64::exceptions::ExceptionFrame) {
    let gic = board_gic();
    let intid = gic.acknowledge();

    if intid == SPURIOUS_INTID {
        return;
    }

    let tick = intid == crate::drivers::timer::TIMER_INTID;
    if intid == BOARD_PL011_INTID {
        // Drain the PL011 RX FIFO; `serial_rx_byte` routes each byte
        // (qemu-virt: `Iorec(0)`'s input ring, rpi4: dropped).
        crate::console::pl011_rx_drain();
    } else if tick {
        // 200 Hz system tick — re-arm and advance `_hz_200`.
        crate::drivers::timer::handle_irq();
    }
    // Unknown / unrouted interrupts are silently EOIed; PHASE-03 will
    // add a registration table for future peripherals.

    gic.end_of_interrupt(intid);

    if tick {
        crate::host_env::irq_input_pump(frame.spsr);
        // The VT52 text cursor's blink (EmuTOS: `blink()` per VBL,
        // `bios/vectors.S:355`); a no-op unless a flashing cursor is shown.
        crate::fbcon::blink_tick();
    }
}

// ---------------------------------------------------------------------------
// Per-board addresses and interrupt IDs
// ---------------------------------------------------------------------------

#[cfg(feature = "board-qemu-virt")]
mod board {
    /// QEMU `aarch64-virt` GIC distributor base. Source: QEMU
    /// `hw/arm/virt.c`, `[VIRT_GIC_DIST] = { 0x08000000, 0x10000 }`.
    pub const GICD_BASE: usize = crate::drivers::mmio_va(0x0800_0000);
    /// QEMU `aarch64-virt` GIC CPU-interface base. Source: same file,
    /// `[VIRT_GIC_CPU] = { 0x08010000, 0x10000 }`.
    pub const GICC_BASE: usize = crate::drivers::mmio_va(0x0801_0000);
    /// PL011 INTID on `aarch64-virt`: SPI 1 → INTID 32 + 1 = 33.
    /// Source: QEMU `hw/arm/virt.c`, `[VIRT_UART] = ... ARM_SPI(1)`.
    pub const PL011_INTID: u32 = 33;
}

#[cfg(feature = "board-rpi4")]
mod board {
    /// BCM2711 GIC-400 distributor base (low-peripheral mode).
    /// Source: BCM2711 ARM Peripherals datasheet §6.3.
    pub const GICD_BASE: usize = crate::drivers::mmio_va(0xFF84_1000);
    /// BCM2711 GIC-400 CPU-interface base.
    pub const GICC_BASE: usize = crate::drivers::mmio_va(0xFF84_2000);
    /// PL011 (UART0) INTID on BCM2711: SPI 121 → INTID 32 + 121 = 153.
    /// Source: BCM2711 ARM Peripherals datasheet §6.3 "VC peripheral
    /// IRQs", VC_UART entry.
    pub const PL011_INTID: u32 = 153;
}

/// Compile-time-resolved INTID of the PL011 console for the current board.
pub const BOARD_PL011_INTID: u32 = board::PL011_INTID;

/// Global GIC instance for the current board.
static GIC: GicV2 = GicV2::new(board::GICD_BASE, board::GICC_BASE);

/// Access the board's GIC instance.
pub fn board_gic() -> &'static GicV2 {
    &GIC
}

/// Bring up the GIC and enable the PL011 RX interrupt.
pub fn init() {
    GIC.init();
    GIC.enable_spi(BOARD_PL011_INTID);
}

/// Unmask IRQs in the current PSTATE (`DAIFClr.I`).
#[inline]
pub fn enable_irqs() {
    // SAFETY: writing DAIFClr is permitted at EL1 and only un-masks
    // interrupts; it has no side effects beyond that.
    unsafe {
        core::arch::asm!("msr daifclr, #0x2", options(nostack, preserves_flags));
    }
}
