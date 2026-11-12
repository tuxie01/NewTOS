//! Sound hardware probe of the debug feature `snd-probe` (TASK/SND03, step 0 of
//! TASK/SND01): measures at boot the facts the PWM jack and the DMA ring of
//! TASK/SND05 depend on and that no pinned source states, and plays a short
//! test tone on the 3.5 mm jack. Text goes to the screen (VT52 console) and to
//! the serial line, in three pages; after each page the probe waits for a key
//! (so the screen can be photographed), then the boot goes on.
//!
//! What it measures (probe list: TASK/SND01.md, "Spec review (researcher,
//! 2026-11-12)" §4):
//!
//! * **H4** — the BCM2711 System Timer (`CS`, `CLO`/`CHI`, compare registers
//!   `C0`-`C3`) and the DMA controller (`ENABLE` with its `PAGE`/`PAGELITE`
//!   fields, `INT_STATUS`, every channel's `CS`/`CONBLK_AD`/`DEBUG`), once as
//!   early in the boot as the console exists ([`snapshot_boot_state`]) and
//!   twice more 100 ms apart when the probe runs, so a compare register the
//!   firmware keeps re-arming shows as moving. Also `PACTL_CS` bit 23 (DREQ 1
//!   mux: PWM1 or DSI0). Read only.
//! * **H3** — a CPU-written buffer copied by a DMA channel into another buffer,
//!   memory to memory (`PERMAP = 0`), once without cache maintenance on the
//!   data and once with it, compared word by word (what the DMA wrote, and
//!   what the CPU sees).
//! * **H5** (the same transfer, measured in QEMU `raspi4b` and on the Pi) —
//!   whether the transfer completes, and whether the channel's interrupt
//!   (`TI.INTEN`) reaches the GIC: the distributor's pending bit for INTID
//!   `112 + n` is read with the SPI disabled, enabled, and after `CS.INT` was
//!   cleared, all with the CPU's IRQs masked; no interrupt is taken.
//! * **H1** — `clk_pwm`. First read only: the firmware's view through the
//!   mailbox property interface (clock id `PWM`: state, rate, measured rate,
//!   maximum rate) and the raw clock-manager words at the offset circle uses
//!   (labelled as unsourced, see [`CM_PWMCTL_TERTIARY`]). Then the effective
//!   rate is timed: PWM1 with both channels on the FIFO at a known `RNG`, the
//!   CPU keeps the FIFO full for 0.5 s and counts the words the PWM consumed
//!   between two moments the FIFO was full, against `CNTPCT_EL0`. When no word
//!   is consumed (no clock), the probe asks the firmware to switch the clock on
//!   ("Set clock state", "Set clock rate" to the datasheet's nominal 100 MHz;
//!   firmware wiki) and times it again. It never writes a clock-manager
//!   register itself.
//! * **H2** — the FIFO depth in PWM mode: words written to `FIF1` with both
//!   channels disabled until `STA.FULL1`, then one more write to see `WERR1`.
//! * **Tone** — with a running clock: GPIO 40/41 to ALT0 (PWM1), about 48 kHz
//!   sample rate, quiet square waves (amplitude 1/16 of the range around the
//!   middle): both channels, left only, right only, and both in mark/space
//!   mode. GPIO 40/41 get their previous function back afterwards.
//!
//! Sources (Track B, `docs/SOURCE-ANALYSIS-PROTOCOL.md`):
//! * BCM2711 ARM Peripherals, Release 4
//!   (`references/datasheets/RP-008248-DS-1-bcm2711-peripherals.pdf`): §1.2.4
//!   (legacy master addresses: peripherals at `0x7Enn_nnnn` = ARM
//!   `0xFEnn_nnnn`; RAM through the DMA engines at `0xC000_0000`-`0xFFFF_FFFF`,
//!   the first GiB with `ENABLE.PAGE = 0`); chapter 4 (DMA: Table 33 channel
//!   bases, Table 34 control block, Table 37 register map incl. `INT_STATUS`
//!   `0xfe0` and `ENABLE` `0xff0`, Table 38 `CS`, Table 40 `TI`, Tables 41-43
//!   address and length registers, Table 46 `DEBUG`, Table 61 `INT_STATUS`,
//!   Table 62 `ENABLE`, §4.2.1.3 `PACTL_CS` bit 23); chapter 5 (GPIO: Table 63
//!   register map, `GPFSEL4` `FSEL40` 2:0 / `FSEL41` 5:3, `100` = ALT0, §5.3
//!   GPIO 40/41 ALT0 = `PWM1_0`/`PWM1_1`); §6.2.4 Table 102 (VC IRQ 16 + n =
//!   DMA n for n = 0-6) with §6.3 (VC IRQs are GIC SPI IDs 96-159); chapter 8
//!   (PWM: base `0x7e20c800` for PWM1, Tables 152-158); chapter 10 (System
//!   Timer, Tables 166-170). The extracted facts: `docs/hardware/pwm-audio.md`.
//! * Raspberry Pi firmware wiki, *Mailbox property interface* (local copy
//!   `references/rpi4-boot/firmware-wiki-mailbox-property-interface.md`): clock
//!   id `0x0000000a` = PWM; tags "Get clock state" `0x00030001`, "Set clock
//!   state" `0x00038001`, "Get clock rate" `0x00030002`, "Get clock rate
//!   measured" `0x00030047`, "Get max clock rate" `0x00030004`, "Set clock
//!   rate" `0x00038002`; the message format is the one `mailbox.rs` already
//!   uses (`docs/hardware/mailbox-framebuffer.md` §2).
//! * GIC distributor `GICD_ISPENDR` at `+0x200`: as read by
//!   `crate::initinfo::diag::uart_gic` (ARM IHI 0048B); enable/disable through
//!   [`crate::drivers::gic::GicV2`].
//! * Cache maintenance by VA to the point of coherency (`dc cvac` / `dc civac`
//!   / `dc ivac`, 64-byte lines): the pattern of
//!   `crate::drivers::bcm2711::mailbox` and `crate::arch::aarch64::mmu`.
//!
//! Hypotheses, not sources (TASK/SND01.md, "Cross-reference from the owner"):
//! the iosoft DMA article; circle's clock-manager offset for PWM.
//!
//! Board: Raspberry Pi 4 / QEMU `raspi4b` only (`board-rpi4`). QEMU 11.1.2 has
//! no PWM block in its memory tree (TASK/SND01.md §1); the boot argument
//! [`ARG_QEMU`] leaves out the registers QEMU does not model, for the QEMU H5
//! run; [`ARG_NOWAIT`] drops the key waits.

