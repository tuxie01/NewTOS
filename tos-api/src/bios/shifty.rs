//! The keyboard shift byte — EmuTOS `shifty` (TASK/BIOS10).
//!
//! Spec: `docs/api/bios/0b-kbshift.md` (decision D1 = 1, user 2026-10-28: one
//! real byte in the keyboard decoder, as EmuTOS keeps it).
//!
//! EmuTOS keeps the modifier state in one `UBYTE shifty`
//! (`references/emutos/bios/ikbd.c:145-153`, `bios/ikbd.h:30`). `Kbshift(flag)`
//! with `flag >= 0` stores the low byte of `flag` into it — all eight bits,
//! whatever they mean — and the keyboard interrupt (`kbd_int`) changes only
//! the bit of the key it handles:
//!
//! * a modifier make sets its bit, a modifier break clears it (Shift, Ctrl,
//!   Alt; `bios/ikbd.c:777-798`, `:831-843`); CapsLock **toggles** on its make
//!   and does nothing on its break (`:844-853`; the Amiga build's set/clear,
//!   `:799-806`, is not the reference machine's);
//! * a Clr/Home or Insert make while Alt is down sets `MODE_HOME` /
//!   `MODE_INSERT` (`:876-883`), the key's break clears the bit whether Alt is
//!   still down or not (`:807-814`);
//! * every other key leaves the byte alone; bit 7 has no `MODE_*` constant and
//!   no code that sets or clears it, so a value a program stored stays.
//!
//! [`key_event`] is that bookkeeping for one IKBD key byte, [`Shifty`] the byte
//! itself. The kernel's IKBD decoder (`kernel::ikbd`) owns the one instance;
//! BIOS `Kbshift` reaches it through `Host::keyboard_shift_state` /
//! `Host::set_keyboard_shift_state`.

use core::sync::atomic::{AtomicU8, Ordering};

pub use super::reset::{MODE_ALT, MODE_CTRL, MODE_LSHIFT, MODE_RSHIFT};

/// `MODE_CAPS` (`include/biosdefs.h:95`): CapsLock engaged.
pub const MODE_CAPS: u8 = 0x10;
/// `MODE_HOME` (`include/biosdefs.h:99`): Clr/Home down, pressed while Alt was.
pub const MODE_HOME: u8 = 0x20;
/// `MODE_INSERT` (`include/biosdefs.h:100`): Insert down, pressed while Alt was.
pub const MODE_INSERT: u8 = 0x40;

/// `KEY_RELEASED` (`bios/ikbd.c:48`): the break bit of an IKBD key byte.
pub const KEY_RELEASED: u8 = 0x80;
/// `KEY_LSHIFT` (`bios/ikbd.c:50`).
pub const KEY_LSHIFT: u8 = 0x2A;
/// `KEY_RSHIFT` (`bios/ikbd.c:51`).
pub const KEY_RSHIFT: u8 = 0x36;
/// `KEY_CTRL` (`bios/ikbd.c:52`).
pub const KEY_CTRL: u8 = 0x1D;
/// `KEY_ALT` (`bios/ikbd.c:53`).
pub const KEY_ALT: u8 = 0x38;
/// `KEY_CAPS` (`bios/ikbd.c:54`).
pub const KEY_CAPS: u8 = 0x3A;
/// `KEY_INSERT` (`bios/ikbd.c:87`).
pub const KEY_INSERT: u8 = 0x52;
/// `KEY_HOME` (`bios/ikbd.c:88`): Clr/Home.
pub const KEY_HOME: u8 = 0x47;

/// `true` for the five modifier keys (make code, release bit clear): the keys
/// `kbd_int`'s make switch handles as `modifier = TRUE` and returns after,
/// without a key for the queue (`bios/ikbd.c:830-870`).
pub const fn is_modifier(code: u8) -> bool {
    matches!(code, KEY_LSHIFT | KEY_RSHIFT | KEY_CTRL | KEY_ALT | KEY_CAPS)
}

