//! The one-pixel line of the VDI — EmuTOS `abline` (`vdi/vdi_line.c:2266`)
//! with `clip_line` (`:1057`) and the 16-bit drawing loops it dispatches to,
//! as a pure walk over the pixels of one segment. Spec:
//! `docs/api/vdi/output/_lines-markers.md` (§ "One segment: `abline`").
//!
//! Hardware-independent: [`abline`] decides *which* store pixels a segment
//! covers, in which order, and the dash bit of each; a backend only combines
//! them into its store (`kernel/src/vdi_backend.rs` `abline`). Host tests
//! drive it directly, and the VDI reference harness draws through it.
//!
//! The store a line runs on in with clipping off ([`LineTarget`]) is also
//! the one a glyph cell lands in with clipping off ([`glyph_unclipped`],
//! TASK/V30) and the one `v_get_pixel` reads (TASK/V24).
//!
//! The arithmetic is EmuTOS' `WORD` arithmetic — deltas, Bresenham terms and
//! loop counters wrap at 16 bits — so a segment whose endpoints lie more
//! than 32767 apart walks as it walks on the m68k. Only `mul_div_round` is
//! computed exactly; see there.

use super::backend::{ClipRect, WriteMode};

/// EmuTOS `LSTLIN` (`bios/lineavars.S:288`, BSS, so FALSE at boot): the one
/// flag the XOR rule of [`abline`] reads (`vdi/vdi_line.c:2319`). One global
/// line-A variable, not per workstation, written by `polyline` alone — FALSE
/// as its loop starts, TRUE before its last segment (`:1106-1108`), so a
/// polyline of fewer than two points leaves it FALSE — and by nothing else
/// (`v_bez`, the other writer, is compiled out: `HAVE_BEZIER` 0,
/// `vdi/vdi_defs.h:18`). Every `polyline` of the VDI writes it, the perimeters
/// of fills and the marker strokes too, and so does every `v_pline` of the
/// AES, which draws through the VDI trap (`aes/gemgsxif.c:573-578`). The text
/// underline calls `abline` directly (`vdi/vdi_text.c:527,534`) and so takes
/// whatever the last `polyline` left: [`lstlin`] (TASK/V40, probe cases TVJ,
/// TVK, L40A–L40Zb of `tosdump/probes/vdprobe.c`).
///
/// Host unit tests run on parallel threads, so there the flag is per thread;
/// every other build has the one global.
#[cfg(not(test))]
static LSTLIN: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

#[cfg(test)]
std::thread_local! {
    static LSTLIN: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
}

/// The current EmuTOS `LSTLIN` (see the static above).
pub fn lstlin() -> bool {
    #[cfg(not(test))]
    {
        LSTLIN.load(core::sync::atomic::Ordering::Relaxed)
    }
    #[cfg(test)]
    {
        LSTLIN.with(|f| f.get())
    }
}

/// Write `LSTLIN` (see the static above): the VDI's `polyline`, and the AES where it
/// draws what EmuTOS' AES draws with a `v_pline`.
pub fn set_lstlin(last: bool) {
    #[cfg(not(test))]
    LSTLIN.store(last, core::sync::atomic::Ordering::Relaxed);
    #[cfg(test)]
    LSTLIN.with(|f| f.set(last));
}

/// The linear pixel store a line is drawn into: `height` rows of `stride`
/// pixels, of which the first `width` of each row are visible — the
/// logical screen (`v_bas_ad`, `v_lin_wr`).
#[derive(Clone, Copy, Debug)]
pub struct LineTarget {
    /// Visible width in pixels.
    pub width: u32,
    /// Height in rows.
    pub height: u32,
    /// Row stride in pixels (`>= width`).
    pub stride: u32,
}

impl LineTarget {
    /// The store index of pixel `(x, y)`: `y * stride + x`, the address
    /// arithmetic of EmuTOS `get_start_addr16` (`vdi/vdi_inline.h:14-17`),
    /// so an `x` past either edge lands in the neighbouring row. `None` when
    /// the index falls outside the store — before its first pixel or past
    /// its last — where EmuTOS writes whatever memory is there and NewTOS
    /// draws nothing (user decision A, TASK/V18; `docs/COMPATIBILITY.md`).
    /// `v_get_pixel` reads through the same index (decision B, TASK/V24).
    pub fn index(&self, x: i32, y: i32) -> Option<usize> {
        let idx = y as i64 * self.stride as i64 + x as i64;
        let len = self.height as i64 * self.stride as i64;
        (0..len).contains(&idx).then_some(idx as usize)
    }
}

/// Blit a glyph cell with clipping off — EmuTOS `text_blt` with `CLIP == 0`:
/// `check_clip` and `do_clip` return 0 at once (`vdi/vdi_textblit.c:128-133`,
/// `:176-183`) and `screen_blit16` writes every cell pixel at
/// `v_bas_ad + DESTX * 2 + (UWORD)(DESTY + DELY - 1) * v_lin_wr` for the
/// bottom row and one `v_lin_wr` less for each row above it (`:1033-1037`),
/// with no bounds check. `plot(index, set)` for every pixel of the
/// `cell_w × cell_h` cell whose top-left is `(x, y)`, `index` the store
/// index [`LineTarget::index`] of `(x + col, row y)` — so a pixel past the
/// right edge lands at the start of the next row, one left of `x = 0` at
/// the end of the previous row — and `set` its glyph bit. `glyph` holds
/// `ceil(cell_w / 8)` bytes per row, leftmost byte first, bit 7 = leftmost
/// pixel (the [`RasterBackend::draw_glyph_sized`](super::backend::RasterBackend::draw_glyph_sized)
/// format). A pixel whose index falls outside the store is not plotted:
/// EmuTOS writes the memory there, NewTOS does not (user decision, TASK/V30,
/// the rule of decision A; `docs/COMPATIBILITY.md`).
///
/// The bottom row is taken `UWORD`, as `screen_blit16` casts it: for a cell
/// that lies wholly above row 0 it is 65536 rows further down, past the end
/// of any store, so that cell is never drawn, whatever its `x`. The column
/// offsets are added in `i32`, as the m68k adds them to the address.
pub fn glyph_unclipped(
    target: &LineTarget,
    x: i16,
    y: i16,
    glyph: &[u8],
    cell_w: i16,
    cell_h: i16,
    mut plot: impl FnMut(usize, bool),
) {
    if cell_w <= 0 || cell_h <= 0 {
        return;
    }
    let bpr = (cell_w as usize).div_ceil(8);
    let bottom = (y as i32 + cell_h as i32 - 1) as u16 as i32;
    for row in 0..cell_h {
        let ry = bottom - (cell_h - 1 - row) as i32;
        for col in 0..cell_w {
            let Some(idx) = target.index(x as i32 + col as i32, ry) else {
                continue;
            };
            let byte = glyph.get(row as usize * bpr + col as usize / 8).copied().unwrap_or(0);
            plot(idx, byte & (0x80u8 >> (col & 7)) != 0);
        }
    }
}