use core::arch::asm;
use core::fmt::Write;

use crate::drivers::mmio_va;

// ─────────────────────────────────────────────────────────────────────
// Register addresses (ARM view, low-peripheral mode, §1.2.4)
// ─────────────────────────────────────────────────────────────────────

/// DMA channel 0 (`0x7e007000`, §4.2); channel n at `+ n * 0x100`, n = 0-14.
const DMA_BASE: usize = mmio_va(0xFE00_7000);
/// Table 37: `INT_STATUS`, one bit per channel (Table 61).
const DMA_INT_STATUS: usize = DMA_BASE + 0xFE0;
/// Table 37: `ENABLE` — `EN0`-`EN14`, `PAGE` 27:24, `PAGELITE` 31:28 (Table 62).
const DMA_ENABLE: usize = DMA_BASE + 0xFF0;
// Channel register offsets, Table 37 (channels 0-10).
const DMA_CS: usize = 0x00;
const DMA_CONBLK_AD: usize = 0x04;
const DMA_TI: usize = 0x08;
const DMA_SOURCE_AD: usize = 0x0C;
const DMA_DEST_AD: usize = 0x10;
const DMA_TXFR_LEN: usize = 0x14;
/// `DEBUG` of channels 0-10 (Table 37); DMA4 channels 11-14 have it at `0x0c`.
const DMA_DEBUG: usize = 0x20;
const DMA4_DEBUG: usize = 0x0C;
// `CS` bits, Table 38.
const CS_ACTIVE: u32 = 1 << 0;
const CS_END: u32 = 1 << 1;
const CS_INT: u32 = 1 << 2;
const CS_ERROR: u32 = 1 << 8;
const CS_WAIT_FOR_OUTSTANDING_WRITES: u32 = 1 << 28;
const CS_RESET: u32 = 1 << 31;
// `TI` bits, Table 40.
const TI_INTEN: u32 = 1 << 0;
const TI_WAIT_RESP: u32 = 1 << 3;
const TI_DEST_INC: u32 = 1 << 4;
const TI_SRC_INC: u32 = 1 << 8;
/// `DEBUG` error bits 2:0, Table 46 (W1C).
const DEBUG_ERRORS: u32 = 0b111;
/// `DEBUG` bits that are fixed or idle state (`LITE`, `VERSION`, `DMA_STATE`,
/// `DMA_ID`), Table 46 — masked when a channel is listed as untouched.
const DEBUG_ID_BITS: u32 = 0x1FFF_FF00;
/// Bus address of RAM for the DMA engines, §1.2.4 (with `ENABLE.PAGE = 0`).
const DMA_RAM_BUS: u32 = 0xC000_0000;
/// Candidate channels for H3, in order: full (non-Lite, non-DMA4) channels the
/// firmware device tree offers to the ARM, `brcm,dma-channel-mask = <0x7f5>`
/// (`docs/hardware/pwm-audio.md` §7.3). The probe still checks at run time
/// that the channel is enabled and idle.
const DMA_CANDIDATES: [usize; 4] = [5, 4, 2, 6];

/// System Timer, `0x7e003000` (§10.1); Table 166.
const SYST_BASE: usize = mmio_va(0xFE00_3000);

/// `PACTL_CS` (`0x7e204e00`, §4.2.1.3); bit 23 `DMA_CNTRL_MUX_2`: 1 = DREQ 1 is
/// PWM1, 0 = DSI0.
const PACTL_CS: usize = mmio_va(0xFE20_4E00);
const PACTL_DMA_MUX_PWM1: u32 = 1 << 23;

/// PWM1, `0x7e20c800` (§8.6); offsets Table 152.
const PWM1_BASE: usize = mmio_va(0xFE20_C800);
const PWM_CTL: usize = 0x00;
const PWM_STA: usize = 0x04;
const PWM_DMAC: usize = 0x08;
const PWM_RNG1: usize = 0x10;
const PWM_FIF1: usize = 0x18;
const PWM_RNG2: usize = 0x20;
// `CTL` bits, Table 153.
const CTL_PWEN1: u32 = 1 << 0;
const CTL_USEF1: u32 = 1 << 5;
const CTL_CLRF: u32 = 1 << 6;
const CTL_MSEN1: u32 = 1 << 7;
const CTL_PWEN2: u32 = 1 << 8;
const CTL_USEF2: u32 = 1 << 13;
const CTL_MSEN2: u32 = 1 << 15;
// `STA` bits, Table 154.
const STA_FULL1: u32 = 1 << 0;
const STA_EMPT1: u32 = 1 << 1;
const STA_WERR1: u32 = 1 << 2;
const STA_RERR1: u32 = 1 << 3;
const STA_GAPO1: u32 = 1 << 4;
const STA_GAPO2: u32 = 1 << 5;
const STA_BERR: u32 = 1 << 8;
/// Every W1C flag of `STA`.
const STA_FLAGS: u32 = STA_WERR1 | STA_RERR1 | STA_GAPO1 | STA_GAPO2 | STA_BERR;
/// `DMAC.ENAB`, Table 155.
const DMAC_ENAB: u32 = 1 << 31;

/// GPIO, `0x7e200000` (§5.2); `GPFSEL4` at `0x10` (Table 63).
const GPFSEL4: usize = mmio_va(0xFE20_0000) + 0x10;
/// `FSEL40` (bits 2:0) and `FSEL41` (bits 5:3) of `GPFSEL4`.
const FSEL40_41_MASK: u32 = 0b111_111;
/// ALT0 (`100`) in both fields.
const FSEL40_41_ALT0: u32 = 0b100_100;

/// The clock-manager (CPRMAN, base `0x7e101000`, §5.4.2) word circle programs
/// as the PWM clock's control register: `ARM_CM_BASE + Clock * 8` with
/// `GPIOClockPWM = 20` (`references/circle/lib/gpioclock.cpp:66`,
/// `references/circle/include/circle/gpioclock.h:37`), divisor at `+ 4`.
/// **Not in any primary source** (`docs/hardware/pwm-audio.md` §5.2, §10.1):
/// the probe only *reads* it, to compare with the mailbox's answer; it is
/// never written.
const CM_PWMCTL_TERTIARY: usize = mmio_va(0xFE10_1000) + 0xA0;

