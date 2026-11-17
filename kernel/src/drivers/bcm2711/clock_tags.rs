//! Pure encoding/decoding of the firmware's clock property tags (FS08
//! part 3): "Get max clock rate", "Get clock rate measured" and "Set clock
//! rate"; "Set clock state" for the PWM clock of the sound backend
//! (TASK/SND05, `docs/hardware/pwm-audio.md` §12.2).
//!
//! Word arithmetic only, no register access and no kernel dependency, so the
//! host test suite (`tools/xtask`, which includes this file by path) can run
//! its tests; `mailbox.rs` does the transaction.
//!
//! Sources (Track B, `docs/hardware/mailbox-framebuffer.md` §3f):
//! * the raspberrypi/firmware wiki page *Mailbox property interface*
//!   (source #4; local copy `references/rpi4-boot/
//!   firmware-wiki-mailbox-property-interface.md`, wiki HEAD `c9e615a`),
//!   sections "Get clock rate measured" (`0x00030047`), "Set clock rate"
//!   (`0x00038002`: request 12 bytes `clock id, rate, skip setting turbo`,
//!   response 8 bytes `clock id, rate`; *"A rate of 0 is returned if the
//!   clock does not exist"*), "Get max clock rate" (`0x00030004`);
//! * cross-checked against the pinned `references/circle`:
//!   `include/circle/bcmpropertytags.h:40,47,50` (tag ids), `:234-241`
//!   (`TPropertyTagSetClockRate`, `SKIP_SETTING_TURBO = 1`),
//!   `lib/cputhrottle.cpp:420-428` (the request: 12 parameter bytes).

/// `PROPTAG_GET_MAX_CLOCK_RATE` — answers `(clock id, rate in Hz)`: *"the
/// maximum supported clock rate for the given clock. Clocks should not be set
/// higher than this."*
pub const PROPTAG_GET_MAX_CLOCK_RATE: u32 = 0x0003_0004;
/// `PROPTAG_GET_CLOCK_RATE_MEASURED` — answers `(clock id, rate in Hz)`: the
/// true rate, *"which respects clamping, throttling and clock divider
/// limitations"*, where "Get clock rate" returns the last requested value.
pub const PROPTAG_GET_CLOCK_RATE_MEASURED: u32 = 0x0003_0047;
/// `PROPTAG_SET_CLOCK_RATE` — request `(clock id, rate in Hz, skip setting
/// turbo)`, answers `(clock id, rate in Hz)`.
pub const PROPTAG_SET_CLOCK_RATE: u32 = 0x0003_8002;

/// `PROPTAG_SET_CLOCK_STATE` — request `(clock id, state)`, answers
/// `(clock id, state)`; state bit 0 = on, bit 1 = "clock does not exist"
/// (wiki section "Set clock state", tag `0x00038001`, request and response
/// length 8).
pub const PROPTAG_SET_CLOCK_STATE: u32 = 0x0003_8001;
/// "Set clock state" bit 0: on.
pub const CLOCK_STATE_ON: u32 = 1 << 0;
/// "Set clock state" bit 1 in a response: the clock does not exist.
pub const CLOCK_STATE_MISSING: u32 = 1 << 1;
/// Clock id `PWM` (wiki "Clocks" list: `0x00000000a: PWM`).
pub const CLOCK_ID_PWM: u32 = 0x0000_000A;

// Word indices into a "Set clock state" message.
/// Tag code word of a "Set clock state" message.
pub const SS_CODE: usize = 4;
/// Value word 1 of a "Set clock state" message: the state.
pub const SS_STATE: usize = 6;
const SS_ENDTAG: usize = 7;
/// Message length in words (a multiple of four: 16-byte units).
pub const SET_CLOCK_STATE_WORDS: usize = (SS_ENDTAG + 1 + 3) & !3;

