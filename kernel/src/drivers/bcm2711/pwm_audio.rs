//! The 3.5 mm jack as a sound backend: PWM1 on GPIO 40/41, fed by DMA
//! channel 5 from a staging ring of duty-cycle words (TASK/SND05, D-tick).
//!
//! The XBIOS side (`Setbuffer`, `Buffoper`, … and the converter that fills
//! the ring) is `tos_api::xbios::sound`; this module is its hardware
//! ([`tos_api::xbios::sound::SoundHw`]): the PWM clock, the pins, the PWM
//! block, the DMA channel and the ring memory. The data path
//! (`docs/hardware/pwm-audio.md` §12.1): program buffer → CPU conversion in
//! the 200 Hz tick → this ring (2 048 words, `L R` interleaved) → DMA,
//! two control blocks chained in a cycle over the two ring halves,
//! `PERMAP = 1` → `FIF1` → GPIO 40/41.
//!
//! Every value below is from the contract `docs/hardware/pwm-audio.md` §12,
//! labelled there CITED (BCM2711 ARM Peripherals Release 4,
//! `references/datasheets/RP-008248-DS-1-bcm2711-peripherals.pdf`; firmware
//! wiki `references/rpi4-boot/firmware-wiki-mailbox-property-interface.md`)
//! or MEASURED (`measurements/measurement-2026-11-12-snd03-pi/RUNINFO.TXT`):
//!
//! * clock (§12.2): "Set clock state" on, "Set clock rate" 100 MHz (reply
//!   ignored), then "Get clock rate measured" for clock id `0x0a` is the rate
//!   every range is computed from — never a constant;
//! * pins and mux (§12.4): `GPFSEL4` `FSEL40`/`FSEL41` = ALT0 (`100`),
//!   read-modify-write; `PACTL_CS` bit 23 = 1 (DREQ 1 is PWM1), read, OR,
//!   write back;
//! * PWM (§12.4, Tables 152-158): `CTL` 0, `DMAC.ENAB` off, `CLRF`, `STA`
//!   flags cleared; `RNG1 = RNG2`; `DMAC = ENAB | PANIC 7 | DREQ 7` (the reset
//!   thresholds, §10.3 open); `PWEN1 | USEF1 | PWEN2 | USEF2` last, after the
//!   DMA has filled the FIFO (§12.4 item 5). `MSEN` stays 0 (both modes ran
//!   without error, which sounds better is not measured);
//! * DMA (§12.5): channel 5, `TI = PERMAP 1 | DEST_DREQ | SRC_INC |
//!   WAIT_RESP`, `INTEN` 0 (D-tick needs no interrupt, §12.7), `DEST_AD =
//!   0x7E20C818` (`FIF1` at its legacy master address);
//! * memory (§12.6): ring and blocks below 1 GiB (bus `0xC000_0000 | PA`),
//!   the ring cleaned (`dc cvac`) after every write, the blocks once.
//!
//! **Detection.** QEMU `raspi4b` models no PWM block: an access aborts, and
//! a read of `PACTL_CS` stalls the boot (`pwm-audio.md` §10.6, measured).
//! The backend therefore touches neither before the firmware has answered
//! "Get clock rate measured" for the PWM clock with a rate — QEMU's firmware
//! model answers that tag with 0 bytes (measured,
//! `measurements/measurement-2026-11-12-snd03/RUNINFO.TXT` RESULT 1), the
//! Pi's with 107 143 064 Hz. Without that answer there is no backend: the
//! `_SND` cookie stays absent and XBIOS 0x80-0x8D answer their function
//! number (ADR-012 amendment 2026-11-12).
//!
//! **Stopping.** Once started the transport keeps running: an idle ring is
//! mid-level silence the engine keeps reading, so no stop click (an empty
//! FIFO drops the output to the `SBIT` level, §12.4 item 6) and no restart
//! per sound.

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use tos_api::xbios::sound::{SoundHw, RING_WORDS};

use super::clock_tags::CLOCK_ID_PWM;
use super::dma::{self, Channel, ControlBlock};
use crate::drivers::mmio_va;

// PWM1, `0x7E20C800` (§8.6, Table 152).
const PWM1_BASE: usize = mmio_va(0xFE20_C800);
const CTL: usize = PWM1_BASE;
const STA: usize = PWM1_BASE + 0x04;
const DMAC: usize = PWM1_BASE + 0x08;
const RNG1: usize = PWM1_BASE + 0x10;
const RNG2: usize = PWM1_BASE + 0x20;
/// `FIF1` at its legacy master address, the DMA's destination (§4.1).
const FIF1_BUS: u32 = 0x7E20_C818;
// `CTL` bits, Table 153.
const CTL_PWEN1: u32 = 1 << 0;
const CTL_USEF1: u32 = 1 << 5;
const CTL_CLRF: u32 = 1 << 6;
const CTL_PWEN2: u32 = 1 << 8;
const CTL_USEF2: u32 = 1 << 13;
// `STA` bits, Table 154.
const STA_FULL1: u32 = 1 << 0;
/// The W1C flags `WERR1 RERR1 GAPO1 GAPO2 BERR` (`0x13C`, §12.4 item 3).
const STA_FLAGS: u32 = 0x13C;
// `DMAC`, Table 155.
const DMAC_ENAB: u32 = 1 << 31;
/// `PANIC` (15:8) and `DREQ` (7:0) at their reset value 7.
const DMAC_THRESHOLDS: u32 = (7 << 8) | 7;

