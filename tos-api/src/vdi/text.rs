//! VDI text output — `v_gtext` (opcode 8) and the GDP `v_justified`
//! (opcode 11 / sub-op 10). Spec: `docs/api/vdi/output/_text.md`,
//! `docs/api/vdi/output/_gdp.md`.
//!
//! Renders the workstation's currently-selected system font
//! ([`Vwk::cur_font`](super::vwk::Vwk) — 6×6, 8×8 or 8×16, chosen by
//! `vst_height`/`vst_point`), one glyph per `INTIN` character code, with
//! horizontal/vertical alignment and the text effects
//! (thicken/light/skew/underline/outline) from `style`. Glyphs blit
//! transparently in `text_color`; rotation is limited to the four right
//! angles (only `chup == 0` is drawn horizontally here).
//!
//! **Scaling (magnification).** When `vst_height`/`vst_point` request a
//! size that is not an exact ring match, the workstation is marked
//! [`scaled`](super::vwk::Vwk::scaled) with a DDA fraction
//! ([`dda_inc`](super::vwk::Vwk::dda_inc), `0xFFFF` = exact double). The
//! render path then DDA-scales the raw glyph *before* the effect stage —
//! EmuTOS `text_blt()`'s order `scale → pre_blit → rotate`
//! (`vdi_textblit.c`): [`scale_glyph`] duplicates/drops rows and doubles
//! bits per the DDA, [`build_cell`] bakes the effects onto the scaled
//! glyph, [`rotate_cell`] turns it, and [`blit_cell`] outputs it.

use super::backend::{ClipRect, RasterBackend, WriteMode};
use super::font::{self, SysFont};
use super::vwk::Vwk;
use super::Vdipb;

// Text effect bits (`vst_effects`, EmuTOS `vdi_defs.h:118-122`).
const T_THICKEN: i16 = 0x01;
const T_LIGHT: i16 = 0x02;
const T_SKEW: i16 = 0x04;
const T_UNDERLINE: i16 = 0x08;
const T_OUTLINE: i16 = 0x10;

/// Outlining adds one pixel all around (EmuTOS `OUTLINE_THICKNESS`,
/// `vdi_defs.h:126`).
const OUTLINE_THICKNESS: i16 = 1;

/// Maximum rows of a glyph cell. The tallest cell is a 2×-scaled 8×16 font
/// (`form_height 16 → 32`, EmuTOS `char_resize(0x7fff, 16)`) plus the two
/// rows outlining adds = 34. Each row is a `u64` (leftmost pixel = bit 63),
/// which spans the widest cell — a 2×-scaled 8-px glyph (16) plus the
/// 2×-scaled skew extent (`left+right = 8 → 16`) plus outline (2) = 34 px —
/// with headroom (the unscaled worst case, glyph 8 + skew 8 + outline 2 =
/// 18 px, fits identically in the high bits).
const MAX_ROWS: usize = 34;

/// A row bit-mask covering the leftmost `w` pixels (bit 63 = pixel 0). Used
/// to keep the thicken smear inside a monospace glyph's own width (EmuTOS
/// leaves `delx` unchanged for `MONO` fonts, so the overstrike never widens
/// the cell).
#[inline]
fn width_mask(w: i16) -> u64 {
    if w <= 0 {
        0
    } else if w >= 64 {
        u64::MAX
    } else {
        !0u64 << (64 - w)
    }
}

/// EmuTOS `char_resize` / `act_siz` (`vdi_textblit.c:1257`,
/// `vdi_text.c:743`): the actual scaled size of a `size`-unit dimension
/// under the DDA `(dda_inc, t_sclsts)` given the accumulator seed `init`,
/// and the accumulator value left after processing it. `0xFFFF` means
/// "exactly double" (returns `size*2`, accumulator untouched — this also
/// avoids EmuTOS' rare exact-double `XDDA` drift, keeping every glyph a
/// clean 2×).
#[inline]
fn resize(init: u16, size: u16, dda_inc: u16, t_sclsts: bool) -> (u16, u16) {
    if dda_inc == 0xFFFF {
        return (size << 1, init);
    }
    let mut accu = init;
    let mut retval: u16 = if t_sclsts { size } else { 0 };
    for _ in 0..size {
        accu = accu.wrapping_add(dda_inc);
        if accu < dda_inc {
            retval += 1; // carry out of 16 bits
        }
    }
    if size != 0 && retval == 0 {
        retval = 1; // non-zero input → at least 1
    }
    (retval, accu)
}

/// The scaled size of a `size`-unit dimension (EmuTOS `act_siz`, seed
/// `0x7fff`); the accumulator is discarded (used only for metric fields).
#[inline]
pub(crate) fn act_siz(size: u16, dda_inc: u16, t_sclsts: bool) -> u16 {
    resize(0x7fff, size, dda_inc, t_sclsts).0
}

/// The scaled scratch `Fonthead` for `f` under the DDA — EmuTOS
/// `make_header` (`vdi_text.c:770-817`). `vst_height`/`vst_point` keep
/// `cur_font` pointing at the base ring font plus the DDA on the `Vwk`; the
/// render path rebuilds this scaled header on demand so the effect metrics
/// (thicken, skew offsets, underline size, cell width) match the reported
/// size. `form_height` is *not* scaled here (`make_header` copies it); the
/// scaled row count comes from [`scale_glyph`].
fn scaled_header(f: &SysFont, dda_inc: u16, t_sclsts: bool) -> SysFont {
    let sz = |v: i16| act_siz(v as u16, dda_inc, t_sclsts) as i16;
    let double = dda_inc == 0xFFFF;
    SysFont {
        id: f.id,
        font_id: f.font_id,
        point: f.point * 2,
        first_ade: f.first_ade,
        last_ade: f.last_ade,
        // The exact-double header quirk: top/ascent/half get `*2+1`.
        top: if double { f.top * 2 + 1 } else { sz(f.top) },
        ascent: if double { f.ascent * 2 + 1 } else { sz(f.ascent) },
        half: if double { f.half * 2 + 1 } else { sz(f.half) },
        descent: sz(f.descent),
        bottom: sz(f.bottom),
        max_char_width: sz(f.max_char_width),
        max_cell_width: sz(f.max_cell_width),
        left_offset: sz(f.left_offset),
        right_offset: sz(f.right_offset),
        thicken: sz(f.thicken),
        ul_size: sz(f.ul_size),
        lighten: f.lighten,
        skew: f.skew,
        form_height: f.form_height,
        name: f.name,
    }
}

