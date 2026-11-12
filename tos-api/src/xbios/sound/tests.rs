//! Host tests of the DMA sound group (TASK/SND05): every function's
//! documented return and error value (spec `docs/api/xbios/_dmasound-group.md`
//! §3, the oracle's `falcontc` column of §9.1.6), the rate table of
//! `docs/hardware/pwm-audio.md` §5.5, the duty conversion, and the converter
//! against a simulated DMA engine (S3-S5 of spec §9.1.2).

use super::*;
use std::cell::{Cell, RefCell};
use std::sync::Mutex;

/// One test at a time owns the global sound state (like the hardware).
static SERIAL: Mutex<()> = Mutex::new(());

fn lock(available: bool) -> std::sync::MutexGuard<'static, ()> {
    let g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    reset_for_test(available);
    g
}

fn call(id: u16, regs: [u64; 5]) -> i64 {
    let args = TrapArgs::kernel([regs[0], regs[1], regs[2], regs[3], regs[4], 0, 0, 0], u64::from(id), 14);
    crate::xbios::dispatch(&args)
}

/// A WORD argument as the trap passes it (sign-extended register).
fn w(v: i16) -> u64 {
    v as i64 as u64
}

/// The PWM clock measured on the real Pi (H1,
/// `measurements/measurement-2026-11-12-snd03-pi/RUNINFO.TXT`).
const PI_CLOCK: u32 = 107_143_019;

// ─── Availability ────────────────────────────────────────────────────────

#[test]
fn without_a_backend_every_call_answers_its_function_number() {
    let _g = lock(false);
    for id in LOCKSND..=BUFFPTR {
        assert_eq!(call(id, [0; 5]), i64::from(id), "id 0x{id:x}");
    }
    // The DSP range below and the slot past the group stay function numbers.
    assert_eq!(call(0x7F, [0; 5]), 0x7F);
    assert_eq!(call(0x8E, [0; 5]), 0x8E);
}

// ─── The functions (oracle falcontc, spec §9.1.6 O1) ─────────────────────

#[test]
fn locksnd_and_unlocksnd() {
    let _g = lock(true);
    assert_eq!(call(LOCKSND, [0; 5]), 1);
    assert_eq!(call(LOCKSND, [0; 5]), SNDLOCKED);
    assert_eq!(call(UNLOCKSND, [0; 5]), 0);
    assert_eq!(call(UNLOCKSND, [0; 5]), SNDNOTLOCK);
}

#[test]
fn soundcmd_boot_state_answers_as_falcontc() {
    let _g = lock(true);
    let got: Vec<i64> = (0..=7).map(|m| call(SOUNDCMD, [m, w(-1), 0, 0, 0])).collect();
    assert_eq!(got, [0, 0, 0x80, 0x80, 3, 3, 3, 0]);
}

#[test]
fn soundcmd_sets_and_returns_the_new_value() {
    let _g = lock(true);
    assert_eq!(soundcmd(0, 0x10), 0x10);
    assert_eq!(soundcmd(0, -1), 0x10);
    assert_eq!(soundcmd(1, 0x50), 0x50);
    assert_eq!(soundcmd(0, -1), 0x10, "right attenuation leaves the left");
    assert_eq!(soundcmd(2, 0x30), 0x30);
    assert_eq!(soundcmd(3, 0x70), 0x70);
    assert_eq!(soundcmd(2, -1), 0x30);
    assert_eq!(soundcmd(4, 0x01), 1);
    assert_eq!(soundcmd(5, 0x02), 2);
    // Prescale: 0-3 set, anything else answers the current value.
    assert_eq!(soundcmd(6, 4), 3, "out of range: unchanged (oracle)");
    assert_eq!(soundcmd(6, 1), 1);
    assert_eq!(soundcmd(6, -1), 1);
    // Unknown modes: 0, no error ("never returns an error code").
    assert_eq!(soundcmd(7, 5), 0);
    assert_eq!(soundcmd(-3, 5), 0);
}

#[test]
fn soundcmd_9_is_the_byte_order_switch() {
    let _g = lock(true);
    assert_eq!(soundcmd(SET16BITFORMAT, -1), 0x05, "default: signed big-endian");
    assert_eq!(soundcmd(SET16BITFORMAT, 0x08), 0x09, "little-endian, stored signed");
    assert_eq!(soundcmd(SET16BITFORMAT, -1), 0x09);
    // Invalid: both orders, none, unsigned, unknown bits — unchanged, no error.
    for bad in [0x0C, 0x01, 0x00, 0x06, 0x0A, 0x14, 0x100] {
        assert_eq!(soundcmd(SET16BITFORMAT, bad), 0x09, "data {bad:#x}");
    }
    assert_eq!(soundcmd(SET16BITFORMAT, 0x05), 0x05);
}