/// GIC-400 distributor (`crate::drivers::gic`), `GICD_ISPENDR` at `+0x200`.
const GICD_ISPENDR: usize = mmio_va(0xFF84_1000) + 0x200;
/// GIC INTID of DMA channel `n` (n = 0-6): VC IRQ `16 + n` (Table 102) as
/// SPI `96 + VC IRQ` (§6.3).
const fn dma_intid(n: usize) -> u32 {
    96 + 16 + n as u32
}

// Mailbox property interface (firmware wiki, see module docs).
const TAG_GET_CLOCK_STATE: u32 = 0x0003_0001;
const TAG_SET_CLOCK_STATE: u32 = 0x0003_8001;
const TAG_GET_CLOCK_RATE: u32 = 0x0003_0002;
const TAG_GET_MAX_CLOCK_RATE: u32 = 0x0003_0004;
const TAG_GET_CLOCK_RATE_MEASURED: u32 = 0x0003_0047;
const TAG_SET_CLOCK_RATE: u32 = 0x0003_8002;
/// Clock id `PWM`.
const CLOCK_ID_PWM: u32 = 0x0000_000A;
/// The rate asked for when the clock is off: "`clk_pwm` which is nominally
/// 100MHz" (§8.1).
const PWM_NOMINAL_HZ: u32 = 100_000_000;

/// Boot argument that leaves out the registers QEMU `raspi4b` does not model:
/// PWM1 (H1 timing, H2, tone) and `PACTL_CS`. The mailbox and clock-manager
/// reads of H1 still run.
pub const ARG_QEMU: &[u8] = b"sndprobe=qemu";
/// Boot argument: no key wait between the pages and at the end.
pub const ARG_NOWAIT: &[u8] = b"sndprobe=nowait";

/// The two boot-argument switches, read once by [`snapshot_boot_state`].
static mut QEMU_MODE: bool = false;
static mut NO_WAIT: bool = false;

fn qemu_mode() -> bool {
    // SAFETY: boot CPU; written once in `snapshot_boot_state`.
    unsafe { QEMU_MODE }
}

// ─────────────────────────────────────────────────────────────────────
// Small helpers
// ─────────────────────────────────────────────────────────────────────

#[inline(always)]
fn rd(addr: usize) -> u32 {
    // SAFETY: every address passed is a register of the mapped peripheral
    // window named above; reads of these registers have no side effects
    // (none is a FIFO read or read-to-clear, Tables 38-62, 152-158, 166-170).
    unsafe { (addr as *const u32).read_volatile() }
}

#[inline(always)]
fn wr(addr: usize, v: u32) {
    // SAFETY: as `rd`; the probe writes only the registers of a DMA channel it
    // checked to be idle, PWM1, and the GPIO 40/41 fields of GPFSEL4.
    unsafe { (addr as *mut u32).write_volatile(v) }
}

#[inline(always)]
fn dsb() {
    // SAFETY: a barrier has no memory effect of its own.
    unsafe { asm!("dsb sy", options(nostack, preserves_flags)) };
}

/// `(CNTPCT_EL0, CNTFRQ_EL0)`.
fn counter() -> (u64, u64) {
    let (c, f): (u64, u64);
    // SAFETY: read-only system-register reads, permitted at EL1.
    unsafe {
        asm!("isb", "mrs {c}, cntpct_el0", "mrs {f}, cntfrq_el0", c = out(reg) c, f = out(reg) f,
            options(nomem, nostack, preserves_flags));
    }
    (c, f)
}

fn now() -> u64 {
    counter().0
}

/// Counter ticks in `us` microseconds.
fn ticks_us(us: u64) -> u64 {
    counter().1 * us / 1_000_000
}

/// Counter ticks as microseconds.
fn us(ticks: u64) -> u64 {
    ticks * 1_000_000 / counter().1.max(1)
}

/// Mask IRQs, returning the previous `DAIF`.
fn irq_mask() -> u64 {
    let daif: u64;
    // SAFETY: reading DAIF and setting the I bit are permitted at EL1.
    unsafe {
        asm!("mrs {0}, daif", "msr daifset, #2", out(reg) daif, options(nomem, nostack, preserves_flags));
    }
    daif
}

fn irq_restore(daif: u64) {
    // SAFETY: restores the value `irq_mask` read.
    unsafe { asm!("msr daif, {0}", in(reg) daif, options(nomem, nostack, preserves_flags)) };
}

const LINE: usize = 64;

/// `dc ivac` over `[va, va + len)`: drop the lines without writing them back.
///
/// # Safety
/// The range must be mapped and owned by the caller (dirty data is lost);
/// `va` and `len` multiples of [`LINE`] so no neighbour shares a line.
unsafe fn invalidate(va: usize, len: usize) {
    let mut p = va;
    while p < va + len {
        // SAFETY: caller's contract.
        unsafe { asm!("dc ivac, {}", in(reg) p, options(nostack, preserves_flags)) };
        p += LINE;
    }
    dsb();
}

/// Screen (VT52 console) and serial line, `\n` as `\r\n`.
struct Out;

impl Write for Out {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        match tos_api::host::host() {
            Some(h) => {
                for b in s.bytes() {
                    if b == b'\n' {
                        h.console_write_bytes(b"\r\n");
                    } else {
                        h.console_write_bytes(&[b]);
                    }
                }
            }
            None => {
                let _ = crate::hal::uart::UartWriter(&crate::console::CONSOLE).write_str(s);
            }
        }
        Ok(())
    }
}

macro_rules! say {
    ($($a:tt)*) => {{ let _ = ::core::writeln!(Out, $($a)*); }};
}

/// End of a screen page: wait for a key (unless [`ARG_NOWAIT`]), then clear
/// the screen for the next page.
fn page_end(what: &str) {
    say!("[snd03] ----- {what}: photograph the screen, then press a key -----");
    // SAFETY: boot CPU; written once in `snapshot_boot_state`.
    if !unsafe { NO_WAIT } {
        if let Some(h) = tos_api::host::host() {
            while h.console_try_read_key().is_none() {
                crate::arch::aarch64::delay_ms(10);
            }
        }
    }
    crate::fbcon::clear_and_home();
}

