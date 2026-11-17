//! XBIOS DMA sound — `Locksnd` … `Buffptr` (XBIOS 0x80–0x8D), and the
//! converter that turns a program's `Setbuffer` frame into the duty-cycle
//! words of the PWM jack's staging ring (TASK/SND05).
//!
//! Spec: `docs/api/xbios/_dmasound-group.md` (owner-approved 2026-11-12);
//! hardware contract `docs/hardware/pwm-audio.md` §12; EmuTOS
//! `references/emutos/bios/dmasound.c:215-803`.
//!
//! **Availability.** EmuTOS answers every call of the group with its function
//! number while no sound chip is present (`SOUND_IS_AVAILABLE`,
//! `bios/dmasound.c:122-126`). NewTOS does the same until a sound backend has
//! initialised ([`set_available`], called by the kernel's sound driver after
//! its probe; on QEMU and on `aarch64-virt` it never is). Then the group
//! answers as EmuTOS' **Falcon branch** (`has_falcon_dmasound`), the machine
//! the `_SND` value `0x14` announces (ADR-012 amendment 2026-11-12), with the
//! NewTOS rules of spec §9:
//!
//! * recording (§9.4): `Setbuffer(SR_RECORD, …)` and the record bits of
//!   `Buffoper` answer `EBADRQ`; `SndBufPtr.record` reads `0`;
//! * the DSP connector (§9.5): `Dsptristate` and `Gpio` answer `EBADRQ`;
//! * the frame-end interrupt (§9.1, option D, SND01 Q-N2): `Setinterrupt`
//!   records the request, warns once per process (ADR-031, as the MFP vector
//!   the frame end would use) and nothing is ever raised; programs poll `Buffoper(-1)` / `Buffptr`;
//! * `SndBufPtr` in the native layout (§9.2, Q-N3): `play` and `record` as
//!   64-bit values at offsets 0 and 8, `res1`/`res2` untouched;
//! * the byte order of 16-bit samples (§9.3.1, Q-N4a-c): big-endian by
//!   default, `Soundcmd(9, SND_FORMATLITTLEENDIAN)` switches, latched per
//!   frame at `Setbuffer`, reset when the setting process ends.
//!
//! **Register image.** The values the inquiries return are kept as EmuTOS
//! keeps them in the Falcon's registers (`bios/dmasound.c:25-72`); the image
//! is internals (spec §8), only what a program can read back through the
//! calls is contract. Fields no call reads back and no playback rule uses
//! (the crossbar bits, `Gpio`'s port) are not kept.
//!
//! **Playback** (spec §4, §9.1.2 S3–S5, the `falcontc` column of §9.1.6).
//! The calls only change state; the work happens in [`tick`], which the
//! kernel calls from the 200 Hz tick (the tick branch of the IRQ handler, not
//! behind the frame-tick gate; `docs/hardware/pwm-audio.md` §12.8). The tick
//! reads the DMA engine's position in the staging ring, accounts the slots
//! the engine has read since the last tick (pointer, end of a single-shot
//! frame), and converts the program's frame ahead of it into the ring. A
//! **slot** is one stereo sample frame = two ring words (`L R`). Every slot
//! carries a record of what it holds (frame, address after the sample, start
//! generation), so that a state change (`Buffoper`, `Setbuffer`, a rate
//! change) can rewrite the ring from just ahead of the engine instead of
//! waiting until the already converted words have played out.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::bdos::process::hooks::{note_running, Hook, Via};
use crate::trap::TrapArgs;
use tos_types::error::EBADRQ;

// ─── Constants ───────────────────────────────────────────────────────────

/// Words of the staging ring (`docs/hardware/pwm-audio.md` §12.8: 2 048 words,
/// two halves of 1 024, four ticks of audio at the top rate).
pub const RING_WORDS: usize = 2048;
/// Stereo sample frames in the ring (two words each, `L R`).
const SLOTS: usize = RING_WORDS / 2;
/// Slots between the engine's read position and the first slot the converter
/// may rewrite: the engine reads ahead in bursts while the tick runs (16
/// slots = 0.32 ms at 50 kHz; inference, not measured).
const GUARD: usize = 16;
/// Slots the converter keeps written ahead of the engine: the whole ring but
/// a guard on either side, so a late tick finds converted audio, not old ring
/// contents (`pwm-audio.md` §12.5 item 6, §12.8).
const LEAD: usize = SLOTS - 2 * GUARD;

// Function numbers, `references/emutos/bios/xbios.c:1500-1513`.
/// `Locksnd`.
pub const LOCKSND: u16 = 0x80;
/// `Unlocksnd`.
pub const UNLOCKSND: u16 = 0x81;
/// `Soundcmd`.
pub const SOUNDCMD: u16 = 0x82;
/// `Setbuffer`.
pub const SETBUFFER: u16 = 0x83;
/// `Setmode`.
pub const SETMODE: u16 = 0x84;
/// `Settracks`.
pub const SETTRACKS: u16 = 0x85;
/// `Setmontracks`.
pub const SETMONTRACKS: u16 = 0x86;
/// `Setinterrupt`.
pub const SETINTERRUPT: u16 = 0x87;
/// `Buffoper`.
pub const BUFFOPER: u16 = 0x88;
/// `Dsptristate`.
pub const DSPTRISTATE: u16 = 0x89;
/// `Gpio`.
pub const GPIO: u16 = 0x8A;
/// `Devconnect`.
pub const DEVCONNECT: u16 = 0x8B;
/// `Sndstatus`.
pub const SNDSTATUS: u16 = 0x8C;
/// `Buffptr`.
pub const BUFFPTR: u16 = 0x8D;

