//! VDI text output — `v_gtext` (opcode 8) and the GDP `v_justified`
//! (opcode 11 / sub-op 10). Spec: `docs/api/vdi/output/_text.md`,
//! `docs/api/vdi/output/_gdp.md`.
//!
//! Renders the workstation's current font ([`Vwk::cur_font`](super::vwk::Vwk) —
//! 6×6, 8×8 or 8×16, or a header of a loaded chain, chosen by
//! `vst_font`/`vst_height`/`vst_point`), one glyph per `INTIN` character code,
//! with horizontal/vertical alignment and the text effects
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
//! The pen moves by [`glyph_advance`] (`char_resize`); a glyph whose cell lies
//! wholly outside the clip window is neither scaled nor drawn
//! ([`wholly_clipped`]), so the horizontal DDA `XDDA` moves only for the glyphs
//! [`scale_glyph`] scales (TASK/V31).
//!
//! **Working store.** EmuTOS scales and assembles into the scratch buffer of the
//! workstation (`SCRTCHP`, the caller's after `vst_load_fonts`). NewTOS assembles
//! into its own [`GlyphStore`], which bounds a glyph cell to
//! [`WIDE_WORDS`] words × [`MAX_ROWS`] rows, 512 × 512 pixels
//! (`docs/COMPATIBILITY.md` "GDOS font chain").
//!
//! **Row width.** The assembly is written once and instantiated for two row
//! widths: a cell that fits [`NARROW_SIDE`] × [`NARROW_SIDE`] ([`fits_narrow`]) —
//! every built-in font, with every effect and scale — works on one word per row,
//! any other on eight. Measured: the eight words took almost a third of the
//! instructions of every `v_gtext` (TASK/V44). Both give the same pixels.

use core::cell::UnsafeCell;

use super::backend::{ClipRect, RasterBackend, WriteMode};
use super::font::{self, SysFont, F_HORZ_OFF};
use super::primitives::line_clip;
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

/// Words of a row wide enough for every cell: a cell is at most 512 pixels wide, a
/// pixel past this is dropped. The leftmost pixel is bit 63 of word 0.
const WIDE_WORDS: usize = 8;
/// Bytes of the widest row.
const WIDE_BYTES: usize = WIDE_WORDS * 8;
/// Maximum rows of a glyph cell. A row past this is dropped.
const MAX_ROWS: usize = 512;
/// Width and height of the largest cell that takes the one-word rows. A built-in
/// cell stays within it: at most 34 × 34, a 2×-scaled 8 × 16 glyph plus its scaled
/// skew extent and the outline.
const NARROW_SIDE: usize = 64;

/// One glyph row of `N` words.
type Row<const N: usize> = [u64; N];

/// Pixel `x` of a row (false past the row).
#[inline]
fn row_test<const N: usize>(r: &Row<N>, x: usize) -> bool {
    x < N * 64 && r[x / 64] & (1u64 << (63 - x % 64)) != 0
}

/// Set pixel `x` of a row (ignored past the row).
#[inline]
fn row_set<const N: usize>(r: &mut Row<N>, x: usize) {
    if x < N * 64 {
        r[x / 64] |= 1u64 << (63 - x % 64);
    }
}

/// A row moved `n` pixels right; what leaves the row is gone.
#[inline]
fn row_shr<const N: usize>(r: &Row<N>, n: usize) -> Row<N> {
    let (words, bits) = (n / 64, (n % 64) as u32);
    let mut out = [0; N];
    for i in words..N {
        out[i] = r[i - words] >> bits;
        if bits != 0 && i > words {
            out[i] |= r[i - words - 1] << (64 - bits);
        }
    }
    out
}

/// A row moved one pixel left.
#[inline]
fn row_shl1<const N: usize>(r: &Row<N>) -> Row<N> {
    let mut out = [0; N];
    for i in 0..N {
        out[i] = r[i] << 1;
        if i + 1 < N {
            out[i] |= r[i + 1] >> 63;
        }
    }
    out
}

/// `a |= b`.
#[inline]
fn row_or<const N: usize>(a: &mut Row<N>, b: &Row<N>) {
    for (x, y) in a.iter_mut().zip(b.iter()) {
        *x |= *y;
    }
}

/// A row mask covering the leftmost `w` pixels. Used to keep the thicken smear
/// inside a monospace glyph's own width (EmuTOS leaves `delx` unchanged for `MONO`
/// fonts, so the overstrike never widens the cell).
#[inline]
fn width_mask<const N: usize>(w: i16) -> Row<N> {
    let mut out = [0; N];
    let mut left = w.max(0) as usize;
    for word in out.iter_mut() {
        if left == 0 {
            break;
        }
        let n = left.min(64);
        *word = !0u64 << (64 - n);
        left -= n;
    }
    out
}

/// Where a glyph is assembled (see the module comment), `R` rows of `N` words.
/// Only the rows of the current glyph are written and read.
struct GlyphStore<const N: usize, const R: usize> {
    /// The raw glyph.
    raw: [Row<N>; R],
    /// The DDA-scaled glyph.
    scaled: [Row<N>; R],
    /// Source of the effect stage: thicken and skew applied.
    source: [Row<N>; R],
    /// The cell, effects applied.
    cell: [Row<N>; R],
    /// The cell, rotated.
    rotated: [Row<N>; R],
    /// The skew shift of each row ([`skew_shifts`]).
    shifts: [i16; R],
}

impl<const N: usize, const R: usize> GlyphStore<N, R> {
    const ZERO: Self = Self {
        raw: [[0; N]; R],
        scaled: [[0; N]; R],
        source: [[0; N]; R],
        cell: [[0; N]; R],
        rotated: [[0; N]; R],
        shifts: [0; R],
    };
}

/// The working store of the text path.
struct GlyphStores {
    /// For a cell that [`fits_narrow`].
    narrow: GlyphStore<1, NARROW_SIDE>,
    /// For every other cell.
    wide: GlyphStore<WIDE_WORDS, MAX_ROWS>,
    /// A cell packed for the backend.
    bytes: [u8; MAX_ROWS * WIDE_BYTES],
}

struct StoreCell(UnsafeCell<GlyphStores>);
// SAFETY: touched only from the single-CPU, IRQ-off VDI trap path — the same
// discipline as the `Vwk` handle table. All zero: it lives in `.bss`.
unsafe impl Sync for StoreCell {}
static STORE: StoreCell = StoreCell(UnsafeCell::new(GlyphStores {
    narrow: GlyphStore::ZERO,
    wide: GlyphStore::ZERO,
    bytes: [0; MAX_ROWS * WIDE_BYTES],
}));

/// The working store of one text call.
///
/// # Safety
/// Single trap path, one borrower: `v_gtext`/`v_justified` take it once per call
/// and no call nests.
#[allow(clippy::mut_from_ref)]
unsafe fn store() -> &'static mut GlyphStores {
    // SAFETY: see `StoreCell`.
    unsafe { &mut *STORE.0.get() }
}

/// EmuTOS `char_resize` / `act_siz` (`vdi_textblit.c:1257-1278`,
/// `vdi_text.c:743-767`): the actual scaled size of a `size`-unit dimension
/// under the DDA `(dda_inc, t_sclsts)` with the accumulator seeded at `init`.
/// `0xFFFF` means "exactly double" (`size*2`); otherwise at least 1 for a
/// non-zero `size`. The accumulator is not kept: `text_blt` reads `XDDA`
/// through `char_resize` and leaves it, only the spacing loop of `scale()`
/// moves it ([`scale_glyph`]).
#[inline]
fn resize(init: u16, size: u16, dda_inc: u16, t_sclsts: bool) -> u16 {
    if dda_inc == 0xFFFF {
        return size.wrapping_shl(1);
    }
    let mut accu = init;
    let mut retval: u16 = if t_sclsts { size } else { 0 };
    for _ in 0..size {
        accu = accu.wrapping_add(dda_inc);
        if accu < dda_inc {
            retval = retval.wrapping_add(1); // carry out of 16 bits
        }
    }
    if size != 0 && retval == 0 {
        retval = 1; // non-zero input → at least 1
    }
    retval
}