/// The workstation's effective font header: [`Vwk::cur_font`] as every
/// EmuTOS consumer of `vwk->cur_font` sees it — the plain ring font, or the
/// DDA-scaled `make_header` copy of it when the workstation is scaled.
///
/// EmuTOS builds that copy once, into `vwk->scratch_head`, and repoints
/// `vwk->cur_font` at it (`vdi_text.c:815-816`), so `vqt_extent`/
/// `vqt_width`/`vqt_fontinfo`/`vqt_attributes` and `output_text` all read
/// the scaled metrics straight off `cur_font`. NewTOS keeps `cur_font` on
/// the base ring font plus the DDA on the `Vwk` and rebuilds the scaled
/// header here on demand, which is the same observable header.
///
/// The **base** font (`font::font(vwk.cur_font)`) is still needed
/// alongside it wherever EmuTOS reads through the header's `off_table`
/// pointer, which `make_header` copies unscaled (`vdi_text.c:810`).
pub(crate) fn effective_font(vwk: &Vwk) -> SysFont {
    let base = font::font(vwk.cur_font);
    if vwk.scaled {
        scaled_header(base, vwk.dda_inc, vwk.t_sclsts)
    } else {
        *base
    }
}

/// Height of a text line in the effective font `f` — EmuTOS `calc_height`
/// (`vdi_text.c:160-170`): `top + bottom + 1` (already the scaled values
/// for a scaled workstation), plus the pixel outlining adds above and
/// below.
pub(crate) fn calc_height(f: &SysFont, vwk: &Vwk) -> i16 {
    let mut height = f.top + f.bottom + 1;
    if vwk.style & T_OUTLINE != 0 {
        height += OUTLINE_THICKNESS * 2;
    }
    height
}

/// A source glyph ready for the effect stage: `h` rows of a monochrome
/// bitmap `w` pixels wide (bit 63 of each `u64` = leftmost pixel). This is
/// either the raw font glyph ([`raw_glyph`]) or, for a scaled workstation,
/// the DDA-magnified glyph ([`scale_glyph`]) — EmuTOS' `FBASE` glyph, or
/// the scratch buffer after `scale()`.
struct SrcGlyph {
    rows: [u64; MAX_ROWS],
    /// Glyph pixel width (the `DELX` cell advance, possibly scaled).
    w: i16,
    /// Glyph row count (`form_height`, possibly scaled).
    h: i16,
}

/// The raw (unscaled) glyph bitmap for `gcode` in font `f`: `form_height`
/// rows placed with the leftmost pixel at bit 63, cell width
/// `max_cell_width`.
fn raw_glyph(f: &SysFont, gcode: u16) -> SrcGlyph {
    let fh = (f.form_height as usize).min(MAX_ROWS);
    let mut rows = [0u64; MAX_ROWS];
    for (row, out) in rows.iter_mut().take(fh).enumerate() {
        *out = (font::glyph_row(f.id, gcode, row) as u64) << 56;
    }
    SrcGlyph { rows, w: f.max_cell_width, h: f.form_height }
}

/// Horizontally scale-up one source row — EmuTOS `scaleup`
/// (`vdi_textblit.c:577-622`). Reads `w` source bits from bit 63 with the
/// horizontal DDA seeded at `xdda`; each source pixel emits one dest pixel,
/// or two on a DDA carry (widening). Leftmost pixel = bit 63.
fn scale_row_up(src_row: u64, w: i16, dda_inc: u16, xdda: u16) -> u64 {
    let mut srcbit = 1u64 << 63;
    let mut dstbit = 1u64 << 63;
    let mut out = 0u64;
    let mut accum = xdda;
    for _ in 0..w {
        let set = src_row & srcbit != 0;
        accum = accum.wrapping_add(dda_inc);
        let carry = accum < dda_inc;
        if set {
            if carry {
                out |= dstbit;
                dstbit >>= 1;
            }
            out |= dstbit;
            dstbit >>= 1;
        } else {
            if carry {
                dstbit >>= 1;
            }
            dstbit >>= 1;
        }
        srcbit >>= 1;
    }
    out
}

/// Horizontally scale-down one source row — EmuTOS `scaledown`
/// (`vdi_textblit.c:628-666`). A set source pixel emits a dest pixel only
/// on a DDA carry (dropping pixels), a clear source pixel always advances
/// one dest position; the EmuTOS asymmetry is reproduced verbatim.
fn scale_row_down(src_row: u64, w: i16, dda_inc: u16, xdda: u16) -> u64 {
    let mut srcbit = 1u64 << 63;
    let mut dstbit = 1u64 << 63;
    let mut out = 0u64;
    let mut accum = xdda;
    for _ in 0..w {
        let set = src_row & srcbit != 0;
        if set {
            accum = accum.wrapping_add(dda_inc);
            if accum < dda_inc {
                out |= dstbit;
                dstbit >>= 1;
            }
        } else {
            dstbit >>= 1;
        }
        srcbit >>= 1;
    }
    out
}