/// `SNDNOTLOCK` — `Unlocksnd` without a lock (`references/emutos/bios/dmasound.h:19`).
pub const SNDNOTLOCK: i64 = -128;
/// `SNDLOCKED` — `Locksnd` while locked (`references/emutos/bios/dmasound.h:20`).
pub const SNDLOCKED: i64 = -129;

/// `Soundcmd` mode `SETPRESCALE` (`references/freemint/sys/mint/falcon.h:243`).
const SETPRESCALE: i16 = 6;
/// `Soundcmd` mode `SET16BITFORMAT` — the NewTOS byte-order switch (spec
/// §9.3.1; number from `sdk/libc/overlay/include/mint/falcon.h:221@SET16BITFORMAT`).
pub const SET16BITFORMAT: i16 = 9;
/// `SND_FORMATSIGNED` (`sdk/libc/overlay/include/mint/falcon.h:352@SND_FORMATSIGNED`).
pub const SND_FORMATSIGNED: u8 = 0x01;
/// `SND_FORMATUNSIGNED` (same header, the line after).
pub const SND_FORMATUNSIGNED: u8 = 0x02;
/// `SND_FORMATBIGENDIAN` (`…/falcon.h:354@SND_FORMATBIGENDIAN`).
pub const SND_FORMATBIGENDIAN: u8 = 0x04;
/// `SND_FORMATLITTLEENDIAN` (`…/falcon.h:355@SND_FORMATLITTLEENDIAN`).
pub const SND_FORMATLITTLEENDIAN: u8 = 0x08;
/// The format after start and after the setting process ends: signed,
/// big-endian (spec §9.3.1 item 1).
pub const FORMAT_DEFAULT: u8 = SND_FORMATSIGNED | SND_FORMATBIGENDIAN;

// `mode_control` bits (`bios/dmasound.c:411-416`).
const MC_PRESCALE: u8 = 0x03;
const MC_16BIT: u8 = 0x40;
const MC_MONO: u8 = 0x80;
// `Buffoper` bits (`references/freemint/sys/mint/falcon.h:272-275`).
const SB_PLA_ENA: u8 = 0x01;
const SB_PLA_RPT: u8 = 0x02;
const SB_REC_BITS: i16 = 0x0C;

/// The STe-compatible rates by `mode_control` bits 0-1 (spec §4.7, the rates
/// FreeMiNT lists, `references/freemint/sys/xdd/audio/falcon.c:209-250`):
/// 0 = 6 285 Hz ("leave it at 6 kHz", `bios/dmasound.c:714`), 1 = 12 517,
/// 2 = 25 033, 3 = 50 066.
pub const STE_RATES: [u32; 4] = [6285, 12517, 25033, 50066];
/// The Falcon's internal clock, 25.175 MHz (`references/freemint/sys/mint/falcon.h:301`
/// `CLK25M`); a prescale `p` gives `25 175 000 / (256 (p + 1))` Hz (spec §4.7).
pub const FALCON_CLOCK_HZ: u64 = 25_175_000;

// ─── The hardware the converter drives ───────────────────────────────────

/// The sound backend's hardware side, implemented by the kernel driver (the
/// PWM jack: DMA channel over the staging ring, `docs/hardware/pwm-audio.md`
/// §12). Called only from [`tick`], i.e. from the 200 Hz tick with IRQs
/// masked.
pub trait SoundHw {
    /// The PWM clock the firmware measured, in Hz (§12.2): the base of every
    /// range value.
    fn clock_hz(&self) -> u32;
    /// Store `words` into the ring from word index `at` on (never past the
    /// ring's end) and make them visible to the DMA engine (cache clean).
    fn write(&self, at: usize, words: &[u32]);
    /// Start the transport: the ring is filled; set both channels' range to
    /// `range`, start the DMA engine at word 0 and enable the PWM.
    fn start(&self, range: u32);
    /// Change the range (= the sample rate) of the running output.
    fn set_range(&self, range: u32);
    /// The ring word the DMA engine reads next, `None` when it cannot be
    /// read (the engine is between two control blocks).
    fn read_index(&self) -> Option<usize>;
}

// ─── State ───────────────────────────────────────────────────────────────

/// Bits of [`Frame::fmt`].
const F_16BIT: u8 = 0x01;
const F_MONO: u8 = 0x02;
const F_LITTLE: u8 = 0x04;

/// One pass over a program buffer, as the converter reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Frame {
    /// `Setbuffer` start address (program address).
    start: u64,
    /// `Setbuffer` end address, exclusive (spec §4.2).
    end: u64,
    /// Kernel alias of `start`, 0 when the range is not readable pool memory
    /// (then the pass plays as silence, at its length).
    kva: u64,
    /// [`F_16BIT`] | [`F_MONO`] | [`F_LITTLE`].
    fmt: u8,
}

impl Frame {
    const NONE: Frame = Frame { start: 0, end: 0, kva: 0, fmt: 0 };