/// EmuTOS `mul_div_round` (`util/miscasm.S:150-195`): `m1 * m2 / d` rounded
/// away from zero, computed as `((m1 * m2 * 2 / d) ± 1) >> 1` — `divs.w`
/// truncates toward zero, `asr.w` floors — so an exact negative quotient
/// also comes out one further from zero (`-300 / 100` → `-4`).
///
/// Exact in `i64`, where the m68k code wraps `m2 * 2` in a `WORD`
/// (`add.w d0,d0`) and leaves `divs.w` overflow undefined: the two differ
/// only for `|m2| > 16383` or a quotient beyond 16 bits, i.e. clip edges
/// more than 16383 pixels from a line's endpoint. `clip_line` never passes
/// `d == 0` (both endpoints would share the outcode bit and the segment is
/// rejected first); the guard only keeps the function total.
fn mul_div_round(m1: i16, m2: i16, d: i16) -> i16 {
    if d == 0 {
        return 0;
    }
    let t = m1 as i64 * m2 as i64 * 2 / d as i64;
    (if t >= 0 { (t + 1) >> 1 } else { (t - 1) >> 1 }) as i16
}

/// EmuTOS `clip_code` (`vdi/vdi_line.c:1034-1048`): where `(x, y)` lies
/// relative to the clip rectangle — 1 left, 2 right, 4 above, 8 below.
fn clip_code(r: &ClipRect, x: i16, y: i16) -> u8 {
    let mut flag = 0;
    if x < r.xmn {
        flag = 1;
    } else if x > r.xmx {
        flag = 2;
    }
    if y < r.ymn {
        flag += 4;
    } else if y > r.ymx {
        flag += 8;
    }
    flag
}

/// More `clip_line` rounds than any converging segment needs: each round
/// puts one endpoint on an edge, and an endpoint needs at most one round per
/// axis plus a rounding correction. EmuTOS' loop has no bound; one that does
/// not converge (`WORD` deltas wrapped by endpoints more than 32767 apart)
/// would hang the kernel, so NewTOS drops that segment instead.
const CLIP_ROUNDS: u32 = 64;

/// EmuTOS `clip_line` (`vdi/vdi_line.c:1057-1092`): Cohen–Sutherland
/// against `r`. Moves an outside endpoint onto the edge it is beyond —
/// left/right first, then top/bottom — computing the other coordinate with
/// [`mul_div_round`] from the *first* endpoint, until both are inside.
/// `None` when the segment lies wholly outside `r`.
pub fn clip_line(r: &ClipRect, seg: (i16, i16, i16, i16)) -> Option<(i16, i16, i16, i16)> {
    let (mut x1, mut y1, mut x2, mut y2) = seg;
    for _ in 0..CLIP_ROUNDS {
        let c1 = clip_code(r, x1, y1);
        let c2 = clip_code(r, x2, y2);
        if c1 | c2 == 0 {
            return Some((x1, y1, x2, y2));
        }
        if c1 & c2 != 0 {
            return None;
        }
        let deltax = x2.wrapping_sub(x1);
        let deltay = y2.wrapping_sub(y1);
        let (flag, first) = if c1 != 0 { (c1, true) } else { (c2, false) };
        let (x, y) = if flag & 1 != 0 {
            (r.xmn, y1.wrapping_add(mul_div_round(deltay, r.xmn.wrapping_sub(x1), deltax)))
        } else if flag & 2 != 0 {
            (r.xmx, y1.wrapping_add(mul_div_round(deltay, r.xmx.wrapping_sub(x1), deltax)))
        } else if flag & 4 != 0 {
            (x1.wrapping_add(mul_div_round(deltax, r.ymn.wrapping_sub(y1), deltay)), r.ymn)
        } else {
            (x1.wrapping_add(mul_div_round(deltax, r.ymx.wrapping_sub(y1), deltay)), r.ymx)
        };
        if first {
            (x1, y1) = (x, y);
        } else {
            (x2, y2) = (x, y);
        }
    }
    None
}

/// The clipping a [`RasterBackend::abline`](super::backend::RasterBackend::abline)
/// caller asks for, applied to one segment. [`ClipRect::OFF`] (`vs_clip(0)`)
/// leaves it as it is — EmuTOS' `polyline` skips `clip_line`
/// (`vdi/vdi_line.c:1115-1116`). Any other rectangle is first bounded by the
/// screen, as `vs_clip` bounds the workstation's (`vdi/vdi_control.c:241-244`;
/// the AES reaches the VDI only through it, `aes/gemgraf.c:91-105`), then
/// [`clip_line`] runs against it.
fn clip_segment(t: &LineTarget, clip: ClipRect, seg: (i16, i16, i16, i16)) -> Option<(i16, i16, i16, i16)> {
    if clip == ClipRect::OFF {
        return Some(seg);
    }
    let xres = t.width.saturating_sub(1).min(i16::MAX as u32) as i16;
    let yres = t.height.saturating_sub(1).min(i16::MAX as u32) as i16;
    let r = ClipRect { xmn: clip.xmn.max(0), ymn: clip.ymn.max(0), xmx: clip.xmx.min(xres), ymx: clip.ymx.min(yres) };
    if r.xmn > r.xmx || r.ymn > r.ymx {
        return None;
    }
    clip_line(&r, seg)
}