// ─────────────────────────────────────────────────────────────────────
// H4 — what the firmware left
// ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct HwState {
    /// `CNTPCT_EL0` when read.
    cnt: u64,
    /// System Timer `CS, CLO, CHI, C0, C1, C2, C3`.
    syst: [u32; 7],
    dma_enable: u32,
    dma_int_status: u32,
    /// Per channel 0-14: `CS`, `CONBLK_AD`, `DEBUG`.
    ch: [[u32; 3]; 15],
    /// `PACTL_CS`, 0 in QEMU mode (not read).
    pactl_cs: u32,
    /// The two clock-manager words at [`CM_PWMCTL_TERTIARY`].
    cm_pwm: [u32; 2],
}

fn read_state() -> HwState {
    let mut s = HwState {
        cnt: now(),
        syst: [0; 7],
        dma_enable: rd(DMA_ENABLE),
        dma_int_status: rd(DMA_INT_STATUS),
        ch: [[0; 3]; 15],
        pactl_cs: if qemu_mode() { 0 } else { rd(PACTL_CS) },
        cm_pwm: [rd(CM_PWMCTL_TERTIARY), rd(CM_PWMCTL_TERTIARY + 4)],
    };
    for (i, w) in s.syst.iter_mut().enumerate() {
        *w = rd(SYST_BASE + 4 * i);
    }
    for (n, c) in s.ch.iter_mut().enumerate() {
        let b = DMA_BASE + n * 0x100;
        let dbg = if n <= 10 { DMA_DEBUG } else { DMA4_DEBUG };
        *c = [rd(b + DMA_CS), rd(b + DMA_CONBLK_AD), rd(b + dbg)];
    }
    s
}

/// Written once by [`snapshot_boot_state`], read by [`run`]; boot CPU only.
static mut BOOT_STATE: Option<HwState> = None;

/// H4: record the System Timer and DMA state as the firmware left it, as early
/// in the boot as possible (before the kernel touches any of it), and read the
/// probe's boot arguments. Read only.
pub fn snapshot_boot_state() {
    let (q, w) = crate::drivers::bcm2711::mailbox::with_command_line(|a| {
        (crate::bootargs::has_token(a, ARG_QEMU), crate::bootargs::has_token(a, ARG_NOWAIT))
    })
    .unwrap_or((false, false));
    // SAFETY: boot CPU, single-threaded, before `run`.
    unsafe {
        QEMU_MODE = q;
        NO_WAIT = w;
    }
    let s = read_state();
    // SAFETY: as above.
    unsafe { BOOT_STATE = Some(s) };
}

fn print_state(label: &str, s: &HwState) {
    say!("[snd03] H4 {label} (CNTPCT {}):", s.cnt);
    say!(
        "[snd03]   SYST CS {:08x} CLO {:08x} CHI {:08x}",
        s.syst[0], s.syst[1], s.syst[2]
    );
    say!(
        "[snd03]   SYST C0 {:08x} C1 {:08x} C2 {:08x} C3 {:08x}",
        s.syst[3], s.syst[4], s.syst[5], s.syst[6]
    );
    say!(
        "[snd03]   DMA ENABLE {:08x} (EN {:04x} PAGE {} PAGELITE {}) INT_STATUS {:08x}",
        s.dma_enable,
        s.dma_enable & 0x7FFF,
        (s.dma_enable >> 24) & 0xF,
        s.dma_enable >> 28,
        s.dma_int_status
    );
    let mut idle = 0u32;
    for (n, c) in s.ch.iter().enumerate() {
        if c[0] == 0 && c[1] == 0 && c[2] & !DEBUG_ID_BITS == 0 {
            idle |= 1 << n;
        } else {
            say!("[snd03]   DMA{n:<2} CS {:08x} CONBLK_AD {:08x} DEBUG {:08x}", c[0], c[1], c[2]);
        }
    }
    say!("[snd03]   DMA channels with CS = CONBLK_AD = 0, no DEBUG error: mask {idle:04x}");
    say!(
        "[snd03]   PACTL_CS {:08x} (bit 23: DREQ 1 = {})",
        s.pactl_cs,
        if qemu_mode() {
            "not read"
        } else if s.pactl_cs & PACTL_DMA_MUX_PWM1 != 0 {
            "PWM1"
        } else {
            "DSI0"
        }
    );
    say!(
        "[snd03]   CM words at circle's PWM offset 0xa0/0xa4 (unsourced): {:08x} {:08x}",
        s.cm_pwm[0], s.cm_pwm[1]
    );
}

fn h4() {
    // SAFETY: boot CPU; written once before.
    match unsafe { BOOT_STATE } {
        Some(s) => print_state("at kernel entry", &s),
        None => say!("[snd03] H4 at kernel entry: no snapshot"),
    }
    let a = read_state();
    crate::arch::aarch64::delay_ms(100);
    let b = read_state();
    print_state("at probe", &a);
    let dt = b.cnt - a.cnt;
    say!("[snd03] H4 changes within {} us after that:", us(dt));
    let names = ["CS", "CLO", "CHI", "C0", "C1", "C2", "C3"];
    for ((name, x), y) in names.iter().zip(a.syst).zip(b.syst) {
        if *name != "CLO" && x != y {
            say!("[snd03]   SYST {name} {x:08x} -> {y:08x}");
        }
    }
    for (n, (x, y)) in a.ch.iter().zip(b.ch.iter()).enumerate() {
        if x != y {
            say!(
                "[snd03]   DMA{n} {:08x} {:08x} {:08x} -> {:08x} {:08x} {:08x}",
                x[0], x[1], x[2], y[0], y[1], y[2]
            );
        }
    }
    say!("[snd03]   SYST CLO advanced {} (1 MHz expected: device tree)", b.syst[1].wrapping_sub(a.syst[1]));
}

// ─────────────────────────────────────────────────────────────────────
// H3 / H5 — DMA memory to memory
// ─────────────────────────────────────────────────────────────────────

const WORDS: usize = 1024;
const BUF_BYTES: usize = WORDS * 4;

#[repr(C, align(4096))]
struct Buf([u32; WORDS]);
/// One control block (Table 34), alone in its cache line.
#[repr(C, align(64))]
struct Cb([u32; 16]);

static mut SRC: Buf = Buf([0; WORDS]);
static mut DST: Buf = Buf([0; WORDS]);
static mut CB: Cb = Cb([0; 16]);