    /// Bytes per sample frame: 8-bit stereo 2, 16-bit stereo 4, 8-bit mono 1
    /// (spec §4.8; FreeMiNT `sys/mint/falcon.h:213-222`).
    fn bytes_per_frame(self) -> u64 {
        let width = if self.fmt & F_16BIT != 0 { 2 } else { 1 };
        if self.fmt & F_MONO != 0 { width } else { 2 * width }
    }

    /// The first address behind the last whole sample frame.
    fn stop(self) -> u64 {
        let len = self.end.saturating_sub(self.start);
        self.start + len / self.bytes_per_frame() * self.bytes_per_frame()
    }
}

/// [`Slot::flags`]: the slot holds a sample of its frame.
const S_SAMPLE: u8 = 0x01;
/// [`Slot::flags`]: a single-shot frame ended before this slot — when the
/// engine reads it, `Buffoper(-1)` bit 0 drops (S3).
const S_END: u8 = 0x02;

/// What one ring slot holds.
#[derive(Clone, Copy, Debug)]
struct Slot {
    frame: Frame,
    /// The program address behind this slot's sample: `Buffptr`'s `play`
    /// once the engine has read the slot.
    after: u64,
    /// The start generation ([`Regs::epoch`]) the slot belongs to.
    epoch: u16,
    flags: u8,
}

impl Slot {
    const SILENT: Slot = Slot { frame: Frame::NONE, after: 0, epoch: 0, flags: 0 };
}

/// Where the converter goes on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cursor {
    /// Silence (mid-level words).
    Silence,
    /// A single-shot frame of generation `epoch` has ended: the next slot is
    /// silence marked [`S_END`].
    Ending { epoch: u16 },
    /// Converting `frame`, next sample at `next`.
    Play { frame: Frame, next: u64, epoch: u16 },
}

/// The sound system's state as programs see it — EmuTOS' register image,
/// as far as a call reads it back or playback uses it.
#[derive(Clone, Copy, Debug)]
struct Regs {
    locked: bool,
    /// `$FFFF8921`: bits 0-1 STe prescale, bit 6 16 bit, bit 7 mono.
    mode_control: u8,
    /// `$FFFF8920`: bits 0-1 play tracks, bits 4-5 monitor track.
    track_control: u8,
    /// `$FFFF8936`.
    record_tracks: u8,
    /// `$FFFF893A`: left atten. bits 8-11, right bits 4-7.
    attenuation: u16,
    /// `$FFFF8939`: left gain bits 4-7, right bits 0-3.
    amplification: u8,
    /// `$FFFF8937` (ADDERIN).
    adder_source: u8,
    /// `$FFFF8938` (ADCINPUT).
    adc_source: u8,
    /// `$FFFF8935`: internal-clock prescale; 0 = STe-compatible rates.
    freq_int: u8,
    /// `$FFFF8934`: external-clock prescale (no external clock: kept only).
    freq_ext: u8,
    /// `$FFFF8900`: bits 0-1 MFP i7 cause, bits 2-3 Timer A cause.
    interrupt: u8,
    /// `$FFFF8901` bits 0-1: play enable, play repeat (the record bits are
    /// never set, §9.4).
    control: u8,
    /// `Soundcmd(9)` format ([`FORMAT_DEFAULT`] or little-endian).
    format: u8,
    /// The process slot that set [`Self::format`] (Q-N4b).
    format_owner: Option<usize>,
    /// `Setbuffer(SR_PLAY)`: start, end, byte order latched (Q-N4c).
    buf_start: u64,
    buf_end: u64,
    buf_little: bool,
    /// `Buffptr`'s `play`.
    play_ptr: u64,
    /// Start generation: counts `Buffoper` starts (bit 0 from clear to set).
    epoch: u16,
    /// A start is waiting for the converter.
    start_pending: bool,
    /// The converter must rewrite the ring ahead of the engine.
    dirty: bool,
}

impl Regs {
    /// The state `falcon_dmasound_init` leaves (`bios/dmasound.c:182-198`,
    /// spec §5): DMA playback → DAC, internal clock, STe-compatible mode
    /// (`Devconnect(0,8,0,0,1)`), 8-bit stereo, gain 8 left and right,
    /// ADDERIN 3, ADCINPUT 3, prescale 3 (50 kHz).
    const BOOT: Regs = Regs {
        locked: false,
        mode_control: 0x03,
        track_control: 0,
        record_tracks: 0,
        attenuation: 0,
        amplification: 0x88,
        adder_source: 0x03,
        adc_source: 0x03,
        freq_int: 0,
        freq_ext: 0,
        interrupt: 0,
        control: 0,
        format: FORMAT_DEFAULT,
        format_owner: None,
        buf_start: 0,
        buf_end: 0,
        buf_little: false,
        play_ptr: 0,
        epoch: 0,
        start_pending: false,
        dirty: false,
    };

