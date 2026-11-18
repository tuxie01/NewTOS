//! A46 — "nicelines": a `DISABLED` `G_STRING` made of nothing but dashes is
//! a menu separator, and `just_draw` draws it as a **line**, not as text.
//!
//! EmuTOS `just_draw`, string block (`aes/gemoblib.c:833-844`, under
//! `CONF_WITH_NICELINES`, default 1 in `include/config.h:1312-1314`):
//!
//! ```c
//! if ((obtype == G_STRING) && (state & DISABLED) && (len > 3) && is_dashes((char *)spec, len))
//!     gsx_cline(t.g_x, t.g_y+t.g_h/2, t.g_x+t.g_w-1, t.g_y+t.g_h/2);
//! else
//!     gsx_tblt(IBM, tmpx, tmpy, len);
//! ```
//!
//! `is_dashes` (`:455-464`) is true when every one of the `len = strlen`
//! (`expand_string`, `util/optimopt.S:41-47`) characters is `'-'`. The line
//! runs the whole **object** width (`t.g_w`, not the text extent) on the row
//! `t.g_y + t.g_h/2`, one pixel wide, solid (`vsl_type 7`/`vsl_udsty 0xffff`,
//! `gemgraf.c:420-422`), in the writing mode the string block just set
//! (`gsx_attr(TRUE, MD_TRANS, tcol)`, `:816`). It is drawn *instead of* the
//! text, but *before* the state decorations — so the trailing `DISABLED`
//! pass (`bb_fill(MD_TRANS, FIS_PATTERN, IP_4PATT, ...)`, `:917-929`) lays
//! the 50 % white dither over the line exactly as it would over the dashes.
//!
//! The `len > 3` gate is the **pinned source's** (`1a7484ab`): it came with
//! `4b0d9518` (2025-08-20), after the EmuTOS 1.4 tag. EmuTOS 1.4 — the
//! measured `etos512de.img` — draws one to three dashes as a line too; NewTOS
//! follows the pinned source, so `"---"` stays text here.
//!
//! Every test renders through the real `objc_draw` body (`render_tree_lp64`)
//! into a pen-indexed framebuffer. The 8×16 IBM hyphen (`font_data.rs`,
//! glyph `0x2D`) has ink only in cell rows 6 and 7 (`0x7E`), so for a 16-tall
//! object the two outcomes never share a row: text puts ink on `y+6`/`y+7`,
//! the line on `y+8`.

use std::sync::Mutex;

use tos_api::aes::object::{render_tree_lp64, Object};
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};

const SCREEN_W: u16 = 320;
const SCREEN_H: u16 = 200;
/// Pen value no drawing operation in these tests ever writes, so "untouched"
/// is distinguishable from both WHITE (0) and BLACK (1).
const UNTOUCHED: Pel = 15;

// ── a pen-indexed framebuffer ─────────────────────────────────────────────

struct PenBackend {
    px: Mutex<Vec<Pel>>,
}

impl PenBackend {
    fn new() -> PenBackend {
        PenBackend { px: Mutex::new(vec![UNTOUCHED; SCREEN_W as usize * SCREEN_H as usize]) }
    }

    fn inside(x: i16, y: i16) -> bool {
        x >= 0 && y >= 0 && (x as i32) < SCREEN_W as i32 && (y as i32) < SCREEN_H as i32
    }

    fn put(&self, x: i16, y: i16, pel: Pel, clip: ClipRect) {
        if !Self::inside(x, y) {
            return;
        }
        if x < clip.xmn || x > clip.xmx || y < clip.ymn || y > clip.ymx {
            return;
        }
        self.px.lock().unwrap()[y as usize * SCREEN_W as usize + x as usize] = pel;
    }

    fn get(&self, x: i16, y: i16) -> Pel {
        if !Self::inside(x, y) {
            return UNTOUCHED;
        }
        self.px.lock().unwrap()[y as usize * SCREEN_W as usize + x as usize]
    }

    /// One screen row over `x0..=x1` as a class string: `#` black, `.` white,
    /// `_` untouched, `?` anything else.
    fn row(&self, y: i16, x0: i16, x1: i16) -> String {
        (x0..=x1)
            .map(|x| match self.get(x, y) {
                BLACK => '#',
                WHITE => '.',
                UNTOUCHED => '_',
                _ => '?',
            })
            .collect()
    }

