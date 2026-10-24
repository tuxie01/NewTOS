//! HID boot keyboard / mouse reports → IKBD bytes, as pure functions over
//! explicit state (USB01 F3, `docs/api/usb/hid-mouse-protocols.md` §9.6).
//!
//! The decoder state lives **per function** (one HID interface the driver
//! polls, §9.2), not once for the whole system: with one global "previous
//! keyboard report" two keyboards diffed against each other's reports, so a
//! key held on one flickered break/make whenever the other reported (§9.6
//! Befund A, H1). `hid.rs` keeps the per-function state and hands every byte
//! to `ikbd::feed`; this file only computes the bytes, so the host test suite
//! (`tools/xtask`, which includes it by path) can drive interleaved report
//! sequences of several functions (§9.8 H-2).
//!
//! Sources: the scancode table is FreeMiNT's `translate_key`
//! (`sys/usb/src.km/udd/hid/keyboard/usb_keyboard.c`), the boot report
//! layouts USB HID 1.11 Appendix B (`references/usb/hid1_11.pdf`), the IKBD
//! side `docs/api/usb/hid-keyboard.md` / `hid-mouse.md`.

/// USB HID usage → Atari scancode (make code; break = `| 0x80`).
/// `translate_key` in FreeMiNT `udd/hid/keyboard/usb_keyboard.c`.
pub fn translate_key(usb: u8) -> u8 {
    match usb {
        0x04 => 0x1e, 0x05 => 0x30, 0x06 => 0x2e, 0x07 => 0x20, 0x08 => 0x12,
        0x09 => 0x21, 0x0a => 0x22, 0x0b => 0x23, 0x0c => 0x17, 0x0d => 0x24,
        0x0e => 0x25, 0x0f => 0x26, 0x10 => 0x32, 0x11 => 0x31, 0x12 => 0x18,
        0x13 => 0x19, 0x14 => 0x10, 0x15 => 0x13, 0x16 => 0x1f, 0x17 => 0x14,
        0x18 => 0x16, 0x19 => 0x2f, 0x1a => 0x11, 0x1b => 0x2d, 0x1c => 0x15,
        0x1d => 0x2c, // Z
        0x1e => 0x02, 0x1f => 0x03, 0x20 => 0x04, 0x21 => 0x05, 0x22 => 0x06,
        0x23 => 0x07, 0x24 => 0x08, 0x25 => 0x09, 0x26 => 0x0a, 0x27 => 0x0b, // 0
        0x28 => 0x1c, // Return
        0x29 => 0x01, // Esc
        0x2a => 0x0e, // Backspace
        0x2b => 0x0f, // Tab
        0x2c => 0x39, // Space
        0x2d => 0x0c, 0x2e => 0x0d, 0x2f => 0x1a, 0x30 => 0x1b, 0x31 => 0x2b,
        0x32 => 0x2b, 0x33 => 0x27, 0x34 => 0x28, 0x35 => 0x29, 0x36 => 0x33,
        0x37 => 0x34, 0x38 => 0x35, 0x39 => 0x3a, // Caps Lock
        0x3a => 0x3b, 0x3b => 0x3c, 0x3c => 0x3d, 0x3d => 0x3e, 0x3e => 0x3f,
        0x3f => 0x40, 0x40 => 0x41, 0x41 => 0x42, 0x42 => 0x43, 0x43 => 0x44, // F1..F10
        0x44 => 0x62, 0x45 => 0x61, // F11→Help, F12→Undo
        0x47 => 0x63, 0x48 => 0x64, 0x49 => 0x52, 0x4a => 0x47, 0x4c => 0x53,
        0x4f => 0x4d, 0x50 => 0x4b, 0x51 => 0x50, 0x52 => 0x48, // arrows
        0x54 => 0x65, 0x55 => 0x66, 0x56 => 0x4a, 0x57 => 0x4e, 0x58 => 0x72,
        0x59 => 0x6d, 0x5a => 0x6e, 0x5b => 0x6f, 0x5c => 0x6a, 0x5d => 0x6b,
        0x5e => 0x6c, 0x5f => 0x67, 0x60 => 0x68, 0x61 => 0x69, 0x62 => 0x70,
        0x63 => 0x71, 0x64 => 0x60,
        _ => 0,
    }
}