/// DDA-magnify a raw glyph — EmuTOS `scale` (`vdi_textblit.c:672-741`): a
/// vertical DDA (seed `0x7fff`) duplicates a source row on a scale-up carry
/// or drops it on scale-down, and each emitted row is horizontally scaled
/// ([`scale_row_up`]/[`scale_row_down`]). The per-glyph cell advance and
/// the running horizontal accumulator `xdda` (the sub-pixel nudge that
/// spreads the fractional character spacing across the string) come from
/// the char-spacing DDA at the end of `scale()`.
fn scale_glyph(src: &SrcGlyph, dda_inc: u16, t_sclsts: bool, xdda: &mut u16) -> SrcGlyph {
    let mut out = SrcGlyph { rows: [0u64; MAX_ROWS], w: 0, h: 0 };
    let mut vaccum: u16 = 0x7fff;
    let mut d = 0usize;
    let seed = *xdda;
    for srow in 0..(src.h as usize).min(MAX_ROWS) {
        vaccum = vaccum.wrapping_add(dda_inc);
        let vcarry = vaccum < dda_inc;
        if t_sclsts {
            if vcarry && d < MAX_ROWS {
                out.rows[d] = scale_row_up(src.rows[srow], src.w, dda_inc, seed);
                d += 1;
            }
            if d < MAX_ROWS {
                out.rows[d] = scale_row_up(src.rows[srow], src.w, dda_inc, seed);
                d += 1;
            }
        } else if vcarry && d < MAX_ROWS {
            out.rows[d] = scale_row_down(src.rows[srow], src.w, dda_inc, seed);
            d += 1;
        }
    }
    out.h = d as i16;
    // Char-spacing DDA: the scaled cell advance + the persistent accumulator.
    let (w, new_xdda) = resize(seed, src.w as u16, dda_inc, t_sclsts);
    out.w = w as i16;
    *xdda = new_xdda;
    out
}

/// Per-glyph-row rightward skew shift, driven by the font's `skew`
/// bit-mask exactly as EmuTOS' `skew_msk` (`vdi_tblit.S:1279`). The blit
/// runs bottom (row `fh-1`) to top (row 0); before each scanline the mask
/// rotates left one bit and, when a 1 rolls out of bit 15, the running
/// shift bumps by one pixel. The total lean over the cell therefore equals
/// the number of set skew bits in the top `fh` bits. For a scaled glyph
/// `fh` is the scaled row count, so the 16-bit mask cycles more often and
/// the lean grows in step with the scaled `left_offset + right_offset`.
fn skew_shifts(f: &SysFont, fh: usize) -> [i16; MAX_ROWS] {
    let mut shifts = [0i16; MAX_ROWS];
    let mut mask = f.skew;
    let mut shift = 0i16;
    for k in 0..fh.min(MAX_ROWS) {
        let carry = mask & 0x8000 != 0; // old bit 15 (rol.w sets carry from it)
        mask = mask.rotate_left(1);
        if carry {
            shift += 1;
        }
        shifts[fh.min(MAX_ROWS) - 1 - k] = shift; // current (bottom-up) scanline
    }
    shifts
}

/// One row of the effect buffer, or 0 for an out-of-range row index.
#[inline]
fn row_or_zero(src: &[u64; MAX_ROWS], g: i32, fh: usize) -> u64 {
    if g >= 0 && (g as usize) < fh {
        src[g as usize]
    } else {
        0
    }
}

/// One assembled glyph cell: `h` rows of a monochrome bitmap, `cell_w`
/// pixels wide (bit 63 of each `u64` = leftmost pixel). This is the buffer
/// EmuTOS' `pre_blit` produces — the glyph with thicken/skew/outline/light
/// already applied — ready to be blitted directly (`chup == 0`) or
/// transformed by [`rotate_cell`] before the blit (`chup != 0`).
struct GlyphCell {
    /// Packed rows, bit 63 = leftmost pixel; `h` rows, `cell_w` bits wide.
    rows: [u64; MAX_ROWS],
    /// Effective cell width in pixels (glyph advance + skew + outline).
    cell_w: i16,
    /// Number of rows in the cell.
    h: i16,
}

