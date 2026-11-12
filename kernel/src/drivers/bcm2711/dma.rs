//! BCM2711 DMA controller — one full channel running a chain of control
//! blocks (TASK/SND05, the transport of the PWM jack's staging ring).
//!
//! Sources (Track B, `docs/SOURCE-ANALYSIS-PROTOCOL.md`): BCM2711 ARM
//! Peripherals, Release 4 (`references/datasheets/RP-008248-DS-1-bcm2711-peripherals.pdf`),
//! chapter 4 — Table 33 (channel bases, `0x7E007000 + n * 0x100`), Table 34
//! (control block: 8 words, 256-bit aligned), Table 37 (register map,
//! `ENABLE` at `0xFF0`), Table 38 (`CS`), Table 40 (`TI`), Table 41
//! (`SOURCE_AD`, "updated by the DMA engine as the transfer progresses"),
//! Table 46 (`DEBUG`), Table 62 (`ENABLE`: "Disabling whilst the DMA is
//! operating will be fatal"); §1.2.4 / §4.1 (the engines address RAM through
//! the legacy window `0xC000_0000 | PA`, the first GiB with `ENABLE.PAGE =
//! 0`, and peripherals at their `0x7E…` legacy master addresses). The start
//! sequence is the one measured on the real Pi and in QEMU on channel 5
//! (H3/H5, `measurements/measurement-2026-11-12-snd03-pi/RUNINFO.TXT`,
//! `measurements/measurement-2026-11-12-snd03/RUNINFO.TXT`; the probe's
//! `dma_copy`, `kernel/src/snd_probe.rs:556@dma_copy`). The contract is
//! `docs/hardware/pwm-audio.md` §12.5-§12.6.

use core::arch::asm;

use crate::drivers::mmio_va;

/// DMA channel 0 (`0x7E007000`, Table 33); channel `n` at `+ n * 0x100`.
const DMA_BASE: usize = mmio_va(0xFE00_7000);
/// `ENABLE` (Table 37 / 62): `EN0`-`EN14`, `PAGE` 27:24, `PAGELITE` 31:28.
const ENABLE: usize = DMA_BASE + 0xFF0;

// Channel register offsets, Table 37.
const CS: usize = 0x00;
const CONBLK_AD: usize = 0x04;
const SOURCE_AD: usize = 0x0C;
const DEBUG: usize = 0x20;

// `CS` bits, Table 38.
const CS_ACTIVE: u32 = 1 << 0;
const CS_END: u32 = 1 << 1;
const CS_INT: u32 = 1 << 2;
const CS_WAIT_FOR_OUTSTANDING_WRITES: u32 = 1 << 28;
const CS_RESET: u32 = 1 << 31;
/// `DEBUG` error bits 2:0 (Table 46, W1C).
const DEBUG_ERRORS: u32 = 0b111;

// `TI` bits, Table 40.
/// `TI.WAIT_RESP` (bit 3).
pub const TI_WAIT_RESP: u32 = 1 << 3;
/// `TI.DEST_DREQ` (bit 6): the peripheral's DREQ paces the writes.
pub const TI_DEST_DREQ: u32 = 1 << 6;
/// `TI.SRC_INC` (bit 8).
pub const TI_SRC_INC: u32 = 1 << 8;
/// `TI.PERMAP` (bits 20:16): the DREQ line that paces the transfer.
pub const fn ti_permap(dreq: u32) -> u32 {
    (dreq & 0x1F) << 16
}

/// Bus address of RAM for the DMA engines with `ENABLE.PAGE = 0` (§1.2.4).
const RAM_BUS: u32 = 0xC000_0000;
/// The engines reach the first GiB of RAM only (§1.2.4, Table 62 `PAGE` 0).
const RAM_BUS_LIMIT: u64 = 0x4000_0000;

/// One control block (Table 34): `TI`, `SOURCE_AD`, `DEST_AD`, `TXFR_LEN`,
/// `STRIDE`, `NEXTCONBK`, two reserved words; 256-bit aligned (64 here: one
/// cache line per block, so a clean never touches a neighbour).
#[repr(C, align(64))]
#[derive(Clone, Copy)]
pub struct ControlBlock {
    /// Transfer information (Table 40).
    pub ti: u32,
    /// Source bus address.
    pub source_ad: u32,
    /// Destination bus address.
    pub dest_ad: u32,
    /// Bytes (Table 43; full channels: 30 bits in linear mode).
    pub txfr_len: u32,
    /// 2D stride; 0.
    pub stride: u32,
    /// Bus address of the next block; 0 stops the channel.
    pub nextconbk: u32,
    /// Reserved, 0.
    pub reserved: [u32; 2],
}