/// The shift byte after the IKBD key byte `scancode` (bit 7 = break), given
/// the byte before it — `kbd_int`'s changes to `shifty` (`bios/ikbd.c:776-883`):
///
/// ```c
/// if (scancode & KEY_RELEASED) {
///     switch (scancode_only) {
///     case KEY_RSHIFT: shifty &= ~MODE_RSHIFT; ...      /* :779-789 */
///     case KEY_HOME:   shifty &= ~MODE_HOME; ...        /* :807-810 */
///     case KEY_INSERT: shifty &= ~MODE_INSERT; ...      /* :811-814 */
///     }
///     return;
/// }
/// switch (scancode) {
/// case KEY_RSHIFT: shifty |= MODE_RSHIFT; ...           /* :832-842 */
/// case KEY_CAPS:   shifty ^= MODE_CAPS; ...             /* :844-853 */
/// }
/// ...
/// if (shifty & MODE_ALT) {                              /* :876-883 */
///     switch (scancode) {
///     case KEY_HOME:   shifty |= MODE_HOME;   break;
///     case KEY_INSERT: shifty |= MODE_INSERT; break;
///     }
/// }
/// ```
///
/// Only the bit of the key changes; the rest of the byte — a value a program
/// stored with `Kbshift`, bit 7 included — stays.
pub const fn key_event(shifty: u8, scancode: u8) -> u8 {
    let code = scancode & !KEY_RELEASED;
    if scancode & KEY_RELEASED != 0 {
        return match code {
            KEY_RSHIFT => shifty & !MODE_RSHIFT,
            KEY_LSHIFT => shifty & !MODE_LSHIFT,
            KEY_CTRL => shifty & !MODE_CTRL,
            KEY_ALT => shifty & !MODE_ALT,
            KEY_HOME => shifty & !MODE_HOME,
            KEY_INSERT => shifty & !MODE_INSERT,
            // CapsLock's break and every other key: no change.
            _ => shifty,
        };
    }
    match code {
        KEY_RSHIFT => shifty | MODE_RSHIFT,
        KEY_LSHIFT => shifty | MODE_LSHIFT,
        KEY_CTRL => shifty | MODE_CTRL,
        KEY_ALT => shifty | MODE_ALT,
        KEY_CAPS => shifty ^ MODE_CAPS,
        KEY_HOME if shifty & MODE_ALT != 0 => shifty | MODE_HOME,
        KEY_INSERT if shifty & MODE_ALT != 0 => shifty | MODE_INSERT,
        _ => shifty,
    }
}

/// The shift byte — EmuTOS `shifty`. One byte: [`Shifty::set`] stores all
/// eight bits, [`Shifty::get`] returns them, [`Shifty::key`] applies one key
/// byte ([`key_event`]) as a single atomic update, so a `Kbshift` store and a
/// key event never mix bits of each other.
pub struct Shifty(AtomicU8);

impl Shifty {
    /// The boot value: no modifier (EmuTOS' `shifty` is BSS, `bios/ikbd.c`).
    pub const fn new() -> Self {
        Self(AtomicU8::new(0))
    }

    /// The current byte — `Kbshift(-1)` (`bios/ikbd.c:147`).
    pub fn get(&self) -> u8 {
        self.0.load(Ordering::Relaxed)
    }

    /// Store `value`, all eight bits — `shifty = flag` for `flag >= 0`
    /// (`bios/ikbd.c:149-150`; the caller has truncated the WORD to its low
    /// byte, as the `UBYTE` assignment does).
    pub fn set(&self, value: u8) {
        self.0.store(value, Ordering::Relaxed);
    }

    /// Apply the IKBD key byte `scancode` ([`key_event`]); returns the new
    /// byte.
    pub fn key(&self, scancode: u8) -> u8 {
        let mut cur = self.0.load(Ordering::Relaxed);
        loop {
            let new = key_event(cur, scancode);
            match self
                .0
                .compare_exchange_weak(cur, new, Ordering::Relaxed, Ordering::Relaxed)
            {
                Ok(_) => return new,
                Err(seen) => cur = seen,
            }
        }
    }
}

impl Default for Shifty {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn brk(code: u8) -> u8 {
        code | KEY_RELEASED
    }

    /// Feed `bytes` in order from `start`; the byte after each.
    fn run(start: u8, bytes: &[u8]) -> Vec<u8> {
        let s = Shifty::new();
        s.set(start);
        bytes.iter().map(|&b| s.key(b)).collect()
    }