/// Assemble one (already raw-or-scaled) source glyph into a [`GlyphCell`]
/// with the active text effects baked in — EmuTOS `pre_blit`
/// (`vdi_textblit.c:352-436`): thicken + skew build the source, `outline()`
/// turns it into a hollow contour, and the light mask dims it. `f` supplies
/// the (possibly scaled) effect metrics; `src` supplies the glyph geometry.
/// Position-independent: the caller places the result (directly for
/// `chup == 0`, or after [`rotate_cell`] for a right angle).
fn build_cell(src: &SrcGlyph, f: &SysFont, style: i16) -> GlyphCell {
    let fh = (src.h as usize).min(MAX_ROWS);
    let outline = style & T_OUTLINE != 0;
    let skew = style & T_SKEW != 0;

    // 1. Build the source rows: each glyph row placed at its skew offset,
    //    with the bold overstrike (thicken) baked in. Leftmost pixel = bit 63.
    let shifts = if skew { skew_shifts(f, fh) } else { [0i16; MAX_ROWS] };
    let wmask = width_mask(src.w);
    let mut work_src = [0u64; MAX_ROWS];
    for (row, out) in work_src.iter_mut().take(fh).enumerate() {
        let mut b = src.rows[row];
        if style & T_THICKEN != 0 {
            // Bold: overstrike the source `weight` pixels to the right
            // (EmuTOS smears by `WEIGHT = fnt_ptr->thicken`, the scaled
            // thicken). Masked to the glyph's own width — MONO fonts keep
            // their advance (`upda_dst`), so it only fattens the glyph.
            let orig = b;
            let weight = f.thicken.clamp(0, 8) as u32;
            for k in 1..=weight {
                b |= orig >> k;
            }
            b &= wmask;
        }
        let s = shifts[row].clamp(0, 48) as u32;
        *out = b >> s;
    }

    // 2. Outline (hollow contour) or the plain source.
    let mut work = [0u64; MAX_ROWS];
    let h;
    if outline {
        // Inset the glyph one pixel right so its left border fits inside the
        // word, then dilate-and-erase: a pixel is set iff it is background yet
        // has a set 8-neighbour (EmuTOS `outline()`, `vdi_textblit.c:266-345`:
        // `result & ~curr`, hollow letters). Two extra rows hold the top and
        // bottom borders.
        let mut ins = [0u64; MAX_ROWS];
        for (dst, s) in ins.iter_mut().zip(work_src.iter()).take(fh) {
            *dst = *s >> OUTLINE_THICKNESS;
        }
        h = (fh + 2).min(MAX_ROWS);
        for (b, out) in work.iter_mut().take(h).enumerate() {
            let g = b as i32 - 1; // buffer row b holds glyph row b-1
            let cur = row_or_zero(&ins, g, fh);
            let above = row_or_zero(&ins, g - 1, fh);
            let below = row_or_zero(&ins, g + 1, fh);
            let neigh = (cur << 1) | (cur >> 1)
                | above | (above << 1) | (above >> 1)
                | below | (below << 1) | (below >> 1);
            *out = neigh & !cur;
        }
    } else {
        h = fh;
        work[..fh].copy_from_slice(&work_src[..fh]);
    }

    // 3. Light: AND each row with the checker mask, phase anchored at the
    //    cell bottom — EmuTOS blits bottom→top and rotates `lite_msk` per
    //    scanline (`vdi_tblit.S:1249`), so the bottom row uses the unrotated
    //    mask. Applied after outlining, matching `screen_blit`'s order.
    if style & T_LIGHT != 0 {
        let l = f.lighten as u64;
        let lite = (l << 48) | (l << 32) | (l << 16) | l; // 0x5555_5555_5555_5555
        for (b, row) in work.iter_mut().take(h).enumerate() {
            *row &= if (h - 1 - b) & 1 == 0 { lite } else { !lite };
        }
    }

    // 4. Effective cell width (glyph advance + skew extent + outline pixels).
    let skew_total = if skew { f.left_offset + f.right_offset } else { 0 };
    let mut cell_w = src.w + skew_total;
    if outline {
        cell_w += OUTLINE_THICKNESS * 2;
    }
    GlyphCell { rows: work, cell_w, h: h as i16 }
}

/// Assemble one character's cell (raw → optional DDA scale → effects) and
/// report its horizontal pen advance (scaled cell width + outline; skew
/// does not advance the pen, EmuTOS `upda_dst`). `base` is the ring font
/// (raw glyph + `first_ade`), `f` the effective — possibly scaled — header.
fn make_glyph(base: &SysFont, f: &SysFont, code: i16, vwk: &Vwk, xdda: &mut u16) -> (GlyphCell, i16) {
    let gcode = font::map_code(base, code);
    let mut src = raw_glyph(base, gcode);
    if vwk.scaled {
        src = scale_glyph(&src, vwk.dda_inc, vwk.t_sclsts, xdda);
    }
    let cell = build_cell(&src, f, vwk.style);
    let outline = if vwk.style & T_OUTLINE != 0 { OUTLINE_THICKNESS } else { 0 };
    (cell, src.w + outline * 2)
}

/// Pack a [`GlyphCell`] to `ceil(cell_w/8)` bytes per row and blit it with
/// its top-left at `(x, y)` in the workstation writing mode + text colour.
fn blit_cell(backend: &dyn RasterBackend, cell: &GlyphCell, x: i16, y: i16, vwk: &Vwk, clip: ClipRect) {
    let cell_w = cell.cell_w;
    let h = (cell.h as usize).min(MAX_ROWS);
    let bpr = (cell_w as usize).div_ceil(8);
    let mut bytes = [0u8; MAX_ROWS * 8];
    for (b, &row) in cell.rows.iter().take(h).enumerate() {
        for j in 0..bpr {
            bytes[b * bpr + j] = (row >> (56 - 8 * j)) as u8;
        }
    }
    let mode = WriteMode::from_wrt_mode(vwk.wrt_mode);
    backend.draw_glyph_sized(x, y, &bytes[..h * bpr], cell_w, cell.h, vwk.text_color, mode, clip);
}

/// Blit one already-assembled glyph horizontally (`chup == 0`): glyph col 0
/// at `x_pen`, cell top at `top_y` (both before the outline shift). The
/// horizontal `v_gtext`/`v_justified` path.
fn draw_cell(backend: &dyn RasterBackend, cell: &GlyphCell, x_pen: i16, top_y: i16, vwk: &Vwk, clip: ClipRect) {
    // Outlined text starts 1 px earlier and 1 row higher (EmuTOS `outline`
    // insets the glyph; `output_text` subtracts `outline` from DESTX/DESTY,
    // `vdi_text.c:410-411`).
    let out_shift = if vwk.style & T_OUTLINE != 0 { OUTLINE_THICKNESS } else { 0 };
    blit_cell(backend, cell, x_pen - out_shift, top_y - out_shift, vwk, clip);
}