/// The complete property message for "Set clock state" of `clock_id`, on or
/// off. The value buffer is 8 bytes, the request carries 8.
pub fn encode_set_clock_state(clock_id: u32, on: bool) -> [u32; SET_CLOCK_STATE_WORDS] {
    let mut b = [0u32; SET_CLOCK_STATE_WORDS];
    b[0] = (SET_CLOCK_STATE_WORDS * 4) as u32;
    b[1] = 0; // request code
    b[SC_TAG] = PROPTAG_SET_CLOCK_STATE;
    b[SC_LEN] = 8; // value buffer size, bytes
    b[SS_CODE] = 8; // request, 8 parameter bytes
    b[SC_ID] = clock_id;
    b[SS_STATE] = if on { CLOCK_STATE_ON } else { 0 };
    b[SS_ENDTAG] = 0;
    b
}

/// Decode a "Set clock state" reply: the state word, or `None` unless the
/// tag was answered with its 8 value bytes and the clock exists (bit 1
/// clear).
pub fn state_reply(code: u32, state: u32) -> Option<u32> {
    if code & TAG_RESPONSE == 0 || (code & !TAG_RESPONSE) < 8 || state & CLOCK_STATE_MISSING != 0 {
        None
    } else {
        Some(state)
    }
}

/// Bit 31 of a tag's code word: set in a response, clear in a request
/// (`docs/hardware/mailbox-framebuffer.md` §2).
pub const TAG_RESPONSE: u32 = 0x8000_0000;

/// Request parameter bytes of "Set clock rate": three `u32` words.
pub const SET_CLOCK_RATE_REQ_LEN: u32 = 12;

// Word indices into a "Set clock rate" message.
/// Tag code word: request/response code (+ value length in a response).
pub const SC_CODE: usize = 4;
/// Value word 1: the rate in Hz (request) / the rate set (response).
pub const SC_RATE: usize = 6;
const SC_TAG: usize = 2;
const SC_LEN: usize = 3;
const SC_ID: usize = 5;
const SC_SKIP: usize = 7;
const SC_ENDTAG: usize = 8;
/// Message length in words: the end tag plus padding, rounded up so the
/// byte length is a multiple of 16 (the property buffer's alignment unit).
pub const SET_CLOCK_RATE_WORDS: usize = (SC_ENDTAG + 1 + 3) & !3;

/// The complete property message for "Set clock rate" of `clock_id` to
/// `rate_hz`.
///
/// `skip_setting_turbo = false` sends `0`, which per the wiki is the default
/// behaviour: *"when setting arm freq above default, other turbo settings will
/// be enabled (e.g. voltage, sdram and gpu frequencies)"*; `true` sends `1`
/// (circle's `SKIP_SETTING_TURBO`) and leaves those settings alone. The value
/// buffer is 12 bytes — the larger of the 12-byte request and the 8-byte
/// response.
pub fn encode_set_clock_rate(
    clock_id: u32,
    rate_hz: u32,
    skip_setting_turbo: bool,
) -> [u32; SET_CLOCK_RATE_WORDS] {
    let mut b = [0u32; SET_CLOCK_RATE_WORDS];
    b[0] = (SET_CLOCK_RATE_WORDS * 4) as u32; // total size in bytes
    b[1] = 0; // request code
    b[SC_TAG] = PROPTAG_SET_CLOCK_RATE;
    b[SC_LEN] = SET_CLOCK_RATE_REQ_LEN; // value buffer size, bytes
    // Request (b31 clear) carrying the parameter length, as circle's
    // `GetTag` writes it (`lib/bcmpropertytags.cpp:49-56`) and
    // `mailbox::query_id_value` does for its 4-byte requests.
    b[SC_CODE] = SET_CLOCK_RATE_REQ_LEN;
    b[SC_ID] = clock_id;
    b[SC_RATE] = rate_hz;
    b[SC_SKIP] = u32::from(skip_setting_turbo);
    b[SC_ENDTAG] = 0; // end tag
    b
}

/// Decode a `(clock id, rate)` reply from its tag code word and rate word.
///
/// `None` unless the tag was answered (`b31` set), the response carries the
/// 8 value bytes the reply format requires, and the rate is non-zero — the
/// wiki's *"A rate of 0 is returned if the clock does not exist"* (same rule
/// as `mailbox::get_clock_rate`).
pub fn rate_reply(code: u32, rate_hz: u32) -> Option<u32> {
    if code & TAG_RESPONSE == 0 || (code & !TAG_RESPONSE) < 8 || rate_hz == 0 {
        None
    } else {
        Some(rate_hz)
    }
}