/// Emit make/break scancodes for the changed modifier bits (`off` = 0 make,
/// 0x80 break). USB byte-0 bits: 0 LCtrl 1 LShift 2 LAlt … 4 RCtrl 5 RShift
/// 6 RAlt. Atari: Ctrl 0x1d, LShift 0x2a, RShift 0x36, Alt 0x38.
fn modifiers(bits: u8, off: u8, emit: &mut impl FnMut(u8)) {
    if bits & 0x11 != 0 {
        emit(0x1d | off);
    }
    if bits & 0x02 != 0 {
        emit(0x2a | off);
    }
    if bits & 0x20 != 0 {
        emit(0x36 | off);
    }
    if bits & 0x44 != 0 {
        emit(0x38 | off);
    }
}

/// Usage 0x01 Keyboard ErrorRollOver: the device's "phantom state", sent in
/// the key array instead of the real key list (HID 1.11 Appendix C p. 62).
const ERROR_ROLL_OVER: u8 = 0x01;

/// A key array entry that is a physical key: not empty (0x00) and not one of
/// the keyboard status codes 0x01 ErrorRollOver, 0x02 POSTFail, 0x03
/// ErrorUndefined (HID Usage Tables 1.5 p. 89, footnote 1 p. 95: "Not a
/// physical key").
fn is_key(usage: u8) -> bool {
    usage > 0x03
}

/// One 8-byte boot keyboard report against the **same function's** previous
/// report `prev` (byte 0 = modifiers, bytes 2..8 = the six key codes, packed
/// little-endian into a `u64`): emit a break for every modifier/key that was
/// held and no longer is, then a make for every one that is new. Returns the
/// state to keep for this function.
///
/// Status codes in the key array (`docs/api/usb/hid-keyboard.md`, "Status
/// codes 0x01–0x03"): a report holding 0x01 ErrorRollOver keeps the previous
/// key set unchanged (no key break/make) while its modifier byte is still
/// diffed; 0x02/0x03 never make or break, the rest of their report counts.
pub fn keyboard_diff(prev: u64, r: &[u8; 8], emit: &mut impl FnMut(u8)) -> u64 {
    let modn = r[0];
    let prev = prev.to_le_bytes();
    let oldmod = prev[0];

    modifiers(oldmod & !modn, 0x80, emit); // released modifiers
    modifiers(modn & !oldmod, 0x00, emit); // pressed modifiers

    let oldkeys = &prev[2..8];
    let newkeys = if r[2..8].contains(&ERROR_ROLL_OVER) { oldkeys } else { &r[2..8] };
    for &k in oldkeys {
        if is_key(k) && !newkeys.contains(&k) {
            let s = translate_key(k);
            if s != 0 {
                emit(s | 0x80); // break
            }
        }
    }
    for &k in newkeys {
        if is_key(k) && !oldkeys.contains(&k) {
            let s = translate_key(k);
            if s != 0 {
                emit(s); // make
            }
        }
    }

    let mut p = [0u8; 8];
    p[0] = modn;
    p[2..8].copy_from_slice(newkeys);
    u64::from_le_bytes(p)
}

/// The mouse buttons the IKBD stream carries: the OR of every mouse
/// function's held buttons (USB order, bit 0 left, bit 1 right), so a button
/// held on one mouse is not released by a report of another (§9.6 Soll 3).
pub fn combined_buttons(per_function: impl IntoIterator<Item = u8>) -> u8 {
    per_function.into_iter().fold(0, |a, b| a | (b & 0x03))
}

/// One boot mouse report (byte 0 = buttons, 1 = dx, 2 = dy, 3 = wheel when
/// sent) → an IKBD relative-mouse packet `[0xF8|wire, dx, dy]` plus the wheel
/// scancodes. `buttons` is the USB button state to send (from
/// [`combined_buttons`]); USB buttons (bit 0 left, bit 1 right) are swapped
/// into IKBD wire order (bit 0 right, bit 1 left); the decoder swaps back to
/// VDI. `len` is how many bytes the device actually sent (the Transfer
/// Event's residual) — the wheel byte exists only in a longer report, see
/// [`wheel`].
pub fn mouse_packet(buttons: u8, r: &[u8; 8], len: usize, emit: &mut impl FnMut(u8)) {
    let wire = ((buttons & 0x01) << 1) | ((buttons & 0x02) >> 1);
    emit(0xF8 | (wire & 0x03));
    emit(r[1]); // dx
    emit(r[2]); // dy (the IKBD decoder owns the screen-Y convention)
    wheel(r, len, emit);
}