    /// The PWM range for the rate the registers select, at PWM clock
    /// `clock_hz` (`docs/hardware/pwm-audio.md` §12.3: `RNG = round(f_pwm /
    /// f_sample)`, no resampling, SND01 Q-N6 and its addendum). `freq_int` 0
    /// selects the STe-compatible rate of `mode_control` bits 0-1 (FreeMiNT
    /// `CLKOLD`, "TT compatible, see Soundcmd(SETPRESCALE)",
    /// `references/freemint/sys/mint/falcon.h:309`); any other value is the
    /// Falcon prescale `p`, `25 175 000 / (256 (p + 1))` Hz (spec §4.7).
    fn range(&self, clock_hz: u32) -> u32 {
        let clock = u64::from(clock_hz);
        let r = if self.freq_int == 0 {
            let hz = u64::from(STE_RATES[usize::from(self.mode_control & MC_PRESCALE)]);
            (clock + hz / 2) / hz
        } else {
            // clock / (FALCON / (256 (p + 1))), rounded.
            let num = clock * 256 * (u64::from(self.freq_int) + 1);
            (num + FALCON_CLOCK_HZ / 2) / FALCON_CLOCK_HZ
        };
        r.clamp(2, u64::from(u32::MAX)) as u32
    }

    /// The frame a new pass reads: the `Setbuffer` registers, the mode of now.
    fn next_frame(&self) -> Frame {
        let mut fmt = 0;
        if self.mode_control & MC_16BIT != 0 {
            fmt |= F_16BIT;
        }
        if self.mode_control & MC_MONO != 0 {
            fmt |= F_MONO;
        }
        if self.buf_little {
            fmt |= F_LITTLE;
        }
        let len = self.buf_end.saturating_sub(self.buf_start);
        let kva = if len == 0 {
            0
        } else {
            crate::bdos::process::usermem::pool_kva(self.buf_start, len).unwrap_or(0)
        };
        Frame { start: self.buf_start, end: self.buf_end, kva, fmt }
    }
}

/// The converter's side: the ring's slot records and where it writes.
struct Engine {
    /// The transport runs (started once, then kept running; an idle ring is
    /// silence the engine keeps reading).
    running: bool,
    /// The range the output runs at.
    range: u32,
    /// The slot the engine had reached at the last tick (all before it are
    /// accounted).
    played: usize,
    /// The next slot the converter writes.
    written: usize,
    /// Silent slots written in a row (a ring full of them needs no writes).
    silent: usize,
    cursor: Cursor,
    slots: [Slot; SLOTS],
}

impl Engine {
    const IDLE: Engine = Engine {
        running: false,
        range: 0,
        played: 0,
        written: 0,
        silent: 0,
        cursor: Cursor::Silence,
        slots: [Slot::SILENT; SLOTS],
    };
}

struct State {
    regs: Regs,
    engine: Engine,
}

struct Shared(UnsafeCell<State>);
// SAFETY: every access goes through `with`/`try_with`, which hold `BUSY`.
unsafe impl Sync for Shared {}

static STATE: Shared = Shared(UnsafeCell::new(State { regs: Regs::BOOT, engine: Engine::IDLE }));
static BUSY: AtomicBool = AtomicBool::new(false);
/// A sound backend has initialised (`SOUND_IS_AVAILABLE`).
static AVAILABLE: AtomicBool = AtomicBool::new(false);

/// Run `f` on the state, from a trap. In the kernel traps run with IRQs
/// masked, so the tick cannot hold the flag; host-test threads wait for it.
fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    while BUSY.compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
        core::hint::spin_loop();
    }
    // SAFETY: `BUSY` is held; nothing else touches `STATE` until it is
    // released below; `f` never re-enters this module's locks.
    let r = f(unsafe { &mut *STATE.0.get() });
    BUSY.store(false, Ordering::Release);
    r
}

/// [`with`] that gives up when the state is busy (the tick never waits).
fn try_with<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    if BUSY.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
        return None;
    }
    // SAFETY: as in `with`.
    let r = f(unsafe { &mut *STATE.0.get() });
    BUSY.store(false, Ordering::Release);
    Some(r)
}

/// The kernel's sound driver reports that a backend has initialised: from
/// now on the group answers (`SOUND_IS_AVAILABLE`). The driver sets the
/// `_SND` cookie itself (ADR-012 amendment 2026-11-12).
pub fn set_available() {
    AVAILABLE.store(true, Ordering::Release);
}

/// A sound backend has initialised.
pub fn available() -> bool {
    AVAILABLE.load(Ordering::Acquire)
}

/// The process in `slot` ends: a byte order it set goes back to big-endian
/// (spec §9.3.1, Q-N4b). Nothing else of the sound state belongs to a
/// process (EmuTOS keeps playing and keeps the lock, `bios/dmasound.c`).
pub fn on_terminate(slot: usize) {
    with(|s| {
        if s.regs.format_owner == Some(slot) {
            s.regs.format = FORMAT_DEFAULT;
            s.regs.format_owner = None;
        }
    });
}

/// The process slot of the caller, `None` for a kernel caller.
fn caller_slot() -> Option<usize> {
    crate::bdos::process::slot_for(crate::bdos::process::current())
}

// ─── Dispatch ────────────────────────────────────────────────────────────