/// `GPFSEL4` (GPIO base `0x7E200000`, offset `0x10`, Table 63).
const GPFSEL4: usize = mmio_va(0xFE20_0000) + 0x10;
const FSEL40_41_MASK: u32 = 0b111_111;
/// ALT0 (`100`) in `FSEL40` (2:0) and `FSEL41` (5:3): `PWM1_0`/`PWM1_1` (§5.3).
const FSEL40_41_ALT0: u32 = 0b100_100;

/// `PACTL_CS` (`0x7E204E00`, §4.2.1.3); bit 23 `DMA_CNTRL_MUX_2`: 1 = DREQ 1 is
/// PWM1.
const PACTL_CS: usize = mmio_va(0xFE20_4E00);
const PACTL_DREQ1_PWM1: u32 = 1 << 23;
/// DREQ 1 = PWM1 with `PACTL_CS` bit 23 set (§4.2.1.3).
const DREQ_PWM1: u32 = 1;

/// The DMA channel (§12.5 item 2: the full channel measured working, no
/// firmware footprint at boot).
const CHANNEL: usize = 5;

/// The rate requested from the firmware (§12.2: "nominally 100MHz"; the
/// reply is ignored, the measured rate is used).
const PWM_REQUEST_HZ: u32 = 100_000_000;

const RING_BYTES: usize = RING_WORDS * 4;

#[repr(C, align(64))]
struct Ring([u32; RING_WORDS]);
static mut RING: Ring = Ring([0; RING_WORDS]);
static mut CBS: [ControlBlock; 2] = [ControlBlock::ZERO; 2];

fn ring_va() -> usize {
    (&raw mut RING) as usize
}

#[inline(always)]
fn rd(addr: usize) -> u32 {
    // SAFETY: a PWM1, GPIO or PACTL register in the mapped peripheral window,
    // reached only after the firmware reported a PWM clock (module docs).
    unsafe { (addr as *const u32).read_volatile() }
}

#[inline(always)]
fn wr(addr: usize, v: u32) {
    // SAFETY: as `rd`.
    unsafe { (addr as *mut u32).write_volatile(v) }
}

/// The backend, once initialised.
struct Jack {
    /// The measured PWM clock in Hz.
    clock: AtomicU32,
    /// Bus address of the ring.
    ring_bus: AtomicU32,
    /// Bus address of control block A.
    cb_bus: AtomicU32,
}

static JACK: Jack = Jack { clock: AtomicU32::new(0), ring_bus: AtomicU32::new(0), cb_bus: AtomicU32::new(0) };
static READY: AtomicBool = AtomicBool::new(false);

/// The channel [`init`] claimed.
const fn channel() -> Channel {
    Channel::owned(CHANNEL)
}

/// Both PWM channels off, `DMAC.ENAB` off, FIFO cleared, flags cleared (the
/// probe's `pwm_off`, `kernel/src/snd_probe.rs:786@pwm_off`).
fn pwm_off() {
    wr(CTL, 0);
    let d = rd(DMAC);
    if d & DMAC_ENAB != 0 {
        wr(DMAC, d & !DMAC_ENAB);
    }
    wr(CTL, CTL_CLRF);
    wr(STA, STA_FLAGS);
}

impl SoundHw for Jack {
    fn clock_hz(&self) -> u32 {
        self.clock.load(Ordering::Relaxed)
    }

    fn write(&self, at: usize, words: &[u32]) {
        let n = words.len().min(RING_WORDS.saturating_sub(at));
        let base = ring_va() + 4 * at;
        for (i, &w) in words[..n].iter().enumerate() {
            // SAFETY: `at + i < RING_WORDS`: inside the ring static; the tick
            // is the ring's only writer (IRQs masked, single CPU).
            unsafe { ((base + 4 * i) as *mut u32).write_volatile(w) };
        }
        // §12.6 item 3 (a): the DMA reads RAM, not the CPU's cache.
        // SAFETY: the range lies inside the ring static.
        unsafe { crate::arch::aarch64::mmu::clean_dcache_range(base, 4 * n) };
    }

    fn start(&self, range: u32) {
        pwm_off();
        wr(RNG1, range);
        wr(RNG2, range);
        wr(DMAC, DMAC_ENAB | DMAC_THRESHOLDS);
        channel().start(self.cb_bus.load(Ordering::Relaxed));
        // The DMA fills the FIFO through DREQ before the outputs start
        // (§12.4 item 5); bounded, a FIFO that does not fill is reported by
        // the outputs staying at the idle level, not by a hang.
        let _ = dma::wait_us(1000, || rd(STA) & STA_FULL1 != 0);
        wr(CTL, CTL_PWEN1 | CTL_USEF1 | CTL_PWEN2 | CTL_USEF2);
    }

