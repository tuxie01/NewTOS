//! V67 — the VT52 cell writer at the 32-plane setting, with the physical
//! workstation open: `docs/api/bios/_vt52-32plane.md` rules C1-C7, through the
//! real console (`tos_api::vdi::alpha_*`) onto the kernel's `VdiScreen` over a
//! host store (`vdi_planes32_common`, 320×200: the 8×8 console font, 40×25
//! cells).
//!
//! - C1/C2: a glyph cell is 8 raw 32-bit words per row, in the **pen pixels**
//!   `0x00RRGGBB` of pen `REV_MAP_COL[reg]` (8 bits per gun, V64 P3), not the
//!   5-bit words replicated; reverse video swaps them.
//! - C4: the cursor complements all 32 bits of every word of the cell, and a
//!   second toggle is the identity, over colours no 5-bit word can hold.
//! - C5: a clear fills the background register's pen pixel.
//! - C6: a scroll moves the raw words and blanks the new row in the background.
//! - Section 4 (ADR-038 point 11): at 16 planes the same bytes give the 16-bit
//!   words of the 5-bit pens, and the two pictures differ by at most 5 per gun
//!   (`docs/api/vdi/raster/_raster.md` P9).
//!
//! The case before any workstation is open (C3) is `vt52_console_32_default_palette.rs`.
//! Ticket: `TASK/V67.md`.

#![cfg(all(target_arch = "aarch64", any(target_os = "macos", target_os = "linux")))]

mod vdi_planes32_common;

use std::sync::MutexGuard;

use tos_api::vdi::backend::VDI_PLANES_WIDE;
use tos_api::vdi::font;
use vdi_planes32_common::*;

const ESC: u8 = 0x1b;
/// The console cell at 200 lines: 8 × 8 (`bios/font.c:72-88`).
const CW: i16 = 8;
const CH: i16 = 8;
const MX: i16 = W as i16 / CW - 1;
const MY: i16 = H as i16 / CH - 1;

/// EmuTOS `REV_MAP_COL` of registers 0..15 on a 256-pen device
/// (`vdi/vdi_col.c:28-29,736-747`).
const REV_MAP_COL: [usize; 16] = [0, 2, 3, 6, 4, 7, 5, 8, 9, 10, 11, 14, 12, 15, 13, 255];

/// The setting, the physical workstation opened at it (as the AES does), and
/// the console reset (`vt52_init`).
fn setup(planes: u16) -> (MutexGuard<'static, ()>, Ws) {
    let g = setting(planes);
    let ws = Ws::open();
    tos_api::vdi::alpha_init(screen());
    (g, ws)
}

fn out(bytes: &[u8]) {
    for &b in bytes {
        tos_api::vdi::alpha_conout(screen(), b);
    }
}

/// The screen pixel of register `reg` at `planes`, from the rules: the pen
/// pixel of pen `REV_MAP_COL[reg]` at 32 (P3), the 5-bit word replicated at
/// 16; pen 255 is 0 (`init_wk`).
fn reg_pixel(planes: u16, reg: usize) -> u32 {
    let pen = REV_MAP_COL[reg];
    if pen == 255 {
        return 0;
    }
    let (r, g, b) = VIDEL_PALETTE1[pen];
    let rgb = [r as i32, g as i32, b as i32];
    if planes == VDI_PLANES_WIDE {
        pixel32(rgb)
    } else {
        word_to_device(word16(rgb))
    }
}

/// The raw words of cell `(col, row)`, row by row.
fn cell(col: i16, row: i16) -> Vec<u32> {
    let mut v = Vec::new();
    for y in 0..CH {
        for x in 0..CW {
            v.push(screen().raw(col * CW + x, row * CH + y));
        }
    }
    v
}

/// What cell must hold for glyph `ch` in `fg` on `bg`.
fn glyph_cell(ch: u8, fg: u32, bg: u32) -> Vec<u32> {
    let rows = font::glyph_rows(font::font_set_default(H as i16), ch as u16);
    assert_eq!(rows.len(), CH as usize);
    let mut v = Vec::new();
    for &bits in rows {
        for x in 0..8 {
            v.push(if bits & (0x80 >> x) != 0 { fg } else { bg });
        }
    }
    v
}

/// `ESC b reg`, `ESC c reg`.
fn colours(fg: u8, bg: u8) {
    out(&[ESC, b'b', b'0' + fg, ESC, b'c', b'0' + bg]);
}

/// Registers 8 (pen 9, grey 533: 8 bits 136, 5 bits replicated 140) and 7
/// (pen 8, grey 733: 187 against 189) — colours where the 8-bit pen pixel
/// and the 5-bit word differ.
const FG: u8 = 8;
const BG: u8 = 7;