#[test]
fn byte_order_resets_when_its_owner_ends() {
    let _g = lock(true);
    soundcmd(SET16BITFORMAT, 0x09);
    with(|s| s.regs.format_owner = Some(3));
    on_terminate(2);
    assert_eq!(soundcmd(SET16BITFORMAT, -1), 0x09, "another process ends: stays");
    on_terminate(3);
    assert_eq!(soundcmd(SET16BITFORMAT, -1), 0x05, "the setter ends: big-endian");
}

#[test]
fn setbuffer_refuses_record_and_bad_modes() {
    let _g = lock(true);
    assert_eq!(call(SETBUFFER, [2, 0x1000, 0x2000, 0, 0]), EBADRQ as i64);
    assert_eq!(call(SETBUFFER, [1, 0x1000, 0x2000, 0, 0]), EBADRQ as i64, "no input (§9.4)");
    assert_eq!(call(SETBUFFER, [0, 0x1_2345_6000, 0x1_2345_8000, 0, 0]), 0);
    with(|s| {
        assert_eq!((s.regs.buf_start, s.regs.buf_end), (0x1_2345_6000, 0x1_2345_8000), "64-bit, no 24-bit cut");
    });
}

#[test]
fn setmode_settracks_setmontracks() {
    let _g = lock(true);
    assert_eq!(setmode(3), EBADRQ as i64);
    assert_eq!(setmode(0), 0);
    assert_eq!(setmode(1), 0);
    with(|s| assert_eq!(s.regs.mode_control & 0xC0, MC_16BIT));
    assert_eq!(setmode(2), 0);
    with(|s| assert_eq!(s.regs.mode_control, MC_MONO | 0x03, "prescale bits kept"));
    assert_eq!(settracks(4, 0), EBADRQ as i64);
    assert_eq!(settracks(0, 4), EBADRQ as i64);
    assert_eq!(settracks(0, 0), 0);
    assert_eq!(setmontracks(4), EBADRQ as i64);
    assert_eq!(setmontracks(0), 0);
    assert_eq!(call(SETMODE, [w(-1), 0, 0, 0, 0]), EBADRQ as i64, "UWORD: -1 is 0xFFFF");
}

#[test]
fn setinterrupt_answers_s1_and_records_the_bits() {
    let _g = lock(true);
    assert_eq!(setinterrupt(2, 0), EBADRQ as i64);
    assert_eq!(setinterrupt(0, 0), 0);
    assert_eq!(setinterrupt(1, 0), 0);
    assert_eq!(setinterrupt(0, 1), 0);
    with(|s| assert_eq!(s.regs.interrupt, 0x04, "$8900 after (0,1), oracle"));
    assert_eq!(setinterrupt(1, 1), 0);
    with(|s| assert_eq!(s.regs.interrupt, 0x05));
    assert_eq!(setinterrupt(0, 0), 0);
    with(|s| assert_eq!(s.regs.interrupt, 0x01));
}

#[test]
fn buffoper_inquiry_record_bits_and_set() {
    let _g = lock(true);
    assert_eq!(buffoper(-1), 0);
    assert_eq!(buffoper(-2), 0);
    assert_eq!(buffoper(4), EBADRQ as i64, "SB_REC_ENA: no input");
    assert_eq!(buffoper(8 | 1), EBADRQ as i64);
    assert_eq!(buffoper(-1), 0, "a refused call changes nothing");
    assert_eq!(buffoper(3), 0);
    assert_eq!(buffoper(-1), 3);
    assert_eq!(buffoper(0), 0);
    assert_eq!(buffoper(-1), 0);
}

#[test]
fn dsptristate_and_gpio_have_no_dsp() {
    let _g = lock(true);
    assert_eq!(call(DSPTRISTATE, [0; 5]), EBADRQ as i64);
    assert_eq!(call(GPIO, [1, 0, 0, 0, 0]), EBADRQ as i64);
    assert_eq!(call(GPIO, [3, 0, 0, 0, 0]), EBADRQ as i64);
}