/// A rate in Hz as whole MHz, rounded to the nearest.
pub fn mhz(rate_hz: u32) -> u32 {
    rate_hz.saturating_add(500_000) / 1_000_000
}

/// A temperature in thousandths of a degree Celsius as `(whole degrees,
/// tenths)`, the tenth truncated.
pub fn deci_celsius(mdeg: u32) -> (u32, u32) {
    (mdeg / 1000, (mdeg % 1000) / 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_clock_rate_message_layout() {
        let b = encode_set_clock_rate(3, 1_500_000_000, false);
        assert_eq!(SET_CLOCK_RATE_WORDS, 12);
        assert_eq!(b[0], 48); // total size, a multiple of 16
        assert_eq!(b[1], 0); // request
        assert_eq!(b[2], 0x0003_8002);
        assert_eq!(b[3], 12); // value buffer
        assert_eq!(b[4], 12); // request, 12 parameter bytes
        assert_eq!(b[5], 3); // CLOCK_ID_ARM
        assert_eq!(b[6], 1_500_000_000);
        assert_eq!(b[7], 0); // turbo settings follow (wiki default)
        assert_eq!(b[8], 0); // end tag
        assert_eq!(&b[9..], &[0, 0, 0]); // padding
        assert_eq!(b[SC_CODE], b[4]);
        assert_eq!(b[SC_RATE], b[6]);
    }

    #[test]
    fn skip_setting_turbo_is_one() {
        let b = encode_set_clock_rate(3, 600_000_000, true);
        assert_eq!(b[7], 1); // circle SKIP_SETTING_TURBO
        assert_eq!(b[6], 600_000_000);
    }

    #[test]
    fn rate_reply_needs_response_bit_length_and_rate() {
        assert_eq!(
            rate_reply(TAG_RESPONSE | 8, 1_500_000_000),
            Some(1_500_000_000)
        );
        assert_eq!(
            rate_reply(TAG_RESPONSE | 12, 700_000_000),
            Some(700_000_000)
        );
        // Unanswered tag: b31 clear (a mailbox that does not implement it).
        assert_eq!(rate_reply(12, 1_500_000_000), None);
        // Too short for (clock id, rate).
        assert_eq!(rate_reply(TAG_RESPONSE | 4, 1_500_000_000), None);
        // Rate 0: the clock does not exist.
        assert_eq!(rate_reply(TAG_RESPONSE | 8, 0), None);
    }

    #[test]
    fn set_clock_state_message_layout() {
        let b = encode_set_clock_state(CLOCK_ID_PWM, true);
        assert_eq!(SET_CLOCK_STATE_WORDS, 8);
        assert_eq!(b, [32, 0, 0x0003_8001, 8, 8, 0x0a, 1, 0]);
        assert_eq!(encode_set_clock_state(CLOCK_ID_PWM, false)[SS_STATE], 0);
    }

    #[test]
    fn state_reply_needs_response_bit_length_and_an_existing_clock() {
        // The real Pi's answer to "on" (H1): state 1.
        assert_eq!(state_reply(TAG_RESPONSE | 8, 1), Some(1));
        assert_eq!(state_reply(TAG_RESPONSE | 8, 0), Some(0));
        assert_eq!(state_reply(8, 1), None, "unanswered");
        assert_eq!(state_reply(TAG_RESPONSE | 4, 1), None, "too short");
        assert_eq!(state_reply(TAG_RESPONSE | 8, CLOCK_STATE_MISSING), None, "no such clock");
    }

    #[test]
    fn mhz_rounds_to_nearest() {
        assert_eq!(mhz(600_000_000), 600);
        assert_eq!(mhz(1_500_000_000), 1500);
        assert_eq!(mhz(1_499_999_999), 1500);
        assert_eq!(mhz(1_500_499_999), 1500);
        assert_eq!(mhz(1_500_500_000), 1501);
        assert_eq!(mhz(u32::MAX), u32::MAX / 1_000_000);
    }

    #[test]
    fn deci_celsius_splits_tenths() {
        assert_eq!(deci_celsius(45_277), (45, 2));
        assert_eq!(deci_celsius(25_000), (25, 0));
        assert_eq!(deci_celsius(999), (0, 9));
    }
}