/// XBIOS 0x80–0x8D. `None` for any other id.
pub fn dispatch(args: &TrapArgs) -> Option<i64> {
    let id = args.func_id();
    if !(LOCKSND..=BUFFPTR).contains(&id) {
        return None;
    }
    if !available() {
        // `if (!SOUND_IS_AVAILABLE) return <function number>;` — every one of
        // the 14 (`bios/dmasound.c:217-783`).
        return Some(i64::from(id));
    }
    Some(match id {
        LOCKSND => locksnd(),
        UNLOCKSND => unlocksnd(),
        SOUNDCMD => soundcmd(args.arg_i16(0), args.arg_i16(1)),
        SETBUFFER => setbuffer(args.arg_u16(0), args.arg_u64(1), args.arg_u64(2)),
        SETMODE => setmode(args.arg_u16(0)),
        SETTRACKS => settracks(args.arg_u16(0), args.arg_u16(1)),
        SETMONTRACKS => setmontracks(args.arg_u16(0)),
        SETINTERRUPT => setinterrupt(args.arg_u16(0), args.arg_i16(1)),
        BUFFOPER => buffoper(args.arg_i16(0)),
        DSPTRISTATE => dsptristate(),
        GPIO => gpio(),
        DEVCONNECT => devconnect(args.arg_i16(0), args.arg_i16(2), args.arg_i16(3)),
        SNDSTATUS => sndstatus(args.arg_i16(0)),
        _ => buffptr(args),
    })
}

// ─── The functions ───────────────────────────────────────────────────────

/// `Locksnd()` (0x80): `SNDLOCKED` when locked, else lock and `1`
/// (`bios/dmasound.c:215-225`). Advisory: no other call reads the lock (§3,
/// §9.7).
pub fn locksnd() -> i64 {
    with(|s| {
        if s.regs.locked {
            SNDLOCKED
        } else {
            s.regs.locked = true;
            1
        }
    })
}

/// `Unlocksnd()` (0x81): `SNDNOTLOCK` when not locked, else unlock and `0`
/// (`bios/dmasound.c:230-240`).
pub fn unlocksnd() -> i64 {
    with(|s| {
        if s.regs.locked {
            s.regs.locked = false;
            0
        } else {
            SNDNOTLOCK
        }
    })
}

/// `Soundcmd(mode, data)` (0x82): `data == -1` inquires, any other value sets
/// and the new value is returned; never an error (`bios/dmasound.c:246-364`,
/// Falcon branch). Mode 6 is handled first (`:347-356`); mode 9 is the NewTOS
/// byte-order switch (spec §9.3.1); every other mode answers `0`.
pub fn soundcmd(mode: i16, data: i16) -> i64 {
    with(|s| {
        let r = &mut s.regs;
        let set = data != -1;
        let v: i64 = match mode {
            SETPRESCALE => {
                if (0..=3).contains(&data) {
                    r.mode_control = (r.mode_control & !MC_PRESCALE) | data as u8;
                    r.dirty = true;
                }
                i64::from(r.mode_control & MC_PRESCALE)
            }
            0 => {
                if set {
                    r.attenuation = (r.attenuation & 0xF0FF) | ((data as u16 & 0x00F0) << 4);
                }
                i64::from((r.attenuation >> 4) & 0x00F0)
            }
            1 => {
                if set {
                    r.attenuation = (r.attenuation & 0xFF0F) | (data as u16 & 0x00F0);
                }
                i64::from(r.attenuation & 0x00F0)
            }
            2 => {
                if set {
                    r.amplification = (r.amplification & 0x0F) | (data as u8 & 0xF0);
                }
                i64::from(r.amplification & 0xF0)
            }
            3 => {
                if set {
                    r.amplification = (r.amplification & 0xF0) | ((data as u8 & 0xF0) >> 4);
                }
                i64::from((r.amplification & 0x0F) << 4)
            }
            4 => {
                if set {
                    r.adder_source = data as u8;
                }
                i64::from(r.adder_source & 0x03)
            }
            5 => {
                if set {
                    r.adc_source = data as u8;
                }
                i64::from(r.adc_source & 0x03)
            }
            SET16BITFORMAT => {
                if let Some(f) = valid_format(data) {
                    r.format = f;
                    r.format_owner = if f == FORMAT_DEFAULT { None } else { caller_slot() };
                }
                i64::from(r.format)
            }
            _ => 0,
        };
        v
    })
}

/// A `Soundcmd(9, data)` value that sets the format: exactly one of
/// big-/little-endian, optionally signed, nothing else (spec §9.3.1 item 3).
/// Samples are always signed, so the stored value carries
/// [`SND_FORMATSIGNED`].
fn valid_format(data: i16) -> Option<u8> {
    if !(0..=0xFF).contains(&data) {
        return None;
    }
    let d = data as u8;
    let order = d & (SND_FORMATBIGENDIAN | SND_FORMATLITTLEENDIAN);
    let known = SND_FORMATSIGNED | SND_FORMATBIGENDIAN | SND_FORMATLITTLEENDIAN;
    if d & !known != 0 || (order != SND_FORMATBIGENDIAN && order != SND_FORMATLITTLEENDIAN) {
        return None;
    }
    Some(order | SND_FORMATSIGNED)
}

/// `Setbuffer(mode, start, end)` (0x83): `mode > 1` → `EBADRQ`
/// (`bios/dmasound.c:374`); `SR_RECORD` (1) → `EBADRQ`, there is no input
/// (spec §9.4); `SR_PLAY` (0) sets the playback frame — full 64-bit
/// addresses (§9.2), the byte order latched now (Q-N4c) — and returns `0`.
/// The addresses are not dereferenced here; the converter reads the frame
/// only if it lies in pool memory, otherwise it plays as silence.
pub fn setbuffer(mode: u16, start: u64, end: u64) -> i64 {
    if mode != 0 {
        return EBADRQ as i64;
    }
    with(|s| {
        s.regs.buf_start = start;
        s.regs.buf_end = end;
        s.regs.buf_little = s.regs.format & SND_FORMATLITTLEENDIAN != 0;
        s.regs.dirty = true;
    });
    0
}