/// Draw one segment of a polyline — EmuTOS `polyline`'s per-segment
/// `clip_line` + `abline` (`vdi/vdi_line.c:1115-1119`, `:2266-2359`) in the
/// 16-bit (`TRUECOLOR_MODE`) build: `plot(index, set)` for every pixel,
/// `index` into the store of `target` ([`LineTarget::index`]), `set` the
/// dash bit. Returns `LN_MASK` after the segment; a segment wholly outside
/// the clip returns `mask` untouched.
///
/// 1. Clip the endpoints ([`clip_segment`]); the segment is then drawn from
///    the clipped points, so the pixel path and the dash phase start there.
/// 2. **Vertical** (`x0 == x1` after clipping): `swblit_vertical_line16`
///    from `(x0, y0)` towards `y1`, rotating the mask before each pixel —
///    and *before* the XOR shortening, which a vertical segment therefore
///    never gets (`:2271-2297`, `CONF_WITH_VDI_VERTLINE` = 1, `include/config.h:1644-1645`).
/// 3. Order left to right; with `mode == Xor` and `!last` (`LSTLIN`) the
///    far `x` is shortened by one so a polyline does not complement its
///    shared vertices twice (`:2319-2325`).
/// 4. **Horizontal**: `draw_rect_common` with the mask as a one-row
///    pattern — the dash bit is the mask bit of the absolute `x`, and
///    `LN_MASK` is not advanced (`:2330-2347`, `swblit_rect_common16`).
/// 5. Otherwise `draw_line16` (`:1623-1770`).
///
/// No pixel is clipped by the walk itself: with clipping off a pixel past
/// the screen edge goes to the store index EmuTOS would address, the
/// neighbouring row ([`LineTarget::index`]).
#[allow(clippy::too_many_arguments)]
pub fn abline(
    target: &LineTarget,
    x0: i16,
    y0: i16,
    x1: i16,
    y1: i16,
    mode: WriteMode,
    clip: ClipRect,
    mask: u16,
    last: bool,
    mut plot: impl FnMut(usize, bool),
) -> u16 {
    let Some((x0, y0, x1, y1)) = clip_segment(target, clip, (x0, y0, x1, y1)) else {
        return mask;
    };
    let mut put = |x: i32, y: i32, set: bool| {
        if let Some(i) = target.index(x, y) {
            plot(i, set);
        }
    };
    if x0 == x1 {
        return vertical_line16(x0, y0, y1, mask, &mut put);
    }
    // `abline:2299-2312`: always from the left endpoint.
    let (ax, ay, mut bx, by) = if x1 < x0 { (x1, y1, x0, y0) } else { (x0, y0, x1, y1) };
    if mode == WriteMode::Xor && !last {
        bx = bx.wrapping_sub(1); // `x1 != x2` here: the `y2--` arm is the vertical path's
    }
    if ay == by {
        let y = ay as i32;
        for x in ax as i32..=bx as i32 {
            put(x, y, mask & (0x8000u16 >> (x & 0xf)) != 0);
        }
        return mask;
    }
    draw_line16(ax, ay, bx, by, mask, &mut put)
}

/// EmuTOS `swblit_vertical_line16` (`vdi/vdi_line.c:2084-2145`): `dy + 1`
/// pixels from `(x, y1)` towards `y2`, the mask rotated before each.
fn vertical_line16(x: i16, y1: i16, y2: i16, mask: u16, put: &mut impl FnMut(i32, i32, bool)) -> u16 {
    let mut dy = y2.wrapping_sub(y1);
    let mut yinc = 1i32;
    if dy < 0 {
        dy = dy.wrapping_neg();
        yinc = -1;
    }
    let (x, mut y) = (x as i32, y1 as i32);
    let mut linemask = mask;
    let mut loopcnt = dy;
    while loopcnt >= 0 {
        linemask = linemask.rotate_left(1);
        put(x, y, linemask & 1 != 0);
        y += yinc;
        loopcnt -= 1;
    }
    linemask
}

/// EmuTOS `draw_line16` (`vdi/vdi_line.c:1623-1770`): Bresenham from the
/// left endpoint `(x1, y1)` (`x1 < x2`), the `eps`/`e1`/`e2` form, the mask
/// rotated before each pixel.
fn draw_line16(x1: i16, y1: i16, x2: i16, y2: i16, mask: u16, put: &mut impl FnMut(i32, i32, bool)) -> u16 {
    let dx = x2.wrapping_sub(x1);
    let mut dy = y2.wrapping_sub(y1);
    let mut yinc = 1i32;
    if dy < 0 {
        dy = dy.wrapping_neg();
        yinc = -1;
    }
    let (mut x, mut y) = (x1 as i32, y1 as i32);
    let mut linemask = mask;
    // The major axis advances every pixel, the minor one when `eps` turns
    // non-negative; for `dx < dy` the two swap roles.
    let (major, minor) = if dx >= dy { (dx, dy) } else { (dy, dx) };
    let e1 = minor.wrapping_mul(2);
    let e2 = major.wrapping_mul(2);
    let mut eps = major.wrapping_neg();
    let mut loopcnt = major;
    while loopcnt >= 0 {
        linemask = linemask.rotate_left(1);
        put(x, y, linemask & 1 != 0);
        if dx >= dy {
            x += 1;
        } else {
            y += yinc;
        }
        eps = eps.wrapping_add(e1);
        if eps >= 0 {
            eps = eps.wrapping_sub(e2);
            if dx >= dy {
                y += yinc;
            } else {
                x += 1;
            }
        }
        loopcnt -= 1;
    }
    linemask
}

// ── The same walk as runs (TASK/V58) ────────────────────────────────────

/// The store rectangle a segment touched, `(c0, r0, c1, r1)`: the smallest
/// one holding `(index % stride, index / stride)` of every store index
/// [`abline`] hands out for it — columns `c0..=c1`, rows `r0..=r1`, columns
/// counted in the stride, so a clipping-off run-on can reach into the pitch
/// padding. What a backend cleans to the point of coherency after a line.
pub type Touched = (u32, u32, u32, u32);