/// Rotate an assembled [`GlyphCell`] by a right angle — EmuTOS `rotate`
/// (`vdi_textblit.c:442-554`). `chup` is 900 (90° CCW), 1800 (180°) or 2700
/// (90° CW). 90°/270° swap the cell's width and height; 180° preserves them.
/// The source pixel `(sx, sy)` in a `cell_w × h` cell maps to:
///
/// * 900:  `(sy, cell_w-1-sx)` — top-left → bottom-left (CCW);
/// * 1800: `(cell_w-1-sx, h-1-sy)` — top-left → bottom-right;
/// * 2700: `(h-1-sy, sx)` — top-left → top-right (CW).
///
/// These reproduce the bit transforms EmuTOS' `rotate` performs (the 180°
/// nybble-reverse-and-stack, and the 90°/270° transpose loops).
fn rotate_cell(cell: &GlyphCell, chup: i16) -> GlyphCell {
    let w = cell.cell_w as usize;
    let h = (cell.h as usize).min(MAX_ROWS);
    let mut out = GlyphCell { rows: [0u64; MAX_ROWS], cell_w: 0, h: 0 };
    for sy in 0..h {
        for sx in 0..w {
            if cell.rows[sy] & (0x8000_0000_0000_0000u64 >> sx) == 0 {
                continue;
            }
            let (dx, dy) = match chup {
                900 => (sy, w - 1 - sx),
                1800 => (w - 1 - sx, h - 1 - sy),
                _ => (h - 1 - sy, sx), // 2700
            };
            if dy < MAX_ROWS {
                out.rows[dy] |= 0x8000_0000_0000_0000u64 >> dx;
            }
        }
    }
    // 90°/270° swap the dimensions; 180° keeps them.
    if chup == 1800 {
        out.cell_w = cell.cell_w;
        out.h = cell.h;
    } else {
        out.cell_w = cell.h;
        out.h = cell.cell_w;
    }
    out
}

/// Vertical-alignment reference → glyph-cell top, for `v_align`
/// (EmuTOS `output_text` `delv`, `vdi_text.c:366-390`). `f` is the
/// effective (possibly scaled) header, so `top`/`bottom` are the scaled
/// metrics; `top + bottom == form_height - 1` for every unscaled font.
fn top_for(f: &SysFont, y: i16, v_align: i16) -> i16 {
    match v_align {
        0 => y - f.top,                    // baseline
        1 => y - (f.top - f.half),         // half line
        2 => y - (f.top - f.ascent),       // ascent (cap top)
        3 => y - (f.top + f.bottom),       // bottom of cell
        4 => y - (f.top + f.descent),      // descent line
        _ => y,                            // top of cell (5)
    }
}

/// On-screen width of a `count`-char run in the current font and effects —
/// EmuTOS `calc_width` (`vdi_text.c:176-210`): the monospace advance
/// (`count * base cell width`), then DDA-scaled as a whole when the
/// workstation is scaled, plus the once-per-string skew extent (scaled) and
/// the per-char outline pixels. Thicken adds nothing for a monospace font.
pub(crate) fn calc_width(base: &SysFont, f: &SysFont, count: i16, vwk: &Vwk) -> i16 {
    // Monospace: the width scales the total, using the *base* cell width
    // (EmuTOS reads the unscaled `off_table` through the scaled header).
    let mut width = count * base.max_cell_width;
    if vwk.scaled {
        width = act_siz(width.max(0) as u16, vwk.dda_inc, vwk.t_sclsts) as i16;
    }
    if vwk.style & T_SKEW != 0 {
        width += f.left_offset + f.right_offset; // scaled offsets
    }
    if vwk.style & T_OUTLINE != 0 {
        width += count * OUTLINE_THICKNESS * 2;
    }
    width
}

/// The underline row offset within the cell (EmuTOS `output_text`,
/// `vdi_text.c:399-405`): `ul_size` plus 1 when the cell has room below the
/// descent (`bottom > ul_size`, the 8×16 case), or minus 1 for an
/// exactly-doubled font whose thick underline must be raised back into the
/// cell (`scaled && dda_inc == 0xFFFF`), else 0.
#[inline]
fn underline_offset(f: &SysFont, vwk: &Vwk) -> i16 {
    f.ul_size
        + if f.bottom > f.ul_size {
            1
        } else if vwk.scaled && vwk.dda_inc == 0xFFFF {
            -1
        } else {
            0
        }
}

/// Draw the underline run if the effect is set: an `ul_size`-thick line
/// below the baseline, spanning `[x0, x0+width)`. `y_cell_top` is the
/// rendered cell top (EmuTOS `DESTY`, i.e. already shifted up by outlining).
fn underline(backend: &dyn RasterBackend, f: &SysFont, vwk: &Vwk, x0: i16, y_cell_top: i16, width: i16, clip: ClipRect) {
    if vwk.style & T_UNDERLINE == 0 || width <= 0 {
        return;
    }
    let y = y_cell_top + f.top + underline_offset(f, vwk);
    let mode = WriteMode::from_wrt_mode(vwk.wrt_mode);
    // Light dots the underline: `LN_MASK = lighten` (0x5555) when F_LIGHT is
    // set, else solid 0xFFFF (EmuTOS `vdi_text.c:513-516`). The backend
    // screen-aligns the mask for a horizontal line, giving a dotted rule.
    let mask = if vwk.style & T_LIGHT != 0 { f.lighten } else { 0xFFFF };
    for r in 0..f.ul_size {
        backend.draw_line(x0, y + r, x0 + width - 1, y + r, vwk.text_color, mode, clip, mask);
    }
}

/// `1` when the outline effect is on, else `0` — EmuTOS `output_text`'s
/// `outline` (`vdi_text.c:347-352`): outlined text starts one pixel earlier.
fn outline_of(vwk: &Vwk) -> i16 {
    if vwk.style & T_OUTLINE != 0 { OUTLINE_THICKNESS } else { 0 }
}

/// Horizontal-alignment offset `delh` for a run of on-screen `width`,
/// including the outline start-1px and the skew slant additives EmuTOS
/// folds into it (`output_text`, `vdi_text.c:354-390`). The same for every
/// `chup` — EmuTOS runs both alignment switches before the rotation switch.
/// `v_gtext` passes the natural width, `v_justified` its `max_x`.
fn delh_for(f: &SysFont, vwk: &Vwk, width: i16, outline: i16) -> i16 {
    let (d1, d2) = if vwk.style & T_SKEW != 0 {
        (f.left_offset, f.right_offset)
    } else {
        (0, 0)
    };
    let delh = match vwk.h_align {
        1 => (width / 2).wrapping_sub(outline), // centre
        2 => width.wrapping_sub(outline * 2),   // right
        _ => 0,                                 // left (0)
    };
    delh.wrapping_add(match vwk.v_align {
        0 => d1,                          // baseline
        1 => (f.half * d2) / f.top,       // half line
        2 => (f.ascent * d2) / f.top,     // ascent
        3 => 0,                           // bottom
        4 => (f.descent * d1) / f.bottom, // descent
        _ => d1 + d2,                     // top (5)
    })
}