/// `Setmode(mode)` (0x84): `mode > 2` → `EBADRQ`; else `STEREO8` 0,
/// `STEREO16` 1 (bit 6), `MONO8` 2 (bit 7), `0` (`bios/dmasound.c:401-419`).
pub fn setmode(mode: u16) -> i64 {
    if mode > 2 {
        return EBADRQ as i64;
    }
    with(|s| {
        let mut mc = s.regs.mode_control & 0x3F;
        if mode == 2 {
            mc |= MC_MONO;
        }
        if mode == 1 {
            mc |= MC_16BIT;
        }
        s.regs.mode_control = mc;
        s.regs.dirty = true;
    });
    0
}

/// `Settracks(play, rec)` (0x85): either `> 3` → `EBADRQ`; else stored, `0`
/// (`bios/dmasound.c:424-436`). Playback always converts track 0
/// (`docs/DEVIATIONS.md`).
pub fn settracks(play: u16, rec: u16) -> i64 {
    if play > 3 || rec > 3 {
        return EBADRQ as i64;
    }
    with(|s| {
        s.regs.track_control = (s.regs.track_control & 0xFC) | play as u8;
        s.regs.record_tracks = rec as u8;
    });
    0
}

/// `Setmontracks(track)` (0x86): `> 3` → `EBADRQ`; else stored, `0`
/// (`bios/dmasound.c:441-452`).
pub fn setmontracks(track: u16) -> i64 {
    if track > 3 {
        return EBADRQ as i64;
    }
    with(|s| s.regs.track_control = (s.regs.track_control & 0xCF) | ((track as u8) << 4));
    0
}

/// `Setinterrupt(mode, cause)` (0x87): `mode > 1` → `EBADRQ`; else `cause & 3`
/// into bits 2-3 (Timer A, mode 0) or 0-1 (MFP i7, mode 1), `0`
/// (`bios/dmasound.c:457-494`, S1). NewTOS raises no interrupt (spec §9.1,
/// option D): a request with a cause warns once per process (ADR-031).
pub fn setinterrupt(mode: u16, cause: i16) -> i64 {
    if mode > 1 {
        return EBADRQ as i64;
    }
    let c = (cause & 3) as u8;
    with(|s| {
        let irq = s.regs.interrupt;
        s.regs.interrupt = if mode == 0 { (irq & 0xF3) | (c << 2) } else { (irq & 0xFC) | c };
    });
    if c != 0 {
        // The frame end would reach the program through Timer A (MFP 13,
        // mode 0) or GPIP7 (MFP 15, mode 1), spec §4.4 / S6.
        let mfp = if mode == 0 { 13 } else { 15 };
        note_running(Hook::Vector(0x40 + mfp), u64::from(c), Via::Setinterrupt);
    }
    0
}

/// `Buffoper(mode)` (0x88): a negative `mode` inquires, `control & 3` (the
/// record bits are never set); a record bit (`SB_REC_ENA`/`SB_REC_RPT`) →
/// `EBADRQ` (spec §9.4); else `control = mode & 3`, `0`
/// (`bios/dmasound.c:499-516`). Setting bit 0 from clear starts a frame from
/// the `Setbuffer` registers; clearing it stops playback (the pointer stays
/// where it is, §9.1.6 `falcontc`).
pub fn buffoper(mode: i16) -> i64 {
    if mode < 0 {
        return with(|s| i64::from(s.regs.control & 0x03));
    }
    if mode & SB_REC_BITS != 0 {
        return EBADRQ as i64;
    }
    let new = (mode & 0x03) as u8;
    with(|s| {
        let r = &mut s.regs;
        if new & SB_PLA_ENA != 0 && r.control & SB_PLA_ENA == 0 {
            r.epoch = r.epoch.wrapping_add(1);
            r.start_pending = true;
            r.play_ptr = r.buf_start;
        }
        if new & SB_PLA_ENA == 0 {
            r.start_pending = false;
        }
        r.control = new;
        r.dirty = true;
    });
    0
}

/// `Dsptristate(dspxmit, dsprec)` (0x89): no DSP → `EBADRQ` (spec §9.5;
/// EmuTOS' answer without one, `bios/dmasound.c:526-527`).
pub fn dsptristate() -> i64 {
    EBADRQ as i64
}

/// `Gpio(mode, data)` (0x8A): no DSP connector → `EBADRQ` (spec §9.5;
/// `bios/dmasound.c:550-551`).
pub fn gpio() -> i64 {
    EBADRQ as i64
}

/// `Devconnect(src, dst, clk, prescale, protocol)` (0x8B): an invalid `src`
/// (not 0-3) → `0`; else the prescale goes to the external (`clk & 1`) or the
/// internal clock's prescaler and the answer is `1`
/// (`bios/dmasound.c:590-686`). The matrix itself is not modelled: DMA
/// playback always reaches the jack (`docs/DEVIATIONS.md`).
pub fn devconnect(src: i16, clk: i16, prescale: i16) -> i64 {
    if !(0..=3).contains(&src) {
        return 0;
    }
    with(|s| {
        if clk & 1 != 0 {
            s.regs.freq_ext = prescale as u8;
        } else {
            s.regs.freq_int = prescale as u8;
            s.regs.dirty = true;
        }
    });
    1
}