#[test]
fn devconnect_returns_0_for_a_bad_source_else_1() {
    let _g = lock(true);
    assert_eq!(call(DEVCONNECT, [4, 8, 0, 0, 1]), 0);
    assert_eq!(call(DEVCONNECT, [w(-1), 8, 0, 0, 1]), 0);
    assert_eq!(call(DEVCONNECT, [0, 8, 0, 0, 1]), 1);
    assert_eq!(call(DEVCONNECT, [0, 8, 0, 3, 1]), 1);
    with(|s| assert_eq!((s.regs.freq_int, s.regs.freq_ext), (3, 0)));
    assert_eq!(call(DEVCONNECT, [2, 8, 1, 5, 0]), 1);
    with(|s| assert_eq!((s.regs.freq_int, s.regs.freq_ext), (3, 5), "external clock: freq_ext"));
}

#[test]
fn sndstatus_reset_zeroes_the_attenuation_only() {
    let _g = lock(true);
    assert_eq!(sndstatus(0), 0);
    soundcmd(0, 0x50);
    soundcmd(4, 1);
    buffoper(3);
    assert_eq!(sndstatus(1), 0);
    assert_eq!(soundcmd(0, -1), 0, "attenuation 0 (EmuTOS' own write)");
    assert_eq!(soundcmd(4, -1), 1, "ADDERIN stays (oracle O4)");
    assert_eq!(buffoper(-1), 3, "playback goes on (oracle O4)");
}

#[test]
fn buffptr_writes_the_native_layout() {
    let _g = lock(true);
    let mut sbp = [0xAAAA_AAAA_AAAA_AAAAu64; 4];
    let p = sbp.as_mut_ptr() as u64;
    assert_eq!(call(BUFFPTR, [p, 0, 0, 0, 0]), 0);
    assert_eq!(sbp, [0, 0, 0xAAAA_AAAA_AAAA_AAAA, 0xAAAA_AAAA_AAAA_AAAA], "play 0 before the first start");
    with(|s| s.regs.play_ptr = 0x1_0000_1234);
    call(BUFFPTR, [p, 0, 0, 0, 0]);
    assert_eq!(sbp[0], 0x1_0000_1234, "64-bit play");
    assert_eq!(&sbp[2..], &[0xAAAA_AAAA_AAAA_AAAA; 2], "res1/res2 untouched");
}

// ─── Rates and conversion ────────────────────────────────────────────────

#[test]
fn range_values_at_the_measured_clock() {
    // `docs/hardware/pwm-audio.md` §5.5, the 107 143 019 Hz table.
    let mut r = Regs::BOOT;
    for (pre, rng) in [(3u8, 2140u32), (2, 4280), (1, 8560), (0, 17047)] {
        r.mode_control = pre;
        assert_eq!(r.range(PI_CLOCK), rng, "STe prescale {pre}");
    }
    for (p, rng) in [(1u8, 2179u32), (2, 3269), (3, 4358), (4, 5448), (5, 6537), (7, 8716), (9, 10895), (11, 13074)] {
        r.freq_int = p;
        assert_eq!(r.range(PI_CLOCK), rng, "Falcon prescale {p}");
    }
}

#[test]
fn duty_maps_the_signed_range_onto_0_to_rng() {
    let rng = 2140;
    assert_eq!(duty(-32768, rng), 0);
    assert_eq!(duty(0, rng), rng / 2, "mid-level = silence");
    assert!(duty(32767, rng) < rng, "never above RNG");
    assert_eq!(duty(32767, rng), rng - 1);
    let mut last = 0;
    for s in (-32768i32..=32767).step_by(97) {
        let d = duty(s as i16, rng);
        assert!(d >= last);
        last = d;
    }
}

// ─── The converter against a simulated engine ────────────────────────────

struct Mock {
    ring: RefCell<Vec<u32>>,
    pos: Cell<usize>,
    started: Cell<Option<u32>>,
    range: Cell<u32>,
}

impl Mock {
    fn new() -> Self {
        Mock { ring: RefCell::new(vec![0xDEAD; RING_WORDS]), pos: Cell::new(0), started: Cell::new(None), range: Cell::new(0) }
    }
    /// The engine reads `slots` more slots; one tick follows.
    fn advance(&self, slots: usize) {
        self.pos.set((self.pos.get() + 2 * slots) % RING_WORDS);
        tick(self);
    }
    fn word(&self, slot: usize) -> (u32, u32) {
        let r = self.ring.borrow();
        let i = 2 * (slot % SLOTS);
        (r[i], r[i + 1])
    }
}

