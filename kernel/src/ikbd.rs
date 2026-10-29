//! Atari ST IKBD (Intelligent Keyboard) decoder.
//!
//! The Atari ST IKBD streams a serial protocol to the host: key make/break
//! scancodes plus mouse / joystick / clock packets. This module is the
//! protocol **decoder** — the byte sink the rest of the system reads
//! keyboard/mouse input through, independent of where the bytes come from.
//!
//! The byte **source** is the USB-HID stack (PHASE-16 / ADR-015): the USB
//! keyboard/mouse class drivers translate HID reports into the exact IKBD
//! byte stream a real keyboard would send and push it through [`feed`].
//! (Wiring a native ST keyboard to UART2 was shelved — too much analogue
//! glue; the `ikbd_uart` driver was removed.)
//!
//! [`feed`] consumes one byte at a time, assembles the multi-byte
//! mouse/joystick/clock packets, turns key make events into ASCII (US
//! layout) → the console RX ring (`Cnecin`/`Cconin`/the shell), and
//! relative-mouse packets → the VDI cursor. A held non-modifier key repeats
//! (EmuTOS' BIOS software repeat, run polled — [`poll_repeat`],
//! [`crate::key_repeat`]). Every key byte, make and break, first passes the
//! running process's `kbdvec` (KEYS02, `tos_api::xbios::kbdvecs`): a program
//! hook receives it as an EL0 upcall; with the default in place the decoder
//! ([`kbd_int`]) runs directly.
//!
//! Protocol + scancode set: Atari IKBD documentation; the scancode→ASCII
//! tables mirror EmuTOS `bios/keyboard.c` (US layout). Country layouts
//! and dead-key handling come from EmuTOS later.

use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use crate::console;
use crate::input_gate::{HeldBytes, HELD_CAP};
use crate::key_repeat::KeyRepeat;
use crate::sync::SpinMutex;

// ── modifier state ────────────────────────────────────────────────────
// Left and right Shift are separate bits, as in EmuTOS' `shifty`
// (`MODE_LSHIFT`/`MODE_RSHIFT`, `bios/ikbd.c:779-783,830-834`): the reset
// combination tells them apart (RST01, `bios/ikbd.c:734-747`).
static LSHIFT: AtomicBool = AtomicBool::new(false);
static RSHIFT: AtomicBool = AtomicBool::new(false);
static CTRL: AtomicBool = AtomicBool::new(false);
static CAPS: AtomicBool = AtomicBool::new(false);
static ALT: AtomicBool = AtomicBool::new(false);

// IKBD scancodes of the modifier keys.
const SC_CTRL: u8 = 0x1D;
const SC_LSHIFT: u8 = 0x2A;
const SC_RSHIFT: u8 = 0x36;
const SC_ALT: u8 = 0x38;
const SC_CAPS: u8 = 0x3A;

/// Remaining data bytes to skip for the packet currently being received
/// (mouse / joystick / status / clock). `0` = expecting a scancode or a
/// new packet header.
static PKT_REMAINING: AtomicU8 = AtomicU8::new(0);

/// Number of data bytes that follow an IKBD status-packet header.
fn packet_data_len(header: u8) -> u8 {
    match header {
        0xF6 => 6,         // status report
        0xF7 => 5,         // absolute mouse
        0xF8..=0xFB => 2,  // relative mouse (low 2 bits = buttons)
        0xFC => 6,         // time-of-day
        0xFD => 2,         // both joysticks
        0xFE | 0xFF => 1,  // joystick 0 / joystick 1
        _ => 0,
    }
}

/// Relative-mouse packet assembly: `0` = idle, `1` = expecting `dx`,
/// `2` = expecting `dy`.
static MS_STEP: AtomicU8 = AtomicU8::new(0);
/// Button bitmask (already swapped to VDI convention) from the packet
/// header, held until the movement bytes complete the packet.
static MS_BUTTONS: AtomicU8 = AtomicU8::new(0);
/// First movement byte (`dx`), held until `dy` arrives.
static MS_DX: AtomicU8 = AtomicU8::new(0);