/// `Sndstatus(reset)` (0x8C): a non-zero `reset` zeroes the attenuation, as
/// EmuTOS does itself (`bios/dmasound.c:759-761`) — playback, the interrupt
/// request and the mode registers stay (oracle, spec §3/0x8C O4). The answer
/// is the CODEC status field (`:766`): `0`, the PWM path has no clipping or
/// clock error to report.
pub fn sndstatus(reset: i16) -> i64 {
    if reset != 0 {
        with(|s| s.regs.attenuation = 0);
    }
    0
}

/// `Buffptr(ptr)` (0x8D): writes `play` and `record` as 64-bit values at
/// `ptr` and `ptr + 8` of the native `SndBufPtr` (spec §9.2, Q-N3; `record`
/// is `0`, nothing records, §9.4), leaves `res1`/`res2` alone, returns `0`
/// (`bios/dmasound.c:772-803`). An unwritable `ptr` ends the caller through
/// the crash path (`crate::uaccess`).
pub fn buffptr(args: &TrapArgs) -> i64 {
    let ptr = args.arg_u64(0);
    let play = with(|s| s.regs.play_ptr);
    if args.space.put_u64(ptr, play).is_err() || args.space.put_u64(ptr.wrapping_add(8), 0).is_err() {
        return i64::from(crate::uaccess::FAULTED);
    }
    0
}

// ─── The converter ───────────────────────────────────────────────────────

/// The duty word for signed 16-bit sample `s` at range `range`: `0` for
/// -32768, `range / 2` for 0, below `range` for 32767 (`pwm-audio.md` §12.1:
/// a FIFO word is a duty value `0…RNG`, Table 157).
pub fn duty(s: i16, range: u32) -> u32 {
    (((i32::from(s) + 32768) as u64 * u64::from(range)) >> 16) as u32
}

/// Read one sample of `frame` at program address `at` as signed 16 bit.
fn sample(frame: Frame, at: u64, wide: bool) -> i16 {
    if frame.kva == 0 {
        return 0;
    }
    let p = frame.kva + (at - frame.start);
    // SAFETY: `frame.kva` is the kernel alias of pool memory covering
    // `[start, end)` (`usermem::pool_kva`), mapped from boot on; `at` (and
    // `at + 1` for a 16-bit sample) lie below `frame.stop() <= end`.
    unsafe {
        if wide {
            let (b0, b1) = ((p as *const u8).read_volatile(), ((p + 1) as *const u8).read_volatile());
            if frame.fmt & F_LITTLE != 0 { i16::from_le_bytes([b0, b1]) } else { i16::from_be_bytes([b0, b1]) }
        } else {
            i16::from((p as *const u8).read_volatile() as i8) << 8
        }
    }
}

impl Engine {
    /// The pass after one of generation `epoch` ended: the next pass when the
    /// repeat bit is set (from the `Setbuffer` registers of now, S5), else
    /// the end of the frame.
    fn end_of_pass(&mut self, r: &Regs, epoch: u16) {
        self.cursor = if r.control == SB_PLA_ENA | SB_PLA_RPT && epoch == r.epoch {
            Cursor::Play { frame: r.next_frame(), next: r.buf_start, epoch }
        } else {
            Cursor::Ending { epoch }
        };
    }

    /// The next slot: its two words and its record.
    fn produce(&mut self, r: &Regs) -> ([u32; 2], Slot) {
        let mid = duty(0, self.range);
        // A pass transition takes one turn; a new pass then always yields.
        for _ in 0..3 {
            match self.cursor {
                Cursor::Silence => return ([mid, mid], Slot::SILENT),
                Cursor::Ending { epoch } => {
                    self.cursor = Cursor::Silence;
                    return ([mid, mid], Slot { flags: S_END, epoch, ..Slot::SILENT });
                }
                Cursor::Play { frame, next, epoch } => {
                    let stop = frame.stop();
                    if stop == frame.start {
                        // An empty frame: one silent slot per pass.
                        self.end_of_pass(r, epoch);
                        return ([mid, mid], Slot { frame, after: frame.end, epoch, flags: S_SAMPLE });
                    }
                    if next >= stop {
                        self.end_of_pass(r, epoch);
                        continue;
                    }
                    let wide = frame.fmt & F_16BIT != 0;
                    let step = if wide { 2 } else { 1 };
                    let left = sample(frame, next, wide);
                    let right = if frame.fmt & F_MONO != 0 { left } else { sample(frame, next + step, wide) };
                    let n = next + frame.bytes_per_frame();
                    self.cursor = Cursor::Play { frame, next: n, epoch };
                    let after = if n >= stop { frame.end } else { n };
                    let words = [duty(left, self.range), duty(right, self.range)];
                    return (words, Slot { frame, after, epoch, flags: S_SAMPLE });
                }
            }
        }
        self.cursor = Cursor::Silence;
        ([mid, mid], Slot::SILENT)
    }