impl ControlBlock {
    /// An all-zero block.
    pub const ZERO: ControlBlock =
        ControlBlock { ti: 0, source_ad: 0, dest_ad: 0, txfr_len: 0, stride: 0, nextconbk: 0, reserved: [0; 2] };
}

/// DMA bus address of the kernel memory at `va` (`len` bytes), `None` when
/// the range is not wholly in the first GiB of RAM (§1.2.4).
pub fn bus_address(va: usize, len: usize) -> Option<u32> {
    let pa = crate::arch::aarch64::mmu::va_to_pa(va as u64);
    (pa + len as u64 <= RAM_BUS_LIMIT).then_some(RAM_BUS | pa as u32)
}

#[inline(always)]
fn rd(addr: usize) -> u32 {
    // SAFETY: a register of the DMA block in the mapped peripheral window;
    // the reads used here have no side effect (Tables 38-62).
    unsafe { (addr as *const u32).read_volatile() }
}

#[inline(always)]
fn wr(addr: usize, v: u32) {
    // SAFETY: as `rd`; written only for the channel this driver owns and
    // the owner's bit of `ENABLE`.
    unsafe { (addr as *mut u32).write_volatile(v) }
}

#[inline(always)]
fn dsb() {
    // SAFETY: a barrier has no memory effect of its own.
    unsafe { asm!("dsb sy", options(nostack, preserves_flags)) };
}

/// `CNTPCT_EL0` and `CNTFRQ_EL0`.
fn counter() -> (u64, u64) {
    let (c, f): (u64, u64);
    // SAFETY: read-only system-register reads, permitted at EL1.
    unsafe {
        asm!("isb", "mrs {c}, cntpct_el0", "mrs {f}, cntfrq_el0", c = out(reg) c, f = out(reg) f,
            options(nomem, nostack, preserves_flags));
    }
    (c, f)
}

/// A full DMA channel (0-6) this driver owns.
#[derive(Clone, Copy)]
pub struct Channel {
    base: usize,
    n: usize,
}

impl Channel {
    /// Take channel `n` (0-6): set its `ENABLE` bit if it is clear (never
    /// clear one, Table 62) and check it is idle (`CS.ACTIVE` clear). `None`
    /// for another channel number or a busy channel.
    pub fn claim(n: usize) -> Option<Channel> {
        if n > 6 {
            return None;
        }
        let en = rd(ENABLE);
        if en & (1 << n) == 0 {
            wr(ENABLE, en | (1 << n));
        }
        let ch = Channel { base: DMA_BASE + n * 0x100, n };
        (rd(ch.base + CS) & CS_ACTIVE == 0).then_some(ch)
    }

    /// The handle of channel `n`, claimed earlier with [`Self::claim`].
    pub const fn owned(n: usize) -> Channel {
        Channel { base: DMA_BASE + n * 0x100, n }
    }

    /// The channel number.
    pub fn number(&self) -> usize {
        self.n
    }

    /// Reset the channel (`CS.RESET`, W1SC) and wait up to 1 ms for `ACTIVE`
    /// to clear.
    pub fn reset(&self) {
        wr(self.base + CS, CS_RESET);
        let (t0, f) = counter();
        while rd(self.base + CS) & CS_ACTIVE != 0 && counter().0 - t0 < f / 1000 {}
    }

    /// Run the control-block chain starting at bus address `cb`: reset,
    /// clear stale `DEBUG` error flags (only when set — QEMU stores a blind
    /// W1C write, measured), load `CONBLK_AD`, start with
    /// `WAIT_FOR_OUTSTANDING_WRITES | END | INT | ACTIVE` (the measured H3
    /// sequence, `pwm-audio.md` §12.5 item 5). The blocks must be cleaned to
    /// memory before.
    pub fn start(&self, cb: u32) {
        self.reset();
        let dbg = rd(self.base + DEBUG) & DEBUG_ERRORS;
        if dbg != 0 {
            wr(self.base + DEBUG, dbg);
        }
        wr(self.base + CONBLK_AD, cb);
        dsb();
        wr(self.base + CS, CS_WAIT_FOR_OUTSTANDING_WRITES | CS_END | CS_INT | CS_ACTIVE);
    }

    /// `SOURCE_AD`: the bus address the engine reads next (Table 41).
    pub fn source_ad(&self) -> u32 {
        rd(self.base + SOURCE_AD)
    }
}

/// A bounded wait of `us` microseconds on the generic timer, returning early
/// once `done` holds; `true` when it did.
pub fn wait_us(us: u64, mut done: impl FnMut() -> bool) -> bool {
    let (t0, f) = counter();
    let limit = f * us / 1_000_000;
    loop {
        if done() {
            return true;
        }
        if counter().0 - t0 >= limit {
            return false;
        }
    }
}