// ── INPUT01: the timer-interrupt pump's capture ───────────────────────
//
// The interrupt pump (`host_env::irq_input_pump`) drains one USB report with
// [`CAPTURE`] raised, so its bytes land in [`HELD`] instead of the decoder,
// and then lets [`capture_end`] decide: pure pointer motion is decoded at
// once, anything else stays held for the next trap-side pump
// ([`release_held`]). Policy and reasons: `crate::input_gate`.

/// Raised while the interrupt pump drains a report: [`feed`] collects.
static CAPTURE: AtomicBool = AtomicBool::new(false);

/// The report the interrupt pump drained but did not decode. Only touched
/// inside the input pump, which never runs twice at once
/// (`input_gate::PumpLock`), so the lock is never contended.
static HELD: SpinMutex<HeldBytes> = SpinMutex::new(HeldBytes::new());

/// Feed one received IKBD byte to the decoder — or, while the interrupt pump
/// captures, to the held report.
pub fn feed(byte: u8) {
    if CAPTURE.load(Ordering::Relaxed) {
        if HELD.lock().push(byte) {
            return;
        }
        // Full — impossible for one boot report (`input_gate::HELD_CAP`).
        // Never drop a byte: decode what is held, in order, then this one.
        decode_held();
    }
    decode_byte(byte);
}

/// Start a capture for the interrupt pump. `false` — and nothing started —
/// while an earlier report is still held: the interrupt then drains nothing,
/// so the reports behind the held one stay in the USB controller, in order.
pub fn capture_begin() -> bool {
    if !HELD.lock().is_empty() {
        return false;
    }
    CAPTURE.store(true, Ordering::Relaxed);
    true
}

/// End the interrupt pump's capture: decode the captured report now if it is
/// pure pointer motion at the current button state
/// ([`crate::input_gate::pure_motion`]) and the decoder sits between packets;
/// otherwise keep it held for [`release_held`].
pub fn capture_end() {
    CAPTURE.store(false, Ordering::Relaxed);
    let idle = MS_STEP.load(Ordering::Relaxed) == 0 && PKT_REMAINING.load(Ordering::Relaxed) == 0;
    let decode_now = {
        let h = HELD.lock();
        idle && crate::input_gate::pure_motion(h.as_slice(), MS_BUTTONS.load(Ordering::Relaxed))
    };
    if decode_now {
        decode_held();
    }
}

/// The trap-side pump: decode the report the interrupt pump held, in order,
/// in place of draining a new one. `true` if there was one.
pub fn release_held() -> bool {
    if HELD.lock().is_empty() {
        return false;
    }
    decode_held();
    true
}

/// Decode and empty the held report (oldest byte first). The bytes are moved
/// out before decoding, so the decoder never runs under the lock.
fn decode_held() {
    let mut bytes = [0u8; HELD_CAP];
    let n = HELD.lock().take_into(&mut bytes);
    for &b in &bytes[..n] {
        decode_byte(b);
    }
}

/// Decode one IKBD byte.
fn decode_byte(byte: u8) {
    // Mid relative-mouse packet → collect dx, then dy, then dispatch.
    match MS_STEP.load(Ordering::Relaxed) {
        1 => {
            MS_DX.store(byte, Ordering::Relaxed);
            MS_STEP.store(2, Ordering::Relaxed);
            return;
        }
        2 => {
            MS_STEP.store(0, Ordering::Relaxed);
            let dx = MS_DX.load(Ordering::Relaxed) as i8 as i16;
            let dy = byte as i8 as i16;
            mouse_event(MS_BUTTONS.load(Ordering::Relaxed), dx, dy);
            return;
        }
        _ => {}
    }

    // Inside a non-mouse status packet → consume and ignore.
    let rem = PKT_REMAINING.load(Ordering::Relaxed);
    if rem > 0 {
        PKT_REMAINING.store(rem - 1, Ordering::Relaxed);
        return;
    }
    // A packet header (>= 0xF6) starts a run of data bytes.
    if byte >= 0xF6 {
        // Relative-mouse report (0xF8..0xFB): the low 2 header bits are
        // the buttons (IKBD wire: bit 0 = right, bit 1 = left). Swap into
        // VDI convention (bit 0 = left, bit 1 = right) and collect the two
        // movement bytes. See docs/api/vdi/input/_mouse-packet.md.
        if (0xF8..=0xFB).contains(&byte) {
            let wire = byte & 0x03;
            let gem = ((wire >> 1) & 0x01) | ((wire & 0x01) << 1);
            MS_BUTTONS.store(gem, Ordering::Relaxed);
            MS_STEP.store(1, Ordering::Relaxed);
            return;
        }
        PKT_REMAINING.store(packet_data_len(byte), Ordering::Relaxed);
        return;
    }
    // Otherwise it is a key scancode (bit 7 = break). EmuTOS `ikbdraw` jumps
    // through `kbdvec` here (`bios/aciavecs.S:407-414`): the running
    // process's hook gets the byte if it installed one (queued for EL0 upcall
    // delivery, KEYS02); with the default still in place — or no process
    // running — the default `kbdvec` (`kbd_int`) runs directly, as before.
    if !tos_api::xbios::kbdvecs::route_key(byte) {
        decode_key(byte);
    }
}