    /// Convert `n` slots from [`Self::written`] on into the ring.
    fn fill(&mut self, r: &Regs, hw: &dyn SoundHw, n: usize) {
        const CHUNK: usize = 32;
        let mut buf = [0u32; 2 * CHUNK];
        let mut left = n;
        while left > 0 {
            let at = self.written;
            let k = left.min(CHUNK).min(SLOTS - at);
            for i in 0..k {
                let (w, slot) = self.produce(r);
                buf[2 * i] = w[0];
                buf[2 * i + 1] = w[1];
                self.silent = if slot.flags == 0 { self.silent.saturating_add(1) } else { 0 };
                self.slots[at + i] = slot;
            }
            hw.write(2 * at, &buf[..2 * k]);
            self.written = (at + k) % SLOTS;
            left -= k;
        }
    }

    /// Account the slots the engine has read since the last tick, up to `to`:
    /// the play pointer and the end of a single-shot frame (S3, S4).
    fn account(&mut self, r: &mut Regs, to: usize) {
        let mut i = self.played;
        while i != to {
            let s = self.slots[i];
            if s.epoch == r.epoch && r.control & SB_PLA_ENA != 0 {
                if s.flags & S_SAMPLE != 0 {
                    r.play_ptr = s.after;
                }
                if s.flags & S_END != 0 {
                    r.control &= !SB_PLA_ENA;
                }
            }
            // Read: a ring lap the converter misses must not count it twice.
            self.slots[i] = Slot::SILENT;
            i = (i + 1) % SLOTS;
        }
        self.played = to;
    }

    /// Where the converter goes on after a state change. `prev` is the record
    /// of the slot before the first one rewritten; `None` after an underrun,
    /// when the ring ahead of the engine holds nothing current and the live
    /// cursor is the only record. The pass goes on when it is still current,
    /// a pending start begins a new one, a stop is silence.
    fn resume(&mut self, r: &Regs, prev: Option<Slot>) {
        self.cursor = if r.start_pending {
            Cursor::Play { frame: r.next_frame(), next: r.buf_start, epoch: r.epoch }
        } else if r.control & SB_PLA_ENA == 0 {
            Cursor::Silence
        } else {
            match prev {
                None => self.cursor,
                Some(p) if p.epoch == r.epoch && p.flags & S_SAMPLE != 0 => {
                    Cursor::Play { frame: p.frame, next: p.after, epoch: p.epoch }
                }
                // Ended (or about to end), or another generation: silent.
                Some(_) => match self.cursor {
                    Cursor::Ending { epoch } if epoch == r.epoch => self.cursor,
                    _ => Cursor::Silence,
                },
            }
        };
    }
}

/// Slots from `a` forward to `b` in the ring.
fn dist(a: usize, b: usize) -> usize {
    (b + SLOTS - a) % SLOTS
}

/// The 200 Hz tick of the sound backend (TASK/SND05, D-tick): account what
/// the DMA engine has read, apply a rate change, and convert ahead. Called by
/// the kernel from the tick branch of its IRQ handler, IRQs masked.
pub fn tick(hw: &dyn SoundHw) {
    if !available() {
        return;
    }
    let _ = try_with(|s| tick_locked(s, hw));
}

fn tick_locked(s: &mut State, hw: &dyn SoundHw) {
    let State { regs: r, engine: e } = s;
    let range = r.range(hw.clock_hz());
    if !e.running {
        if !r.start_pending {
            return;
        }
        // First start: the whole ring from word 0.
        e.range = range;
        e.slots = [Slot::SILENT; SLOTS];
        e.played = 0;
        e.written = 0;
        e.resume(r, None);
        r.start_pending = false;
        r.dirty = false;
        e.fill(r, hw, LEAD);
        let mid = duty(0, range);
        let rest = [mid; 2 * (SLOTS - LEAD)];
        hw.write(2 * LEAD, &rest);
        e.written = LEAD;
        hw.start(range);
        e.running = true;
        return;
    }
    let Some(word) = hw.read_index() else {
        return;
    };
    let at = (word / 2) % SLOTS;
    e.account(r, at);
    if range != e.range {
        hw.set_range(range);
        e.range = range;
        r.dirty = true;
    }
    let guard = (at + GUARD) % SLOTS;
    let ahead = dist(at, e.written);
    if r.dirty || r.start_pending {
        // Rewrite from just ahead of the engine; after an underrun (nothing
        // current is written there) from the live cursor.
        let prev = (ahead >= GUARD).then(|| e.slots[(guard + SLOTS - 1) % SLOTS]);
        e.written = guard;
        e.resume(r, prev);
        r.start_pending = false;
        r.dirty = false;
        e.silent = 0;
    } else if ahead < GUARD {
        // The engine overtook the converter (a late tick): go on ahead of it.
        e.written = guard;
    }
    let target = (at + LEAD) % SLOTS;
    let n = dist(e.written, target);
    if e.cursor == Cursor::Silence && e.silent >= SLOTS {
        // The whole ring is silence already.
        e.written = target;
        return;
    }
    e.fill(r, hw, n);
}

/// Back to the boot state — host tests only.
#[cfg(test)]
pub(crate) fn reset_for_test(available: bool) {
    with(|s| {
        s.regs = Regs::BOOT;
        s.engine = Engine::IDLE;
    });
    AVAILABLE.store(available, Ordering::Release);
}

#[cfg(test)]
mod tests;