/// The run `output_text` draws: `count` codes from `INTIN[first..]`, the
/// alignment point `(x, y)` (`PTSIN[0..1]`), and the `delh`/`outline`
/// shared by every `chup`.
struct Run {
    first: usize,
    count: i16,
    x: i16,
    y: i16,
    delh: i16,
    outline: i16,
}

/// Opcode 8 — `v_gtext`: draw the `CONTRL[3]` characters in `INTIN`
/// (one code per WORD) at `PTSIN[0..1]`, in the current text colour,
/// alignment, effects and selected system font. A non-exact
/// `vst_height`/`vst_point` size renders each glyph DDA-scaled. A
/// right-angle `chup` (900/1800/2700) rotates the run (EmuTOS
/// `output_text`/`rotate`); any other angle — 0, or the 3600 `vst_rotation`
/// can round up to — renders horizontally (EmuTOS' `switch(chup)` default).
pub fn v_gtext(pb: &Vdipb, vwk: &Vwk, backend: &dyn RasterBackend) {
    let count = pb.contrl(3);
    if count <= 0 {
        return;
    }
    let base = font::font(vwk.cur_font);
    let scaled_hdr;
    let f: &SysFont = if vwk.scaled {
        scaled_hdr = scaled_header(base, vwk.dda_inc, vwk.t_sclsts);
        &scaled_hdr
    } else {
        base
    };
    let (x, y) = (pb.ptsin(0), pb.ptsin(1));

    // Horizontal-alignment offset `delh` against the natural run width
    // (`output_text` with `width < 0`, `vdi_text.c:305-306`).
    let outline = outline_of(vwk);
    let width = calc_width(base, f, count, vwk);
    let delh = delh_for(f, vwk, width, outline);

    let clip = vwk.clip_rect();

    // Right-angle rotation takes the assemble→rotate→blit path; every other
    // angle (0 and 3600) falls through to the horizontal path — matching
    // EmuTOS' `switch(vwk->chup)` where only 900/1800/2700 are cased.
    if matches!(vwk.chup, 900 | 1800 | 2700) {
        let run = Run { first: 0, count, x, y, delh, outline };
        output_text_rotated(pb, vwk, backend, base, f, &run, clip, None);
        return;
    }

    // Horizontal (`chup == 0`). `x_pen0` is where char 0's glyph col 0 lands.
    let x_pen0 = x - delh;
    let top_y = top_for(f, y, vwk.v_align);

    // Per-char advance comes from each assembled glyph (scaled cell width +
    // outline); the persistent `xdda` spreads the fractional spacing so the
    // run's total equals `calc_width`'s scaled width (EmuTOS `XDDA = 32767`
    // seed, carried across glyphs).
    let mut xdda: u16 = 0x7fff;
    let mut cx = x_pen0;
    for i in 0..count as usize {
        let (cell, advance) = make_glyph(base, f, pb.intin(i), vwk, &mut xdda);
        draw_cell(backend, &cell, cx, top_y, vwk, clip);
        cx += advance;
    }
    underline(backend, f, vwk, x_pen0 - outline, top_y - outline, cx - x_pen0, clip);
}