    #[test]
    fn set_keeps_all_eight_bits_and_get_returns_them() {
        // Audit probe K04/K05, K08/K09 on EmuTOS: 0x1FF reads back 0xFF,
        // 0x7F81 reads back 0x81 (the caller truncates the WORD to a byte).
        let s = Shifty::new();
        assert_eq!(s.get(), 0, "boot value");
        for v in [0x10u8, 0xFF, 0x81, 0x80, 0x20, 0x40, 0x00] {
            s.set(v);
            assert_eq!(s.get(), v);
        }
        s.set(0x1FFu16 as u8);
        assert_eq!(s.get(), 0xFF);
        s.set(0x7F81u16 as u8);
        assert_eq!(s.get(), 0x81);
    }

    #[test]
    fn modifiers_set_and_clear_their_own_bit() {
        assert_eq!(
            run(0, &[KEY_RSHIFT, KEY_LSHIFT, KEY_CTRL, KEY_ALT]),
            [0x01, 0x03, 0x07, 0x0F]
        );
        assert_eq!(
            run(0x0F, &[brk(KEY_LSHIFT), brk(KEY_ALT), brk(KEY_RSHIFT), brk(KEY_CTRL)]),
            [0x0D, 0x05, 0x04, 0x00]
        );
    }

    #[test]
    fn capslock_toggles_on_the_make_only() {
        assert_eq!(
            run(0, &[KEY_CAPS, brk(KEY_CAPS), KEY_CAPS, brk(KEY_CAPS)]),
            [0x10, 0x10, 0x00, 0x00]
        );
    }

    #[test]
    fn home_and_insert_set_their_bit_only_with_alt_down() {
        // ksprobe A/B/C on EmuTOS (measurement-2026-10-29-bios10):
        // 08 28 08 00, 08 48 08 00, Home without Alt: no change.
        assert_eq!(run(0, &[KEY_ALT, KEY_HOME, brk(KEY_HOME), brk(KEY_ALT)]), [0x08, 0x28, 0x08, 0x00]);
        assert_eq!(
            run(0, &[KEY_ALT, KEY_INSERT, brk(KEY_INSERT), brk(KEY_ALT)]),
            [0x08, 0x48, 0x08, 0x00]
        );
        assert_eq!(run(0, &[KEY_HOME, brk(KEY_HOME), KEY_INSERT, brk(KEY_INSERT)]), [0, 0, 0, 0]);
    }

    #[test]
    fn home_stays_set_after_alt_is_released_until_its_own_break() {
        // ksprobe D/E: 08 28 20 00 and 08 48 68 48 08 00 (biosdefs.h:96-98).
        assert_eq!(run(0, &[KEY_ALT, KEY_HOME, brk(KEY_ALT), brk(KEY_HOME)]), [0x08, 0x28, 0x20, 0x00]);
        assert_eq!(
            run(
                0,
                &[KEY_ALT, KEY_INSERT, KEY_HOME, brk(KEY_HOME), brk(KEY_INSERT), brk(KEY_ALT)]
            ),
            [0x08, 0x48, 0x68, 0x48, 0x08, 0x00]
        );
    }

    #[test]
    fn a_stored_value_survives_key_events_that_do_not_touch_its_bits() {
        // ksprobe F: after Kbshift(0x80): 82 80 88 a8 88 80.
        let bytes = [KEY_LSHIFT, 0x1E, brk(0x1E), brk(KEY_LSHIFT), KEY_ALT, KEY_HOME, brk(KEY_HOME), brk(KEY_ALT)];
        assert_eq!(run(0x80, &bytes), [0x82, 0x82, 0x82, 0x80, 0x88, 0xA8, 0x88, 0x80]);
        // A program-set MODE_HOME is cleared by Home's break even without Alt.
        assert_eq!(run(0x20, &[KEY_HOME, brk(KEY_HOME)]), [0x20, 0x00]);
    }

    #[test]
    fn plain_keys_and_capslock_break_leave_the_byte_alone() {
        for code in [0x1Eu8, 0x30, 0x53, 0x62, 0x48, 0x4B] {
            assert_eq!(key_event(0xA5, code), 0xA5);
            assert_eq!(key_event(0xA5, brk(code)), 0xA5);
        }
        assert_eq!(key_event(0xFF, brk(KEY_CAPS)), 0xFF);
    }

    #[test]
    fn the_modifier_set_is_the_five_keys() {
        for code in 0u8..0x80 {
            let expect = matches!(code, 0x2A | 0x36 | 0x1D | 0x38 | 0x3A);
            assert_eq!(is_modifier(code), expect, "scancode {code:#04x}");
        }
    }
}
