//! V63 — the 32-plane device pixel: rules R1-R6 of
//! `docs/api/vdi/raster/_raster.md` § "32-plane setting: the device pixel
//! (ADR-038 point 4 and 11, V63)", pinned at the 32-plane setting, and the
//! unchanged behaviour at the default 16 planes beside it.
//!
//! - R1/R2: the device pixel is the store word `0xXXRRGGBB`; a pen writes
//!   X = 0 — fills, lines and text draw the pen's pixel.
//! - R3: `v_clrwk` writes a constant; tests assert the colour and what a
//!   masked read returns, never the X byte of a cleared pixel (TASK/V63,
//!   lead's reading, point 3).
//! - R4: logic operations are bitwise on all 32 bits, X included
//!   (`WM_XOR` complements the stored pixel and ignores the pen; the sixteen
//!   BitBlt operations of the kernel's `raster_op`/`copy_rect`).
//! - R5: every read ignores X: `v_get_pixel`, the flood fill's span probes
//!   and its seed read (TASK/V63, lead's reading, point 1); a cleared screen
//!   reads as the white pen.
//! - R6: at 16 planes none of that applies: the reads stay unmasked, a
//!   cleared screen reads `0xFFFF`, a seed fill on it draws nothing (EmuTOS
//!   `vdi/vdi_fill.c:671-681,972`).
//!
//! **The backend is the kernel's `VdiScreen` over a host store**, shared with
//! the V64 tests in `vdi_planes32_common` (the kernel's draw kernels by path,
//! the device-pixel conversions transcribed; see that module).
//!
//! Since V64 a pen's pixel at 32 planes has 8 bits per gun
//! (`_raster.md` P3), so the pen scene at 32 planes is the 16-plane one
//! within one 5-bit step per gun (P9), no longer equal to it.
//!
//! Until V65 `vro_cpyfm`/`vrt_cpyfm` refused a 32-plane screen
//! (`CHUNKY_PLANES`, `_raster.md` §4), so R4 for the sixteen operations is
//! pinned on the kernel's `raster_op` and `copy_rect` directly (the memory
//! forms of V65 are pinned in `vdi_v65_memory_forms.rs`). Since V65
//! `v_get_pixel` at 32 planes answers the high and the low word of the masked
//! pixel (`_color-pixel-input.md`, "the wide `v_get_pixel` answer"), which
//! replaced the interim 16-bit answer the V63 tests first pinned.
//!
//! Ticket: `TASK/V63.md`.

#![cfg(all(target_arch = "aarch64", any(target_os = "macos", target_os = "linux")))]
#![allow(clippy::too_many_arguments)]

mod vdi_planes32_common;

use tos_api::vdi::backend::{ClipRect, Pel, RasterBackend, Rect};
use vdi_planes32_common::*;

// ── R1, R2: a pen writes X = 0 ───────────────────────────────────────────

/// The pens this test draws with: the default sixteen and three of the
/// extended table.
const PENS: [i16; 19] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 100, 255];