/// Draw a run rotated to a right angle — EmuTOS `output_text`'s `chup`
/// 900/1800/2700 cases (`vdi_text.c:408-543`) driving `text_blt` + `rotate`
/// (`vdi_textblit.c:442-554`, `1331-1342`, `1402-1432`). Each glyph is
/// assembled ([`make_glyph`], scaling included), rotated ([`rotate_cell`])
/// and blitted; the pen advances along the rotated baseline and the
/// underline (if any) runs parallel to it. `v_gtext` passes no
/// [`JustInfo`]; `v_justified` passes its deltas, which the apply loop adds
/// to the pen after each glyph (`vdi_text.c:476-492`) — `gdp_justified` has
/// already put them on the rotated axis.
#[allow(clippy::too_many_arguments)]
fn output_text_rotated(
    pb: &Vdipb,
    vwk: &Vwk,
    backend: &dyn RasterBackend,
    base: &SysFont,
    f: &SysFont,
    run: &Run,
    clip: ClipRect,
    mut just: Option<&mut JustInfo>,
) {
    let Run { first, count, x, y, delh, outline } = *run;
    let chup = vwk.chup;
    let top = f.top;
    let bottom = f.bottom;
    let under = underline_offset(f, vwk);

    // Vertical-alignment shift `delv` (EmuTOS `switch(vwk->v_align)` `delv`,
    // `vdi_text.c:366-390`).
    let delv = match vwk.v_align {
        0 => f.top,
        1 => f.top - f.half,
        2 => f.top - f.ascent,
        3 => f.top + f.bottom,
        4 => f.top + f.descent,
        _ => 0,
    };

    // The per-glyph cell width (incl. skew) shifts a glyph's top-left in
    // `text_blt`'s `chup` switch; the pen advance excludes skew (EmuTOS
    // `upda_dst` recomputes `delx` without `LOFF+ROFF`,
    // `vdi_textblit.c:1403-1417`). `skew_total` is a per-run constant.
    let skew_total = if vwk.style & T_SKEW != 0 { f.left_offset + f.right_offset } else { 0 };

    // Anchor (first glyph, EmuTOS `output_text` `vdi_text.c:417-440`), the
    // underline start and its thickness step/orientation. `(top+bottom)` is
    // the scaled `form_height - 1`.
    let ax;
    let ay;
    let startx;
    let starty;
    let (xfact, yfact); // underline thickness step
    let vertical; // underline orientation
    // WORD arithmetic, as EmuTOS: `delh` comes from `v_justified`'s
    // caller-chosen `max_x` and may carry the anchor past the i16 range.
    match chup {
        900 => {
            ax = x - delv - outline;
            ay = y.wrapping_add(delh).wrapping_add(outline + 1);
            startx = ax + top + under;
            starty = ay;
            xfact = 1;
            yfact = 0;
            vertical = true;
        }
        1800 => {
            ax = x.wrapping_add(delh).wrapping_add(outline + 1);
            ay = y - ((top + bottom) - delv) - outline;
            startx = ax;
            starty = ay + bottom - under;
            xfact = 0;
            yfact = -1;
            vertical = false;
        }
        _ => {
            // 2700 — no vars.DEST adjustment in `text_blt`'s switch.
            ax = x - ((top + bottom) - delv) - outline;
            ay = y.wrapping_sub(delh).wrapping_sub(outline);
            startx = ax + bottom - under;
            starty = ay;
            xfact = -1;
            yfact = 0;
            vertical = true;
        }
    }

    let mut xdda: u16 = 0x7fff;
    let mut gx = ax;
    let mut gy = ay;
    for i in 0..count as usize {
        let code = pb.intin(first + i);
        let (cell, advance) = make_glyph(base, f, code, vwk, &mut xdda);
        let rot = rotate_cell(&cell, chup);
        // Glyph placement shift includes skew (EmuTOS `text_blt` `chup`
        // switch); the pen advance below does not.
        let cell_adj = advance + skew_total;
        let (bx, by) = match chup {
            900 => (gx, gy.wrapping_sub(cell_adj)),  // vars.DESTY -= delx
            1800 => (gx.wrapping_sub(cell_adj), gy), // vars.DESTX -= delx
            _ => (gx, gy),                           // 2700 — no adjustment
        };
        blit_cell(backend, &rot, bx, by, vwk, clip);
        match chup {
            900 => gy = gy.wrapping_sub(advance),  // DESTY -= delx (move up)
            1800 => gx = gx.wrapping_sub(advance), // DESTX -= delx (move left)
            _ => gy = gy.wrapping_add(advance),    // 2700 — DESTY += delx (move down)
        }
        if let Some(j) = just.as_deref_mut() {
            j.step(code, &mut gx, &mut gy);
        }
    }

    // Underline parallel to the rotated baseline (EmuTOS `output_text`,
    // `vdi_text.c:500-543`): vertical for 900/2700, horizontal for 1800; the
    // far endpoint is the pen's final position (`gx`/`gy`). `ul_size` thin
    // lines stepped by (xfact, yfact) give the thickness.
    if vwk.style & T_UNDERLINE != 0 {
        let mask = if vwk.style & T_LIGHT != 0 { f.lighten } else { 0xFFFF };
        let mode = WriteMode::from_wrt_mode(vwk.wrt_mode);
        let mut x1 = startx;
        let mut y1 = starty;
        let (mut x2, mut y2) = if vertical { (startx, gy) } else { (gx, starty) };
        for _ in 0..f.ul_size {
            backend.draw_line(x1, y1, x2, y2, vwk.text_color, mode, clip, mask);
            x1 += xfact;
            x2 += xfact;
            y1 += yfact;
            y2 += yfact;
        }
    }
}

/// EmuTOS `JUSTINFO` (`vdi_text.c:139-149`): the per-glyph justification
/// deltas that `gdp_justified` computes and the `output_text` apply-loop
/// consumes. In each x/y pair only one component is ever non-zero — `chup`
/// selects the axis: x at 0 and 1800, y at 900 and 2700. `rmword`/`rmchar` are
/// the leftover pixels from the integer division, spread one at a time over
/// the leading gaps/characters so the run lands exactly on the target width.
#[derive(Default)]
struct JustInfo {
    /// #pixels added to each space (interword base delta).
    wordx: i16,
    wordy: i16,
    /// Remaining interword pixels, spread one-per-gap.
    rmword: i16,
    rmwordx: i16,
    rmwordy: i16,
    /// #pixels added after each character (interchar base delta).
    charx: i16,
    chary: i16,
    /// Remaining interchar pixels, spread one-per-character.
    rmchar: i16,
    rmcharx: i16,
    rmchary: i16,
}

impl JustInfo {
    /// The apply loop after one glyph (EmuTOS `output_text`,
    /// `vdi_text.c:476-492`): add the interchar delta and one remainder
    /// pixel while any is left, then — after a space — the interword delta
    /// and one of its remainder pixels. `text_blt` has already moved the pen
    /// `(x, y)` by the glyph's own width. WORD arithmetic, as EmuTOS.
    fn step(&mut self, code: i16, x: &mut i16, y: &mut i16) {
        *x = x.wrapping_add(self.charx);
        *y = y.wrapping_add(self.chary);
        if self.rmchar != 0 {
            *x = x.wrapping_add(self.rmcharx);
            *y = y.wrapping_add(self.rmchary);
            self.rmchar -= 1;
        }
        if code == b' ' as i16 {
            *x = x.wrapping_add(self.wordx);
            *y = y.wrapping_add(self.wordy);
            if self.rmword != 0 {
                *x = x.wrapping_add(self.rmwordx);
                *y = y.wrapping_add(self.rmwordy);
                self.rmword -= 1;
            }
        }
    }
}

