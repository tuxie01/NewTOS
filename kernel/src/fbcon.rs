//! The `Bconout(2, …)` screen device — the VT52 framebuffer text console.
//!
//! Renders BIOS/GEMDOS console output (`Bconout(2,…)`, `Cconout`, `Cconws`,
//! `Fwrite` to a CON handle …) to the framebuffer in the VDI's system font —
//! 8×16, or 8×8 on a screen under 400 lines (`font_set_default`,
//! `bios/font.c:76`). The kernel
//! [`Host::console_write_byte`](crate::host_env) forwards each byte here (in
//! addition to the UART).
//!
//! **This module owns no state.** The VT52 automaton, the cursor, the
//! colours, the reverse/wrap flags and the cell rendering live in the VDI's
//! alpha layer ([`tos_api::vdi::alpha_conout`], spec
//! `docs/api/bios/_vt52.md`), which is the *same* state the VDI escape
//! sub-functions (`v_enter_cur`, `v_curtext`, `v_rvon`, …) drive — exactly as
//! in EmuTOS, where those escapes are literally `Bconout(2, ch)` into the one
//! BIOS VT52 console (`vdi/vdi_esc.c:29-42,83,95,205`). That sharing is what
//! makes the classic TOS "GEM program runs a TOS program" sequence work:
//! `v_enter_cur` clears and homes, and the child's console output then starts
//! at the home position on the screen it just cleared. `Bconout(5)` (raw,
//! no automaton) reaches the same layer from `tos_api::bios::chardev`
//! directly.
//!
//! Console output is **not gated** on any mode flag, for the same reason it
//! is not in EmuTOS: `bconout2` writes into screen memory unconditionally,
//! whether the AES considers itself in graphics mode or not. A program that
//! prints while the GEM screen is up paints over it — as it does on TOS.

use crate::vdi_backend::{self, VDI_SCREEN};

/// `vt52_init` (EmuTOS `bios/vt52.c:921-953`, run from `bios_init`): reset
/// the console — cursor home and hidden, no line wrap, normal video, colours
/// 15 on 0 — and clear the screen. Once, when the framebuffer is up and
/// before the first program prints.
pub fn init() {
    if !vdi_backend::available() {
        return;
    }
    tos_api::vdi::alpha_init(&VDI_SCREEN);
}

/// Clear the screen and home the cursor — the VT52 `ESC E` — for the
/// kernel's own console (`cls`, the EmuCON stand-in, the demos).
pub fn clear_and_home() {
    if !vdi_backend::available() {
        return;
    }
    tos_api::vdi::alpha_clear_home(&VDI_SCREEN);
}

/// Write one byte to the console screen through the VT52 automaton.
pub fn write_byte(b: u8) {
    if !vdi_backend::available() {
        return;
    }
    tos_api::vdi::alpha_conout(&VDI_SCREEN, b);
}

/// The text cursor's blink step, from the 200 Hz timer interrupt — EmuTOS
/// runs `blink()` in every VBL (`bios/vectors.S:355`). It does nothing
/// unless the cursor is visible and flashing, and skips the tick while a
/// console operation is in progress.
pub fn blink_tick() {
    if !vdi_backend::available() {
        return;
    }
    tos_api::vdi::alpha_blink(&VDI_SCREEN);
}