/// One scene of fills, lines and text in replace mode on a cleared screen;
/// returns the visible store.
fn pen_scene(planes: u16) -> Vec<u32> {
    let _g = setting(planes);
    let mut ws = Ws::open();
    ws.v_clrwk();
    let scr = screen();
    for (i, &pen) in PENS.iter().enumerate() {
        let (x0, y0) = (4 + 16 * i as i16, 4);
        ws.solid_fill(pen);
        ws.vr_recfl(x0, y0, x0 + 11, y0 + 11);
        // The pen's pixel, X = 0 (R2): at 16 planes the pseudo-palette word
        // through the kernel's pen step, at 32 the 8-bit rule of the request
        // (V64, `_raster.md` P3, P6).
        let want = if planes == 32 {
            default_pen_pixel(32, pen as usize)
        } else {
            word_to_device(scr.pel_to_device16(pen as Pel))
        };
        for (x, y) in [(x0, y0), (x0 + 11, y0 + 11), (x0 + 5, y0 + 6)] {
            let got = scr.raw(x, y);
            assert_eq!(got, want, "{planes} planes, pen {pen}: fill pixel at ({x},{y})");
            assert_eq!(x_of(got), 0, "{planes} planes, pen {pen}: a fill writes X = 0");
        }
        // A horizontal solid line one row below.
        ws.call(17, 0, &[pen], &[]); // vsl_color
        ws.call(6, 0, &[], &[x0, 20, x0 + 11, 20]); // v_pline
        for x in x0..=x0 + 11 {
            assert_eq!(scr.raw(x, 20), want, "{planes} planes, pen {pen}: line pixel at ({x},20)");
        }
    }
    // Text in replace mode: every cell pixel is the text pen's or pen 0's
    // pixel, both X = 0; some are the text pen's.
    let before = scr.snapshot();
    ws.call(22, 0, &[2], &[]); // vst_color red
    ws.call(8, 0, &[65, 66, 67, 87], &[40, 80]); // v_gtext "ABCW"
    let (red, white) = if planes == 32 {
        (default_pen_pixel(32, 2), default_pen_pixel(32, 0))
    } else {
        (word_to_device(scr.pel_to_device16(2)), word_to_device(scr.pel_to_device16(0)))
    };
    let after = scr.snapshot();
    let changed: Vec<u32> = before.iter().zip(&after).filter(|(b, a)| b != a).map(|(_, &a)| a).collect();
    assert!(changed.iter().filter(|&&p| p == red).count() > 20, "{planes} planes: v_gtext drew no text pixels");
    for p in &changed {
        assert!(*p == red || *p == white, "{planes} planes: a text pixel 0x{p:08X} is neither pen");
        assert_eq!(x_of(*p), 0, "{planes} planes: text writes X = 0");
    }
    after
}

/// Fills, lines and text draw the pen's pixel `0x00RRGGBB` at 32 planes
/// (R1, R2). Since V64 the 32-plane pens have 8 bits per gun, so the scene
/// is the 16-plane one projected to 5 bits per gun within one step
/// (`_raster.md` P9), exactly equal where a gun is 0 or 1000 (both rules
/// give the extremes). The comparison masks X, because a cleared pixel's X
/// byte is not a contract at 32 planes (R3); every drawn pixel's X is
/// asserted above.
#[test]
fn fills_lines_and_text_draw_the_pen_pixel_with_x_zero() {
    let at16 = pen_scene(16);
    let at32 = pen_scene(32);
    assert_eq!(at16.len(), at32.len());
    for (i, (a, b)) in at16.iter().zip(&at32).enumerate() {
        let (a5, b5) = (guns5(*a), guns5(*b));
        let step = (a5.0 - b5.0).abs().max((a5.1 - b5.1).abs()).max((a5.2 - b5.2).abs());
        assert!(step <= 1, "pixel {i}: 0x{a:08X} at 16 and 0x{b:08X} at 32 differ by {step} 5-bit steps");
        let extreme = |c: u32| c == 0 || c == 0xFF;
        let (ra, ga, ba) = ((a >> 16) & 0xFF, (a >> 8) & 0xFF, a & 0xFF);
        if extreme(ra) && extreme(ga) && extreme(ba) {
            assert_eq!(a & COLOUR, b & COLOUR, "pixel {i}: a full-intensity colour differs between 16 and 32");
        }
    }
}

// ── R3, R5: a cleared screen reads as the white pen ──────────────────────