/// The default `kbdvec` — EmuTOS `kbd_int(scancode)` (`bios/ikbd.c:725-910`)
/// for one key byte, bit 7 = break. Called by [`feed`] on the fast path and,
/// for a process whose hook chained to the default, from
/// `Ssystem(S_NEWTOS_KBDVEC)` (the `kbdvec_default` stub, KEYS02 D6).
pub fn kbd_int(raw: u8) {
    decode_key(raw);
}

/// Apply a decoded relative-mouse event to the VDI mouse state. The byte
/// decode (hardware) lives here; the cursor semantics (position
/// accumulation + clamp, with `buttons` already in VDI convention) live
/// in the TOS API layer.
fn mouse_event(buttons: u8, dx: i16, dy: i16) {
    // No screen device yet → nothing to position the cursor against.
    if !crate::vdi_backend::available() {
        return;
    }
    tos_api::vdi::feed_relative(dx, dy, buttons, &crate::vdi_backend::VDI_SCREEN);
}

/// The seven IKBD scancodes that are **not** keys but extra mouse buttons and
/// wheel detents — EmuTOS `kbd_int`'s `CONF_WITH_EXTENDED_MOUSE` block
/// (`bios/ikbd.c:748-760`), which lists them in this order:
///
/// ```c
/// if (   scancode_only == 0x37  /* Mouse button 3 */
///     || scancode_only == 0x5e  /* Mouse button 4 */
///     || scancode_only == 0x5f  /* Mouse button 5 */
///     || scancode_only == 0x59  /* Wheel up */
///     || scancode_only == 0x5a  /* Wheel down */
///     || scancode_only == 0x5c  /* Wheel left */
///     || scancode_only == 0x5d) /* Wheel right */
/// ```
///
/// Compared on the **make** code (`scancode & ~KEY_RELEASED`), so both the
/// press and the release form reach the vector.
const fn is_extended_mouse(code: u8) -> bool {
    matches!(code, 0x37 | 0x59 | 0x5A | 0x5C | 0x5D | 0x5E | 0x5F)
}

