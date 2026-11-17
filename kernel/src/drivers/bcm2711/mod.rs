//! BCM2711 (Raspberry Pi 4) peripheral drivers.
//!
//! Register addresses use the **low-peripheral mode** mapping
//! (`0xFE00_0000` base), which is the firmware default in 64-bit mode.
//! Source: BCM2711 ARM Peripherals datasheet §1.2.4.

pub mod clock_tags;
pub mod dma;
pub mod emmc;
#[cfg(feature = "sd-diag")]
pub mod emmc_diag;
#[cfg(feature = "sd-fault")]
pub mod emmc_fault;
pub mod emmc_xfer;
pub mod framebuffer;
pub mod mailbox;
pub mod pl011;
pub mod pm;
pub mod pwm_audio;