fn src_va() -> usize {
    (&raw mut SRC) as usize
}
fn dst_va() -> usize {
    (&raw mut DST) as usize
}
fn cb_va() -> usize {
    (&raw mut CB) as usize
}

/// DMA bus address of a kernel static (§1.2.4), `None` above the first GiB.
fn bus(va: usize, len: usize) -> Option<u32> {
    let pa = crate::arch::aarch64::mmu::va_to_pa(va as u64);
    (pa + len as u64 <= 0x4000_0000).then_some(DMA_RAM_BUS | pa as u32)
}

fn fill(va: usize, base: u32) {
    for i in 0..WORDS {
        // SAFETY: `va` is SRC or DST, `WORDS` words long.
        unsafe { ((va + 4 * i) as *mut u32).write_volatile(base | i as u32) };
    }
}

/// Words of the buffer equal to `base | i` for each of the three `bases`, and
/// the rest.
fn count(va: usize, bases: [u32; 3]) -> [usize; 4] {
    let mut hits = [0usize; 4];
    for i in 0..WORDS {
        // SAFETY: as `fill`.
        let w = unsafe { ((va + 4 * i) as *const u32).read_volatile() };
        let k = bases.iter().position(|&b| w == b | i as u32).unwrap_or(3);
        hits[k] += 1;
    }
    hits
}

struct DmaRun {
    done: bool,
    ticks: u64,
    cs: u32,
    debug: u32,
    int_status: u32,
    /// `TI`, `SOURCE_AD`, `DEST_AD`, `TXFR_LEN` after the run (Tables 40-43:
    /// the address and length registers are updated by the engine as the
    /// transfer progresses).
    regs: [u32; 4],
    /// `GICD_ISPENDR` bit of the channel's INTID: SPI disabled, SPI enabled,
    /// enabled after `CS.INT` was cleared.
    pend: [bool; 3],
}

fn gic_pending(intid: u32) -> bool {
    rd(GICD_ISPENDR + (intid as usize / 32) * 4) & (1 << (intid % 32)) != 0
}

/// One memory-to-memory transfer of [`BUF_BYTES`] from `src` to `dst` (bus
/// addresses) on channel `ch`, `PERMAP = 0`, `INTEN` set, control block at bus
/// address `cb`; polls `CS.END` up to 100 ms.
fn dma_copy(ch: usize, src: u32, dst: u32, cb: u32) -> DmaRun {
    let b = DMA_BASE + ch * 0x100;
    // Control block, Table 34: TI, SOURCE_AD, DEST_AD, TXFR_LEN, STRIDE,
    // NEXTCONBK = 0 (stop after this one), two reserved words = 0.
    let words = [TI_INTEN | TI_WAIT_RESP | TI_DEST_INC | TI_SRC_INC, src, dst, BUF_BYTES as u32, 0, 0, 0, 0];
    for (i, w) in words.iter().enumerate() {
        // SAFETY: CB is 16 words; boot CPU.
        unsafe { ((cb_va() + 4 * i) as *mut u32).write_volatile(*w) };
    }
    // The control block is always cleaned: a stale one would send the engine
    // to an arbitrary address.
    // SAFETY: CB is a mapped static, alone in its line.
    unsafe { crate::arch::aarch64::mmu::clean_invalidate_dcache_range(cb_va(), LINE) };

    wr(b + DMA_CS, CS_RESET);
    let t = now();
    while rd(b + DMA_CS) & CS_ACTIVE != 0 && now() - t < ticks_us(1000) {}
    // Clear stale error flags (W1C); only when set — QEMU 11.1.2 stores the
    // written value instead (measured: a blind write read back as 7).
    let dbg = rd(b + DMA_DEBUG) & DEBUG_ERRORS;
    if dbg != 0 {
        wr(b + DMA_DEBUG, dbg);
    }
    wr(b + DMA_CONBLK_AD, cb);
    dsb();
    let t0 = now();
    wr(b + DMA_CS, CS_WAIT_FOR_OUTSTANDING_WRITES | CS_END | CS_INT | CS_ACTIVE);
    let limit = ticks_us(100_000);
    let mut done = false;
    while now() - t0 < limit {
        if rd(b + DMA_CS) & CS_END != 0 {
            done = true;
            break;
        }
    }
    let ticks = now() - t0;
    dsb();
    let cs = rd(b + DMA_CS);
    let debug = rd(b + DMA_DEBUG);
    let int_status = rd(DMA_INT_STATUS);
    let regs = [rd(b + DMA_TI), rd(b + DMA_SOURCE_AD), rd(b + DMA_DEST_AD), rd(b + DMA_TXFR_LEN)];

    // H5: the interrupt line at the GIC, IRQs masked so nothing is taken.
    let intid = dma_intid(ch);
    let gic = crate::drivers::gic::board_gic();
    let daif = irq_mask();
    let p_dis = gic_pending(intid);
    gic.enable_spi(intid);
    let p_en = gic_pending(intid);
    wr(b + DMA_CS, CS_END | CS_INT);
    dsb();
    let p_clr = gic_pending(intid);
    gic.disable_spi(intid);
    irq_restore(daif);

    if !done {
        // Stop the engine before the buffers are touched again.
        wr(b + DMA_CS, CS_RESET);
    }
    DmaRun { done, ticks, cs, debug, int_status, regs, pend: [p_dis, p_en, p_clr] }
}

fn report_run(name: &str, r: &DmaRun, ch: usize) {
    say!(
        "[snd03] {name}: {} after {} us, CS {:08x} (END {} INT {} ERROR {})",
        if r.done { "END" } else { "TIMEOUT" },
        us(r.ticks),
        r.cs,
        u8::from(r.cs & CS_END != 0),
        u8::from(r.cs & CS_INT != 0),
        u8::from(r.cs & CS_ERROR != 0)
    );
    say!(
        "[snd03]   DEBUG {:08x} INT_STATUS {:08x}; TI {:08x} SOURCE_AD {:08x} DEST_AD {:08x} TXFR_LEN {:08x}",
        r.debug, r.int_status, r.regs[0], r.regs[1], r.regs[2], r.regs[3]
    );
    say!(
        "[snd03]   H5 GIC INTID {} pending: SPI disabled {}, SPI enabled {}, after CS.INT cleared {}",
        dma_intid(ch),
        u8::from(r.pend[0]),
        u8::from(r.pend[1]),
        u8::from(r.pend[2])
    );
}