    /// Number of `pen` pixels inside `(x, y, w, h)`.
    fn count(&self, x: i16, y: i16, w: i16, h: i16, pen: Pel) -> usize {
        let mut n = 0;
        for yy in y..y + h {
            for xx in x..x + w {
                if self.get(xx, yy) == pen {
                    n += 1;
                }
            }
        }
        n
    }
}

impl RasterBackend for PenBackend {
    fn width(&self) -> u16 {
        SCREEN_W
    }
    fn height(&self) -> u16 {
        SCREEN_H
    }
    fn num_colors(&self) -> u16 {
        16
    }
    fn planes(&self) -> u16 {
        4
    }
    fn set_color(&self, _index: u16, _r: u16, _g: u16, _b: u16) {}
    fn query_color(&self, _index: u16) -> (u16, u16, u16) {
        (0, 0, 0)
    }
    fn set_active_palette(&self, _palette: &[u32; 256]) {}
    fn pel_to_device16(&self, pel: Pel) -> u16 {
        pel as u16
    }
    fn device_to_rgb565(&self, argb: u32) -> u16 {
        argb as u16
    }
    fn rgb565_to_device(&self, px: u16) -> u32 {
        px as u32
    }
    fn read_pixel(&self, x: i16, y: i16) -> Pel {
        self.get(x, y)
    }
    fn fill_rect(&self, x: i16, y: i16, w: i16, h: i16, pel: Pel, clip: ClipRect) {
        for yy in y..y.saturating_add(h) {
            for xx in x..x.saturating_add(w) {
                self.put(xx, yy, pel, clip);
            }
        }
    }
    fn abline(
        &self,
        x0: i16,
        y0: i16,
        x1: i16,
        y1: i16,
        pel: Pel,
        _mode: WriteMode,
        clip: ClipRect,
        mask: u16,
        _last: bool,
    ) -> u16 {
        // Axis-parallel lines only, with the line mask applied per column /
        // row exactly like the product backend (bit `0x8000 >> (x & 0xF)`).
        assert!(x0 == x1 || y0 == y1, "these tests only draw axis-parallel lines");
        for yy in y0.min(y1)..=y0.max(y1) {
            for xx in x0.min(x1)..=x0.max(x1) {
                let i = if y0 == y1 { xx } else { yy };
                if mask & (0x8000u16 >> (i.rem_euclid(16) as u16)) != 0 {
                    self.put(xx, yy, pel, clip);
                }
            }
        }
        mask
    }
    fn fill_pattern(&self, x: i16, y: i16, w: i16, h: i16, fill: PatternFill, clip: ClipRect) {
        for yy in y..y.saturating_add(h) {
            let row = fill.pattern[(yy.rem_euclid(fill.pattern.len() as i16)) as usize];
            for xx in x..x.saturating_add(w) {
                let set = row & (0x8000u16 >> (xx.rem_euclid(16) as u16)) != 0;
                if set {
                    self.put(xx, yy, fill.fg, clip);
                } else if fill.mode == WriteMode::Replace {
                    self.put(xx, yy, fill.bg, clip);
                }
            }
        }
    }
    fn draw_glyph_sized(
        &self,
        x: i16,
        y: i16,
        glyph: &[u8],
        cell_w: i16,
        cell_h: i16,
        fg: Pel,
        _mode: WriteMode,
        clip: ClipRect,
    ) {
        assert!(cell_w <= 8, "these tests only use single-byte-per-row cells");
        for row in 0..cell_h {
            let byte = glyph[row as usize];
            for col in 0..cell_w {
                if byte & (0x80u8 >> col) != 0 {
                    self.put(x + col, y + row, fg, clip);
                }
            }
        }
    }
    fn blit_mono(&self, dst: Rect, src: MonoSrc, fg: Pel, bg: Pel, mode: WriteMode, clip: ClipRect) {
        for row in 0..dst.h {
            for col in 0..dst.w {
                let (sx, sy) = (src.x + col, src.y + row);
                let wi = (sy as usize) * src.stride_words as usize + (sx as usize) / 16;
                let w = src.bits.word(wi).unwrap_or(0);
                let set = w & (0x8000u16 >> (sx.rem_euclid(16)) as u16) != 0;
                if set {
                    self.put(dst.x + col, dst.y + row, fg, clip);
                } else if mode != WriteMode::Transparent {
                    self.put(dst.x + col, dst.y + row, bg, clip);
                }
            }
        }
    }
    fn copy_rect(&self, _src: Rect, _dx: i16, _dy: i16, _op: u8, _clip: ClipRect) {}
    fn get_device_pixel(&self, x: i16, y: i16) -> u32 {
        self.get(x, y) as u32
    }
    fn put_device_pixel(&self, x: i16, y: i16, argb: u32) {
        self.put(x, y, argb as Pel, ClipRect::full(SCREEN_W, SCREEN_H));
    }
    fn flush_rect(&self, _x0: i16, _y0: i16, _x1: i16, _y1: i16) {}
}