#[test]
fn a_glyph_cell_holds_the_pen_pixels_at_32() {
    let (_g, _ws) = setup(32);
    assert_eq!(reg_pixel(32, FG as usize), 0x0088_8888, "the formula's grey 533");
    assert_eq!(reg_pixel(32, BG as usize), 0x00BB_BBBB, "the formula's grey 733");
    colours(FG, BG);
    out(b"\x1bHX");
    assert_eq!(cell(0, 0), glyph_cell(b'X', reg_pixel(32, 8), reg_pixel(32, 7)));
    // Reverse video swaps foreground and background (`M_REVID`).
    out(b"\x1bpY\x1bq");
    assert_eq!(cell(1, 0), glyph_cell(b'Y', reg_pixel(32, 7), reg_pixel(32, 8)));
    // The default colours: register 15 (pen 255, pixel 0) on register 0
    // (pen 0, white): the same pixels at both settings.
    colours(15, 0);
    out(b"Z");
    assert_eq!(cell(2, 0), glyph_cell(b'Z', 0, 0x00FF_FFFF));
}

#[test]
fn the_cursor_complements_all_32_bits_and_twice_is_the_identity_at_32() {
    let (_g, _ws) = setup(32);
    colours(FG, BG);
    out(b"\x1bHX\x1bH");
    let before = cell(0, 0);
    assert_eq!(before, glyph_cell(b'X', reg_pixel(32, 8), reg_pixel(32, 7)));
    out(&[ESC, b'e']); // cursor on: one toggle
    let on = cell(0, 0);
    let want: Vec<u32> = before.iter().map(|&w| !w).collect();
    assert_eq!(on, want, "every word complemented, X included");
    assert!(on.iter().all(|&w| w >> 24 == 0xFF), "the X byte is complemented too (C4)");
    out(&[ESC, b'f']); // cursor off: the second toggle
    assert_eq!(cell(0, 0), before, "two toggles leave the cell bit for bit");
}

#[test]
fn a_clear_fills_the_background_pen_pixel_at_32() {
    let (_g, _ws) = setup(32);
    colours(FG, BG);
    out(&[ESC, b'E']);
    let px = screen().snapshot();
    assert!(px.iter().all(|&w| w == reg_pixel(32, 7)), "ESC E: every pixel the background pen pixel");
    // Reverse video does not change the clear colour (`conout.c:611`).
    out(b"\x1bp\x1bE\x1bq");
    assert!(screen().snapshot().iter().all(|&w| w == reg_pixel(32, 7)), "the background, never the foreground");
}

#[test]
fn a_scroll_moves_the_raw_words_and_blanks_the_new_row_at_32() {
    let (_g, _ws) = setup(32);
    colours(FG, BG);
    out(&[ESC, b'E']);
    // Fill the last row, then a line feed on it scrolls.
    out(&[ESC, b'Y', 32 + MY as u8, 32]);
    out(&vec![b'A'; (MX + 1) as usize]);
    out(&[ESC, b'Y', 32 + MY as u8, 32, b'\n']);
    let glyph = glyph_cell(b'A', reg_pixel(32, 8), reg_pixel(32, 7));
    for col in 0..=MX {
        assert_eq!(cell(col, MY - 1), glyph, "row {} col {col} moved up", MY - 1);
        assert!(cell(col, MY).iter().all(|&w| w == reg_pixel(32, 7)), "the new row is blank, col {col}");
    }
}

#[test]
fn at_16_planes_the_cells_are_the_16_bit_words() {
    let (_g, _ws) = setup(16);
    colours(FG, BG);
    out(b"\x1bHX\x1bH");
    let before = cell(0, 0);
    assert_eq!(before, glyph_cell(b'X', reg_pixel(16, 8), reg_pixel(16, 7)));
    // The cursor at 16 is the 16-bit word complemented (`conout.c:305-313`),
    // seen through the store; twice is the identity.
    out(&[ESC, b'e']);
    let want: Vec<u32> = before.iter().map(|&w| word_to_device(!device_to_word(w))).collect();
    assert_eq!(cell(0, 0), want);
    out(&[ESC, b'f']);
    assert_eq!(cell(0, 0), before);
}

#[test]
fn the_16_and_32_plane_consoles_agree_within_the_pen_tolerance() {
    let draw = || {
        for reg in 0..16u8 {
            out(&[ESC, b'b', b'0' + reg, ESC, b'c', b'0' + (15 - reg)]);
            out(b"X");
        }
        screen().snapshot()
    };
    let p16 = {
        let (_g, _ws) = setup(16);
        draw()
    };
    let p32 = {
        let (_g, _ws) = setup(32);
        draw()
    };
    let mut differ = 0;
    for (i, (&a, &b)) in p16.iter().zip(p32.iter()).enumerate() {
        let d = |s: u32| ((a >> s) & 0xff).abs_diff((b >> s) & 0xff);
        assert!(d(16) <= 5 && d(8) <= 5 && d(0) <= 5, "pixel {i}: 16 {a:08x} vs 32 {b:08x}");
        assert_eq!(b >> 24, 0, "X = 0 at 32");
        differ += usize::from(a != b);
    }
    assert!(differ > 0, "the greys differ between the settings, within the tolerance");
}