/// The receiver of [`abline_runs`]: the pixels of one segment, the very
/// store indices and dash bits [`abline`] hands to its `plot`, in the same
/// order, grouped into the runs a store can draw at once (TASK/V58). Every
/// index lies in the store (`< height * stride`); a run is never empty.
///
/// A segment arrives as exactly one [`span`](Self::span) (horizontal), one
/// [`column`](Self::column) (vertical) or a sequence of
/// [`pixel`](Self::pixel)s (any other), or as nothing at all.
pub trait LineRuns {
    /// Store pixels `idx`, `idx + 1`, … `idx + n - 1`: a horizontal segment,
    /// one row, or with clipping off a run on across row ends. Pixel `i`'s
    /// dash bit is `mask & (0x8000 >> ((phase + i) & 15)) != 0` (`phase` =
    /// the first pixel's `x & 15`, the mask screen-aligned).
    fn span(&mut self, idx: usize, n: usize, phase: u32, mask: u16);
    /// `n` store pixels from `idx` on, each `step` (`stride` or `-stride`)
    /// after the one before: a vertical segment in walk order. Pixel `i`'s
    /// dash bit is `mask & (0x8000 >> ((phase + i) & 15)) != 0` (`phase` =
    /// the walk pixels before the first one in the store, modulo 16).
    fn column(&mut self, idx: usize, n: usize, step: isize, phase: u32, mask: u16);
    /// One pixel of a segment that is neither, in walk order.
    fn pixel(&mut self, idx: usize, set: bool);
}

/// [`abline`] as runs: the same clipping, XOR rule, pixels, order, dash
/// bits and returned `LN_MASK`, handed to `runs` as a span, a column or
/// single pixels instead of one `plot` per pixel — so a backend can draw a
/// horizontal segment as one span and a vertical one as one stride loop,
/// with its pixel store, colours and mode resolved once (TASK/V58). Returns
/// `LN_MASK` and the store rectangle touched (`None`: no pixel).
///
/// The walks are [`abline`]'s, rewritten over store indices: a horizontal
/// segment is the index range `y * stride + ax ..= y * stride + bx`, cut to
/// the store once, its dash bit the mask bit of the absolute `x`; a vertical
/// one `dy + 1` indices `stride` apart, cut to the store once, pixel `k`'s
/// dash bit `rotl(mask, k + 1) & 1` = `mask & (0x8000 >> (k & 15))`
/// ([`vertical_line16`]); any other `draw_line16`'s Bresenham in its `WORD`
/// terms, stepping the index by `1` and `±stride` — when both endpoints lie
/// on a screen of at most 16384 × 16384 every pixel does and no term can
/// wrap, so no pixel needs a check; otherwise each pixel is checked against
/// the store and its column and row are carried along (a column past the
/// stride starts the next row), so no pixel needs a division. Tests compare
/// the two walks pixel for pixel.
///
/// Read for technique only (ADR-035): fVDI's 16-bit line steps the
/// address by one "one step" and one "both steps" offset per Bresenham
/// decision, one routine per write mode
/// (`references/fvdi/fvdi/drivers/16_bit/16b_line.c:33-63`, `:709-841`).
#[allow(clippy::too_many_arguments)]
pub fn abline_runs(
    target: &LineTarget,
    x0: i16,
    y0: i16,
    x1: i16,
    y1: i16,
    mode: WriteMode,
    clip: ClipRect,
    mask: u16,
    last: bool,
    runs: &mut impl LineRuns,
) -> (u16, Option<Touched>) {
    let Some((x0, y0, x1, y1)) = clip_segment(target, clip, (x0, y0, x1, y1)) else {
        return (mask, None);
    };
    let s = target.stride as i64;
    let len = target.height as i64 * s;
    if x0 == x1 {
        return vertical_runs(x0, y0, y1, s, len, mask, runs);
    }
    // `abline:2299-2325`: from the left endpoint, the XOR rule.
    let (ax, ay, mut bx, by) = if x1 < x0 { (x1, y1, x0, y0) } else { (x0, y0, x1, y1) };
    if mode == WriteMode::Xor && !last {
        bx = bx.wrapping_sub(1);
    }
    if ay == by {
        let first = ay as i64 * s + ax as i64;
        let (lo, hi) = (first.max(0), (ay as i64 * s + bx as i64).min(len - 1));
        if lo > hi {
            return (mask, None);
        }
        // The first pixel in the store has `x = ax + (lo - first)`.
        let phase = ((ax as i64 + (lo - first)) & 15) as u32;
        runs.span(lo as usize, (hi - lo + 1) as usize, phase, mask);
        return (mask, Some(span_touched(lo, hi, s)));
    }
    diagonal_runs(target, (ax, ay, bx, by), s, len, mask, runs)
}

/// The rectangle of the store indices `lo..=hi` (`0 <= lo <= hi`, `s > 0`):
/// one row, or a run on across row ends, which holds column `s - 1` of its
/// first row and column 0 of its last.
fn span_touched(lo: i64, hi: i64, s: i64) -> Touched {
    let (r0, r1) = (lo / s, hi / s);
    if r0 == r1 {
        ((lo % s) as u32, r0 as u32, (hi % s) as u32, r1 as u32)
    } else {
        (0, r0 as u32, (s - 1) as u32, r1 as u32)
    }
}

/// [`vertical_line16`] as one column: walk pixel `k` (`0..dy + 1`) is store
/// index `first + k * step`, which is monotonic in `k`, so the pixels in the
/// store are one range `klo..=khi` of `k`, found with two divisions.
fn vertical_runs(
    x: i16,
    y1: i16,
    y2: i16,
    s: i64,
    len: i64,
    mask: u16,
    runs: &mut impl LineRuns,
) -> (u16, Option<Touched>) {
    let mut dy = y2.wrapping_sub(y1);
    let mut yinc = 1i64;
    if dy < 0 {
        dy = dy.wrapping_neg();
        yinc = -1;
    }
    // `dy + 1` pixels; none when the negation leaves `dy` negative (-32768).
    let count = if dy >= 0 { dy as i64 + 1 } else { 0 };
    let after = mask.rotate_left((count % 16) as u32);
    if count == 0 || len == 0 {
        return (after, None);
    }
    let first = y1 as i64 * s + x as i64;
    let step = yinc * s;
    let ceil_div = |a: i64| (a + s - 1) / s;
    let (klo, khi) = if step > 0 {
        (if first >= 0 { 0 } else { ceil_div(-first) }, if first < len { (len - 1 - first) / s } else { -1 })
    } else {
        (if first < len { 0 } else { ceil_div(first - (len - 1)) }, if first >= 0 { first / s } else { -1 })
    };
    let khi = khi.min(count - 1);
    if klo > khi {
        return (after, None);
    }
    let (idx, end) = (first + klo * step, first + khi * step);
    runs.column(idx as usize, (khi - klo + 1) as usize, step as isize, (klo & 15) as u32, mask);
    let col = (idx % s) as u32;
    (after, Some((col, (idx.min(end) / s) as u32, col, (idx.max(end) / s) as u32)))
}

