//! PL011 UART driver for the QEMU `aarch64-virt` machine.
//!
//! References:
//!   - ARM PrimeCell UART (PL011) Technical Reference Manual (DDI 0183)
//!   - QEMU source: `hw/arm/virt.c` — UART base address `0x0900_0000`
//!   - QEMU source: `hw/char/pl011.c` — register layout (matches PL011 TRM)
//!
//! QEMU's PL011 model accepts any baud-rate divisors (it is a pure
//! emulation), but we still program 115200 @ 24 MHz reference clock to keep
//! behavior parallel to the BCM2711 driver.
//!   IBRD = trunc(24_000_000 / (16 * 115_200))            = 13
//!   FBRD = round((24_000_000 / (16 * 115_200) - 13) * 64) = 1
//!
//! No pin-muxing is necessary: the virt machine wires the UART directly.

use crate::hal::uart::Uart;

const UART0_BASE: usize = crate::drivers::mmio_va(0x0900_0000);

const UART_DR:    usize = 0x00;
const UART_FR:    usize = 0x18;
const UART_IBRD:  usize = 0x24;
const UART_FBRD:  usize = 0x28;
const UART_LCRH:  usize = 0x2C;
const UART_CR:    usize = 0x30;
const UART_IFLS:  usize = 0x34;
const UART_IMSC:  usize = 0x38;
const UART_ICR:   usize = 0x44;

const FR_TXFF: u32 = 1 << 5;
const FR_RXFE: u32 = 1 << 4;

const LCRH_FEN:    u32 = 1 << 4;
const LCRH_WLEN_8: u32 = 0b11 << 5;

const CR_UARTEN: u32 = 1 << 0;
const CR_TXE:    u32 = 1 << 8;
const CR_RXE:    u32 = 1 << 9;

// Interrupt mask / clear bits.
const INT_RX: u32 = 1 << 4;   // RXIM / RXIC
const INT_RT: u32 = 1 << 6;   // RTIM / RTIC (receive timeout)

/// PL011 driver instance bound to a specific MMIO base.
pub struct Pl011 {
    base: usize,
}

impl Pl011 {
    /// Construct a driver bound to the given MMIO base.
    pub const fn new(base: usize) -> Self {
        Self { base }
    }

    #[inline(always)]
    fn reg(&self, off: usize) -> *mut u32 {
        (self.base + off) as *mut u32
    }
}

impl Uart for Pl011 {
    fn init(&self) {
        // SAFETY: PL011 MMIO is valid on the QEMU virt machine.
        unsafe {
            self.reg(UART_CR).write_volatile(0);
            // The kernel owns `IMSC` from here on rather than inheriting
            // whatever the previous stage left unmasked;
            // `enable_rx_interrupt` re-unmasks the RX sources.
            self.reg(UART_IMSC).write_volatile(0);
            self.reg(UART_ICR).write_volatile(0x7FF);
            self.reg(UART_IBRD).write_volatile(13);
            self.reg(UART_FBRD).write_volatile(1);
            self.reg(UART_LCRH).write_volatile(LCRH_FEN | LCRH_WLEN_8);
            self.reg(UART_CR).write_volatile(CR_UARTEN | CR_TXE | CR_RXE);
            // Start from an empty RX FIFO: on this board a byte left in it
            // would be delivered as AUX input (`Iorec(0)`'s ring,
            // `console::serial_rx_byte`). Bounded at twice the FIFO depth.
            for _ in 0..64 {
                if self.reg(UART_FR).read_volatile() & FR_RXFE != 0 {
                    break;
                }
                let _dr = self.reg(UART_DR).read_volatile();
                #[cfg(feature = "initinfo-diag")]
                crate::initinfo::diag::note_uart_flush(_dr as u8);
            }
            self.reg(UART_ICR).write_volatile(0x7FF);
        }
    }

    fn write_byte(&self, byte: u8) {
        // SAFETY: see `init`.
        unsafe {
            while self.reg(UART_FR).read_volatile() & FR_TXFF != 0 {
                core::hint::spin_loop();
            }
            self.reg(UART_DR).write_volatile(byte as u32);
        }
    }

    fn read_byte(&self) -> Option<u8> {
        // SAFETY: see `init`.
        unsafe {
            if self.reg(UART_FR).read_volatile() & FR_RXFE != 0 {
                None
            } else {
                Some(self.reg(UART_DR).read_volatile() as u8)
            }
        }
    }

    fn enable_rx_interrupt(&self) {
        // SAFETY: see `init`.
        unsafe {
            // Trigger at 1/8 RX-FIFO (4 bytes) — IFLS reset value already.
            self.reg(UART_IFLS).write_volatile(0b000 << 3);
            // Unmask RX + receive-timeout. RT covers the case where less
            // than the trigger threshold is present but no further input
            // arrives, which is the common interactive-typing pattern.
            let prev = self.reg(UART_IMSC).read_volatile();
            self.reg(UART_IMSC).write_volatile(prev | INT_RX | INT_RT);
        }
    }

    fn clear_rx_interrupt(&self) {
        // SAFETY: see `init`.
        unsafe {
            self.reg(UART_ICR).write_volatile(INT_RX | INT_RT);
        }
    }
}

/// Global console instance for this board.
pub static CONSOLE: Pl011 = Pl011::new(UART0_BASE);