fn h3_h5() {
    let en = rd(DMA_ENABLE);
    if (en >> 24) & 0xF != 0 {
        say!("[snd03] H3/H5 skipped: ENABLE.PAGE = {} (RAM window not the first GiB)", (en >> 24) & 0xF);
        return;
    }
    let ch = DMA_CANDIDATES.iter().copied().find(|&n| {
        let b = DMA_BASE + n * 0x100;
        en & (1 << n) != 0 && rd(b + DMA_CS) & CS_ACTIVE == 0 && rd(b + DMA_CONBLK_AD) == 0
    });
    let Some(ch) = ch else {
        say!("[snd03] H3/H5 skipped: none of DMA channels {DMA_CANDIDATES:?} is enabled and idle");
        return;
    };
    let (Some(src), Some(dst), Some(cb)) = (bus(src_va(), BUF_BYTES), bus(dst_va(), BUF_BYTES), bus(cb_va(), LINE))
    else {
        say!("[snd03] H3/H5 skipped: a buffer lies above the first GiB");
        return;
    };
    say!("[snd03] H3/H5 DMA channel {ch}, {BUF_BYTES} bytes memory to memory, PERMAP 0");
    say!("[snd03]   bus addresses SRC {src:08x} DST {dst:08x} CB {cb:08x}");

    // Patterns: word i = base | i.
    const Z: u32 = 0x5A5A_0000; // SRC as DRAM holds it before run 1
    const M: u32 = 0x3C3C_0000; // DST as DRAM holds it before each run
    const P: u32 = 0xC0DE_0000; // SRC as the CPU writes it, run 1
    const Q: u32 = 0xC1DE_0000; // SRC as the CPU writes it, run 2
    fn civac(va: usize) {
        // SAFETY: `va` is SRC or DST, this module's statics, line aligned and
        // `BUF_BYTES` long.
        unsafe { crate::arch::aarch64::mmu::clean_invalidate_dcache_range(va, BUF_BYTES) }
    }

    // Run 1, no cache maintenance on the data between the CPU's writes and the
    // DMA, nor between the DMA and the CPU's read.
    fill(src_va(), Z);
    fill(dst_va(), M);
    civac(src_va());
    civac(dst_va());
    fill(src_va(), P); // dirty in the cache, DRAM still Z
    let _ = count(dst_va(), [M, M, M]); // DST lines now cached clean as M
    let r1 = dma_copy(ch, src, dst, cb);
    report_run("H3 run 1 (no maintenance)", &r1, ch);
    let c = count(dst_va(), [P, Z, M]);
    say!(
        "[snd03]   CPU reads DST: {} new SRC (P), {} old SRC (Z), {} old DST (M), {} other",
        c[0], c[1], c[2], c[3]
    );
    // SAFETY: DST is this module's static, line aligned; nothing in it is needed.
    unsafe { invalidate(dst_va(), BUF_BYTES) };
    let c = count(dst_va(), [P, Z, M]);
    say!(
        "[snd03]   after dc ivac DST (what the DMA wrote): {} P, {} Z, {} M, {} other",
        c[0], c[1], c[2], c[3]
    );

    // Run 2, with maintenance: clean SRC to PoC, DST invalidated before and
    // after the transfer.
    fill(dst_va(), M);
    civac(dst_va());
    fill(src_va(), Q);
    // SAFETY: as above.
    unsafe { crate::arch::aarch64::mmu::clean_dcache_range(src_va(), BUF_BYTES) };
    let r2 = dma_copy(ch, src, dst, cb);
    report_run("H3 run 2 (dc cvac SRC; dc civac + ivac DST)", &r2, ch);
    // SAFETY: as above.
    unsafe { invalidate(dst_va(), BUF_BYTES) };
    let c = count(dst_va(), [Q, P, M]);
    say!(
        "[snd03]   DST: {} of {WORDS} words = SRC (Q), {} P, {} M, {} other",
        c[0], c[1], c[2], c[3]
    );
}

// ─────────────────────────────────────────────────────────────────────
// H1 — clk_pwm through the firmware (read, and switch on only if off)
// ─────────────────────────────────────────────────────────────────────

#[repr(C, align(16))]
struct PropBuf([u32; 12]);
static mut PROP: PropBuf = PropBuf([0; 12]);

/// One-tag property message: `req` request words (at most 3), a value buffer
/// of 3 words. `None` = the transaction failed; else `(tag code word, value
/// words)` — the code word's bit 31 says whether the firmware answered the
/// tag, its low bits the answer's length in bytes.
fn prop(tag: u32, req: &[u32]) -> Option<(u32, [u32; 3])> {
    let va = (&raw mut PROP) as usize;
    let mut w = [0u32; 12];
    w[0] = 48; // buffer size, bytes
    w[2] = tag;
    w[3] = 12; // value buffer size, bytes
    w[4] = (req.len() * 4) as u32; // request: parameter bytes
    w[5..5 + req.len()].copy_from_slice(req);
    // w[8] = 0: end tag; w[1] = 0: request code.
    // SAFETY: boot CPU; exclusive use of the static.
    unsafe { PROP.0 = w };
    let pa = crate::arch::aarch64::mmu::va_to_pa(va as u64) as u32;
    // SAFETY: 16-byte aligned static, mapped; boot CPU.
    if !unsafe { crate::drivers::bcm2711::mailbox::call_property(va, pa, 48) } {
        return None;
    }
    // SAFETY: as above.
    let r = unsafe { PROP.0 };
    Some((r[4], [r[5], r[6], r[7]]))
}

fn show_prop(name: &str, r: Option<(u32, [u32; 3])>) {
    match r {
        None => say!("[snd03]   {name}: mailbox transaction failed"),
        Some((code, _)) if code & 0x8000_0000 == 0 => {
            say!("[snd03]   {name}: tag not answered (code {code:08x})")
        }
        Some((code, v)) => say!(
            "[snd03]   {name}: {} bytes: {:08x} {:08x} (= {})",
            code & 0x7FFF_FFFF,
            v[0],
            v[1],
            v[1]
        ),
    }
}