/// At 32 planes `v_clrwk` shows white and reads as the pixel of the white
/// pen through `v_get_pixel` (R3, R5; the wide answer of V65, `0x00FF,
/// 0xFFFF`, replaced V63's interim `0, 0xFFDF`), equal to a pen-0 pixel.
/// At 16 planes the answer stays EmuTOS's `0xFFFF`, one `OVERLAY_BIT` apart
/// from pen 0 (R6, `vdi/vdi_fill.c:671-681`).
#[test]
fn a_cleared_screen_reads_as_the_white_pen_at_32_planes() {
    for planes in [32u16, 16] {
        let _g = setting(planes);
        let mut ws = Ws::open();
        ws.v_clrwk();
        let scr = screen();
        for (x, y) in [(0, 0), (5, 5), (W as i16 - 1, H as i16 - 1)] {
            assert_eq!(scr.raw(x, y) & COLOUR, WHITE_PIXEL, "{planes} planes: the cleared colour at ({x},{y})");
        }
        ws.solid_fill(0);
        ws.vr_recfl(100, 100, 109, 109);
        assert_eq!(word_to_device(scr.pel_to_device16(0)), WHITE_PIXEL, "pen 0 is the white pen");
        let pen0 = ws.v_get_pixel(105, 105);
        let want = if planes == 32 { ((WHITE_PIXEL >> 16) as i16, WHITE_PIXEL as u16) } else { (0, WHITE_WORD) };
        assert_eq!(pen0, want, "{planes} planes: v_get_pixel of a pen-0 pixel");
        for (x, y) in [(0, 0), (5, 5), (W as i16 - 1, H as i16 - 1)] {
            let cleared = ws.v_get_pixel(x, y);
            if planes == 32 {
                assert_eq!(cleared, pen0, "32 planes: a cleared pixel at ({x},{y}) reads as the white pen");
            } else {
                assert_eq!(cleared, (0, CLEARED_WORD_16), "16 planes: v_get_pixel after v_clrwk (EmuTOS)");
                assert_eq!(cleared.1 ^ pen0.1, OVERLAY_BIT, "16 planes: cleared and pen 0 differ by OVERLAY_BIT");
            }
        }
    }
}

// ── R5: every read ignores X ─────────────────────────────────────────────

/// `v_get_pixel` at 32 planes answers for `X | colour` what it answers for
/// the colour with X = 0, for every X (R5): the high and the low word of the
/// colour (V65). At 16 planes the top bit of X
/// still comes back as `OVERLAY_BIT` (R6, the unmasked read).
#[test]
fn v_get_pixel_ignores_the_x_byte_at_32_planes() {
    let colours = [0x00FF_FFFFu32, 0x0000_0000, 0x00FF_0000, 0x0000_FF00, 0x0000_00FF, 0x00BD_BDBD, 0x0012_3456];
    let xs = [0x00u32, 0x01, 0x20, 0x5A, 0x7F, 0x80, 0xA5, 0xFF];
    for planes in [32u16, 16] {
        let _g = setting(planes);
        let mut ws = Ws::open();
        let scr = screen();
        for &c in &colours {
            scr.put_device_pixel(10, 10, c);
            let plain = ws.v_get_pixel(10, 10);
            let want = if planes == 32 { ((c >> 16) as i16, c as u16) } else { (0, device_to_word(c)) };
            assert_eq!(plain, want, "{planes} planes: v_get_pixel of 0x{c:08X}");
            for &x in &xs {
                scr.put_device_pixel(10, 10, x << 24 | c);
                let got = ws.v_get_pixel(10, 10);
                if planes == 32 {
                    assert_eq!(got, plain, "32 planes: X 0x{x:02X} over 0x{c:06X} changed the read");
                } else {
                    let overlay = if x & 0x80 != 0 { OVERLAY_BIT } else { 0 };
                    assert_eq!(got, (0, plain.1 | overlay), "16 planes: X 0x{x:02X} over 0x{c:06X}");
                }
            }
        }
    }
}

/// A white region whose pixels carry random X noise, inside a black frame,
/// on a cleared screen; the seed pixel's X has its top bit set. Returns the
/// region's inclusive box.
fn noisy_region(ws: &mut Ws, rng: &mut Rng) -> (i16, i16, i16, i16) {
    ws.v_clrwk();
    let scr = screen();
    let (x0, y0, x1, y1) = (40i16, 30i16, 99i16, 69i16);
    let black = word_to_device(scr.pel_to_device16(1));
    for y in y0 - 1..=y1 + 1 {
        for x in x0 - 1..=x1 + 1 {
            let inside = (x0..=x1).contains(&x) && (y0..=y1).contains(&y);
            scr.put_device_pixel(x, y, if inside { rng.x_byte() | WHITE_PIXEL } else { black });
        }
    }
    scr.put_device_pixel(60, 50, 0xA5 << 24 | WHITE_PIXEL);
    (x0, y0, x1, y1)
}