// SAFETY: the only mutable state is behind a Mutex.
unsafe impl Sync for PenBackend {}
unsafe impl Send for PenBackend {}

// ── the object ────────────────────────────────────────────────────────────

const NIL: i16 = -1;
const MAX_DEPTH: i16 = 8;
const G_STRING: u16 = 28;
const LASTOB: u16 = 0x0020;
const DISABLED: u16 = 0x0008;

const WHITE: Pel = 0;
const BLACK: Pel = 1;

/// A menu-item-shaped `G_STRING`: 16 tall (one IBM cell), and wider than
/// its text like every drop-down entry (the box is sized to the longest
/// item), so a full-width line and a text-wide row of dashes are tellable
/// apart by their extent alone.
const X: i16 = 40;
const Y: i16 = 50;
const W: i16 = 120;
const H: i16 = 16;

fn string(text: &'static [u8], state: u16) -> [Object; 1] {
    [Object {
        ob_next: NIL,
        ob_head: NIL,
        ob_tail: NIL,
        ob_type: G_STRING,
        ob_flags: LASTOB,
        ob_state: state,
        ob_spec: text.as_ptr() as u64,
        ob_x: X,
        ob_y: Y,
        ob_width: W,
        ob_height: H,
    }]
}

/// Render the one-object tree unclipped through the `objc_draw` body.
fn render(tree: &[Object; 1]) -> PenBackend {
    let fb = PenBackend::new();
    // SAFETY: `tree` is a live, well-formed native (LP64) OBJECT array whose
    // root index 0 is in range for the whole call.
    unsafe { render_tree_lp64(tree.as_ptr(), 0, MAX_DEPTH, 0, 0, 0, 0, &fb) };
    fb
}

/// The row the line goes on: `t.g_y + t.g_h/2`.
const LINE_ROW: i16 = Y + H / 2;

/// What the `DISABLED` dither leaves of a BLACK row at `y` over `x0..=x1`:
/// `IP_4PATT` is the 50 % checkerboard (`0xAAAA`/`0x5555`, screen-aligned),
/// painted WHITE in transparent mode — set bits turn white, the others keep
/// the line.
fn dithered_line(y: i16, x0: i16, x1: i16) -> String {
    let pat: u16 = if y % 2 == 0 { 0xAAAA } else { 0x5555 };
    (x0..=x1)
        .map(|x| if pat & (0x8000u16 >> (x.rem_euclid(16) as u16)) != 0 { '.' } else { '#' })
        .collect()
}

/// Ink rows of the IBM hyphen for a string starting at `(X, tmpy)` with
/// `tmpy = Y + (H - 16)/2 = Y`: cell rows 6 and 7, columns 1..=6 of each cell.
fn text_rows() -> (i16, i16) {
    (Y + 6, Y + 7)
}

// ── (a) the separator ─────────────────────────────────────────────────────