fn firmware_clock(label: &str) {
    say!("[snd03] H1 firmware view of clock id PWM ({CLOCK_ID_PWM}), {label}:");
    show_prop("Get clock state (b0 on, b1 missing)", prop(TAG_GET_CLOCK_STATE, &[CLOCK_ID_PWM]));
    show_prop("Get clock rate (Hz)", prop(TAG_GET_CLOCK_RATE, &[CLOCK_ID_PWM]));
    show_prop("Get clock rate measured (Hz)", prop(TAG_GET_CLOCK_RATE_MEASURED, &[CLOCK_ID_PWM]));
    show_prop("Get max clock rate (Hz)", prop(TAG_GET_MAX_CLOCK_RATE, &[CLOCK_ID_PWM]));
    say!(
        "[snd03]   CM words at circle's PWM offset 0xa0/0xa4 (unsourced, read only): {:08x} {:08x}",
        rd(CM_PWMCTL_TERTIARY),
        rd(CM_PWMCTL_TERTIARY + 4)
    );
}

// ─────────────────────────────────────────────────────────────────────
// H2 / H1 timing / tone — PWM1
// ─────────────────────────────────────────────────────────────────────

fn pwm(off: usize) -> usize {
    PWM1_BASE + off
}

/// Both channels off, `DMAC.ENAB` off, FIFO cleared, flags cleared.
fn pwm_off() {
    wr(pwm(PWM_CTL), 0);
    let dmac = rd(pwm(PWM_DMAC));
    if dmac & DMAC_ENAB != 0 {
        wr(pwm(PWM_DMAC), dmac & !DMAC_ENAB);
    }
    wr(pwm(PWM_CTL), CTL_CLRF);
    wr(pwm(PWM_STA), STA_FLAGS);
}

fn sta_flags(sta: u32) -> [u8; 6] {
    [STA_EMPT1, STA_GAPO1, STA_GAPO2, STA_RERR1, STA_WERR1, STA_BERR].map(|m| u8::from(sta & m != 0))
}

/// H2: FIFO depth with both channels disabled. Returns the count.
fn h2() -> u32 {
    say!(
        "[snd03] H2 PWM1 as found: CTL {:08x} STA {:08x} DMAC {:08x} RNG1 {:08x} RNG2 {:08x}",
        rd(pwm(PWM_CTL)),
        rd(pwm(PWM_STA)),
        rd(pwm(PWM_DMAC)),
        rd(pwm(PWM_RNG1)),
        rd(pwm(PWM_RNG2))
    );
    pwm_off();
    let sta0 = rd(pwm(PWM_STA));
    let mut n = 0u32;
    while rd(pwm(PWM_STA)) & STA_FULL1 == 0 && n < 4096 {
        wr(pwm(PWM_FIF1), n);
        n += 1;
    }
    let sta_full = rd(pwm(PWM_STA));
    wr(pwm(PWM_FIF1), 0xFFFF);
    let sta_over = rd(pwm(PWM_STA));
    say!(
        "[snd03] H2 FIFO depth, channels disabled: {n} words until FULL1{}",
        if n >= 4096 { " (NEVER FULL, stopped at 4096)" } else { "" }
    );
    say!(
        "[snd03]   STA after CLRF {sta0:08x}, at FULL1 {sta_full:08x}, after one more write {sta_over:08x} (WERR1 {})",
        u8::from(sta_over & STA_WERR1 != 0)
    );
    pwm_off();
    n
}

enum Fed {
    /// No word consumed for 200 ms.
    Stalled,
    /// The FIFO was never full again after a write: the CPU did not keep up.
    NeverFull { written: u64 },
    /// `ticks` between the first and the last moment the FIFO was found full
    /// right after a write, and the words written in that span (= consumed).
    Span { written: u64, ticks: u64, words: u64 },
}

/// Keep the FIFO full with the words `next` yields, for `dur_us`.
fn feed(dur_us: u64, mut next: impl FnMut() -> u32) -> Fed {
    let stall = ticks_us(200_000);
    let end = now() + ticks_us(dur_us);
    let hard_end = end + ticks_us(1_000_000);
    let mut written = 0u64;
    let mut first: Option<(u64, u64)> = None;
    let mut last = (0u64, 0u64);
    let mut progress = now();
    loop {
        if rd(pwm(PWM_STA)) & STA_FULL1 == 0 {
            wr(pwm(PWM_FIF1), next());
            written += 1;
            progress = now();
            if rd(pwm(PWM_STA)) & STA_FULL1 != 0 {
                let t = now();
                if first.is_none() {
                    first = Some((t, written));
                }
                last = (t, written);
                if t >= end {
                    break;
                }
            }
            if progress >= hard_end {
                break;
            }
        } else if now() - progress > stall {
            return Fed::Stalled;
        }
    }
    match first {
        Some((t0, n0)) if last.0 > t0 => Fed::Span { written, ticks: last.0 - t0, words: last.1 - n0 },
        _ => Fed::NeverFull { written },
    }
}

/// Configure PWM1 for the FIFO on both channels at range `rng`, prefill with
/// `fillv`, then enable both channels (PWM mode, `MSEN` per `ms`).
fn pwm_start(rng: u32, fillv: u32, ms: bool) {
    pwm_off();
    wr(pwm(PWM_RNG1), rng);
    wr(pwm(PWM_RNG2), rng);
    // At most 4096 words: H2 reports a FIFO that never says FULL1.
    for _ in 0..4096 {
        if rd(pwm(PWM_STA)) & STA_FULL1 != 0 {
            break;
        }
        wr(pwm(PWM_FIF1), fillv);
    }
    let msen = if ms { CTL_MSEN1 | CTL_MSEN2 } else { 0 };
    wr(pwm(PWM_CTL), CTL_PWEN1 | CTL_USEF1 | CTL_PWEN2 | CTL_USEF2 | msen);
}