impl SoundHw for Mock {
    fn clock_hz(&self) -> u32 {
        PI_CLOCK
    }
    fn write(&self, at: usize, words: &[u32]) {
        assert!(at + words.len() <= RING_WORDS, "write past the ring");
        self.ring.borrow_mut()[at..at + words.len()].copy_from_slice(words);
    }
    fn start(&self, range: u32) {
        assert!(self.started.get().is_none(), "started twice");
        self.started.set(Some(range));
        self.range.set(range);
    }
    fn set_range(&self, range: u32) {
        self.range.set(range);
    }
    fn read_index(&self) -> Option<usize> {
        Some(self.pos.get())
    }
}

fn play_ptr() -> u64 {
    with(|s| s.regs.play_ptr)
}

const RNG50K: u32 = 2140;

#[test]
fn single_shot_8bit_stereo_plays_and_ends() {
    let _g = lock(true);
    let hw = Mock::new();
    // 100 stereo frames: left = i, right = -i.
    let buf: Vec<u8> = (0..100i8).flat_map(|i| [i as u8, (-i) as u8]).collect();
    let (a, e) = (buf.as_ptr() as u64, buf.as_ptr() as u64 + buf.len() as u64);
    assert_eq!(setbuffer(0, a, e), 0);
    assert_eq!(buffoper(1), 0);
    assert_eq!(play_ptr(), a, "start address once started");
    tick(&hw);
    assert_eq!(hw.started.get(), Some(RNG50K), "50 kHz at boot (prescale 3)");
    for i in 0..100usize {
        let s = i as i8 as i16;
        assert_eq!(hw.word(i), (duty(s << 8, RNG50K), duty((-s) << 8, RNG50K)), "slot {i}");
    }
    assert_eq!(hw.word(100), (RNG50K / 2, RNG50K / 2), "silence after the frame");
    // S3: bit 0 stays while the engine is inside the frame.
    hw.advance(50);
    assert_eq!(buffoper(-1), 1);
    assert_eq!(play_ptr(), a + 100, "after slot 49");
    hw.advance(50);
    assert_eq!(buffoper(-1), 1, "last sample read, the end slot not yet");
    assert_eq!(play_ptr(), e, "pointer at end (falcontc)");
    hw.advance(1);
    assert_eq!(buffoper(-1), 0, "S3: bit 0 dropped at the frame end");
    hw.advance(200);
    assert_eq!(play_ptr(), e, "stays at end");
}

#[test]
fn loop_wraps_and_setbuffer_takes_effect_at_the_next_pass() {
    let _g = lock(true);
    let hw = Mock::new();
    let a_buf = vec![0x10u8; 64]; // 32 frames
    let b_buf = vec![0x20u8; 64];
    let a = a_buf.as_ptr() as u64;
    setbuffer(0, a, a + 64);
    buffoper(3);
    tick(&hw);
    let va = duty(0x10 << 8, RNG50K);
    let vb = duty(0x20 << 8, RNG50K);
    assert_eq!(hw.word(31).0, va);
    assert_eq!(hw.word(32).0, va, "second pass of A, no gap");
    hw.advance(40);
    assert_eq!(buffoper(-1), 3);
    assert!(play_ptr() >= a && play_ptr() <= a + 64, "in [start, end]");
    assert_eq!(play_ptr(), a + 16, "8 slots into the second pass");
    // S5: B set in the middle of pass 2 (slots 32..64) — pass 3 plays B.
    setbuffer(0, b_buf.as_ptr() as u64, b_buf.as_ptr() as u64 + 64);
    hw.advance(1);
    assert_eq!(hw.word(63).0, va, "pass 2 of A plays to its end");
    assert_eq!(hw.word(64).0, vb, "the next pass is B");
    assert_eq!(hw.word(95).0, vb);
}

#[test]
fn stop_freezes_the_pointer_and_writes_silence() {
    let _g = lock(true);
    let hw = Mock::new();
    let buf = vec![0x40u8; 2000];
    let a = buf.as_ptr() as u64;
    setbuffer(0, a, a + 2000);
    buffoper(1);
    tick(&hw);
    hw.advance(100);
    assert_eq!(play_ptr(), a + 200);
    buffoper(0);
    assert_eq!(buffoper(-1), 0);
    hw.advance(10);
    assert_eq!(play_ptr(), a + 200, "frozen where it stopped");
    let mid = RNG50K / 2;
    for slot in 110 + GUARD..110 + LEAD {
        assert_eq!(hw.word(slot), (mid, mid), "slot {slot} rewritten as silence");
    }
}