/// The flood fill at 32 planes ignores X noise in the span probes and in
/// the seed read (R5, lead's reading point 1): a fill-while-same from a
/// noisy seed and a fill up to the black boundary both fill the whole
/// region with the fill pen's pixel and nothing beyond the frame. At 16
/// planes the seed read stays unmasked as in EmuTOS (`vdi_fill.c:972`): a
/// seed whose X top bit is set never equals a masked span colour, so the
/// fill-while-same draws nothing, while the boundary fill (pen colour vs.
/// masked spans) fills (R6).
#[test]
fn the_flood_fill_ignores_x_noise_at_32_planes() {
    for planes in [32u16, 16] {
        for (intin0, name) in [(-1i16, "fill while same"), (1, "fill to the black boundary")] {
            let _g = setting(planes);
            let mut ws = Ws::open();
            let mut rng = Rng(0x9E37_79B9_7F4A_7C15 ^ planes as u64);
            let (x0, y0, x1, y1) = noisy_region(&mut ws, &mut rng);
            let scr = screen();
            let before = scr.snapshot();
            ws.vswr_mode(1);
            ws.solid_fill(2);
            ws.v_contourfill(60, 50, intin0);
            let red = word_to_device(scr.pel_to_device16(2));
            let after = scr.snapshot();
            let inside = |i: usize| {
                let (x, y) = ((i % W as usize) as i16, (i / W as usize) as i16);
                (x0..=x1).contains(&x) && (y0..=y1).contains(&y)
            };
            let fills = planes == 32 || intin0 >= 0;
            for (i, (b, a)) in before.iter().zip(&after).enumerate() {
                if !inside(i) {
                    assert_eq!(a, b, "{planes} planes, {name}: pixel {i} outside the region changed");
                } else if fills {
                    assert_eq!(*a, red, "{planes} planes, {name}: pixel {i} of the region not filled");
                } else {
                    assert_eq!(a, b, "{planes} planes, {name}: the unmasked seed must fill nothing");
                }
            }
        }
    }
}

/// A fill-while-same seeded on a freshly cleared screen fills the clip
/// rectangle at 32 planes (the masked seed reads as the white pen) and
/// draws nothing at 16 planes (EmuTOS: the unmasked seed `0xFFFF` never
/// equals a masked span `0xFFDF`, `vdi_fill.c:755,763,972`).
#[test]
fn a_seed_fill_on_a_cleared_screen_fills_at_32_and_draws_nothing_at_16() {
    for planes in [32u16, 16] {
        let _g = setting(planes);
        let mut ws = Ws::open();
        ws.v_clrwk();
        let scr = screen();
        ws.vs_clip(true, 10, 10, 49, 29);
        ws.vswr_mode(1);
        ws.solid_fill(2);
        let before = scr.snapshot();
        ws.v_contourfill(20, 20, -1);
        ws.vs_clip(false, 0, 0, 0, 0);
        let red = word_to_device(scr.pel_to_device16(2));
        let after = scr.snapshot();
        for (i, (b, a)) in before.iter().zip(&after).enumerate() {
            let (x, y) = ((i % W as usize) as i16, (i / W as usize) as i16);
            let in_clip = (10..=49).contains(&x) && (10..=29).contains(&y);
            if planes == 32 && in_clip {
                assert_eq!(*a, red, "32 planes: ({x},{y}) in the clip not filled");
            } else {
                assert_eq!(a, b, "{planes} planes: ({x},{y}) changed");
            }
        }
    }
}

// ── R4: logic operations are bitwise on all 32 bits ──────────────────────

