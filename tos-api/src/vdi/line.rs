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
}
