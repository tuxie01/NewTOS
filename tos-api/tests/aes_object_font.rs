//! Which font the AES Object Manager draws with, and where it puts the text.
//!
//! EmuTOS keeps **two** desktop font sizes open at once (`gsx_start`,
//! `aes/gemgraf.c:405-410`): the normal one (`gl_wchar` × `gl_hchar` = 8×16
//! here) and the small one (`gl_wschar` × `gl_hschar` = 6×6, selected by
//! `vst_height(SIZ_TAB[1])`). Every text-drawing site in the Object Manager
//! names one of them:
//!
//! | site | EmuTOS | font |
//! |------|--------|------|
//! | icon overlay character | `gemgraf.c:802` | `SMALL` |
//! | icon label | `gemgraf.c:806` | `SMALL` |
//! | `G_TEXT`/`G_BOXTEXT`/`G_FTEXT`/`G_FBOXTEXT` | `gemoblib.c:757` | `TEDINFO.te_font` |
//! | `G_BOXCHAR` | `gemoblib.c:729` | `IBM` (forced) |
//! | `G_STRING`/`G_TITLE`/`G_BUTTON` | `gemoblib.c:845` | `IBM` |
//! | `CHECKED` mark | `gemoblib.c:907` | `IBM` |
//!
//! The size is a contract with the *resource*: a GEM icon resource sizes its
//! `ib_wtext`/`ib_htext` label box for the small font. EmuTOS's own
//! `desk/icon.rsc` and TeraDesk's `icons.rsc` both carry
//! `ib_wtext = 12 * 6 = 72`, `ib_htext = 6 + 2 = 8` — an 8×16 label does not
//! fit either axis of that box.
//!
//! The second half is *position*: `gr_gtext` (`gemgraf.c:595`) does not just
//! centre, it first clamps the text extent to the box and drops whole
//! characters that do not fit (`gsx_tcalc`, `gemgraf.c:480-497`), then rounds
//! each half-margin **up** (`gr_just`, `gemgraf.c:567-589`). A box shorter than
//! one character cell yields `numchs = 0` — nothing is drawn at all.
//!
//! Every test here renders through the real `objc_draw` body
//! (`render_tree_lp64`) into a pen-indexed framebuffer and compares the pixels
//! against the glyph rasters, so it fails on a font change *and* on a
//! placement change.

use std::sync::Mutex;

use tos_api::aes::object::{render_tree_lp64, IconBlk, Object, TedInfo};
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};
use tos_api::vdi::font;

const SCREEN_W: u16 = 320;
const SCREEN_H: u16 = 200;
/// Pen value no drawing operation in these tests ever writes, so "untouched"
/// is distinguishable from both WHITE (0) and BLACK (1).
const UNTOUCHED: Pel = 15;