    fn set_range(&self, range: u32) {
        wr(RNG1, range);
        wr(RNG2, range);
    }

    fn read_index(&self) -> Option<usize> {
        let off = channel().source_ad().wrapping_sub(self.ring_bus.load(Ordering::Relaxed)) as usize;
        (off < RING_BYTES).then_some(off / 4)
    }
}

/// Bring the backend up at boot (after the 200 Hz tick exists, before IRQs
/// open). On success the XBIOS group answers and `_SND` = `0x14` is in the
/// jar; otherwise nothing changed and the reason is logged.
pub fn init() -> bool {
    use super::mailbox::{get_clock_rate_measured, set_clock_rate, set_clock_state};
    // §12.2: on, request 100 MHz (reply ignored), use the measured rate.
    let state = set_clock_state(CLOCK_ID_PWM, true);
    let _ = set_clock_rate(CLOCK_ID_PWM, PWM_REQUEST_HZ, true);
    let Some(clock) = get_clock_rate_measured(CLOCK_ID_PWM) else {
        crate::kprintln!(
            "[sound] no PWM clock (firmware: set clock state {:?}, no measured rate) — no sound backend, _SND absent",
            state
        );
        return false;
    };
    let ring_bus = dma::bus_address(ring_va(), RING_BYTES);
    let cbs_va = (&raw mut CBS) as usize;
    let cb_bus = dma::bus_address(cbs_va, core::mem::size_of::<[ControlBlock; 2]>());
    let (Some(ring_bus), Some(cb_bus)) = (ring_bus, cb_bus) else {
        crate::kprintln!("[sound] ring or control blocks above 1 GiB — no sound backend, _SND absent");
        return false;
    };
    let Some(ch) = Channel::claim(CHANNEL) else {
        crate::kprintln!("[sound] DMA channel {} busy — no sound backend, _SND absent", CHANNEL);
        return false;
    };

    // Pins and DREQ mux (§12.4 items 1-2), PWM off with the DMA request on.
    let f = rd(GPFSEL4);
    wr(GPFSEL4, (f & !FSEL40_41_MASK) | FSEL40_41_ALT0);
    wr(PACTL_CS, rd(PACTL_CS) | PACTL_DREQ1_PWM1);
    pwm_off();

    // Two blocks, one per ring half, chained in a cycle (§12.5 items 3-4).
    let half = (RING_BYTES / 2) as u32;
    let ti = dma::ti_permap(DREQ_PWM1) | dma::TI_DEST_DREQ | dma::TI_SRC_INC | dma::TI_WAIT_RESP;
    let block = core::mem::size_of::<ControlBlock>() as u32;
    let blocks = [
        ControlBlock { ti, source_ad: ring_bus, dest_ad: FIF1_BUS, txfr_len: half, nextconbk: cb_bus + block, ..ControlBlock::ZERO },
        ControlBlock { ti, source_ad: ring_bus + half, dest_ad: FIF1_BUS, txfr_len: half, nextconbk: cb_bus, ..ControlBlock::ZERO },
    ];
    // SAFETY: boot CPU; nothing reads the blocks before the channel starts.
    unsafe {
        CBS = blocks;
        crate::arch::aarch64::mmu::clean_dcache_range(cbs_va, core::mem::size_of::<[ControlBlock; 2]>());
    }

    JACK.clock.store(clock, Ordering::Relaxed);
    JACK.ring_bus.store(ring_bus, Ordering::Relaxed);
    JACK.cb_bus.store(cb_bus, Ordering::Relaxed);
    READY.store(true, Ordering::Release);
    tos_api::xbios::sound::set_available();
    // The sound driver sets `_SND` after its probe (ADR-012 amendment
    // 2026-11-12).
    let cookie = tos_api::cookie::set(tos_api::cookie::SND, tos_api::cookie::SND_VALUE);
    crate::kprintln!(
        "[sound] PWM jack backend: clk_pwm measured {} Hz, DMA channel {}, ring {} words at bus {:#010x}, _SND = {:#x}{}",
        clock,
        ch.number(),
        RING_WORDS,
        ring_bus,
        tos_api::cookie::SND_VALUE,
        if cookie { "" } else { " (NOT set: jar full)" }
    );
    true
}

/// A sound backend initialised.
pub fn ready() -> bool {
    READY.load(Ordering::Acquire)
}

/// The 200 Hz tick's sound work, from the tick branch of the IRQ handler
/// (after the EOI, IRQs masked; `pwm-audio.md` §12.8: not behind the frame
/// tick's gate).
pub fn tick() {
    if ready() {
        tos_api::xbios::sound::tick(&JACK);
    }
}