/// The wheel half of a boot mouse report → the IKBD extended-mouse scancodes
/// (`docs/api/usb/hid-mouse.md`, GUI04).
///
/// **Where byte 3 comes from.** The HID boot protocol's own report descriptor
/// (USB HID 1.11 Appendix B.2) defines exactly three bytes — buttons, X, Y —
/// so byte 3 is *not* part of the boot protocol proper. It is the de-facto
/// extension every wheel mouse adds, and the way to know whether a given
/// device sends it is the **transfer length**, not the byte's value. FreeMiNT's
/// boot-mouse driver — the source `docs/api/usb/hid-mouse.md` was extracted
/// from — does precisely that:
///
/// ```c
/// /* actlen is actual length of the mouse report */
/// if ((r != 0) || (actlen < 3)) return;
/// …
/// /* if there is a 4th byte, it must be z (scroll wheel) */
/// if (actlen > 3) { if (mse_data.new[3] != 0) wheel_change = 1; }
/// ```
///
/// (`references/freemint/sys/usb/src.km/udd/hid/mouse-boot/usb_mouse.c:239-
/// 261`.) So: a mouse that sends only three bytes produces `len == 3`, this
/// function returns immediately and nothing about the existing behaviour
/// changes — no phantom detents from a byte the device never wrote.
///
/// **Direction.** FreeMiNT reads `wheel = mse_data.new[3]` and turns a
/// **positive** value into "up" and a negative one into "down" (`:288-311`),
/// which is the USB HID Usage Tables' `Wheel` (Generic Desktop 0x38)
/// convention — positive = rotation away from the user. In the Atari IKBD
/// extension that is scancode `0x59` (wheel up) resp. `0x5a` (wheel down)
/// (`references/emutos/bios/ikbd.c:752-753`).
///
/// **One scancode per detent.** FreeMiNT emits `|wheel|` events (`for (i = 0;
/// i < wheel; i++)`); so do we. Its surrounding `MAX_WHEEL_INTERVAL` loop,
/// which *multiplies* them by how recently the last report arrived, is a
/// FreeMiNT acceleration heuristic with no EmuTOS counterpart and is not
/// reproduced.
///
/// **Horizontal (`0x5c`/`0x5d`) is deliberately not synthesised here.** A
/// fifth report byte is commonly AC Pan, but nothing in the boot protocol says
/// so and FreeMiNT's boot-mouse driver does not read it; knowing would require
/// parsing the HID report descriptor, which the boot protocol exists to avoid.
/// The IKBD side handles both codes (`vdi_mousex_handler`), so a source that
/// *can* identify a horizontal wheel travels the same road without a change
/// here.
fn wheel(r: &[u8; 8], len: usize, emit: &mut impl FnMut(u8)) {
    if len <= 3 {
        return;
    }
    let w = r[3] as i8 as i32;
    let (code, n) = match w {
        0 => return,
        w if w > 0 => (0x59u8, w),   // wheel up
        w => (0x5Au8, -w),           // wheel down
    };
    // Make + break, like every other IKBD key event: `kbd_int` hands the full
    // scancode to `mousexvec` and `vdi_mousex_handler` acts on the make code
    // only (`references/emutos/vdi/vdi_mouse.c:546-553`), but a real IKBD
    // source sends both and the decoder must cope with both.
    for _ in 0..n {
        emit(code);
        emit(code | 0x80);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: u8 = 0x04; // usage 'a' → scancode 0x1e
    const B: u8 = 0x05; // usage 'b' → scancode 0x30
    const SC_A: u8 = 0x1e;
    const SC_B: u8 = 0x30;

    fn kb(modifiers: u8, keys: &[u8]) -> [u8; 8] {
        let mut r = [0u8; 8];
        r[0] = modifiers;
        r[2..2 + keys.len()].copy_from_slice(keys);
        r
    }

    /// Feed `(function, report)` pairs through the diff; `shared` = one state
    /// for every function (the decoder before F3), else one per function.
    fn run(seq: &[(usize, [u8; 8])], shared: bool) -> Vec<u8> {
        let mut state = [0u64; 4];
        let mut out = Vec::new();
        for (f, r) in seq {
            let i = if shared { 0 } else { *f };
            state[i] = keyboard_diff(state[i], r, &mut |b| out.push(b));
        }
        out
    }

    /// H-2 (i): the cable keyboard (function 0) holds `a`, the receiver's
    /// keyboard (function 1) keeps reporting "nothing held". With one shared
    /// state every empty report releases `a` and the next cable report
    /// presses it again (the flicker of §9.6 Befund A); per function, `a` is
    /// pressed once and never released.
    #[test]
    fn held_key_survives_the_other_keyboards_empty_reports() {
        let seq = [(0, kb(0, &[A])), (1, kb(0, &[])), (0, kb(0, &[A])), (1, kb(0, &[])), (0, kb(0, &[A]))];
        assert_eq!(run(&seq, true), [SC_A, SC_A | 0x80, SC_A, SC_A | 0x80, SC_A], "shared state flickers");
        assert_eq!(run(&seq, false), [SC_A], "per function: one make, no break");
    }

    /// H-2 (ii): two keyboards hold `a` and `b` and report alternately.
    #[test]
    fn two_keyboards_holding_different_keys() {
        let seq = [(0, kb(0, &[A])), (1, kb(0, &[B])), (0, kb(0, &[A])), (1, kb(0, &[B]))];
        let shared = run(&seq, true);
        assert_eq!(shared, [SC_A, SC_A | 0x80, SC_B, SC_B | 0x80, SC_A, SC_A | 0x80, SC_B]);
        assert_eq!(run(&seq, false), [SC_A, SC_B], "each key made once, nothing released");
        // Releasing on one keyboard releases only that key.
        let mut seq = seq.to_vec();
        seq.push((0, kb(0, &[])));
        assert_eq!(run(&seq, false), [SC_A, SC_B, SC_A | 0x80]);
    }

    /// H-2 (iii): a modifier held on one keyboard is not dropped by the other
    /// keyboard's report without modifiers.
    #[test]
    fn modifier_held_on_one_keyboard() {
        const LSHIFT: u8 = 0x02;
        let seq = [(0, kb(LSHIFT, &[])), (1, kb(0, &[B])), (0, kb(LSHIFT, &[A]))];
        assert_eq!(run(&seq, true), [0x2a, 0x2a | 0x80, SC_B, 0x2a, SC_B | 0x80, SC_A]);
        assert_eq!(run(&seq, false), [0x2a, SC_B, SC_A]);
    }

    /// H-2 (iv): releasing everything = an empty report against every
    /// function's own state; functions that hold nothing emit nothing.
    #[test]
    fn release_all_walks_every_function() {
        let mut state = [0u64; 3];
        let mut out = Vec::new();
        state[0] = keyboard_diff(state[0], &kb(0x02, &[A]), &mut |b| out.push(b));
        state[2] = keyboard_diff(state[2], &kb(0, &[B]), &mut |b| out.push(b));
        out.clear();
        for s in state.iter_mut().filter(|s| **s != 0) {
            *s = keyboard_diff(*s, &[0; 8], &mut |b| out.push(b));
        }
        assert_eq!(out, [0x2a | 0x80, SC_A | 0x80, SC_B | 0x80]);
        assert_eq!(state, [0; 3]);
    }

    /// One keyboard alone behaves exactly as before F3 (a single state).
    #[test]
    fn single_keyboard_unchanged() {
        let seq = [(0, kb(0, &[A])), (0, kb(0, &[A, B])), (0, kb(0, &[B])), (0, kb(0, &[]))];
        assert_eq!(run(&seq, false), run(&seq, true));
        assert_eq!(run(&seq, false), [SC_A, SC_B, SC_A | 0x80, SC_B | 0x80]);
    }

    /// The phantom report of the capture: ErrorRollOver in all six slots.
    const ROLLOVER: [u8; 8] = [0x00, 0x00, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01];

    /// Feed reports of one function from `state`, collecting the bytes.
    fn feed(mut state: u64, seq: &[[u8; 8]]) -> (u64, Vec<u8>) {
        let mut out = Vec::new();
        for r in seq {
            state = keyboard_diff(state, r, &mut |b| out.push(b));
        }
        (state, out)
    }

    /// Real capture `measurements/measurement-2026-10-24-usb01-f3-pi/usb/05AC020C.TXT`
    /// seq 116-118 (Apple Extended USB Keyboard): `,` (usage 0x36, scancode
    /// 0x33) and `m` (0x10 → 0x32) held, the phantom report, then `,` and `j`
    /// (0x0d → 0x24). Against the state of seq 116 the only events are break
    /// `m` and make `j`; `,` is neither released nor pressed again
    /// (`docs/api/usb/hid-keyboard.md`, status codes, rule 1).
    #[test]
    fn error_rollover_keeps_the_key_set_capture_116_118() {
        let (s, _) = feed(0, &[[0, 0, 0x36, 0x10, 0, 0, 0, 0]]); // seq 116
        let (_, out) = feed(s, &[ROLLOVER, [0, 0, 0x36, 0x0d, 0, 0, 0, 0]]); // seq 117, 118
        assert_eq!(out, [0x32 | 0x80, 0x24], "break m (usage 0x10), make j (usage 0x0d), nothing for ,");
    }

    /// The same capture, seq 158-160 (`j k` → phantom → `k u`: break j, make
    /// u, k held) and seq 164-166 (`l o` → phantom → `l`: break o, l held).
    /// The phantom report alone emits nothing.
    #[test]
    fn error_rollover_capture_158_160_and_164_166() {
        let (s, _) = feed(0, &[[0, 0, 0x0d, 0x0e, 0, 0, 0, 0]]); // 158
        let (s, out) = feed(s, &[ROLLOVER]); // 159
        assert_eq!(out, [], "phantom report: no event");
        let (_, out) = feed(s, &[[0, 0, 0x0e, 0x18, 0, 0, 0, 0]]); // 160
        assert_eq!(out, [0x24 | 0x80, 0x16], "break j, make u");

        let (s, _) = feed(0, &[[0, 0, 0x0f, 0x12, 0, 0, 0, 0]]); // 164
        let (_, out) = feed(s, &[ROLLOVER, [0, 0, 0x0f, 0, 0, 0, 0, 0]]); // 165, 166
        assert_eq!(out, [0x18 | 0x80], "break o only");
    }

    /// Rule 2: the modifier byte of a phantom report is still evaluated
    /// (HID 1.11 Appendix C p. 63 "Modifiers still reported"); the key set
    /// stays.
    #[test]
    fn error_rollover_still_evaluates_modifiers() {
        const LSHIFT: u8 = 0x02;
        const RALT: u8 = 0x40;
        let mut out = Vec::new();
        let mut s = keyboard_diff(0, &kb(LSHIFT, &[A, B]), &mut |_| {});
        let mut phantom = ROLLOVER;
        phantom[0] = RALT; // LShift released, RAlt pressed, inside the phantom report
        s = keyboard_diff(s, &phantom, &mut |b| out.push(b));
        assert_eq!(out, [0x2a | 0x80, 0x38], "modifier break + make, no key event");
        out.clear();
        keyboard_diff(s, &kb(RALT, &[A, B]), &mut |b| out.push(b));
        assert_eq!(out, [], "keys a, b were held all along; RAlt unchanged");
    }

    /// Rule 1 holds per function (F3): one keyboard's phantom report does not
    /// touch the other keyboard's held key.
    #[test]
    fn error_rollover_is_per_function() {
        let seq = [(0, kb(0, &[A])), (1, kb(0, &[B])), (1, ROLLOVER), (0, kb(0, &[])), (1, kb(0, &[B]))];
        assert_eq!(run(&seq, false), [SC_A, SC_B, SC_A | 0x80]);
    }

    /// Rule 3: 0x02 POSTFail / 0x03 ErrorUndefined in a slot neither make nor
    /// break, and do not stop the rest of the report from being evaluated.
    #[test]
    fn post_fail_and_error_undefined_are_not_keys() {
        let seq = [
            (0, kb(0, &[A])),
            (0, kb(0, &[A, 0x02, B])),  // make b; 0x02 is nothing
            (0, kb(0, &[0x03, B])),     // break a; 0x02 gone, 0x03 new: nothing
            (0, kb(0x02, &[0x03])),     // break b, make LShift; 0x03 is nothing
            (0, kb(0x02, &[])),         // 0x03 gone: nothing
        ];
        assert_eq!(run(&seq, false), [SC_A, SC_B, SC_A | 0x80, 0x2a, SC_B | 0x80]);
    }

    /// §9.6 Soll 3: the buttons sent are the OR over the mouse functions; the
    /// wire byte swaps left/right; the wheel byte only counts when sent.
    #[test]
    fn mouse_buttons_are_or_ed_and_wheel_needs_length() {
        assert_eq!(combined_buttons([0x01, 0x00]), 0x01, "mouse A holds left, B moves");
        assert_eq!(combined_buttons([0x01, 0x02, 0x04]), 0x03, "middle is not an IKBD button");
        assert_eq!(combined_buttons([]), 0);
        let mut out = Vec::new();
        mouse_packet(0x01, &[0x00, 5, 0xFE, 0x01, 0, 0, 0, 0], 3, &mut |b| out.push(b));
        assert_eq!(out, [0xFA, 5, 0xFE], "left = wire bit 1; 3-byte report: no wheel");
        out.clear();
        mouse_packet(0x02, &[0x02, 0, 0, 0xFE, 0, 0, 0, 0], 4, &mut |b| out.push(b));
        assert_eq!(out, [0xF9, 0, 0, 0x5A, 0xDA, 0x5A, 0xDA], "right = wire bit 0; wheel down 2");
    }
}