/// [`draw_line16`] over store indices, from the left endpoint `(x1, y1)` of
/// `seg` — the same `WORD` terms in the same order, the index stepped by
/// `1` for `x` and `±stride` for `y`.
fn diagonal_runs(
    t: &LineTarget,
    (x1, y1, x2, y2): (i16, i16, i16, i16),
    s: i64,
    len: i64,
    mask: u16,
    runs: &mut impl LineRuns,
) -> (u16, Option<Touched>) {
    let dx = x2.wrapping_sub(x1);
    let mut dy = y2.wrapping_sub(y1);
    let mut yinc = 1i64;
    if dy < 0 {
        dy = dy.wrapping_neg();
        yinc = -1;
    }
    let xmajor = dx >= dy;
    let (major, minor) = if xmajor { (dx, dy) } else { (dy, dx) };
    let e1 = minor.wrapping_mul(2);
    let e2 = major.wrapping_mul(2);
    let mut eps = major.wrapping_neg();
    // `loopcnt` runs from `major` down to 0: `major + 1` pixels, or none.
    let count = if major >= 0 { major as u32 + 1 } else { 0 };
    let after = mask.rotate_left(count % 16);
    if count == 0 || len == 0 {
        return (after, None);
    }
    let mut linemask = mask;
    let on = |x: i16, y: i16| x >= 0 && (x as u32) < t.width && y >= 0 && (y as u32) < t.height;
    if t.width <= t.stride && t.width <= 16384 && t.height <= 16384 && on(x1, y1) && on(x2, y2) {
        // Both ends on the screen: `0 <= dx, |dy| < 16384`, so no term wraps;
        // the walk stays in the ends' bounding box — after `j` steps the
        // minor axis has moved `floor((2 * minor * j + major) / (2 * major))`
        // <= `minor` — and ends on `(x2, y2)`. Every pixel is on the screen,
        // its column its `x`, its row its `y`. As no term wraps, they are
        // carried in `i32`, and the mask as two copies in one `u32`, whose
        // rotation by one is the `WORD` rotation in each half.
        let (major_step, minor_step) =
            if xmajor { (1isize, (yinc * s) as isize) } else { ((yinc * s) as isize, 1isize) };
        let mut idx = (y1 as i64 * s + x1 as i64) as isize;
        let (e1, e2, mut eps) = (e1 as i32, e2 as i32, eps as i32);
        let mut m = mask as u32 * 0x0001_0001;
        for _ in 0..count {
            m = m.rotate_left(1);
            runs.pixel(idx as usize, m & 1 != 0);
            idx = idx.wrapping_add(major_step);
            eps += e1;
            if eps >= 0 {
                eps -= e2;
                idx = idx.wrapping_add(minor_step);
            }
        }
        return (after, Some((x1 as u32, y1.min(y2) as u32, x2 as u32, y1.max(y2) as u32)));
    }
    // Anywhere else (clipping off): the index `y * stride + x` and its store
    // column and row, carried along; a pixel is in the store when its row is.
    let mut idx = y1 as i64 * s + x1 as i64;
    let (mut row, mut col) = (idx.div_euclid(s), idx.rem_euclid(s));
    let rows = t.height as i64;
    let mut touched: Option<(i64, i64, i64, i64)> = None;
    let step_x = |idx: &mut i64, col: &mut i64, row: &mut i64| {
        *idx += 1;
        *col += 1;
        if *col == s {
            *col = 0;
            *row += 1;
        }
    };
    let step_y = |idx: &mut i64, row: &mut i64| {
        *idx += yinc * s;
        *row += yinc;
    };
    for _ in 0..count {
        linemask = linemask.rotate_left(1);
        if (0..rows).contains(&row) {
            runs.pixel(idx as usize, linemask & 1 != 0);
            touched = Some(match touched {
                None => (col, row, col, row),
                Some((c0, r0, c1, r1)) => (c0.min(col), r0.min(row), c1.max(col), r1.max(row)),
            });
        }
        if xmajor {
            step_x(&mut idx, &mut col, &mut row);
        } else {
            step_y(&mut idx, &mut row);
        }
        eps = eps.wrapping_add(e1);
        if eps >= 0 {
            eps = eps.wrapping_sub(e2);
            if xmajor {
                step_y(&mut idx, &mut row);
            } else {
                step_x(&mut idx, &mut col, &mut row);
            }
        }
    }
    (after, touched.map(|(c0, r0, c1, r1)| (c0 as u32, r0 as u32, c1 as u32, r1 as u32)))
}

#[cfg(test)]
mod tests {
    extern crate std;
    use std::vec::Vec;

    use super::*;

    /// The Falcon truecolor screen of the oracle: 640×480, no pitch padding.
    const FALCON: LineTarget = LineTarget { width: 640, height: 480, stride: 640 };

    fn clip(xmn: i16, ymn: i16, xmx: i16, ymx: i16) -> ClipRect {
        ClipRect { xmn, ymn, xmx, ymx }
    }

    /// Every pixel the walk hands out, as `(column, row, dash bit)` of the
    /// store, in walk order, plus the mask it returns.
    #[allow(clippy::too_many_arguments)]
    fn walk(
        t: &LineTarget,
        (x0, y0, x1, y1): (i16, i16, i16, i16),
        mode: WriteMode,
        c: ClipRect,
        mask: u16,
        last: bool,
    ) -> (Vec<(u32, u32, bool)>, u16) {
        let mut px = Vec::new();
        let m = abline(t, x0, y0, x1, y1, mode, c, mask, last, |i, set| {
            assert!(i < t.height as usize * t.stride as usize, "index {i} outside the store");
            px.push(((i % t.stride as usize) as u32, (i / t.stride as usize) as u32, set));
        });
        (px, m)
    }

    /// The set pixels only, as `(column, row)`.
    fn set_pixels(px: &[(u32, u32, bool)]) -> Vec<(u32, u32)> {
        px.iter().filter(|p| p.2).map(|p| (p.0, p.1)).collect()
    }

    // ── VDI-DRAW-001: the XOR shortening skips vertical segments ─────────