fn decode_key(raw: u8) {
    // Keyboard warm/cold reset — EmuTOS `kbd_int`'s first test
    // (`bios/ikbd.c:734-747`): the Delete make code with Ctrl+Alt and not
    // LShift; RShift in addition is the cold reset. Before everything else,
    // as there. Warm and cold both restart the machine for now (RST01
    // decision 2; the warm start that keeps the sysvars is RST02). Only a
    // board without a restart mechanism returns, and the key then goes on as
    // a normal key, as when the combination does not match.
    if let Some(kind) = tos_api::bios::reset::reset_key(raw, shift_state()) {
        crate::reset::restart(kind, "keyboard Ctrl+Alt+Del");
    }
    let make = raw & 0x80 == 0;
    let code = raw & 0x7F;
    // `CONF_WITH_EXTENDED_MOUSE` is `1` in the reference build
    // (`include/config.h:1491-1498`): the extra mouse buttons and the wheel
    // arrive as IKBD scancodes and "use a separate vector" — `kbd_int` hands
    // them to `mousexvec(scancode)` and **returns** (`bios/ikbd.c:746-760`),
    // before the modifier bookkeeping, the ASCII translation and the key
    // queue, so none of the seven is ever seen as a key. EmuTOS places the
    // block right after its Ctrl-Alt-Del reset check — as here. The **full** scancode (release bit included) is passed on; the
    // handler tells press from release by it.
    if is_extended_mouse(code) {
        tos_api::bios::vectors::mousexvec(raw);
        return;
    }
    match code {
        // Modifier makes and breaks only update the shift state; a running
        // key repeat continues (EmuTOS `bios/ikbd.c:778-801,825-870`).
        SC_LSHIFT => LSHIFT.store(make, Ordering::Relaxed),
        SC_RSHIFT => RSHIFT.store(make, Ordering::Relaxed),
        SC_CTRL => CTRL.store(make, Ordering::Relaxed),
        SC_ALT => ALT.store(make, Ordering::Relaxed), // AltGr for the DE layout
        SC_CAPS if make => {
            CAPS.fetch_xor(true, Ordering::Relaxed);
        }
        _ if make => {
            // Queue every make key as the IOREC entry `shifty << 24 |
            // scancode << 16 | ascii` with the shift state of this moment
            // (EmuTOS `kb_last.k.shifty = shifty` then
            // `push_ikbdiorec(kb_last.key)`, `bios/ikbd.c:898-909`; XBIOS01
            // E2). Keys with no character (arrows, function keys) carry ascii
            // 0 so the AES still sees the scancode (GEMDOS console readers
            // skip them).
            let word = key_word(code);
            let shifty = shift_state();
            console::push_rx(tos_api::xbios::iorec::entry_from_word(word, shifty));
            // It becomes the repeating key, armed with the current initial
            // delay (EmuTOS `kbd_int` `:875,891,898-903`).
            let (initial, _) = tos_api::xbios::system::kb_rate();
            // `kb_last_ikbdsys = kbdvecs.ikbdsys` (`:902`): the repeat stops
            // if that vector changes before the key is released (`:453`).
            REPEAT.lock().make(
                code,
                word,
                shifty,
                initial,
                tos_api::bios::tick::hz_200(),
                tos_api::xbios::kbdvecs::current_ikbdsys(),
            );
        }
        // A non-modifier break (CapsLock's included, as in EmuTOS' `default`
        // arm): stops the repeat if it is the repeating key, or Home/Insert
        // (`bios/ikbd.c:807-817`).
        _ => REPEAT.lock().brk(code),
    }
}

/// The key word `scancode << 8 | ascii` for `code` under the current
/// modifiers (ascii 0 for keys with no character).
fn key_word(code: u8) -> u16 {
    let ascii = to_ascii(code).unwrap_or(0);
    (code as u16) << 8 | ascii as u16
}

/// Key auto-repeat state (EmuTOS `kb_ticks`/`kb_last`/`kb_last_actual`).
/// Touched only from the input pump — the decoder above (fed by
/// `usb::poll`) and [`poll_repeat`] — never from an interrupt handler, so the
/// lock is never contended. (INPUT01's timer-interrupt pump decodes pointer
/// motion only; key bytes and the repeat stay in the trap-side pump.)
static REPEAT: SpinMutex<KeyRepeat> = SpinMutex::new(KeyRepeat::new());