/// GDP sub-op 10 — `v_justified`: draw a run of text spread to fill a
/// target width. `INTIN[0]` = interword flag, `INTIN[1]` = interchar
/// flag, `INTIN[2..]` = char codes (`CONTRL[3] - 2` of them).
/// `PTSIN[0..1]` = position, `PTSIN[2]` = target width.
///
/// EmuTOS `gdp_justified` (`vdi_text.c:1177-1289`) and the justification
/// branch of the `output_text` apply-loop (`vdi_text.c:476-492`). At a
/// right-angle `chup` the run goes through `v_gtext`'s rotated path
/// ([`output_text_rotated`]) with the `JustInfo` deltas on the rotated axis
/// and `delh` taken against `width = max_x` (`output_text(vwk, cnt, str,
/// max_x, &just)`, `:1288`; a negative `max_x` falls back to `calc_width`,
/// `:305-306`). Spec: `docs/api/vdi/output/_gdp.md` "Justified-text
/// rendering".
pub fn v_justified(pb: &Vdipb, vwk: &Vwk, backend: &dyn RasterBackend) {
    // EmuTOS: cnt = CONTRL[3] - 2 (strip the two leading flag words).
    let cnt = (pb.contrl(3) - 2).max(0);
    if cnt <= 0 {
        return; // output_text quick-out for count <= 0
    }
    let base = font::font(vwk.cur_font);
    let scaled_hdr;
    let f: &SysFont = if vwk.scaled {
        scaled_hdr = scaled_header(base, vwk.dda_inc, vwk.t_sclsts);
        &scaled_hdr
    } else {
        base
    };
    let interword = pb.intin(0) != 0;
    let interchar = pb.intin(1) != 0;
    let x0 = pb.ptsin(0);
    let top_y = top_for(f, pb.ptsin(1), vwk.v_align);
    let max_x = pb.ptsin(2);

    // Count spaces only when interword adjustment is required
    // (EmuTOS gdp_justified:1197-1201).
    let mut spaces = 0i16;
    if interword {
        for i in 0..cnt as usize {
            if pb.intin(2 + i) as u16 == b' ' as u16 {
                spaces += 1;
            }
        }
    }

    // Natural on-screen width — EmuTOS calc_width (:1203); this is the scaled
    // advance plus skew/outline extents, NOT nchars*cell_w. Kept as the base
    // that the slack is measured against and that the interchar block divides.
    let mut width = calc_width(base, f, cnt, vwk);

    let mut just = JustInfo::default(); // bzero default (:1207)

    // Interword spacing (:1212-1253).
    if interword && spaces != 0 {
        let mut delword = (max_x - width) / spaces;
        just.rmword = (max_x - width) % spaces;
        let direction;
        if just.rmword < 0 {
            direction = -1;
            just.rmword = -just.rmword;
        } else {
            direction = 1;
        }

        // Clamp the per-gap delta to +/-half a cell ONLY when interchar
        // spacing also runs (:1222); pure interword spreads the full slack
        // (fixes AUD-V3-003). When clamped the remainder is dropped, then the
        // interword slack is subtracted from `width` so the interchar block
        // divides only what is left (fixes AUD-V4-007).
        if interchar {
            let expand = f.max_cell_width / 2;
            if delword > expand {
                delword = expand;
                just.rmword = 0;
            }
            if delword < -expand {
                delword = -expand;
                just.rmword = 0;
            }
            width += delword * spaces + just.rmword * direction;
        }

        match vwk.chup {
            900 => {
                just.wordy = -delword;
                just.rmwordy = -direction;
            }
            1800 => {
                just.wordx = -delword;
                just.rmwordx = -direction;
            }
            2700 => {
                just.wordy = delword;
                just.rmwordy = direction;
            }
            _ => {
                just.wordx = delword;
                just.rmwordx = direction;
            }
        }
    }

    // Intercharacter spacing (:1258-1286).
    if interchar && cnt > 1 {
        let delchar = (max_x - width) / (cnt - 1);
        just.rmchar = (max_x - width) % (cnt - 1);
        let direction;
        if just.rmchar < 0 {
            direction = -1;
            just.rmchar = -just.rmchar;
        } else {
            direction = 1;
        }

        match vwk.chup {
            900 => {
                just.chary = -delchar;
                just.rmchary = -direction;
            }
            1800 => {
                just.charx = -delchar;
                just.rmcharx = -direction;
            }
            2700 => {
                just.chary = delchar;
                just.rmchary = direction;
            }
            _ => {
                just.charx = delchar;
                just.rmcharx = direction;
            }
        }
    }

    let clip = vwk.clip_rect();

    // Right angle: `v_gtext`'s rotated `output_text` with the deltas
    // (`gdp_justified` → `output_text(vwk, cnt, str, max_x, &just)`, :1288).
    if matches!(vwk.chup, 900 | 1800 | 2700) {
        let outline = outline_of(vwk);
        let run_width = if max_x < 0 { calc_width(base, f, cnt, vwk) } else { max_x };
        let delh = delh_for(f, vwk, run_width, outline);
        let run = Run { first: 2, count: cnt, x: x0, y: pb.ptsin(1), delh, outline };
        output_text_rotated(pb, vwk, backend, base, f, &run, clip, Some(&mut just));
        return;
    }

    // Horizontal apply-loop — EmuTOS output_text (:459-492). Each glyph
    // blits at the current pen; the base advance moves the pen (EmuTOS
    // `text_blt`'s DELX), then [`JustInfo::step`] adds the interchar and —
    // on a space — the interword delta, each spreading its saved remainder
    // one pixel at a time. At chup 0 the y-components are zero, so `desty`
    // stays on the cell top.
    let mut xdda: u16 = 0x7fff;
    let mut destx = x0;
    let mut desty = top_y;
    for i in 0..cnt as usize {
        let code = pb.intin(2 + i);
        let (cell, advance) = make_glyph(base, f, code, vwk, &mut xdda);
        draw_cell(backend, &cell, destx, desty, vwk, clip);
        destx += advance; // base glyph advance (EmuTOS text_blt DELX)
        just.step(code, &mut destx, &mut desty);
    }
    underline(backend, f, vwk, x0, top_y, destx - x0, clip);
}