    /// EmuTOS draws a vertical segment through `swblit_vertical_line16` and
    /// returns before the `LSTLIN` shortening (`vdi/vdi_line.c:2275-2283`,
    /// `:2319`): all 21 pixels of (20,20)-(20,40), the shared vertex
    /// included, from the first point on.
    #[test]
    fn xor_vertical_segment_that_is_not_last_keeps_its_end_point() {
        let (px, _) = walk(&FALCON, (20, 20, 20, 40), WriteMode::Xor, ClipRect::OFF, 0xFFFF, false);
        let want: Vec<(u32, u32)> = (20..=40).map(|y| (20, y)).collect();
        assert_eq!(set_pixels(&px), want);
    }

    /// Upwards: from (20,40) to (20,20), never (20,19) — the old shortening
    /// `by -= 1` walked one pixel *past* an upward segment (vdi-v18 case 32).
    #[test]
    fn xor_upward_vertical_segment_does_not_overshoot() {
        let (px, _) = walk(&FALCON, (20, 40, 20, 20), WriteMode::Xor, ClipRect::OFF, 0xFFFF, false);
        let want: Vec<(u32, u32)> = (20..=40).rev().map(|y| (20, y)).collect();
        assert_eq!(set_pixels(&px), want);
    }

    /// The other segments keep the DRI kludge: only the far `x` is
    /// shortened by one (`vdi/vdi_line.c:2319-2325`), horizontal and
    /// diagonal alike — so the diagonal (10,10)-(20,20) is drawn as
    /// (10,10)-(19,20), eleven pixels ending on row 20.
    #[test]
    fn xor_horizontal_and_diagonal_segments_that_are_not_last_lose_their_far_x() {
        let (px, _) = walk(&FALCON, (20, 40, 40, 40), WriteMode::Xor, ClipRect::OFF, 0xFFFF, false);
        assert_eq!(set_pixels(&px), (20..=39).map(|x| (x, 40)).collect::<Vec<_>>());
        let (px, _) = walk(&FALCON, (10, 10, 20, 20), WriteMode::Xor, ClipRect::OFF, 0xFFFF, false);
        assert_eq!(px.len(), 11);
        assert_eq!(px.last().map(|p| (p.0, p.1)), Some((19, 20)));
        let (px, _) = walk(&FALCON, (10, 10, 20, 20), WriteMode::Xor, ClipRect::OFF, 0xFFFF, true);
        assert_eq!(px.last().map(|p| (p.0, p.1)), Some((20, 20)));
    }

    // ── VDI-DRAW-002: clipping moves the endpoints, then draws ───────────

    /// vdi-v18 cases 40/43: clip (30,30)-(90,90), line (0,31)-(127,33).
    /// `clip_line` gives (30,31)-(90,32) — `31 + mul_div_round(2, 30, 127)`
    /// = 31, then `31 + mul_div_round(2, 60, 97)` = 32 — and the Bresenham
    /// of that segment steps down after its 30th pixel.
    #[test]
    fn clipped_segment_is_the_bresenham_of_the_clipped_endpoints() {
        let (px, _) = walk(&FALCON, (0, 31, 127, 33), WriteMode::Replace, clip(30, 30, 90, 90), 0xFFFF, true);
        let mut want: Vec<(u32, u32)> = (30..=59).map(|x| (x, 31)).collect();
        want.extend((60..=90).map(|x| (x, 32)));
        assert_eq!(set_pixels(&px), want);
    }

    /// `mul_div_round` rounds an exact negative quotient one further from
    /// zero (`util/miscasm.S:183-194`: `divs.w`, `subq.w #1`, `asr.w #1`):
    /// line (0,20)-(100,10) against a left edge at 30 gives
    /// `20 + mul_div_round(-10, 30, 100)` = `20 + ((-6 - 1) >> 1)` = 16,
    /// not the exact 17.
    #[test]
    fn clipped_start_uses_emutos_rounding() {
        let (px, _) = walk(&FALCON, (0, 20, 100, 10), WriteMode::Replace, clip(30, 0, 639, 479), 0xFFFF, true);
        assert_eq!(px.first().map(|p| (p.0, p.1)), Some((30, 16)));
        assert_eq!(px.last().map(|p| (p.0, p.1)), Some((100, 10)));
    }

    /// The dash starts again at the clipped endpoint (probe K2D): a vertical
    /// line from y 0 with one dash bit in 16, clipped at y 5, draws its
    /// first set pixel at y 5 and rotates the mask once per *drawn* pixel
    /// (16 of them), so it comes back unrotated.
    #[test]
    fn clipped_dashed_vertical_restarts_the_dash_at_the_clip_edge() {
        let (px, m) = walk(&FALCON, (10, 0, 10, 20), WriteMode::Transparent, clip(0, 5, 639, 479), 0x8000, true);
        assert_eq!(set_pixels(&px), [(10, 5)]);
        assert_eq!(px.len(), 16);
        assert_eq!(m, 0x8000);
    }

    /// A clip rectangle beyond the screen is first cut to the screen, as
    /// `vs_clip` does (`vdi/vdi_control.c:241-244`): no run-on then.
    #[test]
    fn clip_rectangle_is_bounded_by_the_screen() {
        let (px, _) = walk(&FALCON, (600, 10, 700, 10), WriteMode::Replace, clip(0, 0, 1000, 1000), 0xFFFF, true);
        assert_eq!(set_pixels(&px), (600..=639).map(|x| (x, 10)).collect::<Vec<_>>());
    }

    /// A segment wholly outside the clip draws nothing and leaves the mask.
    #[test]
    fn segment_outside_the_clip_draws_nothing() {
        let (px, m) = walk(&FALCON, (0, 0, 20, 10), WriteMode::Replace, clip(30, 30, 90, 90), 0x1234, true);
        assert!(px.is_empty());
        assert_eq!(m, 0x1234);
    }

    // ── VDI-DRAW-003 and decision A: clipping off ────────────────────────

    /// vdi-v18 case 45: with clipping off, x 640..700 of row 120 are
    /// x 0..60 of row 121 (`get_start_addr16`, `vdi/vdi_inline.h:14-17`).
    #[test]
    fn clip_off_horizontal_runs_into_the_next_row() {
        let (px, _) = walk(&FALCON, (600, 120, 700, 120), WriteMode::Replace, ClipRect::OFF, 0xFFFF, true);
        let mut want: Vec<(u32, u32)> = (600..=639).map(|x| (x, 120)).collect();
        want.extend((0..=60).map(|x| (x, 121)));
        assert_eq!(set_pixels(&px), want);
    }