// ── a pen-indexed framebuffer ─────────────────────────────────────────────
//
// Storing the VDI pen directly (rather than an RGB expansion) keeps the
// assertions about *which* pen landed where, which is what the object
// renderer decides; colour conversion is the board backend's business.

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

    /// Bounding box of every pixel in `(x, y, w, h)` that carries `pen`,
    /// as `(x0, y0, x1, y1)`; `None` when there is none.
    fn ink_bbox(&self, x: i16, y: i16, w: i16, h: i16, pen: Pel) -> Option<(i16, i16, i16, i16)> {
        let mut bb: Option<(i16, i16, i16, i16)> = None;
        for yy in y..y + h {
            for xx in x..x + w {
                if self.get(xx, yy) == pen {
                    bb = Some(match bb {
                        None => (xx, yy, xx, yy),
                        Some((x0, y0, x1, y1)) => (x0.min(xx), y0.min(yy), x1.max(xx), y1.max(yy)),
                    });
                }
            }
        }
        bb
    }

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
        // Straight-line only; the object renderer draws nothing else here.
        for yy in y0.min(y1)..=y0.max(y1) {
            for xx in x0.min(x1)..=x0.max(x1) {
                self.put(xx, yy, pel, clip);
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

// ── object-tree scaffolding ───────────────────────────────────────────────

const NIL: i16 = -1;
const ROOT: i16 = 0;
const MAX_DEPTH: i16 = 8;

const G_BOX: u16 = 20;
const G_TEXT: u16 = 21;
const G_STRING: u16 = 28;
const G_ICON: u16 = 31;

const WHITE: Pel = 0;
const BLACK: Pel = 1;

/// An all-zero 32×32 mask/data pair: the icon bitmap blits nothing, so the
/// only ink inside an icon's rectangle is what the *text* paths put there.
static BLANK_ICON: [u16; 64] = [0; 64];

fn root_box(w: i16, h: i16, child: i16) -> Object {
    Object {
        ob_next: NIL,
        ob_head: child,
        ob_tail: child,
        ob_type: G_BOX,
        ob_flags: 0,
        ob_state: 0,
        // IP_HOLLOW / white interior / no border: a plain white ground.
        ob_spec: 0x0000_0000,
        ob_x: 0,
        ob_y: 0,
        ob_width: w,
        ob_height: h,
    }
}

fn child(ob_type: u16, spec: u64, x: i16, y: i16, w: i16, h: i16) -> Object {
    Object {
        ob_next: ROOT,
        ob_head: NIL,
        ob_tail: NIL,
        ob_type,
        ob_flags: 0,
        ob_state: 0,
        ob_spec: spec,
        ob_x: x,
        ob_y: y,
        ob_width: w,
        ob_height: h,
    }
}

/// Render a two-object tree (white ground + one child) unclipped.
fn render(tree: &[Object; 2]) -> PenBackend {
    let fb = PenBackend::new();
    // SAFETY: `tree` is a live, well-formed native (LP64) OBJECT array whose
    // ROOT index is in range for the whole call.
    unsafe { render_tree_lp64(tree.as_ptr(), ROOT, MAX_DEPTH, 0, 0, 0, 0, &fb) };
    fb
}

/// An `ICONBLK` with the geometry every stock GEM icon resource uses:
/// a 32×32 bitmap centred in a 72-pixel-wide cell, and a label box directly
/// below it that is `12 * gl_wschar = 72` wide and `gl_hschar + 2 = 8` tall
/// (`references/emutos/desk/icon.rsc`; `sdk/apps/teradesk/icons.rsc` is
/// identical). `ib_xchar`/`ib_ychar` are relative to the **bitmap**.
fn stock_iconblk(text: &[u8], ch: u8) -> IconBlk {
    IconBlk {
        ib_pmask: BLANK_ICON.as_ptr() as u64,
        ib_pdata: BLANK_ICON.as_ptr() as u64,
        ib_ptext: text.as_ptr() as u64,
        // fg = BLACK (1), bg = WHITE (0), low byte = the overlay character.
        ib_char: 0x1000 | ch as i16,
        ib_xchar: 13,
        ib_ychar: 16,
        ib_xicon: 20,
        ib_yicon: 0,
        ib_wicon: 32,
        ib_hicon: 32,
        ib_xtext: 0,
        ib_ytext: 32,
        ib_wtext: 72,
        ib_htext: 8,
    }
}

/// Assert that `text` is rendered pixel-for-pixel from the 6×6 raster with
/// the first cell's top-left at `(x, y)`, and that nothing else in
/// `(bx, by, bw, bh)` carries ink.
fn assert_small_text_at(fb: &PenBackend, text: &[u8], x: i16, y: i16, region: (i16, i16, i16, i16)) {
    let (bx, by, bw, bh) = region;
    let mut expected = 0usize;
    for (i, &c) in text.iter().enumerate() {
        let g = font::glyph_small(c as u16);
        for (row, &bits) in g.iter().enumerate() {
            for col in 0..font::SMALL_CELL_W {
                let want_ink = bits & (0x80u8 >> col) != 0;
                let px = x + i as i16 * font::SMALL_CELL_W + col;
                let py = y + row as i16;
                if want_ink {
                    expected += 1;
                    assert_eq!(
                        fb.get(px, py),
                        BLACK,
                        "expected 6x6 ink at ({px},{py}) for {:?} char {i}",
                        core::str::from_utf8(text).unwrap_or("?")
                    );
                }
            }
        }
    }
    assert_eq!(
        fb.count(bx, by, bw, bh, BLACK),
        expected,
        "ink outside the expected 6x6 glyph pixels"
    );
}

/// One raster row of a glyph in the face `te_font` names.
fn glyph_row(te_font: i16, c: u8, row: i16) -> u8 {
    if te_font == 5 {
        font::glyph_small(c as u16)[row as usize]
    } else {
        font::glyph(c as u16)[row as usize]
    }
}

// ── the tests ─────────────────────────────────────────────────────────────

/// `gr_gicon` draws the label with `gr_gtext(TE_CNTR, SMALL, …)`
/// (`gemgraf.c:806`). An 11-character name in the stock 72×8 box therefore
/// occupies 66 px, centred with the half-margin rounded up
/// (`gr_just`: `(72-66+1)/2 = 3`), and 6 of the 8 rows, likewise rounded up
/// (`(8-6+1)/2 = 1`).
#[test]
fn icon_label_is_drawn_in_the_small_font_and_fits_its_box() {
    let label = b"KERNEL8.IMG\0"; // 11 characters
    let ib = stock_iconblk(label, 0);
    let tree =
        [root_box(160, 80, 1), child(G_ICON, &ib as *const IconBlk as u64, 10, 4, 72, 40)];
    let fb = render(&tree);

    // Label box in absolute coordinates.
    let (bx, by) = (10 + ib.ib_xtext, 4 + ib.ib_ytext);
    let tx = bx + (ib.ib_wtext - 11 * font::SMALL_CELL_W + 1) / 2; // 3
    let ty = by + (ib.ib_htext - font::SMALL_CELL_H + 1) / 2; // 1
    assert_eq!((tx - bx, ty - by), (3, 1), "gr_just rounds the half-margin up");

    assert_small_text_at(&fb, &label[..11], tx, ty, (bx, by, ib.ib_wtext, ib.ib_htext));

    // Nothing spilled out of the label box in either direction — the whole
    // point of the small font. One row above and one row below, and the
    // column strips to each side, must be untouched by text ink.
    assert_eq!(fb.count(bx, by - 2, ib.ib_wtext, 2, BLACK), 0, "ink above the label box");
    assert_eq!(fb.count(bx, by + ib.ib_htext, ib.ib_wtext, 4, BLACK), 0, "ink below the box");
}

/// `gsx_tcalc` clamps the text width to the box and then drops **whole**
/// characters (`numchs = min(numchs, textw / wc)`, `gemgraf.c:494`) — it does
/// not clip a glyph in half. 72 / 6 = 12 characters fit exactly.
#[test]
fn icon_label_drops_whole_characters_that_do_not_fit() {
    let label = b"A_VERY_LONG_LABEL_INDEED\0"; // 24 characters
    let ib = stock_iconblk(label, 0);
    let tree =
        [root_box(160, 80, 1), child(G_ICON, &ib as *const IconBlk as u64, 10, 4, 72, 40)];
    let fb = render(&tree);

    let (bx, by) = (10 + ib.ib_xtext, 4 + ib.ib_ytext);
    // textw == the box width, so there is no slack: the run starts at bx.
    let ty = by + (ib.ib_htext - font::SMALL_CELL_H + 1) / 2;
    assert_small_text_at(&fb, &label[..12], bx, ty, (bx, by, ib.ib_wtext, ib.ib_htext));
}

/// A box shorter than one character cell makes `gsx_tcalc` return
/// `numchs = 0` (`*ptexth / hc == 0`, `gemgraf.c:493-496`) and `gr_gtext`
/// draws nothing — it never renders a vertically cropped line.
#[test]
fn icon_label_box_shorter_than_the_cell_draws_nothing() {
    let label = b"NAME\0";
    let mut ib = stock_iconblk(label, 0);
    ib.ib_htext = font::SMALL_CELL_H - 1; // 5 < 6
    let tree =
        [root_box(160, 80, 1), child(G_ICON, &ib as *const IconBlk as u64, 10, 4, 72, 40)];
    let fb = render(&tree);

    let (bx, by) = (10 + ib.ib_xtext, 4 + ib.ib_ytext);
    assert_eq!(fb.count(bx, by - 4, ib.ib_wtext, 16, BLACK), 0, "no label at all");
}

/// The overlay character (a drive letter) is drawn in the SMALL font at
/// `ib_xicon + ib_xchar` / `ib_yicon + ib_ychar` — the offsets are relative to
/// the icon **bitmap**, not to the object (`gemgraf.c:802`).
#[test]
fn icon_overlay_char_is_small_and_offset_by_ib_xicon() {
    let ib = stock_iconblk(b"\0", b'C');
    let (ox, oy) = (10i16, 4i16);
    let tree =
        [root_box(160, 80, 1), child(G_ICON, &ib as *const IconBlk as u64, ox, oy, 72, 40)];
    let fb = render(&tree);

    let cx = ox + ib.ib_xicon + ib.ib_xchar; // 10 + 20 + 13 = 43
    let cy = oy + ib.ib_yicon + ib.ib_ychar; // 4 + 0 + 16 = 20
    assert_small_text_at(&fb, b"C", cx, cy, (ox, oy, 72, 32));

    // Not at the un-offset position the object-relative reading would give.
    let g = font::glyph_small(b'C' as u16);
    let first_ink_col = (0..6).find(|c| g[1] & (0x80 >> c) != 0);
    assert!(first_ink_col.is_some(), "the C glyph has ink on row 1");
    assert_eq!(
        fb.count(ox + ib.ib_xchar, cy, font::SMALL_CELL_W, font::SMALL_CELL_H, BLACK),
        0,
        "character drawn without the ib_xicon offset"
    );
}

/// A `TEDINFO`'s `te_font` selects the size for `G_TEXT`/`G_BOXTEXT`/
/// `G_FTEXT`/`G_FBOXTEXT` (`gr_gtext(edblk.te_just, edblk.te_font, …)`,
/// `gemoblib.c:757`). `SMALL` = 5, `IBM` = 3 (`obdefs.h:62-63`).
#[test]
fn tedinfo_te_font_picks_the_cell_size() {
    for (te_font, cell_w, cell_h) in
        [(5i16, font::SMALL_CELL_W, font::SMALL_CELL_H), (3, font::CELL_W, font::CELL_H)]
    {
        let text = b"AB\0";
        let ted = TedInfo {
            te_ptext: text.as_ptr() as u64,
            te_ptmplt: 0,
            te_pvalid: 0,
            te_font,
            te_junk1: 0,
            te_just: 0, // TE_LEFT
            te_color: 0x1100, // border black, text black, hollow, white interior
            te_junk2: 0,
            te_thickness: 0,
            te_txtlen: 3,
            te_tmplen: 0,
        };
        let (ox, oy, ow, oh) = (10i16, 4i16, 60i16, 20i16);
        let tree = [
            root_box(160, 80, 1),
            child(G_TEXT, &ted as *const TedInfo as u64, ox, oy, ow, oh),
        ];
        let fb = render(&tree);

        // gr_just: vertically centred with the half-margin rounded up, left
        // justified so no horizontal shift.
        let ty = oy + (oh - cell_h + 1) / 2;
        // First inked raster row of 'A' in the selected face, so the assertion
        // is about placement and not about a glyph's own top bearing.
        let first_row = (0..cell_h)
            .find(|&r| glyph_row(te_font, b'A', r) != 0)
            .expect("the A glyph has ink");
        let bb = fb.ink_bbox(ox, oy, ow, oh, BLACK).expect("text drawn");
        assert_eq!(bb.1, ty + first_row, "te_font {te_font}: first ink row");
        assert!(
            bb.3 < ty + cell_h,
            "te_font {te_font}: ink below the {cell_h}px cell (bbox {bb:?})"
        );
        assert!(
            bb.2 < ox + 2 * cell_w,
            "te_font {te_font}: ink past two {cell_w}px cells (bbox {bb:?})"
        );
    }
}

/// `G_STRING` is *not* drawn through `gr_gtext`: EmuTOS justifies it inline
/// (`gemoblib.c:817-845`) at `t.g_x`, vertically centred on `gl_hchar` with a
/// truncating `/2`, in the IBM font, with **no** clamping to the box and no
/// whole-character truncation. Guards the normal-font path against the icon
/// fix.
#[test]
fn g_string_stays_in_the_normal_font_at_the_box_origin() {
    let s = b"Desk\0";
    let (ox, oy, ow, oh) = (10i16, 4i16, 40i16, 19i16);
    let tree =
        [root_box(160, 80, 1), child(G_STRING, s.as_ptr() as u64, ox, oy, ow, oh)];
    let fb = render(&tree);

    let ty = oy + (oh - font::CELL_H) / 2; // truncating /2, not (n+1)/2
    let mut expected = 0usize;
    for (i, &c) in s[..4].iter().enumerate() {
        let g = font::glyph(c as u16);
        for (row, &bits) in g.iter().enumerate() {
            for col in 0..font::CELL_W {
                if bits & (0x80u8 >> col) != 0 {
                    expected += 1;
                    let (px, py) = (ox + i as i16 * font::CELL_W + col, ty + row as i16);
                    assert_eq!(fb.get(px, py), BLACK, "expected 8x16 ink at ({px},{py})");
                }
            }
        }
    }
    assert_eq!(fb.count(ox - 4, oy - 4, ow + 8, oh + 8, BLACK), expected, "stray ink");
    assert_eq!(fb.get(ox, oy), WHITE, "the box ground is white");
}