#[test]
fn restart_after_the_end_begins_a_new_pass() {
    let _g = lock(true);
    let hw = Mock::new();
    let buf = vec![0x7Fu8; 20];
    let a = buf.as_ptr() as u64;
    setbuffer(0, a, a + 20);
    buffoper(1);
    tick(&hw);
    hw.advance(30);
    assert_eq!(buffoper(-1), 0);
    buffoper(1);
    assert_eq!(play_ptr(), a);
    hw.advance(5);
    let v = duty(0x7F << 8, RNG50K);
    assert_eq!(hw.word(35 + GUARD).0, v, "the new pass starts just ahead of the engine");
    assert_eq!(hw.word(35 + GUARD + 9).0, v);
    assert_eq!(hw.word(35 + GUARD + 10).0, RNG50K / 2);
    hw.advance(GUARD + 11);
    assert_eq!(buffoper(-1), 0, "ended again");
    assert_eq!(play_ptr(), a + 20);
}

#[test]
fn sixteen_bit_byte_order_is_latched_at_setbuffer() {
    let _g = lock(true);
    let hw = Mock::new();
    // One stereo frame: L = 0x1234 BE / 0x3412 LE, R = 0x0100 BE / 0x0001 LE.
    let buf = [0x12u8, 0x34, 0x01, 0x00];
    let a = buf.as_ptr() as u64;
    setmode(1);
    soundcmd(SET16BITFORMAT, SND_FORMATLITTLEENDIAN as i16);
    setbuffer(0, a, a + 4);
    soundcmd(SET16BITFORMAT, SND_FORMATBIGENDIAN as i16);
    buffoper(1);
    tick(&hw);
    assert_eq!(hw.word(0), (duty(0x3412, RNG50K), duty(0x0001, RNG50K)), "little-endian, latched");
    hw.advance(5);
    setbuffer(0, a, a + 4);
    buffoper(1);
    hw.advance(1);
    assert_eq!(hw.word(6 + GUARD), (duty(0x1234, RNG50K), duty(0x0100, RNG50K)), "big-endian");
}

#[test]
fn mono_8bit_feeds_both_channels() {
    let _g = lock(true);
    let hw = Mock::new();
    let buf = [0x80u8, 0x7F, 0x00];
    let a = buf.as_ptr() as u64;
    setmode(2);
    setbuffer(0, a, a + 3);
    buffoper(1);
    tick(&hw);
    assert_eq!(hw.word(0), (0, 0), "-128 = duty 0");
    assert_eq!(hw.word(1), (duty(0x7F00, RNG50K), duty(0x7F00, RNG50K)));
    assert_eq!(hw.word(2), (RNG50K / 2, RNG50K / 2));
}

#[test]
fn a_rate_change_reaches_the_output() {
    let _g = lock(true);
    let hw = Mock::new();
    let buf = vec![0u8; 4000];
    setbuffer(0, buf.as_ptr() as u64, buf.as_ptr() as u64 + 4000);
    buffoper(1);
    tick(&hw);
    assert_eq!(call(DEVCONNECT, [0, 8, 0, 1, 1]), 1);
    hw.advance(10);
    assert_eq!(hw.range.get(), 2179, "Falcon prescale 1 = 49 170 Hz");
    assert_eq!(hw.word(10 + GUARD).0, 2179 / 2, "rewritten at the new range");
    soundcmd(6, 0);
    call(DEVCONNECT, [0, 8, 0, 0, 1]);
    hw.advance(10);
    assert_eq!(hw.range.get(), 17047, "back to STe mode, prescale 0 = 6 285 Hz");
}

#[test]
fn an_empty_frame_ends_at_once() {
    let _g = lock(true);
    let hw = Mock::new();
    setbuffer(0, 0x1000, 0x1000);
    buffoper(1);
    tick(&hw);
    hw.advance(3);
    assert_eq!(buffoper(-1), 0);
}

#[test]
fn an_idle_ring_needs_no_writes() {
    let _g = lock(true);
    let hw = Mock::new();
    let buf = [1u8; 2];
    setbuffer(0, buf.as_ptr() as u64, buf.as_ptr() as u64 + 2);
    buffoper(1);
    tick(&hw);
    for _ in 0..3 {
        hw.advance(SLOTS / 2);
    }
    // Poison the ring: an idle tick must not write it.
    hw.ring.borrow_mut().iter_mut().for_each(|w| *w = 7);
    hw.advance(100);
    assert!(hw.ring.borrow().iter().all(|&w| w == 7));
}