    /// Left of the screen runs back into the row above; a vertical line at
    /// x 700 is column 60 one row further down.
    #[test]
    fn clip_off_left_edge_and_vertical_run_on() {
        let (px, _) = walk(&FALCON, (-2, 11, 1, 11), WriteMode::Replace, ClipRect::OFF, 0xFFFF, true);
        assert_eq!(set_pixels(&px), [(638, 10), (639, 10), (0, 11), (1, 11)]);
        let (px, _) = walk(&FALCON, (700, 10, 700, 12), WriteMode::Replace, ClipRect::OFF, 0xFFFF, true);
        assert_eq!(set_pixels(&px), [(60, 11), (60, 12), (60, 13)]);
    }

    /// Decision A (TASK/V18, `docs/COMPATIBILITY.md`): past the end of the
    /// store, and before its start, the line is cut — EmuTOS writes the
    /// memory there. The part inside is drawn as EmuTOS draws it.
    #[test]
    fn clip_off_is_cut_at_the_store_bounds() {
        let (px, _) = walk(&FALCON, (630, 479, 700, 479), WriteMode::Replace, ClipRect::OFF, 0xFFFF, true);
        assert_eq!(set_pixels(&px), (630..=639).map(|x| (x, 479)).collect::<Vec<_>>());
        let (px, _) = walk(&FALCON, (-10, 0, 5, 0), WriteMode::Replace, ClipRect::OFF, 0xFFFF, true);
        assert_eq!(set_pixels(&px), (0..=5).map(|x| (x, 0)).collect::<Vec<_>>());
        let (px, _) = walk(&FALCON, (10, 470, 10, 500), WriteMode::Replace, ClipRect::OFF, 0xFFFF, true);
        assert_eq!(set_pixels(&px), (470..=479).map(|y| (10, y)).collect::<Vec<_>>());
    }

    /// The run-on follows the store's stride, not the visible width: with a
    /// padded pitch the first pixels past the edge land in the padding.
    #[test]
    fn clip_off_runs_on_through_the_pitch_padding() {
        let padded = LineTarget { width: 640, height: 4, stride: 648 };
        let (px, _) = walk(&padded, (638, 0, 650, 0), WriteMode::Replace, ClipRect::OFF, 0xFFFF, true);
        let mut want: Vec<(u32, u32)> = (638..=647).map(|x| (x, 0)).collect();
        want.extend((0..=2).map(|x| (x, 1)));
        assert_eq!(set_pixels(&px), want);
    }

    // ── draw_line16 ──────────────────────────────────────────────────────

    /// EmuTOS `draw_line16` (`vdi/vdi_line.c:1623-1770`), transcribed
    /// independently: the `(x, y, dash bit)` sequence of a segment already
    /// ordered left to right, and the mask after it.
    fn emutos_draw_line16(x1: i16, y1: i16, x2: i16, y2: i16, mask: u16) -> (Vec<(i32, i32, bool)>, u16) {
        let dx = x2 - x1;
        let mut dy = y2 - y1;
        let mut yinc = 1i32;
        if dy < 0 {
            dy = -dy;
            yinc = -1;
        }
        let (mut x, mut y) = (x1 as i32, y1 as i32);
        let mut linemask = mask;
        let mut out = Vec::new();
        if dx >= dy {
            let (e1, e2) = (2 * dy, 2 * dx);
            let mut eps = -dx;
            for _ in 0..=dx {
                linemask = linemask.rotate_left(1);
                out.push((x, y, linemask & 1 != 0));
                x += 1;
                eps += e1;
                if eps >= 0 {
                    eps -= e2;
                    y += yinc;
                }
            }
        } else {
            let (e1, e2) = (2 * dx, 2 * dy);
            let mut eps = -dy;
            for _ in 0..=dy {
                linemask = linemask.rotate_left(1);
                out.push((x, y, linemask & 1 != 0));
                y += yinc;
                eps += e1;
                if eps >= 0 {
                    eps -= e2;
                    x += 1;
                }
            }
        }
        (out, linemask)
    }

    /// Every direction from (100,100) to the border of a 129×129 square,
    /// drawn both ways round, dashed: the walk is EmuTOS' pixel for pixel,
    /// dash bit for dash bit (the probe K3 measured the kernel equal).
    #[test]
    fn general_segments_follow_draw_line16() {
        let mut ends = Vec::new();
        for k in -64i16..=64 {
            ends.extend([(100 + k, 36), (100 + k, 164), (36, 100 + k), (164, 100 + k)]);
        }
        for &(ex, ey) in &ends {
            for &(sx, sy, tx, ty) in &[(100, 100, ex, ey), (ex, ey, 100, 100)] {
                if sx == tx || sy == ty {
                    continue; // vertical and horizontal have their own paths
                }
                let (px, m) = walk(&FALCON, (sx, sy, tx, ty), WriteMode::Replace, ClipRect::OFF, 0xF391, true);
                let (ox1, oy1, ox2, oy2) = if tx < sx { (tx, ty, sx, sy) } else { (sx, sy, tx, ty) };
                let (want, wm) = emutos_draw_line16(ox1, oy1, ox2, oy2, 0xF391);
                let got: Vec<(i32, i32, bool)> = px.iter().map(|p| (p.0 as i32, p.1 as i32, p.2)).collect();
                assert_eq!(got, want, "({sx},{sy})-({tx},{ty})");
                assert_eq!(m, wm, "mask after ({sx},{sy})-({tx},{ty})");
            }
        }
    }

    // ── abline_runs: the same walk as runs (TASK/V58) ────────────────────

    /// What [`abline_runs`] hands out, expanded to `(index, dash bit)` in
    /// order, and which kinds of run it used.
    #[derive(Default)]
    struct Expand {
        px: Vec<(usize, bool)>,
        spans: usize,
        columns: usize,
        pixels: usize,
    }
    impl LineRuns for Expand {
        fn span(&mut self, idx: usize, n: usize, phase: u32, mask: u16) {
            assert!(n > 0, "empty span");
            self.spans += 1;
            for i in 0..n {
                self.px.push((idx + i, mask & (0x8000u16 >> ((phase as usize + i) & 15)) != 0));
            }
        }
        fn column(&mut self, idx: usize, n: usize, step: isize, phase: u32, mask: u16) {
            assert!(n > 0, "empty column");
            self.columns += 1;
            for i in 0..n {
                let at = idx as isize + i as isize * step;
                self.px.push((at as usize, mask & (0x8000u16 >> ((phase as usize + i) & 15)) != 0));
            }
        }
        fn pixel(&mut self, idx: usize, set: bool) {
            self.pixels += 1;
            self.px.push((idx, set));
        }
    }

