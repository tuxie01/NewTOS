//! HID boot keyboard / mouse reports → the IKBD byte stream.
//!
//! Translate per `docs/api/usb/hid-keyboard.md` / `hid-mouse.md`: turn USB
//! HID boot reports into the exact bytes a real Atari IKBD would send and push
//! them through [`crate::ikbd::feed`], reusing the whole existing decoder
//! (scancode→ASCII→console, relative-mouse→VDI cursor).
//!
//! **State per function** (USB01 F3, `docs/api/usb/hid-mouse-protocols.md`
//! §9.6 Soll 1/3): every polled HID boot interface (`xhci::MAX_FUNC` of them)
//! keeps its own previous keyboard report and its own held mouse buttons, so
//! two keyboards no longer diff against each other's reports and a button
//! held on one mouse is not released by another. All functions feed the one
//! IKBD stream (an Atari has one keyboard, §9.6 Soll 2). The byte logic is in
//! [`super::hid_decode`] (pure, host-tested).

use core::sync::atomic::{AtomicU64, AtomicU8, Ordering};

use super::hid_decode;
use super::xhci::MAX_FUNC;
use crate::ikbd;

/// Previous keyboard report per function (byte 0 = modifiers, bytes 2..8 =
/// six keycodes), packed into a u64 so the diff is allocation-free.
static PREV: [AtomicU64; MAX_FUNC] = [const { AtomicU64::new(0) }; MAX_FUNC];

/// USB buttons (bit 0 left, bit 1 right) of each mouse function's last
/// report.
static MOUSE_BUTTONS: [AtomicU8; MAX_FUNC] = [const { AtomicU8::new(0) }; MAX_FUNC];

/// Release every key and mouse button the decoder last saw held, on every
/// function: an empty keyboard report against each keyboard function's own
/// state (breaks for every key and modifier it holds, which also stops a
/// running key repeat) and, if any button is down, one mouse report with no
/// button and no motion. Called once when the USB snapshot's MUTE starts
/// (`usb::poll`, `docs/api/usb/usb-snapshot.md` §9.1).
pub fn release_all() {
    for (f, prev) in PREV.iter().enumerate() {
        if prev.load(Ordering::Relaxed) != 0 {
            keyboard_report(f, &[0u8; 8]);
        }
    }
    if MOUSE_BUTTONS.iter().any(|b| b.load(Ordering::Relaxed) != 0) {
        for b in &MOUSE_BUTTONS {
            b.store(0, Ordering::Relaxed);
        }
        hid_decode::mouse_packet(0, &[0u8; 8], 3, &mut ikbd::feed);
    }
}

/// Process one 8-byte boot keyboard report of function `func`: diff against
/// that function's previous report and feed make/break scancodes into the
/// IKBD decoder.
pub fn keyboard_report(func: usize, r: &[u8; 8]) {
    let Some(prev) = PREV.get(func) else { return };
    let next = hid_decode::keyboard_diff(prev.load(Ordering::Relaxed), r, &mut ikbd::feed);
    prev.store(next, Ordering::Relaxed);
}

/// Process one boot mouse report of function `func` (byte 0 = buttons, 1 =
/// dx, 2 = dy) → an IKBD relative-mouse packet fed to the decoder. The
/// buttons sent are the OR of every mouse function's held buttons.
///
/// `len` is how many bytes the device actually sent (from the Transfer
/// Event's residual, `xhci::Xhci::poll_interrupt`) — the wheel byte exists
/// only in a longer report (`hid_decode::mouse_packet`).
pub fn mouse_report(func: usize, r: &[u8; 8], len: usize) {
    let Some(held) = MOUSE_BUTTONS.get(func) else { return };
    held.store(r[0] & 0x03, Ordering::Relaxed);
    let buttons = hid_decode::combined_buttons(MOUSE_BUTTONS.iter().map(|b| b.load(Ordering::Relaxed)));
    hid_decode::mouse_packet(buttons, r, len, &mut ikbd::feed);
}