/// A `DISABLED` string of eight dashes is drawn as a one-pixel line across
/// the **whole object width** on row `Y + H/2`, under the DISABLED dither;
/// no hyphen glyph is drawn (rows `Y+6`/`Y+7` carry no ink).
#[test]
fn disabled_all_dashes_string_is_a_line_across_the_object() {
    let fb = render(&string(b"--------\0", DISABLED));

    // The line row: black wherever the dither did not paint white, from X
    // to X+W-1 inclusive — 120 pixels, 56 pixels past the text's 64.
    assert_eq!(
        fb.row(LINE_ROW, X, X + W - 1),
        dithered_line(LINE_ROW, X, X + W - 1),
        "line row {LINE_ROW}: expected the full-width line under the 50 % dither"
    );
    // The line stops at the object's edges (`t.g_x` .. `t.g_x+t.g_w-1`).
    assert_eq!(fb.get(X - 1, LINE_ROW), UNTOUCHED, "pixel left of the object");
    assert_eq!(fb.get(X + W, LINE_ROW), UNTOUCHED, "pixel right of the object");

    // No text: the hyphen rows are dither only (white / untouched), and the
    // line row is the only row inside the object with any ink at all.
    let (r6, r7) = text_rows();
    assert_eq!(fb.count(X, r6, W, 1, BLACK), 0, "glyph row {r6} must carry no ink");
    assert_eq!(fb.count(X, r7, W, 1, BLACK), 0, "glyph row {r7} must carry no ink");
    let ink_on_line = fb.count(X, LINE_ROW, W, 1, BLACK);
    assert_eq!(ink_on_line, (W / 2) as usize, "half the line survives the 50 % dither");
    assert_eq!(fb.count(X, Y, W, H, BLACK), ink_on_line, "no ink anywhere but the line row");
}

// ── (b) the pinned source's `len > 3` gate ────────────────────────────────

/// Three dashes are **not** a separator in the pinned source (`len > 3`,
/// `4b0d9518`): the string is drawn as text. (EmuTOS 1.4 differs — see the
/// module doc.)
#[test]
fn disabled_three_dashes_stay_text() {
    let fb = render(&string(b"---\0", DISABLED));
    assert_text_not_line(&fb, 3, "DISABLED \"---\"");
}

// ── (c) not DISABLED ──────────────────────────────────────────────────────

/// Dashes in an enabled string are just dashes: no line, glyphs drawn.
#[test]
fn enabled_all_dashes_string_stays_text() {
    let fb = render(&string(b"--------\0", 0));
    assert_text_not_line(&fb, 8, "enabled \"--------\"");
    // Enabled: no dither either, so the glyph pixels (transparent, on an
    // untouched ground — a G_STRING has no fill) are all that is there.
    let (r6, r7) = text_rows();
    assert_eq!(fb.row(r6, X, X + 15), "_######__######_", "hyphen glyph row {r6}");
    assert_eq!(fb.row(r7, X, X + 15), "_######__######_", "hyphen glyph row {r7}");
    assert_eq!(fb.count(X, Y, W, H, WHITE), 0, "an enabled string is not dithered");
}

// ── (d) not all dashes ────────────────────────────────────────────────────

/// A `DISABLED` string long enough but with a non-dash in it (`"-- -"`) is
/// text: `is_dashes` rejects the blank.
#[test]
fn disabled_string_with_a_non_dash_stays_text() {
    let fb = render(&string(b"-- -\0", DISABLED));
    assert_text_not_line(&fb, 4, "DISABLED \"-- -\"");
}

/// Text was drawn: the hyphen rows carry ink inside the `len` cells, nothing
/// is drawn past the text extent, and the line row is ink-free.
fn assert_text_not_line(fb: &PenBackend, len: i16, what: &str) {
    let (r6, r7) = text_rows();
    let text_w = len * 8;
    assert!(fb.count(X, r6, text_w, 1, BLACK) > 0, "{what}: hyphen row {r6} must carry ink");
    assert!(fb.count(X, r7, text_w, 1, BLACK) > 0, "{what}: hyphen row {r7} must carry ink");
    assert_eq!(
        fb.count(X + text_w, Y, W - text_w, H, BLACK),
        0,
        "{what}: nothing may be drawn past the text extent (a line would be)"
    );
    assert_eq!(
        fb.count(X, LINE_ROW, W, 1, BLACK),
        0,
        "{what}: the line row {LINE_ROW} must be ink-free\n  got |{}|",
        fb.row(LINE_ROW, X, X + W - 1)
    );
}