    /// [`abline`] and [`abline_runs`] on one segment: the same indices, dash
    /// bits and order, the same mask, and the rectangle of the indices.
    #[allow(clippy::too_many_arguments)]
    fn same_walk(t: &LineTarget, seg: (i16, i16, i16, i16), mode: WriteMode, c: ClipRect, mask: u16, last: bool) {
        let (x0, y0, x1, y1) = seg;
        let mut want = Vec::new();
        let wm = abline(t, x0, y0, x1, y1, mode, c, mask, last, |i, set| want.push((i, set)));
        let mut box_want: Option<Touched> = None;
        let s = t.stride as usize;
        for &(i, _) in &want {
            let (col, row) = ((i % s) as u32, (i / s) as u32);
            box_want = Some(match box_want {
                None => (col, row, col, row),
                Some((c0, r0, c1, r1)) => (c0.min(col), r0.min(row), c1.max(col), r1.max(row)),
            });
        }
        let mut got = Expand::default();
        let (gm, gbox) = abline_runs(t, x0, y0, x1, y1, mode, c, mask, last, &mut got);
        let what = std::format!("{t:?} {seg:?} {mode:?} {c:?} mask {mask:04x} last {last}");
        assert_eq!(got.px, want, "pixels of {what}");
        assert_eq!(gm, wm, "mask after {what}");
        assert_eq!(gbox, box_want, "touched rectangle of {what}");
        assert!(got.spans + got.columns <= 1, "one span or column: {what}");
        assert!(got.pixels == 0 || got.spans + got.columns == 0, "runs and pixels mixed: {what}");
    }

    const MODES: [WriteMode; 4] = [WriteMode::Replace, WriteMode::Transparent, WriteMode::Xor, WriteMode::RevTransparent];

    /// xorshift64*, deterministic.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
        fn i16_in(&mut self, lo: i32, hi: i32) -> i16 {
            (lo + (self.next() >> 33) as i32 % (hi - lo + 1)) as i16
        }
    }

    /// Every direction and length 0..=70 from a centre, horizontal and
    /// vertical included, both ways round, XOR with and without `last`, on
    /// the oracle's screen and on a padded pitch, clipping off and on.
    #[test]
    fn runs_walk_as_abline_in_every_direction_and_length() {
        let padded = LineTarget { width: 200, height: 160, stride: 211 };
        for t in [FALCON, padded] {
            for d in -70i16..=70 {
                for e in [-70i16, -64, -33, -17, -16, -15, -2, -1, 0, 1, 2, 15, 16, 17, 33, 64, 70] {
                    for (dx, dy) in [(d, e), (e, d)] {
                        let seg = (100, 80, 100 + dx, 80 + dy);
                        for mode in [WriteMode::Replace, WriteMode::Xor] {
                            for last in [true, false] {
                                for mask in [0xFFFF, 0xF391, 0x8001] {
                                    same_walk(&t, seg, mode, ClipRect::OFF, mask, last);
                                    same_walk(&t, seg, mode, clip(60, 50, 140, 120), mask, last);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Horizontal and vertical segments at every start phase and length, the
    /// mask advanced or not, every write mode.
    #[test]
    fn runs_walk_as_abline_for_spans_and_columns_at_every_phase() {
        for mode in MODES {
            for x in 0i16..=33 {
                for n in 0i16..=70 {
                    for mask in [0xFFFF, 0x0000, 0x8000, 0x0001, 0xAAAA, 0xF391] {
                        for last in [true, false] {
                            same_walk(&FALCON, (x, 7, x + n, 7), mode, ClipRect::OFF, mask, last);
                            same_walk(&FALCON, (x + n, 7, x, 7), mode, ClipRect::OFF, mask, last);
                            same_walk(&FALCON, (x, 9 + n, x, 9), mode, ClipRect::OFF, mask, last);
                            same_walk(&FALCON, (x, 9, x, 9 + n), mode, clip(0, 20, 639, 50), mask, last);
                        }
                    }
                }
            }
        }
    }

    /// Clipping off with endpoints anywhere in `WORD` range on small stores,
    /// padded and not: run-on into the neighbouring rows, cuts at both ends
    /// of the store, `WORD`-wrapped deltas; and clip rectangles anywhere.
    #[test]
    fn runs_walk_as_abline_with_clipping_off_and_wild_endpoints() {
        let mut rng = Rng(0x5635_3800_0000_0001);
        let targets = [
            LineTarget { width: 7, height: 5, stride: 7 },
            LineTarget { width: 7, height: 5, stride: 9 },
            LineTarget { width: 40, height: 30, stride: 48 },
            LineTarget { width: 1, height: 1, stride: 1 },
            LineTarget { width: 640, height: 4, stride: 648 },
        ];
        for t in targets {
            for round in 0..4000 {
                let r = match round % 4 {
                    0 => 12,
                    1 => 100,
                    2 => 3000,
                    _ => 32767,
                };
                let mut p = || rng.i16_in(-r, r);
                let seg = (p(), p(), p(), p());
                let c = if round % 3 == 0 {
                    let (a, b, c2, d) = (p(), p(), p(), p());
                    clip(a.min(c2), b.min(d), a.max(c2), b.max(d))
                } else {
                    ClipRect::OFF
                };
                let mode = MODES[round % 4];
                let mask = (rng.next() >> 48) as u16;
                same_walk(&t, seg, mode, c, mask, round % 5 != 0);
            }
        }
    }

    /// A store of no pixels takes nothing and still rotates the mask.
    #[test]
    fn runs_walk_as_abline_on_an_empty_store() {
        for t in [LineTarget { width: 0, height: 0, stride: 0 }, LineTarget { width: 8, height: 0, stride: 8 }] {
            for seg in [(0, 0, 5, 0), (0, 0, 0, 5), (0, 0, 5, 3), (-3, 2, 9, -4)] {
                same_walk(&t, seg, WriteMode::Replace, ClipRect::OFF, 0x1234, true);
            }
        }
    }
}