/// Run the key auto-repeat for this poll: credit the 50 Hz steps `_hz_200`
/// crossed since the previous call and queue the repeating key's entry if a
/// repeat fell due — at most one per call. The entry carries the shift state
/// of the repeat (`kb_last.k.shifty`, refreshed when the modifiers changed,
/// `bios/ikbd.c:502-508`, pushed at `:524`).
///
/// EmuTOS runs this (`do_key_repeat`, `bios/ikbd.c:451-528`) from the
/// timer-C interrupt. NewTOS runs it **polled**, and the caller must invoke it
/// **after** draining the input source (`drivers::usb::poll`), so a break that
/// arrived in the meantime has already stopped the repeat — no ghost keys
/// after the release. Sanctioned deviation, spec `docs/api/xbios/_state.md`
/// §"Key-repeat handler"; state machine and burst guard in
/// [`crate::key_repeat`].
pub fn poll_repeat() {
    let now = tos_api::bios::tick::hz_200();
    let conterm = tos_api::bios::chardev::current_conterm();
    let (_, repeat) = tos_api::xbios::system::kb_rate();
    let shifty = shift_state();
    let ikbdsys = tos_api::xbios::kbdvecs::current_ikbdsys();
    let word = REPEAT
        .lock()
        .poll(now, conterm, shifty, repeat, ikbdsys, key_word);
    if let Some(word) = word {
        // `poll` reconverted `word` under `shifty` if the modifiers changed,
        // so `shifty` is the repeat's `kb_last.k.shifty`.
        console::push_rx(tos_api::xbios::iorec::entry_from_word(word, shifty));
    }
}

/// The current keyboard shift-state bitmask, in BIOS `Kbshift` /
/// VDI `vq_key_s` encoding (`MODE_*` bits, `include/biosdefs.h`):
/// `0x01` right-shift, `0x02` left-shift, `0x04` ctrl, `0x08` alt,
/// `0x10` caps-lock — left and right Shift each on their own bit, as
/// EmuTOS keeps them (`bios/ikbd.c:779-783,830-834`).
pub fn shift_state() -> u8 {
    let mut m = 0u8;
    if RSHIFT.load(Ordering::Relaxed) {
        m |= 0x01; // MODE_RSHIFT
    }
    if LSHIFT.load(Ordering::Relaxed) {
        m |= 0x02; // MODE_LSHIFT
    }
    if CTRL.load(Ordering::Relaxed) {
        m |= 0x04; // MODE_CTRL
    }
    if ALT.load(Ordering::Relaxed) {
        m |= 0x08; // MODE_ALT
    }
    if CAPS.load(Ordering::Relaxed) {
        m |= 0x10; // MODE_CAPS
    }
    m
}

/// Overwrite the keyboard shift-state from a `Kbshift(flag)` set call
/// (`flag >= 0`). Mirrors EmuTOS storing `flag` into the single `shifty`
/// byte (`bios/ikbd.c:150`): each modifier atomic is written from the
/// matching `MODE_*` bit so a following [`shift_state`] reads the value
/// back, and a later physical make/break still updates its own bit. Bits 5-7
/// (`MODE_HOME`/`MODE_INSERT`/reserved) are TT/Falcon-era and not
/// represented, so they do not round-trip (`docs/api/bios/0b-kbshift.md`).
pub fn set_shift_state(mask: u8) {
    RSHIFT.store(mask & 0x01 != 0, Ordering::Relaxed); // MODE_RSHIFT
    LSHIFT.store(mask & 0x02 != 0, Ordering::Relaxed); // MODE_LSHIFT
    CTRL.store(mask & 0x04 != 0, Ordering::Relaxed); // MODE_CTRL
    ALT.store(mask & 0x08 != 0, Ordering::Relaxed); // MODE_ALT
    CAPS.store(mask & 0x10 != 0, Ordering::Relaxed); // MODE_CAPS
}

/// Translate a key scancode to ASCII against the active keyboard layout
/// ([`crate::keymap`]) and the current modifiers, folding Ctrl + letter to
/// its control code. Returns `None` when the key has no character (cursor /
/// function keys, unmapped scancodes).
fn to_ascii(code: u8) -> Option<u8> {
    let shift = LSHIFT.load(Ordering::Relaxed) || RSHIFT.load(Ordering::Relaxed);
    let caps = CAPS.load(Ordering::Relaxed);
    let alt = ALT.load(Ordering::Relaxed);
    let mut ch = crate::keymap::to_ascii(code, shift, caps, alt);
    if ch == 0 {
        return None;
    }
    // Ctrl + letter → the corresponding control code (Ctrl-A = 0x01 …).
    if CTRL.load(Ordering::Relaxed) {
        let up = ch.to_ascii_uppercase();
        if up.is_ascii_uppercase() {
            ch = up - b'A' + 1;
        }
    }
    Some(ch)
}