/// The scaled size of a `size`-unit dimension (EmuTOS `act_siz`, seed
/// `0x7fff`).
#[inline]
pub(crate) fn act_siz(size: u16, dda_inc: u16, t_sclsts: bool) -> u16 {
    resize(0x7fff, size, dda_inc, t_sclsts)
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
    let twice_plus_1 = |v: i16| v.wrapping_mul(2).wrapping_add(1);
    SysFont {
        id: f.id,
        font_id: f.font_id,
        point: f.point.wrapping_mul(2),
        first_ade: f.first_ade,
        last_ade: f.last_ade,
        // The exact-double header quirk: top/ascent/half get `*2+1`.
        top: if double { twice_plus_1(f.top) } else { sz(f.top) },
        ascent: if double { twice_plus_1(f.ascent) } else { sz(f.ascent) },
        half: if double { twice_plus_1(f.half) } else { sz(f.half) },
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
        f_default: f.f_default,
        loaded: f.loaded,
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
/// The **base** font ([`font::head_or_system`] of `vwk.cur_font`) is still needed
/// alongside it wherever EmuTOS reads through the header's `off_table`
/// pointer, which `make_header` copies unscaled (`vdi_text.c:810`).
pub(crate) fn effective_font(pb: &Vdipb, vwk: &Vwk) -> SysFont {
    let base = font::head_or_system(pb, vwk.cur_font);
    if vwk.scaled {
        scaled_header(&base, vwk.dda_inc, vwk.t_sclsts)
    } else {
        base
    }
}

/// Height of a text line in the effective font `f` — EmuTOS `calc_height`
/// (`vdi_text.c:160-170`): `top + bottom + 1` (already the scaled values
/// for a scaled workstation), plus the pixel outlining adds above and
/// below.
pub(crate) fn calc_height(f: &SysFont, vwk: &Vwk) -> i16 {
    let mut height = f.top.wrapping_add(f.bottom).wrapping_add(1);
    if vwk.style & T_OUTLINE != 0 {
        height = height.wrapping_add(OUTLINE_THICKNESS * 2);
    }
    height
}

/// `off_table[i]` of a loaded font (`i` a signed `WORD`, as in EmuTOS' pointer
/// arithmetic); `None` when it cannot be read.
fn off_entry(pb: &Vdipb, t: &font::LoadedTables, i: i16) -> Option<u16> {
    let va = t.off_table.wrapping_add_signed(2 * i as i64);
    let mut b = [0u8; 2];
    font::read_mem(pb, t.at, va, &mut b).then(|| u16::from_ne_bytes(b))
}

/// `off_table[i + 1] - off_table[i]` of `base`: the base cell width of a built-in
/// font (monospaced).
pub(crate) fn table_width(pb: &Vdipb, base: &SysFont, i: i16) -> i16 {
    match &base.loaded {
        None => base.max_cell_width,
        Some(t) => match (off_entry(pb, t, i), off_entry(pb, t, i.wrapping_add(1))) {
            (Some(a), Some(b)) => b.wrapping_sub(a) as i16,
            _ => 0,
        },
    }
}

/// `hor_table[i]` of `f`, a byte of that table; 0 when unreadable.
fn hor_entry(pb: &Vdipb, t: &font::LoadedTables, i: i64) -> i16 {
    let mut b = [0u8; 1];
    if font::read_mem(pb, t.at, t.hor_table.wrapping_add_signed(i), &mut b) { b[0] as i16 } else { 0 }
}

/// `vqt_width`'s two offsets with `F_HORZ_OFF` (`vdi_text.c:1089-1092`).
pub(crate) fn horz_offsets(pb: &Vdipb, f: &SysFont, i: i16) -> Option<(i16, i16)> {
    let t = f.loaded.as_ref().filter(|t| t.flags & F_HORZ_OFF != 0)?;
    let k = i as i64;
    Some((hor_entry(pb, t, k * 2), hor_entry(pb, t, k * 2 + 1)))
}

/// The pen's step behind a glyph with `F_HORZ_OFF`: `DESTX += hor_table[temp]`
/// (`vdi_text.c:495-496`, every `chup`).
fn horz_step(pb: &Vdipb, f: &SysFont, gcode: u16) -> i16 {
    match &f.loaded {
        Some(t) if t.flags & F_HORZ_OFF != 0 => hor_entry(pb, t, gcode as i16 as i64),
        _ => 0,
    }
}

/// The size of a glyph in [`GlyphStore`]: `h` rows, `w` pixels wide (the `DELX`
/// cell advance, possibly scaled; `form_height`, possibly scaled).
#[derive(Clone, Copy)]
struct SrcSize {
    w: i16,
    h: i16,
}

/// Read `w` pixels at bit address `bit` (from `base`) of a loaded data table into
/// a row: the table is a byte stream, bit 7 first (`font` module comment).
fn load_row<const N: usize>(pb: &Vdipb, t: &font::LoadedTables, base: u64, bit: i64, w: usize, out: &mut Row<N>) -> bool {
    *out = [0; N];
    if w == 0 {
        return true;
    }
    let off = (bit & 7) as usize;
    let va = base.wrapping_add_signed(bit >> 3);
    let n = (off + w).div_ceil(8);
    let mut b = [0u8; WIDE_BYTES + 1];
    if !font::read_mem(pb, t.at, va, &mut b[..n]) {
        return false;
    }
    let mut bytes = [0u8; WIDE_BYTES];
    for j in 0..w.div_ceil(8) {
        bytes[j] = if off == 0 { b[j] } else { (b[j] << off) | (b[j + 1] >> (8 - off)) };
    }
    for (word, chunk) in out.iter_mut().zip(bytes.chunks_exact(8)) {
        let mut a = [0u8; 8];
        a.copy_from_slice(chunk);
        *word = u64::from_be_bytes(a);
    }
    let mask: Row<N> = width_mask(w as i16);
    for (x, m) in out.iter_mut().zip(mask.iter()) {
        *x &= *m;
    }
    true
}

/// The raw (unscaled) glyph bitmap for `gcode` in font `f` into `raw`:
/// `form_height` rows placed with the leftmost pixel at bit 63 of word 0. A built-in
/// font's cell is `max_cell_width`; a loaded one's comes from `off_table`
/// (`SOURCEX`/`DELX`, `vdi_text.c:467-468`), `SOURCEX` a `WORD`, and the rows are
/// `form_width` bytes apart.
fn raw_glyph<const N: usize, const R: usize>(pb: &Vdipb, f: &SysFont, gcode: u16, raw: &mut [Row<N>; R]) -> SrcSize {
    let fh = (f.form_height.max(0) as usize).min(R);
    let Some(t) = &f.loaded else {
        for (row, out) in raw.iter_mut().take(fh).enumerate() {
            *out = [0; N];
            out[0] = (font::glyph_row(f.id, gcode, row) as u64) << 56;
        }
        return SrcSize { w: f.max_cell_width, h: f.form_height };
    };
    let i = gcode as i16;
    let sourcex = off_entry(pb, t, i).unwrap_or(0) as i16;
    let delx = table_width(pb, f, i);
    let w = (delx.max(0) as usize).min(N * 64);
    for (row, out) in raw.iter_mut().take(fh).enumerate() {
        let base = t.dat_table.wrapping_add_signed(row as i64 * t.form_width as i64);
        if !load_row(pb, t, base, sourcex as i64, w, out) {
            *out = [0; N];
        }
    }
    SrcSize { w: delx, h: f.form_height }
}

/// Horizontally scale-up one source row — EmuTOS `scaleup`
/// (`vdi_textblit.c:577-622`). Reads `w` source bits from the left with the
/// horizontal DDA seeded at `xdda`; each source pixel emits one dest pixel,
/// or two on a DDA carry (widening).
fn scale_row_up<const N: usize>(src_row: &Row<N>, w: i16, dda_inc: u16, xdda: u16) -> Row<N> {
    let mut d = 0usize;
    let mut out = [0; N];
    let mut accum = xdda;
    for s in 0..w.max(0) as usize {
        let set = row_test(src_row, s);
        accum = accum.wrapping_add(dda_inc);
        let carry = accum < dda_inc;
        if set {
            if carry {
                row_set(&mut out, d);
                d += 1;
            }
            row_set(&mut out, d);
            d += 1;
        } else {
            if carry {
                d += 1;
            }
            d += 1;
        }
    }
    out
}

/// Horizontally scale-down one source row — EmuTOS `scaledown`
/// (`vdi_textblit.c:628-666`). A set source pixel emits a dest pixel only
/// on a DDA carry (dropping pixels), a clear source pixel always advances
/// one dest position; the EmuTOS asymmetry is reproduced verbatim.
fn scale_row_down<const N: usize>(src_row: &Row<N>, w: i16, dda_inc: u16, xdda: u16) -> Row<N> {
    let mut d = 0usize;
    let mut out = [0; N];
    let mut accum = xdda;
    for s in 0..w.max(0) as usize {
        if row_test(src_row, s) {
            accum = accum.wrapping_add(dda_inc);
            if accum < dda_inc {
                row_set(&mut out, d);
                d += 1;
            }
        } else {
            d += 1;
        }
    }
    out
}

/// DDA-magnify a raw glyph — EmuTOS `scale` (`vdi_textblit.c:672-741`): a
/// vertical DDA (seed `0x7fff`) duplicates a source row on a scale-up carry
/// or drops it on scale-down, and each emitted row is horizontally scaled
/// ([`scale_row_up`]/[`scale_row_down`]). The width of the scaled glyph and
/// the running horizontal accumulator `xdda` (the sub-pixel nudge that
/// spreads the fractional character spacing across the string) come from
/// the char-spacing DDA at the end of `scale()` (`:722-733`). That loop has
/// neither `char_resize`'s exact-double shortcut nor its "at least 1": at
/// `0xFFFF` each glyph takes `xdda` down by its `DELX`, and the glyph that
/// finds it below `DELX` doubles one source column less ([`scale_row_up`]
/// seeds every row with it) and is one pixel narrower, while the pen still
/// moves `DELX * 2` ([`glyph_advance`]; K14, TASK/V31).
fn scale_glyph<const N: usize, const R: usize>(
    src: &[Row<N>; R],
    size: SrcSize,
    dda_inc: u16,
    t_sclsts: bool,
    xdda: &mut u16,
    out: &mut [Row<N>; R],
) -> SrcSize {
    let mut vaccum: u16 = 0x7fff;
    let mut d = 0usize;
    let seed = *xdda;
    for s in src.iter().take((size.h.max(0) as usize).min(R)) {
        vaccum = vaccum.wrapping_add(dda_inc);
        let vcarry = vaccum < dda_inc;
        if t_sclsts {
            if vcarry && d < R {
                out[d] = scale_row_up(s, size.w, dda_inc, seed);
                d += 1;
            }
            if d < R {
                out[d] = scale_row_up(s, size.w, dda_inc, seed);
                d += 1;
            }
        } else if vcarry && d < R {
            out[d] = scale_row_down(s, size.w, dda_inc, seed);
            d += 1;
        }
    }
    // Char-spacing DDA: the scaled glyph's width + the persistent accumulator
    // (`delx = SCALDIR ? DELX : 0`, one step per source column, `WORD` loop
    // counter).
    let mut accum = seed;
    let mut w = if t_sclsts { size.w } else { 0 };
    for _ in 0..size.w.max(0) {
        accum = accum.wrapping_add(dda_inc);
        if accum < dda_inc {
            w = w.wrapping_add(1);
        }
    }
    *xdda = accum;
    SrcSize { w, h: d as i16 }
}

/// Per-glyph-row rightward skew shift, driven by the font's `skew`
/// bit-mask exactly as EmuTOS' `skew_msk` (`vdi_tblit.S:1279`). The blit
/// runs bottom (row `fh-1`) to top (row 0); before each scanline the mask
/// rotates left one bit and, when a 1 rolls out of bit 15, the running
/// shift bumps by one pixel. The total lean over the cell therefore equals
/// the number of set skew bits in the top `fh` bits. For a scaled glyph
/// `fh` is the scaled row count, so the 16-bit mask cycles more often and
/// the lean grows in step with the scaled `left_offset + right_offset`.
/// Fills the first `fh` entries of `shifts`, returns the lean (the top row's shift).
fn skew_shifts<const R: usize>(f: &SysFont, fh: usize, shifts: &mut [i16; R]) -> i16 {
    let mut mask = f.skew;
    let mut shift = 0i16;
    for k in 0..fh {
        let carry = mask & 0x8000 != 0; // old bit 15 (rol.w sets carry from it)
        mask = mask.rotate_left(1);
        if carry {
            shift += 1;
        }
        shifts[fh - 1 - k] = shift; // current (bottom-up) scanline
    }
    shift
}

/// One row of the effect buffer, or nothing for an out-of-range row index.
#[inline]
fn row_or_zero<const N: usize, const R: usize>(src: &[Row<N>; R], g: i32, fh: usize) -> Row<N> {
    if g >= 0 && (g as usize) < fh {
        src[g as usize]
    } else {
        [0; N]
    }
}

/// The size of an assembled glyph cell: `h` rows, `cell_w` pixels wide. This is the
/// buffer EmuTOS' `pre_blit` produces — the glyph with thicken/skew/outline/light
/// already applied — ready to be blitted directly (`chup == 0`) or
/// transformed by [`rotate_cell`] before the blit (`chup != 0`).
#[derive(Clone, Copy)]
struct CellSize {
    /// Effective cell width in pixels (glyph advance + skew + outline).
    cell_w: i16,
    /// Number of rows in the cell.
    h: i16,
}

/// Assemble one (already raw-or-scaled) source glyph into a cell in `work`
/// with the active text effects baked in — EmuTOS `pre_blit`
/// (`vdi_textblit.c:352-436`): thicken + skew build the source, `outline()`
/// turns it into a hollow contour, and the light mask dims it. `f` supplies
/// the (possibly scaled) effect metrics; `src` supplies the glyph geometry.
/// Position-independent: the caller places the result (directly for
/// `chup == 0`, or after [`rotate_cell`] for a right angle).
fn build_cell<const N: usize, const R: usize>(
    rows: &[Row<N>; R],
    src: SrcSize,
    f: &SysFont,
    style: i16,
    shifts: &mut [i16; R],
    work_src: &mut [Row<N>; R],
    work: &mut [Row<N>; R],
) -> CellSize {
    let fh = (src.h.max(0) as usize).min(R);
    let outline = style & T_OUTLINE != 0;
    let skew = style & T_SKEW != 0;
    let mono = f.mono();

    // 1. Build the source rows: each glyph row placed at its skew offset,
    //    with the bold overstrike (thicken) baked in. Leftmost pixel = bit 63.
    let lean = if skew { skew_shifts(f, fh, shifts) } else { 0 };
    let wmask: Row<N> = width_mask(src.w);
    let weight = if style & T_THICKEN != 0 { (f.thicken.max(0) as usize).min(N * 64) } else { 0 };
    for (row, out) in work_src.iter_mut().take(fh).enumerate() {
        let mut b = rows[row];
        if weight != 0 {
            // Bold: overstrike the source `weight` pixels to the right
            // (EmuTOS smears by `WEIGHT = fnt_ptr->thicken`, the scaled
            // thicken). A MONO font keeps its advance (`upda_dst`), so it is
            // masked to the glyph's own width and only fattens the glyph; a
            // proportional one grows by the weight (`text_blt`).
            let orig = b;
            for k in 1..=weight {
                row_or(&mut b, &row_shr(&orig, k));
            }
            if mono {
                for (x, m) in b.iter_mut().zip(wmask.iter()) {
                    *x &= *m;
                }
            }
        }
        *out = if skew { row_shr(&b, shifts[row].max(0) as usize) } else { b };
    }

    // 2. Outline (hollow contour) or the plain source.
    let h;
    if outline {
        // Inset the glyph one pixel right so its left border fits inside the
        // word, then dilate-and-erase: a pixel is set iff it is background yet
        // has a set 8-neighbour (EmuTOS `outline()`, `vdi_textblit.c:266-345`:
        // `result & ~curr`, hollow letters). Two extra rows hold the top and
        // bottom borders.
        for s in work_src.iter_mut().take(fh) {
            *s = row_shr(s, OUTLINE_THICKNESS as usize);
        }
        h = (fh + 2).min(R);
        for (b, out) in work.iter_mut().take(h).enumerate() {
            let g = b as i32 - 1; // buffer row b holds glyph row b-1
            let cur = row_or_zero(work_src, g, fh);
            let above = row_or_zero(work_src, g - 1, fh);
            let below = row_or_zero(work_src, g + 1, fh);
            let mut neigh = row_shl1(&cur);
            row_or(&mut neigh, &row_shr(&cur, 1));
            for r in [&above, &below] {
                row_or(&mut neigh, r);
                row_or(&mut neigh, &row_shl1(r));
                row_or(&mut neigh, &row_shr(r, 1));
            }
            for (x, c) in neigh.iter_mut().zip(cur.iter()) {
                *x &= !*c;
            }
            *out = neigh;
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
            let m = if (h - 1 - b) & 1 == 0 { lite } else { !lite };
            for x in row.iter_mut() {
                *x &= m;
            }
        }
    }

    // 4. Effective cell width (glyph advance + thicken of a proportional font +
    //    skew extent + outline pixels).
    //    A font's offsets need not add up to the lean its mask gives: EmuTOS'
    //    `screen_blit16` slides its window along with the lean, so ink past
    //    `DELX` still lands (probe TXQ3); here the cell grows to hold it.
    let skew_total = if skew { f.left_offset.wrapping_add(f.right_offset).max(lean) } else { 0 };
    let mut cell_w = src.w.wrapping_add(skew_total);
    if !mono {
        cell_w = cell_w.wrapping_add(weight as i16);
    }
    if outline {
        cell_w = cell_w.wrapping_add(OUTLINE_THICKNESS * 2);
    }
    CellSize { cell_w, h: h as i16 }
}

/// What the pen does behind one glyph.
#[derive(Clone, Copy)]
struct Advance {
    /// Scaled cell width + outline + a proportional font's thicken; skew does not
    /// advance the pen (EmuTOS `upda_dst`).
    pen: i16,
    /// `hor_table[temp]` with `F_HORZ_OFF`, 0 otherwise ([`horz_step`]).
    hor: i16,
}

/// What every glyph of one text call shares: `base` is the ring font (raw glyph +
/// `first_ade`), `f` the effective — possibly scaled — header.
struct TextCall<'a> {
    pb: &'a Vdipb,
    vwk: &'a Vwk,
    backend: &'a dyn RasterBackend,
    base: &'a SysFont,
    f: &'a SysFont,
    clip: ClipRect,
}

/// Whether the cell of glyph `gcode` fits [`NARROW_SIDE`] × [`NARROW_SIDE`] at every
/// stage of [`assemble_and_blit`], rotated too. Bounds, not the exact size: the DDA
/// at most doubles a dimension ([`resize`], [`scale_glyph`]), a skew adds at most one
/// pixel per row, a cell's width may also come from the font's own offsets
/// ([`build_cell`]). A negative `DELX` takes the wide rows: scaled, it is a large
/// `UWORD`.
fn fits_narrow(call: &TextCall, gcode: u16) -> bool {
    let (vwk, f) = (call.vwk, call.f);
    let delx = table_width(call.pb, call.base, gcode as i16) as i32;
    let dda = if vwk.scaled { 2 } else { 1 };
    let rows = dda * f.form_height.max(0) as i32;
    let outline = if vwk.style & T_OUTLINE != 0 { OUTLINE_THICKNESS as i32 * 2 } else { 0 };
    let mut w = dda * delx + outline;
    if vwk.style & T_THICKEN != 0 {
        w += f.thicken.max(0) as i32;
    }
    if vwk.style & T_SKEW != 0 {
        w += (f.left_offset.wrapping_add(f.right_offset) as i32).max(rows);
    }
    let side = NARROW_SIDE as i32;
    delx >= 0 && w <= side && rows + outline <= side
}

/// Assemble one character's cell (raw → optional DDA scale → effects) into
/// `st.cell`. Scaling moves `xdda` ([`scale_glyph`]).
fn make_glyph<const N: usize, const R: usize>(
    st: &mut GlyphStore<N, R>,
    call: &TextCall,
    gcode: u16,
    xdda: &mut u16,
) -> CellSize {
    let (pb, base, f, vwk) = (call.pb, call.base, call.f, call.vwk);
    let mut size = raw_glyph(pb, base, gcode, &mut st.raw);
    let scaled = vwk.scaled;
    if scaled {
        size = scale_glyph(&st.raw, size, vwk.dda_inc, vwk.t_sclsts, xdda, &mut st.scaled);
    }
    let rows = if scaled { &st.scaled } else { &st.raw };
    build_cell(rows, size, f, vwk.style, &mut st.shifts, &mut st.source, &mut st.cell)
}

/// The pen's move behind glyph `gcode` — EmuTOS `text_blt`'s `upda_dst`
/// (`vdi_textblit.c:1402-1417`): `DELX`, or `char_resize(XDDA, DELX)` taken
/// on entry (`:1304-1308`) when the workstation is scaled, plus the outline
/// pixels and a proportional font's `WEIGHT`. `xdda` is the accumulator before
/// the glyph; it is read, not moved.
fn glyph_advance(call: &TextCall, gcode: u16, xdda: u16) -> Advance {
    let (pb, base, f, vwk) = (call.pb, call.base, call.f, call.vwk);
    let delx = table_width(pb, base, gcode as i16);
    let mut pen = if vwk.scaled { resize(xdda, delx as u16, vwk.dda_inc, vwk.t_sclsts) as i16 } else { delx };
    if vwk.style & T_OUTLINE != 0 {
        pen = pen.wrapping_add(OUTLINE_THICKNESS * 2);
    }
    if vwk.style & T_THICKEN != 0 && !f.mono() {
        pen = pen.wrapping_add(f.thicken);
    }
    Advance { pen, hor: horz_step(pb, f, gcode) }
}

/// Whether the cell of a glyph whose pen advance is `pen`, placed with its
/// top-left at `(x, y)`, lies wholly outside the clip window — EmuTOS
/// `check_clip` returning `-1` (`vdi_textblit.c:128-166`), which makes
/// `text_blt` skip `scale()` and the blit and only move the pen
/// (`:1344-1346`). The cell is the one `text_blt` hands it (`:1301-1342`):
/// `delx` = the advance plus the skew offsets `LOFF + ROFF`, `dely` = the
/// scaled `DELY` (`char_resize(0x7fff, form_height)`) plus the outline pixels,
/// swapped at 900 and 2700. Never with clipping off (`:132-133`). `WORD`
/// arithmetic (`-mshort`).
fn wholly_clipped(call: &TextCall, pen: i16, x: i16, y: i16) -> bool {
    let (vwk, f) = (call.vwk, call.f);
    if !vwk.clip {
        return false;
    }
    let mut delx = pen;
    if vwk.style & T_SKEW != 0 {
        delx = delx.wrapping_add(f.left_offset.wrapping_add(f.right_offset));
    }
    let mut dely = if vwk.scaled {
        resize(0x7fff, f.form_height as u16, vwk.dda_inc, vwk.t_sclsts) as i16
    } else {
        f.form_height
    };
    if vwk.style & T_OUTLINE != 0 {
        dely = dely.wrapping_add(OUTLINE_THICKNESS * 2);
    }
    let (w, h) = if matches!(vwk.chup, 900 | 2700) { (dely, delx) } else { (delx, dely) };
    let c = vwk.clip_rect();
    (x < c.xmn && x.wrapping_add(w) <= c.xmn)
        || x > c.xmx
        || (y < c.ymn && y.wrapping_add(h) <= c.ymn)
        || y > c.ymx
}

/// Pack a cell to `ceil(cell_w/8)` bytes per row and blit it with its top-left at
/// `(x, y)` in the workstation writing mode + text colour.
fn blit_cell<const N: usize, const R: usize>(
    call: &TextCall,
    rows: &[Row<N>; R],
    bytes: &mut [u8; MAX_ROWS * WIDE_BYTES],
    cell: CellSize,
    x: i16,
    y: i16,
) {
    let cell_w = cell.cell_w.min((N * 64) as i16);
    let cell_h = cell.h.min(R as i16);
    if cell_w <= 0 || cell_h <= 0 {
        return;
    }
    let h = cell_h as usize;
    let bpr = (cell_w as usize).div_ceil(8);
    for (b, row) in rows.iter().take(h).enumerate() {
        for j in 0..bpr {
            bytes[b * bpr + j] = (row[j / 8] >> (56 - 8 * (j % 8))) as u8;
        }
    }
    let vwk = call.vwk;
    let mode = WriteMode::from_wrt_mode(vwk.wrt_mode);
    call.backend.draw_glyph_sized(x, y, &bytes[..h * bpr], cell_w, cell_h, vwk.text_color, mode, call.clip);
}

/// Assemble one glyph in `st` and blit it, rotated at a right-angle `chup`. `place`
/// gives the cell's top-left from the pen advance. A cell wholly outside the clip
/// window is skipped before it is scaled, `xdda` stays ([`wholly_clipped`]).
fn assemble_and_blit<const N: usize, const R: usize>(
    st: &mut GlyphStore<N, R>,
    bytes: &mut [u8; MAX_ROWS * WIDE_BYTES],
    call: &TextCall,
    gcode: u16,
    xdda: &mut u16,
    place: impl FnOnce(i16) -> (i16, i16),
) -> Advance {
    let advance = glyph_advance(call, gcode, *xdda);
    let (x, y) = place(advance.pen);
    if wholly_clipped(call, advance.pen, x, y) {
        return advance; // `goto upda_dst`
    }
    let cell = make_glyph(st, call, gcode, xdda);
    let chup = call.vwk.chup;
    if matches!(chup, 900 | 1800 | 2700) {
        let rot = rotate_cell(&st.cell, cell, chup, &mut st.rotated);
        blit_cell(call, &st.rotated, bytes, rot, x, y);
    } else {
        blit_cell(call, &st.cell, bytes, cell, x, y);
    }
    advance
}

/// Draw the glyph of `code`, in the rows its cell needs ([`fits_narrow`]).
fn output_glyph(
    st: &mut GlyphStores,
    call: &TextCall,
    code: i16,
    xdda: &mut u16,
    place: impl FnOnce(i16) -> (i16, i16),
) -> Advance {
    let gcode = font::map_code(call.base, code);
    if fits_narrow(call, gcode) {
        assemble_and_blit(&mut st.narrow, &mut st.bytes, call, gcode, xdda, place)
    } else {
        assemble_and_blit(&mut st.wide, &mut st.bytes, call, gcode, xdda, place)
    }
}

/// Rotate an assembled cell by a right angle — EmuTOS `rotate`
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
fn rotate_cell<const N: usize, const R: usize>(rows: &[Row<N>; R], cell: CellSize, chup: i16, out: &mut [Row<N>; R]) -> CellSize {
    let w = (cell.cell_w.max(0) as usize).min(N * 64);
    let h = (cell.h.max(0) as usize).min(R);
    // 90°/270° swap the dimensions; 180° keeps them.
    let (out_w, out_h) = if chup == 1800 { (w, h) } else { (h, w) };
    for r in out.iter_mut().take(out_h) {
        *r = [0; N];
    }
    for (sy, row) in rows.iter().enumerate().take(h) {
        for sx in 0..w {
            if !row_test(row, sx) {
                continue;
            }
            let (dx, dy) = match chup {
                900 => (sy, w - 1 - sx),
                1800 => (w - 1 - sx, h - 1 - sy),
                _ => (h - 1 - sy, sx), // 2700
            };
            if dy < R {
                row_set(&mut out[dy], dx);
            }
        }
    }
    CellSize { cell_w: out_w as i16, h: out_h as i16 }
}

/// Where a horizontal glyph's cell goes (`chup == 0`): glyph col 0 at `x_pen`, cell
/// top at `top_y` (both before the outline shift). The horizontal
/// `v_gtext`/`v_justified` path.
fn horizontal_at(vwk: &Vwk, x_pen: i16, top_y: i16) -> (i16, i16) {
    // Outlined text starts 1 px earlier and 1 row higher (EmuTOS `outline`
    // insets the glyph; `output_text` subtracts `outline` from DESTX/DESTY,
    // `vdi_text.c:410-411`).
    let out_shift = outline_of(vwk);
    (x_pen.wrapping_sub(out_shift), top_y.wrapping_sub(out_shift))
}

/// Vertical-alignment reference → glyph-cell top, for `v_align`
/// (EmuTOS `output_text` `delv`, `vdi_text.c:366-390`). `f` is the
/// effective (possibly scaled) header, so `top`/`bottom` are the scaled
/// metrics; `top + bottom == form_height - 1` for every unscaled font.
/// WORD arithmetic: a loaded font's metrics are the program's to choose.
fn top_for(f: &SysFont, y: i16, v_align: i16) -> i16 {
    match v_align {
        0 => y.wrapping_sub(f.top),                                 // baseline
        1 => y.wrapping_sub(f.top.wrapping_sub(f.half)),            // half line
        2 => y.wrapping_sub(f.top.wrapping_sub(f.ascent)),          // ascent (cap top)
        3 => y.wrapping_sub(f.top.wrapping_add(f.bottom)),          // bottom of cell
        4 => y.wrapping_sub(f.top.wrapping_add(f.descent)),         // descent line
        _ => y,                                                     // top of cell (5)
    }
}

/// On-screen width of a `count`-char run in the current font and effects —
/// EmuTOS `calc_width` (`vdi_text.c:176-210`): the advance — `count` cells of the
/// first table entry for a monospace font, the table entries of the codes from
/// `INTIN[first..]` for a proportional one, unchecked as there — DDA-scaled as a whole
/// when the workstation is scaled, plus the thicken of a proportional font per
/// char, the once-per-string skew extent (scaled) and the per-char outline
/// pixels.
pub(crate) fn calc_width(pb: &Vdipb, first: usize, base: &SysFont, f: &SysFont, count: i16, vwk: &Vwk) -> i16 {
    // The width scales the total, using the *base* table (EmuTOS reads the
    // unscaled `off_table` through the scaled header).
    //
    // WORD arithmetic, as EmuTOS: a run of more than 4095 8-px characters
    // wraps, and `act_siz` takes the width as a `UWORD`.
    let mono = base.mono();
    let mut width = if mono {
        count.wrapping_mul(table_width(pb, base, 0))
    } else {
        let mut sum = 0i16;
        for i in 0..count.max(0) as usize {
            let chr = pb.intin(first + i).wrapping_sub(base.first_ade);
            sum = sum.wrapping_add(table_width(pb, base, chr));
        }
        sum
    };
    if vwk.scaled {
        width = act_siz(width as u16, vwk.dda_inc, vwk.t_sclsts) as i16;
    }
    if vwk.style & T_THICKEN != 0 && !mono {
        width = width.wrapping_add(count.wrapping_mul(f.thicken));
    }
    if vwk.style & T_SKEW != 0 {
        width = width.wrapping_add(f.left_offset.wrapping_add(f.right_offset)); // scaled offsets
    }
    if vwk.style & T_OUTLINE != 0 {
        width = width.wrapping_add(count.wrapping_mul(OUTLINE_THICKNESS * 2));
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
    f.ul_size.wrapping_add(if (f.bottom as u16) > (f.ul_size as u16) {
        1
    } else if vwk.scaled && vwk.dda_inc == 0xFFFF {
        -1
    } else {
        0
    })
}

/// The lines of an underline — EmuTOS `output_text`'s loop
/// (`vdi_text.c:513-542`, every `chup`): `ul_size` lines from `start` to
/// `end`, stepped by `step`, all in the text colour and writing mode. `abline`
/// draws the end point too, so a horizontal run is one pixel longer than the
/// glyph cells, and `LN_MASK` carries over from line to line: `abline`'s own
/// rotation (a vertical line turns it once per pixel, a horizontal or wholly
/// clipped one not at all), then `rorw1` after every line (`:541`) — which is
/// what makes the second line of a light, doubled underline the opposite dots.
/// `clip` is the line clip ([`line_clip`]): with clipping off EmuTOS skips
/// `clip_line` (`:519-534`) and the rule runs on like any line with clipping
/// off (TASK/V30). `abline` is called directly, not through `polyline`, so its
/// XOR end-pixel rule follows the `LSTLIN` the last polyline left
/// ([`super::line::lstlin`]): after one of fewer than two points a
/// non-vertical rule in XOR mode loses its far end (probe TVK, TASK/V40).
fn underline_lines(
    backend: &dyn RasterBackend,
    f: &SysFont,
    vwk: &Vwk,
    start: (i16, i16),
    end: (i16, i16),
    step: (i16, i16),
    clip: ClipRect,
) {
    if vwk.style & T_UNDERLINE == 0 {
        return;
    }
    let mode = WriteMode::from_wrt_mode(vwk.wrt_mode);
    // `LN_MASK`: `lighten` when F_LIGHT is set, else solid (`:513-516`).
    let mut mask = if vwk.style & T_LIGHT != 0 { f.lighten } else { 0xFFFF };
    let last = super::line::lstlin();
    let ((mut x1, mut y1), (mut x2, mut y2)) = (start, end);
    for _ in 0..f.ul_size as u16 {
        mask = backend.abline(x1, y1, x2, y2, vwk.text_color, mode, clip, mask, last).rotate_right(1);
        x1 = x1.wrapping_add(step.0);
        x2 = x2.wrapping_add(step.0);
        y1 = y1.wrapping_add(step.1);
        y2 = y2.wrapping_add(step.1);
    }
}

/// The underline of a horizontal run (`chup` 0, `vdi_text.c:410-413,509`):
/// from `startx` to `DESTX`, the pen *behind* the last glyph, on the row
/// `DESTY + top + underline`. All three come in as EmuTOS has them, shifted
/// by the outline: `y_cell_top` is the rendered cell top (`DESTY`).
fn underline(backend: &dyn RasterBackend, f: &SysFont, vwk: &Vwk, startx: i16, y_cell_top: i16, destx: i16, clip: ClipRect) {
    let y = y_cell_top.wrapping_add(f.top).wrapping_add(underline_offset(f, vwk));
    underline_lines(backend, f, vwk, (startx, y), (destx, y), (0, 1), clip);
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
    // `-mshort`: `UWORD * WORD / UWORD` is 16-bit unsigned. A loaded font's `top` or
    // `bottom` may be 0: a divide-by-zero exception on the 68000, whose result is
    // no value; here the term is 0 (`docs/COMPATIBILITY.md` "GDOS font chain").
    let div = |a: i16, b: i16, c: i16| -> i16 {
        (a as u16).wrapping_mul(b as u16).checked_div(c as u16).unwrap_or(0) as i16
    };
    delh.wrapping_add(match vwk.v_align {
        0 => d1,                       // baseline
        1 => div(f.half, d2, f.top),   // half line
        2 => div(f.ascent, d2, f.top), // ascent
        3 => 0,                        // bottom
        4 => div(f.descent, d1, f.bottom), // descent
        _ => d1.wrapping_add(d2),      // top (5)
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
    let base_hdr = font::head_or_system(pb, vwk.cur_font);
    let base = &base_hdr;
    let scaled_hdr;
    let f: &SysFont = if vwk.scaled {
        scaled_hdr = scaled_header(base, vwk.dda_inc, vwk.t_sclsts);
        &scaled_hdr
    } else {
        base
    };
    // SAFETY: the one borrower of this call (`store`).
    let st = unsafe { store() };
    let (x, y) = (pb.ptsin(0), pb.ptsin(1));

    // Horizontal-alignment offset `delh` against the natural run width
    // (`output_text` with `width < 0`, `vdi_text.c:305-306`).
    let outline = outline_of(vwk);
    let width = calc_width(pb, 0, base, f, count, vwk);
    let delh = delh_for(f, vwk, width, outline);

    // With clipping off `text_blt` cuts nothing (`check_clip`,
    // `vdi_textblit.c:128-133`) and the underline skips `clip_line`
    // (`vdi_text.c:519-534`): glyphs and rule run on in the store
    // ([`ClipRect::OFF`], TASK/V30).
    let clip = line_clip(vwk);
    let call = TextCall { pb, vwk, backend, base, f, clip };

    // Right-angle rotation takes the assemble→rotate→blit path; every other
    // angle (0 and 3600) falls through to the horizontal path — matching
    // EmuTOS' `switch(vwk->chup)` where only 900/1800/2700 are cased.
    if matches!(vwk.chup, 900 | 1800 | 2700) {
        let run = Run { first: 0, count, x, y, delh, outline };
        output_text_rotated(st, &call, &run, None);
        return;
    }

    // Horizontal (`chup == 0`). `x_pen0` is where char 0's glyph col 0 lands.
    let x_pen0 = x.wrapping_sub(delh);
    let top_y = top_for(f, y, vwk.v_align);

    // Per-char advance from [`glyph_advance`] (scaled cell width + outline);
    // the persistent `xdda` spreads the fractional spacing over the glyphs
    // that are scaled (EmuTOS `XDDA = 32767` seed, `vdi_text.c:457`, carried
    // across glyphs; a glyph wholly outside the clip window leaves it).
    let mut xdda: u16 = 0x7fff;
    let mut cx = x_pen0;
    for i in 0..count as usize {
        let advance = output_glyph(st, &call, pb.intin(i), &mut xdda, |_| horizontal_at(vwk, cx, top_y));
        cx = cx.wrapping_add(advance.pen).wrapping_add(advance.hor); // `DESTX` is a WORD
    }
    // `startx` = the first `DESTX`, the end = the last (both shifted by the
    // outline, `vdi_text.c:410-413`).
    underline(backend, f, vwk, x_pen0.wrapping_sub(outline), top_y.wrapping_sub(outline), cx.wrapping_sub(outline), clip);
}

/// Draw a run rotated to a right angle — EmuTOS `output_text`'s `chup`
/// 900/1800/2700 cases (`vdi_text.c:408-543`) driving `text_blt` + `rotate`
/// (`vdi_textblit.c:442-554`, `1331-1342`, `1402-1432`). Each glyph is
/// assembled ([`make_glyph`], scaling included), rotated ([`rotate_cell`])
/// and blitted ([`output_glyph`]); the pen advances along the rotated baseline and the
/// underline (if any) runs parallel to it. `v_gtext` passes no
/// [`JustInfo`]; `v_justified` passes its deltas, which the apply loop adds
/// to the pen after each glyph (`vdi_text.c:476-492`) — `gdp_justified` has
/// already put them on the rotated axis.
fn output_text_rotated(st: &mut GlyphStores, call: &TextCall, run: &Run, mut just: Option<&mut JustInfo>) {
    let (pb, vwk, f) = (call.pb, call.vwk, call.f);
    let Run { first, count, x, y, delh, outline } = *run;
    let chup = vwk.chup;
    let top = f.top;
    let bottom = f.bottom;
    let under = underline_offset(f, vwk);

    // Vertical-alignment shift `delv` (EmuTOS `switch(vwk->v_align)` `delv`,
    // `vdi_text.c:366-390`).
    let delv = match vwk.v_align {
        0 => f.top,
        1 => f.top.wrapping_sub(f.half),
        2 => f.top.wrapping_sub(f.ascent),
        3 => f.top.wrapping_add(f.bottom),
        4 => f.top.wrapping_add(f.descent),
        _ => 0,
    };

    // The per-glyph cell width (incl. skew) shifts a glyph's top-left in
    // `text_blt`'s `chup` switch; the pen advance excludes skew (EmuTOS
    // `upda_dst` recomputes `delx` without `LOFF+ROFF`,
    // `vdi_textblit.c:1403-1417`). `skew_total` is a per-run constant.
    let skew_total = if vwk.style & T_SKEW != 0 { f.left_offset.wrapping_add(f.right_offset) } else { 0 };

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
    // caller-chosen `max_x`, a loaded font's metrics from its program, and either
    // may carry the anchor past the i16 range.
    let below = top.wrapping_add(bottom).wrapping_sub(delv); // `(top + bottom) - delv`
    match chup {
        900 => {
            ax = x.wrapping_sub(delv).wrapping_sub(outline);
            ay = y.wrapping_add(delh).wrapping_add(outline + 1);
            startx = ax.wrapping_add(top).wrapping_add(under);
            starty = ay;
            xfact = 1;
            yfact = 0;
            vertical = true;
        }
        1800 => {
            ax = x.wrapping_add(delh).wrapping_add(outline + 1);
            ay = y.wrapping_sub(below).wrapping_sub(outline);
            startx = ax;
            starty = ay.wrapping_add(bottom).wrapping_sub(under);
            xfact = 0;
            yfact = -1;
            vertical = false;
        }
        _ => {
            // 2700 — no vars.DEST adjustment in `text_blt`'s switch.
            ax = x.wrapping_sub(below).wrapping_sub(outline);
            ay = y.wrapping_sub(delh).wrapping_sub(outline);
            startx = ax.wrapping_add(bottom).wrapping_sub(under);
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
        let adv = output_glyph(st, call, code, &mut xdda, |pen| {
            // Glyph placement shift includes skew (EmuTOS `text_blt` `chup`
            // switch); the pen advance below does not.
            let cell_adj = pen.wrapping_add(skew_total);
            match chup {
                900 => (gx, gy.wrapping_sub(cell_adj)),  // vars.DESTY -= delx
                1800 => (gx.wrapping_sub(cell_adj), gy), // vars.DESTX -= delx
                _ => (gx, gy),                           // 2700 — no adjustment
            }
        });
        let advance = adv.pen;
        match chup {
            900 => gy = gy.wrapping_sub(advance),  // DESTY -= delx (move up)
            1800 => gx = gx.wrapping_sub(advance), // DESTX -= delx (move left)
            _ => gy = gy.wrapping_add(advance),    // 2700 — DESTY += delx (move down)
        }
        if let Some(j) = just.as_deref_mut() {
            j.step(code, &mut gx, &mut gy);
        }
        gx = gx.wrapping_add(adv.hor); // `DESTX`, in every `chup`
    }

    // Underline parallel to the rotated baseline (EmuTOS `output_text`,
    // `vdi_text.c:500-543`): vertical for 900/2700, horizontal for 1800; the
    // far endpoint is the pen's final position (`gx`/`gy`). `ul_size` thin
    // lines stepped by (xfact, yfact) give the thickness.
    let end = if vertical { (startx, gy) } else { (gx, starty) };
    underline_lines(call.backend, f, vwk, (startx, starty), end, (xfact, yfact), call.clip);
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
/// branch of the `output_text` apply-loop (`vdi_text.c:476-492`). At every
/// `chup` `delh` is taken against `width = max_x` (`output_text(vwk, cnt,
/// str, max_x, &just)`, `:1288`; a negative `max_x` falls back to
/// `calc_width`, `:305-306`), as `v_gtext` takes it against the natural
/// width (TASK/V37). At a right-angle `chup` the run goes through `v_gtext`'s
/// rotated path ([`output_text_rotated`]) with the `JustInfo` deltas on the
/// rotated axis. Spec: `docs/api/vdi/output/_gdp.md` "Justified-text
/// rendering".
pub fn v_justified(pb: &Vdipb, vwk: &Vwk, backend: &dyn RasterBackend) {
    // EmuTOS: cnt = CONTRL[3] - 2 (strip the two leading flag words).
    let cnt = (pb.contrl(3) - 2).max(0);
    if cnt <= 0 {
        return; // output_text quick-out for count <= 0
    }
    let base_hdr = font::head_or_system(pb, vwk.cur_font);
    let base = &base_hdr;
    let scaled_hdr;
    let f: &SysFont = if vwk.scaled {
        scaled_hdr = scaled_header(base, vwk.dda_inc, vwk.t_sclsts);
        &scaled_hdr
    } else {
        base
    };
    // SAFETY: the one borrower of this call (`store`).
    let st = unsafe { store() };
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
    let natural = calc_width(pb, 2, base, f, cnt, vwk);
    let mut width = natural;

    let mut just = JustInfo::default(); // bzero default (:1207)

    // Interword spacing (:1212-1253).
    if interword && spaces != 0 {
        let mut delword = max_x.wrapping_sub(width) / spaces;
        just.rmword = max_x.wrapping_sub(width) % spaces;
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
            width = width.wrapping_add(delword.wrapping_mul(spaces)).wrapping_add(just.rmword * direction);
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
        let delchar = max_x.wrapping_sub(width) / (cnt - 1);
        just.rmchar = max_x.wrapping_sub(width) % (cnt - 1);
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

    // Clipping off: glyphs and underline run on in the store, as in
    // `v_gtext` (TASK/V30).
    let clip = line_clip(vwk);
    let call = TextCall { pb, vwk, backend, base, f, clip };

    // `output_text(vwk, cnt, str, max_x, &just)` (:1288): the alignment
    // offset `delh` — `h_align` against `max_x`, the outline's start pixel,
    // the skew additive of `v_align` — for every `chup`; only a negative
    // `max_x` falls back to `calc_width` (`vdi_text.c:305-306`, `:347-390`).
    let outline = outline_of(vwk);
    let run_width = if max_x < 0 { natural } else { max_x };
    let delh = delh_for(f, vwk, run_width, outline);

    // Right angle: `v_gtext`'s rotated `output_text` with the deltas.
    if matches!(vwk.chup, 900 | 1800 | 2700) {
        let run = Run { first: 2, count: cnt, x: x0, y: pb.ptsin(1), delh, outline };
        output_text_rotated(st, &call, &run, Some(&mut just));
        return;
    }

    // Horizontal apply-loop — EmuTOS output_text (:459-492). The run starts
    // at `DESTX = x - delh - outline` (`:410`; [`horizontal_at`] takes the
    // outline). Each glyph blits at the current pen; the base advance moves
    // the pen (EmuTOS `text_blt`'s DELX), then [`JustInfo::step`] adds the
    // interchar and — on a space — the interword delta, each spreading its
    // saved remainder one pixel at a time. At chup 0 the y-components are
    // zero, so `desty` stays on the cell top.
    let x_pen0 = x0.wrapping_sub(delh);
    let mut xdda: u16 = 0x7fff;
    let mut destx = x_pen0;
    let mut desty = top_y;
    for i in 0..cnt as usize {
        let code = pb.intin(2 + i);
        let advance = output_glyph(st, &call, code, &mut xdda, |_| horizontal_at(vwk, destx, desty));
        destx = destx.wrapping_add(advance.pen); // base glyph advance (EmuTOS text_blt DELX), a WORD
        just.step(code, &mut destx, &mut desty);
        destx = destx.wrapping_add(advance.hor);
    }
    // As in `v_gtext`: `startx` is the first `DESTX` (`:412`); `DESTX`/`DESTY`
    // carry the outline shift (`vdi_text.c:410-411`).
    underline(backend, f, vwk, x_pen0.wrapping_sub(outline), top_y.wrapping_sub(outline), destx.wrapping_sub(outline), clip);
}

#[cfg(test)]
mod tests {
    use super::super::backend::{MonoSrc, PatternFill, Pel, Rect};
    use super::super::vwk::Vwk;
    use super::*;
    use crate::bdos::process::Abi;
    use crate::uaccess::Space;
    use std::sync::Mutex;

    /// One blit: position, packed rows, cell size.
    type Blit = (i16, i16, Vec<u8>, i16, i16);

    /// What the path hands the backend: the blits, nothing else.
    #[derive(Default)]
    struct Recorder(Mutex<Vec<Blit>>);

    impl RasterBackend for Recorder {
        fn width(&self) -> u16 {
            640
        }
        fn height(&self) -> u16 {
            480
        }
        fn num_colors(&self) -> u16 {
            2
        }
        fn planes(&self) -> u16 {
            1
        }
        fn set_color(&self, _: u16, _: u16, _: u16, _: u16) {}
        fn query_color(&self, _: u16) -> (u16, u16, u16) {
            (0, 0, 0)
        }
        fn set_active_palette(&self, _: &[u16; 256]) {}
        fn pel_to_device16(&self, _: Pel) -> u16 {
            0
        }
        fn device_to_rgb565(&self, _: u32) -> u16 {
            0
        }
        fn rgb565_to_device(&self, _: u16) -> u32 {
            0
        }
        fn read_pixel(&self, _: i16, _: i16) -> Pel {
            0
        }
        fn fill_rect(&self, _: i16, _: i16, _: i16, _: i16, _: Pel, _: ClipRect) {}
        fn abline(&self, _: i16, _: i16, _: i16, _: i16, _: Pel, _: WriteMode, _: ClipRect, mask: u16, _: bool) -> u16 {
            mask
        }
        fn fill_pattern(&self, _: i16, _: i16, _: i16, _: i16, _: PatternFill, _: ClipRect) {}
        fn draw_glyph_sized(&self, x: i16, y: i16, g: &[u8], w: i16, h: i16, _: Pel, _: WriteMode, _: ClipRect) {
            self.0.lock().unwrap().push((x, y, g.to_vec(), w, h));
        }
        fn blit_mono(&self, _: Rect, _: MonoSrc, _: Pel, _: Pel, _: WriteMode, _: ClipRect) {}
        fn copy_rect(&self, _: Rect, _: i16, _: i16, _: u8, _: ClipRect) {}
        fn get_device_pixel(&self, _: i16, _: i16) -> u32 {
            0
        }
        fn put_device_pixel(&self, _: i16, _: i16, _: u32) {}
        fn flush_rect(&self, _: i16, _: i16, _: i16, _: i16) {}
    }

    /// A kernel-space parameter block; the text path reads nothing from it here but
    /// a chain's tables through its space.
    fn kernel_pb() -> Vdipb {
        let words: &'static mut [i16] = Vec::leak(vec![0i16; 64]);
        let p = words.as_mut_ptr() as u64;
        let blk: &'static [u64; 5] = Box::leak(Box::new([p; 5]));
        // SAFETY: five valid pointers to a large enough WORD array, leaked.
        match unsafe { Vdipb::from_raw(blk.as_ptr() as u64, Abi::Lp64, Space::Kernel) } {
            Ok(pb) => pb,
            Err(_) => panic!("kernel parameter block"),
        }
    }

    /// A chain font: glyphs of the given widths, every row and pixel from a pattern
    /// with both the first and the last pixel of a cell set often enough.
    fn loaded_font(widths: &[i16], rows: i16, flags: u16, left: i16, right: i16, thicken: i16) -> SysFont {
        let mut off = vec![0u16];
        for w in widths {
            off.push(off.last().unwrap().wrapping_add(*w as u16));
        }
        let bits = off.iter().map(|o| *o as usize).max().unwrap();
        let fw = bits.div_ceil(8) + 1;
        let mut dat = vec![0u8; fw * rows as usize];
        for r in 0..rows as usize {
            for c in 0..fw {
                dat[r * fw + c] = ((c * 29 + r * 53) as u8) ^ 0xA5 | 0x81;
            }
        }
        let off: &'static [u16] = Vec::leak(off);
        let dat: &'static [u8] = Vec::leak(dat);
        SysFont {
            first_ade: 0,
            last_ade: widths.len() as i16 - 1,
            left_offset: left,
            right_offset: right,
            thicken,
            form_height: rows,
            top: rows - 1,
            bottom: 0,
            max_cell_width: widths.iter().copied().max().unwrap(),
            loaded: Some(font::LoadedTables {
                at: font::LoadedRef { addr: 0, owner: font::FontOwner::Kernel },
                flags,
                hor_table: 0,
                off_table: off.as_ptr() as u64,
                dat_table: dat.as_ptr() as u64,
                form_width: fw as i16,
                next_font: 0,
                name: [0; font::NAME_LEN],
            }),
            ..font::FON_8X16
        }
    }

    fn vwk(style: i16, chup: i16, scale: Option<(u16, bool)>) -> Vwk {
        let mut v = Vwk::EMPTY;
        v.style = style;
        v.chup = chup;
        if let Some((dda_inc, t_sclsts)) = scale {
            v.scaled = true;
            v.dda_inc = dda_inc;
            v.t_sclsts = t_sclsts;
        }
        v
    }

    /// Stores for a whole run, so stale rows of the previous glyph are in place.
    struct Pair {
        narrow: Box<GlyphStore<1, NARROW_SIDE>>,
        wide: Box<GlyphStore<WIDE_WORDS, MAX_ROWS>>,
        bytes: Box<[u8; MAX_ROWS * WIDE_BYTES]>,
    }

    fn pair() -> Pair {
        // Filled, not zero: a row outside the current glyph must not be read.
        let mut p = Pair {
            narrow: Box::new(GlyphStore::ZERO),
            wide: Box::new(GlyphStore::ZERO),
            bytes: Box::new([0xEE; MAX_ROWS * WIDE_BYTES]),
        };
        for r in p.narrow.raw.iter_mut().chain(p.narrow.scaled.iter_mut()) {
            *r = [!0; 1];
        }
        for r in p.narrow.source.iter_mut().chain(p.narrow.cell.iter_mut()).chain(p.narrow.rotated.iter_mut()) {
            *r = [!0; 1];
        }
        p.narrow.shifts = [0x55; NARROW_SIDE];
        for r in p.wide.raw.iter_mut().chain(p.wide.scaled.iter_mut()) {
            *r = [!0; WIDE_WORDS];
        }
        for r in p.wide.source.iter_mut().chain(p.wide.cell.iter_mut()).chain(p.wide.rotated.iter_mut()) {
            *r = [!0; WIDE_WORDS];
        }
        p.wide.shifts = [0x55; MAX_ROWS];
        p
    }

    /// For each code of a run: where [`fits_narrow`] holds, the narrow and the wide
    /// rows give the same blit, advance and DDA accumulator. Returns how many did.
    fn agree(pb: &Vdipb, stores: &mut Pair, base: &SysFont, v: &Vwk, codes: &[i16]) -> usize {
        let scaled_hdr;
        let f = if v.scaled {
            scaled_hdr = scaled_header(base, v.dda_inc, v.t_sclsts);
            &scaled_hdr
        } else {
            base
        };
        let (narrow, wide) = (Recorder::default(), Recorder::default());
        let call_n = TextCall { pb, vwk: v, backend: &narrow, base, f, clip: ClipRect::OFF };
        let call_w = TextCall { pb, vwk: v, backend: &wide, base, f, clip: ClipRect::OFF };
        let (mut xn, mut xw) = (0x7fffu16, 0x7fffu16);
        let mut checked = 0;
        for &code in codes {
            let gcode = font::map_code(base, code);
            let an = fits_narrow(&call_n, gcode).then(|| {
                assemble_and_blit(&mut *stores.narrow, &mut stores.bytes, &call_n, gcode, &mut xn, |pen| (pen, 7))
            });
            let aw = assemble_and_blit(&mut *stores.wide, &mut stores.bytes, &call_w, gcode, &mut xw, |pen| (pen, 7));
            let Some(an) = an else {
                xn = xw;
                wide.0.lock().unwrap().pop();
                continue;
            };
            assert_eq!((an.pen, an.hor, xn), (aw.pen, aw.hor, xw), "code {code} style {:#x} chup {}", v.style, v.chup);
            let (n, w) = (narrow.0.lock().unwrap().pop(), wide.0.lock().unwrap().pop());
            assert_eq!(n, w, "code {code} style {:#x} chup {}", v.style, v.chup);
            checked += 1;
        }
        checked
    }

    const SCALES: [Option<(u16, bool)>; 5] =
        [None, Some((0xFFFF, true)), Some((0x8000, true)), Some((0x2000, true)), Some((0x8000, false))];

    /// The built-in fonts: every effect, scale and right angle, in the narrow rows
    /// (the bound of the speed of `v_gtext`, TASK/V44) and with the wide rows' result.
    #[test]
    fn built_in_fonts_always_fit_and_agree() {
        let pb = kernel_pb();
        let mut stores = pair();
        let codes: Vec<i16> = (0..256).step_by(7).collect();
        for id in font::RING {
            for scale in SCALES {
                for style in 0..32 {
                    for chup in [0, 900, 1800, 2700] {
                        let v = vwk(style, chup, scale);
                        let n = agree(&pb, &mut stores, font::font(id), &v, &codes);
                        assert_eq!(n, codes.len(), "{id:?} scale {scale:?} style {style:#x} chup {chup}");
                    }
                }
            }
        }
    }

    /// A chain font, mono and proportional, around the bounds: whatever [`fits_narrow`]
    /// lets through agrees with the wide rows.
    #[test]
    fn loaded_fonts_agree_where_they_fit() {
        let pb = kernel_pb();
        let mut stores = pair();
        let widths = [0i16, 1, 3, 8, 17, 24, 31, 32, 33, 40, 55, 56, 57, 60, 61, 62, 63, 64, 65, 66, 100, 512];
        let codes: Vec<i16> = (0..widths.len() as i16).collect();
        let mut checked = 0;
        for rows in [1i16, 16, 31, 32, 33, 60, 62, 63, 64, 65, 100] {
            for flags in [0, font::F_MONOSPACE] {
                for (left, right, thicken) in
                    [(0, 0, 0), (1, 7, 1), (0, 40, 3), (-8, 2, 9), (32767, 1, 2), (-32768, -1, 1), (70, 0, 70), (0, 0, -3)]
                {
                    let base = loaded_font(&widths, rows, flags, left, right, thicken);
                    for scale in SCALES {
                        for style in [0, 1, 2, 4, 8, 16, 5, 17, 20, 21, 31] {
                            for chup in [0, 900, 1800, 2700] {
                                let v = vwk(style, chup, scale);
                                checked += agree(&pb, &mut stores, &base, &v, &codes);
                            }
                        }
                    }
                }
            }
        }
        assert!(checked > 10_000, "the bound must let cases through: {checked}");
    }

    /// The bound itself: 64 × 64 fits, one more does not, in either direction. A
    /// pixel at column 64 is what the narrow rows could not hold.
    #[test]
    fn narrow_bound_is_64_by_64() {
        let pb = kernel_pb();
        let wide_font = loaded_font(&[64, 65], 64, 0, 0, 0, 0);
        let tall_font = loaded_font(&[8], 65, 0, 0, 0, 0);
        let v = vwk(0, 0, None);
        let rec = Recorder::default();
        let call = TextCall { pb: &pb, vwk: &v, backend: &rec, base: &wide_font, f: &wide_font, clip: ClipRect::OFF };
        assert!(fits_narrow(&call, 0));
        assert!(!fits_narrow(&call, 1));
        let call = TextCall { pb: &pb, vwk: &v, backend: &rec, base: &tall_font, f: &tall_font, clip: ClipRect::OFF };
        assert!(!fits_narrow(&call, 0));
    }

    /// A descending `off_table` gives a negative `DELX`, never the narrow rows.
    #[test]
    fn negative_delx_takes_the_wide_rows() {
        let pb = kernel_pb();
        let base = loaded_font(&[8, -3], 16, 0, 0, 0, 0);
        let v = vwk(0, 0, None);
        let rec = Recorder::default();
        let call = TextCall { pb: &pb, vwk: &v, backend: &rec, base: &base, f: &base, clip: ClipRect::OFF };
        assert!(!fits_narrow(&call, 1));
    }

    /// The limits of V33 hold: a cell of 512 × 512 is drawn whole, in the wide rows.
    #[test]
    fn largest_cell_is_still_512_by_512() {
        let pb = kernel_pb();
        let base = loaded_font(&[512], 512, 0, 0, 0, 0);
        let v = vwk(0, 0, None);
        let rec = Recorder::default();
        let call = TextCall { pb: &pb, vwk: &v, backend: &rec, base: &base, f: &base, clip: ClipRect::OFF };
        let mut stores = pair();
        let mut xdda = 0x7fff;
        assert!(!fits_narrow(&call, 0));
        assemble_and_blit(&mut *stores.wide, &mut stores.bytes, &call, 0, &mut xdda, |_| (0, 0));
        let blits = rec.0.lock().unwrap();
        let (_, _, ref bytes, w, h) = blits[0];
        assert_eq!((w, h), (512, 512));
        assert_eq!(bytes.len(), 512 * 64);
        assert_eq!(bytes[511 * 64 + 63] & 0x01, 0x01, "the last pixel of the last row");
    }

    /// K14 (TASK/V31): at an exact double `scale()`'s spacing loop runs anyway
    /// (`vdi_textblit.c:722-733`). Seeded at 15 an 8-pixel row doubles every
    /// column and leaves 7; seeded at 7 the eighth column finds the accumulator
    /// at 0, is not doubled, and the glyph is 15 wide. The pen moves 16 either
    /// way (`char_resize`, `:1261-1263`).
    #[test]
    fn exact_double_spacing_dda_drifts() {
        let mut src = [[0u64; 1]; NARROW_SIDE];
        src[0][0] = 0xFF << 56;
        let mut out = [[0u64; 1]; NARROW_SIDE];
        let row = SrcSize { w: 8, h: 1 };

        let mut xdda = 15;
        let s = scale_glyph(&src, row, 0xFFFF, true, &mut xdda, &mut out);
        assert_eq!((s.w, s.h, xdda), (16, 2, 7));
        assert_eq!(out[0][0] >> 48, 0xFFFF);

        let s = scale_glyph(&src, row, 0xFFFF, true, &mut xdda, &mut out);
        assert_eq!((s.w, s.h, xdda), (15, 2, 0xFFFF));
        assert_eq!(out[0][0] >> 48, 0xFFFE);
        assert_eq!(out[1][0] >> 48, 0xFFFE);

        assert_eq!(resize(7, 8, 0xFFFF, true), 16);
    }
}