/// `WM_XOR` complements the whole stored pixel, X included, and ignores the
/// pen (R4; EmuTOS `vdi_line.c:494,501`): a solid fill and a solid line over
/// pixels with X noise, at both settings; a second pass restores the store.
#[test]
fn wm_xor_complements_all_32_bits_and_ignores_the_pen() {
    for planes in [32u16, 16] {
        let _g = setting(planes);
        let mut ws = Ws::open();
        let scr = screen();
        let mut rng = Rng(0x0123_4567_89AB_CDEF);
        for y in 0..H as i16 {
            for x in 0..W as i16 {
                scr.put_device_pixel(x, y, rng.u32());
            }
        }
        let start = scr.snapshot();
        ws.vswr_mode(3);
        ws.solid_fill(2);
        ws.vr_recfl(20, 20, 59, 39);
        ws.call(17, 0, &[5], &[]); // vsl_color, ignored by XOR
        ws.call(6, 0, &[], &[100, 50, 179, 50]); // v_pline, solid
        let once = scr.snapshot();
        for (i, (s, o)) in start.iter().zip(&once).enumerate() {
            let (x, y) = ((i % W as usize) as i16, (i / W as usize) as i16);
            let in_fill = (20..=59).contains(&x) && (20..=39).contains(&y);
            let on_line = (100..=179).contains(&x) && y == 50;
            if in_fill || on_line {
                assert_eq!(*o, !s, "{planes} planes: ({x},{y}) 0x{s:08X} not complemented on 32 bits");
            } else {
                assert_eq!(o, s, "{planes} planes: ({x},{y}) outside the XOR changed");
            }
        }
        ws.vr_recfl(20, 20, 59, 39);
        ws.call(6, 0, &[], &[100, 50, 179, 50]);
        assert_eq!(scr.snapshot(), start, "{planes} planes: XOR twice does not restore the store");
        ws.vswr_mode(1);
    }
}

/// The independent definition of the sixteen BitBlt operations on a 32-bit
/// pixel (`_raster.md` R4: the bitwise table on 32 bits; EmuTOS's on 16,
/// `vdi/vdi_raster.c:1007-1215`, `ALL_BLACK` writes all ones `:1213`).
fn bitblt(op: u8, s: u32, d: u32) -> u32 {
    let mut r = 0u32;
    for bit in 0..32 {
        let (sb, db) = ((s >> bit) & 1, (d >> bit) & 1);
        // op bit 3 = f(0,0), bit 2 = f(0,1), bit 1 = f(1,0), bit 0 = f(1,1)
        let idx = match (sb, db) {
            (0, 0) => 3,
            (0, 1) => 2,
            (1, 0) => 1,
            _ => 0,
        };
        r |= ((op as u32 >> idx) & 1) << bit;
    }
    r
}

/// The kernel's `raster_op` and its screen-to-screen `copy_rect` (the
/// `vro_cpyfm` and window-move path) act bitwise on all 32 bits, X included
/// (R4): every one of the sixteen operations on sources and destinations
/// with X noise; `ALL_BLACK` writes `0xFFFFFFFF`, `ALL_WHITE` 0.
#[test]
fn the_sixteen_logic_operations_are_bitwise_on_32_bits() {
    let mut rng = Rng(0xDEAD_BEEF_F00D_1234);
    for op in 0u8..16 {
        for _ in 0..64 {
            let (s, d) = (rng.u32(), rng.u32());
            assert_eq!(raster_copy::raster_op(op, s, d), bitblt(op, s, d), "raster_op {op} on 0x{s:08X}, 0x{d:08X}");
        }
    }
    assert_eq!(raster_copy::raster_op(15, 0x1234_5678, 0x00AB_CDEF), 0xFFFF_FFFF);
    assert_eq!(raster_copy::raster_op(0, 0x1234_5678, 0xFFAB_CDEF), 0);

    let _g = setting(32);
    let scr = screen();
    for op in 0u8..16 {
        for y in 0..H as i16 {
            for x in 0..W as i16 {
                scr.put_device_pixel(x, y, rng.u32());
            }
        }
        let start = scr.snapshot();
        let at = |x: i16, y: i16| start[y as usize * W as usize + x as usize];
        scr.copy_rect(Rect { x: 10, y: 10, w: 37, h: 9 }, 120, 60, op, ClipRect::full(W, H));
        let end = scr.snapshot();
        for y in 0..H as i16 {
            for x in 0..W as i16 {
                let in_dst = (120..157).contains(&x) && (60..69).contains(&y);
                let want = if in_dst { bitblt(op, at(x - 110, y - 50), at(x, y)) } else { at(x, y) };
                assert_eq!(end[y as usize * W as usize + x as usize], want, "copy_rect op {op} at ({x},{y})");
            }
        }
    }
}