/// H1 timing: `Ok(the effective clk_pwm in Hz)`, `Err(true)` = no word was
/// consumed (no clock), `Err(false)` = no number for another reason.
fn h1_time(label: &str) -> Result<u64, bool> {
    const R: u32 = 10_000;
    pwm_start(R, R / 2, false);
    let daif = irq_mask();
    let r = feed(500_000, || R / 2);
    irq_restore(daif);
    let sta = rd(pwm(PWM_STA));
    pwm_off();
    let (_, frq) = counter();
    let f = sta_flags(sta);
    say!(
        "[snd03] H1 timing {label}, RNG {R}: STA {sta:08x} (EMPT1 {} GAPO1 {} GAPO2 {} RERR1 {} WERR1 {} BERR {})",
        f[0], f[1], f[2], f[3], f[4], f[5]
    );
    match r {
        Fed::Stalled => {
            say!("[snd03]   no FIFO word consumed in 200 ms: clk_pwm NOT running");
            Err(true)
        }
        Fed::NeverFull { written } => {
            say!("[snd03]   FIFO never full again ({written} words written): CPU slower than the PWM, no number");
            Err(false)
        }
        Fed::Span { written, ticks, words } => {
            // Each channel takes every second word, one per RNG clocks.
            let hz = words * u64::from(R) * frq / (2 * ticks);
            say!(
                "[snd03]   {words} words consumed in {ticks} ticks (CNTFRQ {frq}), {written} written"
            );
            say!(
                "[snd03]   => clk_pwm = {hz} Hz ({} periods/s per channel; a GAPO above invalidates it)",
                hz / u64::from(R)
            );
            Ok(hz)
        }
    }
}

/// One tone segment: square waves, `l_hz`/`r_hz` = 0 for silence (middle).
fn tone(name: &str, f_pwm: u64, ms: bool, l_hz: u64, r_hz: u64, dur_ms: u64) {
    const RATE: u64 = 48_000;
    let rng = f_pwm.div_ceil(RATE).max(16) as u32;
    let fs = f_pwm / u64::from(rng);
    let mid = rng / 2;
    let amp = rng / 16;
    say!(
        "[snd03] TONE {name}: RNG {rng}, {fs} Hz, left {l_hz} Hz, right {r_hz} Hz, {dur_ms} ms{}",
        if ms { ", mark/space" } else { "" }
    );
    let level = |hz: u64, k: u64| -> u32 {
        if hz == 0 {
            mid
        } else if (k * 2 * hz / fs).is_multiple_of(2) {
            mid + amp
        } else {
            mid - amp
        }
    };
    let mut i: u64 = 0;
    pwm_start(rng, mid, ms);
    let daif = irq_mask();
    let r = feed(dur_ms * 1000, || {
        // Word A of a pair goes to channel 1 (GPIO 40), word B to channel 2
        // (GPIO 41): the FIFO's interleave (`FIF1` text before Table 158).
        let k = i / 2;
        let w = if i.is_multiple_of(2) { level(l_hz, k) } else { level(r_hz, k) };
        i += 1;
        w
    });
    // End on the middle value before switching off.
    let _ = feed(20_000, || mid);
    irq_restore(daif);
    let sta = rd(pwm(PWM_STA));
    pwm_off();
    let f = sta_flags(sta);
    say!(
        "[snd03]   {} STA {sta:08x} (GAPO1 {} GAPO2 {} RERR1 {})",
        match r {
            Fed::Span { .. } => "played,",
            Fed::NeverFull { .. } => "played, CPU did not keep the FIFO full,",
            Fed::Stalled => "STALLED,",
        },
        f[1],
        f[2],
        f[3]
    );
}

fn tones(f_pwm: u64) {
    let fsel = rd(GPFSEL4);
    wr(GPFSEL4, (fsel & !FSEL40_41_MASK) | FSEL40_41_ALT0);
    say!("[snd03] TONE GPFSEL4 {fsel:08x} -> {:08x} (GPIO 40/41 ALT0 = PWM1)", rd(GPFSEL4));
    crate::arch::aarch64::delay_ms(1000);
    tone("T1 both", f_pwm, false, 440, 440, 2000);
    crate::arch::aarch64::delay_ms(700);
    tone("T2 left only (GPIO 40)", f_pwm, false, 440, 0, 1500);
    crate::arch::aarch64::delay_ms(700);
    tone("T3 right only (GPIO 41)", f_pwm, false, 0, 880, 1500);
    crate::arch::aarch64::delay_ms(700);
    tone("T4 both, mark/space mode", f_pwm, true, 440, 440, 2000);
    wr(GPFSEL4, (rd(GPFSEL4) & !FSEL40_41_MASK) | (fsel & FSEL40_41_MASK));
    say!("[snd03] TONE done, GPFSEL4 back to {:08x}", rd(GPFSEL4));
}

// ─────────────────────────────────────────────────────────────────────
// Entry
// ─────────────────────────────────────────────────────────────────────

/// Run the probe page by page; then the boot continues.
pub fn run() {
    crate::fbcon::clear_and_home();
    say!("[snd03] ===== SND03 sound hardware probe ({}), page 1 of 3 =====", crate::BOARD_NAME);
    h4();
    page_end("page 1 of 3 (H4) done");

    say!("[snd03] ===== page 2 of 3 =====");
    h3_h5();
    firmware_clock("as found");
    page_end("page 2 of 3 (H3, H5, H1 firmware view) done");

    say!("[snd03] ===== page 3 of 3 =====");
    if qemu_mode() {
        say!("[snd03] PWM part skipped (boot argument `sndprobe=qemu`)");
    } else {
        let depth = h2();
        let mut f = h1_time("as found");
        let found = f;
        if f == Err(true) {
            // Only when no clock runs: ask the firmware (never a CPRMAN write).
            say!("[snd03] H1 asking the firmware: Set clock state on, Set clock rate {PWM_NOMINAL_HZ} Hz");
            show_prop("Set clock state (on)", prop(TAG_SET_CLOCK_STATE, &[CLOCK_ID_PWM, 1]));
            show_prop("Set clock rate", prop(TAG_SET_CLOCK_RATE, &[CLOCK_ID_PWM, PWM_NOMINAL_HZ, 1]));
            firmware_clock("after the request");
            f = h1_time("after the request");
        }
        match f {
            Ok(hz) => tones(hz),
            Err(_) => say!("[snd03] TONE skipped: no measured clk_pwm"),
        }
        let show = |r: Result<u64, bool>| match r {
            Ok(hz) => alloc::format!("{hz} Hz"),
            Err(true) => alloc::string::String::from("not running"),
            Err(false) => alloc::string::String::from("no number"),
        };
        say!(
            "[snd03] SUMMARY H2 FIFO depth {depth} words; H1 clk_pwm as found {}, at the tone {}",
            show(found),
            show(f)
        );
    }
    page_end("page 3 of 3 done; the boot continues after the key");
}
