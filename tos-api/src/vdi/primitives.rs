//! VDI output primitives — `vr_recfl` (filled rectangle) and `v_pline`
//! (polyline). Spec: `docs/api/vdi/output/`.
//!
//! The geometry/attribute handling lives here (hardware-independent);
//! the actual pixels go through the [`RasterBackend`]. Coordinates are
//! raster (RC, origin top-left), matching the screen workstation.

use core::cell::UnsafeCell;

use super::backend::{ClipRect, PatternFill, Pel, RasterBackend, WriteMode};
use super::fill_patterns;
use super::vwk::Vwk;
use super::Vdipb;

/// The six built-in line-style dash masks (`vdi_line.c LINE_STYLE`);
/// index 6 (style 7) is the user mask `ud_ls`.
const LINE_STYLE: [u16; 6] = [0xFFFF, 0xFFF0, 0xC0C0, 0xFF18, 0xFF00, 0xF191];

/// The active line dash mask for the workstation's line style.
pub(super) fn line_mask(vwk: &Vwk) -> u16 {
    let idx = vwk.line_index;
    if (0..6).contains(&idx) {
        LINE_STYLE[idx as usize]
    } else {
        vwk.ud_ls // style 7 → user-defined
    }
}

/// Line end styles (EmuTOS `vdi_defs.h:89-91`): `SQUARED`, `ARROWED`,
/// `ROUND`. `line_beg`/`line_end` hold one of these.
const SQUARED: i16 = 0;
const ARROWED: i16 = 1;

/// Maximum wide-line width (`vdi_defs.h:53`, `SIZ_TAB[6]`) and the derived
/// circle-DDA row count (`vdi_line.c:54-55`). `MAX_PIXEL_ASPECT_RATIO` = 2.
const MAX_LINE_WIDTH: i16 = 40;
const MAX_QC_LINES: usize = (MAX_LINE_WIDTH as usize * 2) / 2 + 1; // 41

/// A single solid pattern row (`vdi_fill.c SOLID`), used to fill the
/// wide-line boxes / arrow triangles / round joints.
const SOLID_PATTERN: [u16; 1] = [0xFFFF];

/// Draw a width-1 polyline through `n` `(x,y)` pairs, threading the dash
/// mask across the whole polyline exactly as EmuTOS' `polyline`
/// (`vdi_line.c:1101-1121`) threads the persistent `LN_MASK`: each segment
/// starts from the mask the previous segment left, and the last segment
/// sets `LSTLIN` so the backend applies the XOR shared-vertex kludge only
/// there. The caller passes the *initial* mask (EmuTOS `set_LN_MASK` value)
/// — resetting once at the start, never per segment.
///
/// **Shared-vertex dash drift is EmuTOS-faithful.** Every segment draws
/// both endpoints inclusively (`abline`), so the vertex shared with the
/// next segment is plotted twice and the dash mask rotates one extra step
/// there — a straight line of the same length is NOT an equivalent oracle;
/// EmuTOS' bent dashed polyline drifts by exactly one mask step after each
/// inner vertex. The endpoint decrement that would suppress this is
/// **`WM_XOR`-only** (`vdi_line.c:1789-1798,2319`: the DRI kludge exists
/// solely to stop a polyline XOR'ing itself at intermediate points); in the
/// dash / replace path it is deliberately not applied, so the vertex is
/// drawn twice on purpose.
///
/// `clip` is the *line* clip: the workstation rectangle when clipping is on,
/// [`ClipRect::OFF`] when it is off ([`line_clip`]) — EmuTOS clips each
/// segment with `clip_line` only `if (vwk->clip)` (`vdi_line.c:1115-1117`),
/// and the backend's `abline` does exactly that per segment
/// ([`crate::vdi::line::abline`]).
pub(super) fn polyline(
    backend: &dyn RasterBackend,
    n: usize,
    mut pt: impl FnMut(usize) -> (i16, i16),
    color: Pel,
    mode: WriteMode,
    clip: ClipRect,
    mut mask: u16,
) {
    if n < 2 {
        return;
    }
    for i in 0..n - 1 {
        let (x0, y0) = pt(i);
        let (x1, y1) = pt(i + 1);
        let last = i == n - 2; // EmuTOS sets LSTLIN only for the final segment
        mask = backend.abline(x0, y0, x1, y1, color, mode, clip, mask, last);
    }
}

/// The clip a width-1 line of `vwk` is drawn with: its rectangle when
/// clipping is on, [`ClipRect::OFF`] when it is off — EmuTOS `polyline`
/// calls `clip_line` only `if (vwk->clip)` (`vdi_line.c:1115-1117`). With
/// clipping off `vs_clip` leaves the rectangle at the full screen
/// (`vdi_control.c:245-249`), which the fills keep using; a line then runs
/// unclipped (V20).
pub(super) fn line_clip(vwk: &Vwk) -> ClipRect {
    if vwk.clip {
        vwk.clip_rect()
    } else {
        ClipRect::OFF
    }
}

// ── integer maths (EmuTOS intmath.c / miscasm.S), reimplemented for AArch64 ──

/// EmuTOS `mul_div` (`intmath.h:51`): `(m1 * m2) / d`, truncating toward
/// zero, with a LONG intermediate.
pub(super) fn mul_div(m1: i32, m2: i32, d: i32) -> i32 {
    if d == 0 {
        return 0;
    }
    m1 * m2 / d
}

/// EmuTOS `mul_div_round` (`miscasm.S:150`): `(m1 * m2) / d` rounded away
/// from zero, computed as `((m1*m2*2 / d) ± 1) >> 1` (arithmetic shift).
fn mul_div_round(m1: i32, m2: i32, d: i32) -> i32 {
    if d == 0 {
        return 0;
    }
    let t = m1 * m2 * 2 / d;
    if t >= 0 {
        (t + 1) >> 1
    } else {
        (t - 1) >> 1
    }
}

/// EmuTOS `Isqrt` (`intmath.c:18`): integer square root rounded to nearest.
fn isqrt(a: u32) -> u32 {
    if a == 0 {
        return 0;
    }
    let a = a - 1; // converges to (a + a_LSB)^1/2, so decrement first
    let mut x = a;
    let mut y = 8u32;
    while x > y {
        x /= 2;
        y *= 2;
    }
    y = (2 + x + y / 2) / 4; // first estimate
    // Positive-bias rounded Newton-Raphson iterator.
    while x != y {
        x = y;
        y = (1 + a / y + y) / 2;
    }
    y
}

/// The device pixel dimensions `(xsize, ysize)` = `DEV_TAB[3]/[4]`
/// (`vdi_defs.h:96-97`), used for the wide-line aspect correction. Our
/// screen workstation always reports **square pixels** (see
/// [`super::workstation::get_std_pixel_size`], 278/278 for the live
/// ≥640×≥400 framebuffer).
fn pixel_aspect(backend: &dyn RasterBackend) -> (i32, i32) {
    let (xs, ys) = super::workstation::get_std_pixel_size(backend.width() as i16, backend.height() as i16);
    (xs as i32, ys as i32)
}

/// EmuTOS `cir_dda` (`vdi_line.c:1240`): build the quarter-circle DDA that
/// fattens and rounds a wide line of diameter `line_width`. `q[k]` is the
/// horizontal offset of the circle edge on the k-th row from the centre;
/// the return is `num_qc_lines` (rows in the quarter).
///
/// Because our screen reports **square pixels** (`xsize == ysize`, see
/// [`pixel_aspect`]) this takes EmuTOS' `xsize == ysize` early-return path
/// (Bresenham circle, `:1251-1271`); the tall/wide-pixel aspect averaging
/// (`:1279-1311`) is unreachable on our device and therefore not carried.
fn cir_dda(line_width: i16, xsize: i32, ysize: i32) -> ([i16; MAX_QC_LINES], usize) {
    let lw = line_width.clamp(1, MAX_LINE_WIDTH);
    let mut q = [0i16; MAX_QC_LINES];
    let mut num = (lw as i32 * xsize / ysize) / 2 + 1;
    if num > MAX_QC_LINES as i32 {
        num = MAX_QC_LINES as i32; // circles will be flattened
    }

    // Bresenham circle octant; radius = (line_width + 1) / 2.
    let mut x = 0i16;
    let mut y = (lw + 1) / 2;
    let mut d = 3 - 2 * y;
    while x <= y {
        q[y as usize] = x;
        q[x as usize] = y;
        if d < 0 {
            d += 4 * x + 6;
        } else {
            d += 4 * (x - y) + 10;
            y -= 1;
        }
        x += 1;
    }
    (q, num as usize)
}

/// EmuTOS `quad_xform` (`vdi_line.c:1133`): mirror `(x,y)` into/out of the
/// first quadrant per `quad` (1=SE, 2=SW, 3=NW, 4=NE).
fn quad_xform(quad: i16, x: i16, y: i16) -> (i16, i16) {
    let tx = if quad == 2 || quad == 3 { -x } else { x };
    let ty = if quad == 3 || quad == 4 { -y } else { y };
    (tx, ty)
}

/// EmuTOS `perp_off` (`vdi_line.c:1169`): given a segment vector `(vx,vy)`,
/// return the perpendicular half-width offsets forming one wide-line box
/// edge, found by a DDA walk of the quarter circle `q`.
///
/// The magnitude comparison is deliberately truncated to 16 bits
/// (`WORD magnitude`, `:1172,1200`) to match EmuTOS bit-for-bit — it can
/// overflow for a long segment, which selects a slightly different circle
/// point; short (typical) segments never overflow.
fn perp_off(vx: i16, vy: i16, q: &[i16; MAX_QC_LINES], num_qc_lines: usize) -> (i16, i16) {
    // Mirror the vector into the first quadrant.
    let quad = if vx >= 0 {
        if vy >= 0 {
            1
        } else {
            4
        }
    } else if vy >= 0 {
        2
    } else {
        3
    };
    let (qx, qy) = quad_xform(quad, vx, vy);
    let (x, y) = (qx as i32, qy as i32);

    let mut min_val = 32767i32;
    let mut x_val = 0i32;
    let mut y_val = 0i32;
    let mut u = q[0] as i32;
    let mut v = 0i32;
    loop {
        // ABS of the full 32-bit product, then truncated to WORD (as EmuTOS).
        let magnitude = ((u * y - v * x).abs() as i16) as i32;
        if magnitude < min_val
            || (magnitude == min_val && (x_val - y_val).abs() > (u - v).abs())
        {
            min_val = magnitude;
            x_val = u;
            y_val = v;
        } else {
            break;
        }
        // Step to the next pixel of the quarter circle.
        if v == num_qc_lines as i32 - 1 {
            if u == 1 {
                break;
            }
            u -= 1;
        } else if q[(v + 1) as usize] as i32 >= u - 1 {
            v += 1;
            u = q[v as usize] as i32;
        } else {
            u -= 1;
        }
    }
    quad_xform(quad, x_val as i16, y_val as i16)
}

/// Draw a solid horizontal span `[x1..=x2]` at `y` in the line colour and
/// writing mode (EmuTOS `horzline` → `draw_rect`, solid). The backend's
/// `abline` clips the row's endpoints against `clip` — the `clip_line`
/// `do_circ` runs on every row whatever `vwk->clip` says
/// (`vdi_line.c:1335,1346`) — and draws it screen-aligned, solid.
fn horzspan(
    backend: &dyn RasterBackend,
    x1: i16,
    x2: i16,
    y: i16,
    color: Pel,
    mode: WriteMode,
    clip: ClipRect,
) {
    backend.abline(x1, y, x2, y, color, mode, clip, 0xFFFF, true);
}

/// EmuTOS `do_circ` (`vdi_line.c:1322`): draw a filled disc of the current
/// wide-line radius centred at `(cx,cy)` — a round joint between segments
/// or a round cap. Each row is a solid horizontal span.
#[allow(clippy::too_many_arguments)]
fn do_circ(
    backend: &dyn RasterBackend,
    cx: i16,
    cy: i16,
    q: &[i16; MAX_QC_LINES],
    num_qc_lines: usize,
    color: Pel,
    mode: WriteMode,
    clip: ClipRect,
) {
    for (k, &off) in q.iter().enumerate().take(num_qc_lines) {
        // Upper semicircle plus the centre row.
        horzspan(backend, cx - off, cx + off, cy - k as i16, color, mode, clip);
        if k == 0 {
            continue;
        }
        // Lower semicircle.
        horzspan(backend, cx - off, cx + off, cy + k as i16, color, mode, clip);
    }
}

/// EmuTOS `polygon` under the wide-line attribute environment
/// (`vdi_fill.c:604-608` with `s_fa_attr`): fill the convex polygon solid
/// in `color`, then draw its closed perimeter solid (width 1). Used for the
/// wide-line boxes and the arrow triangles. `clip` bounds the fill,
/// `line_clip` ([`line_clip`]) the perimeter.
fn polygon(
    backend: &dyn RasterBackend,
    pts: &[(i16, i16)],
    n: usize,
    color: Pel,
    mode: WriteMode,
    clip: ClipRect,
    line_clip: ClipRect,
) {
    let fill = PatternFill {
        fg: color,
        bg: 0,
        pattern: &SOLID_PATTERN,
        mode,
    };
    fill_polygon(backend, n, pts, fill, clip);
    // Perimeter: close back to the first vertex, solid width-1 (LN_MASK
    // forced to 0xffff by EmuTOS' polygon()).
    polyline(backend, n + 1, |i| pts[i % n], color, mode, line_clip, 0xFFFF);
}

/// EmuTOS `wideline` (`vdi_line.c:1389`): draw a polyline of width > 1 as a
/// chain of perpendicular parallelogram boxes with round joints between
/// segments and round/arrow caps at the ends. `pts[0..count]` are the
/// vertices; the array is mutated by the arrow shortening. `clip` bounds
/// the fills and the round joints (`do_circ` clips its rows with `clip_line`
/// whatever `vwk->clip` says, `vdi_line.c:1335,1346`), `line_clip` the
/// width-1 perimeters ([`line_clip`]).
#[allow(clippy::too_many_arguments)]
fn wideline(
    backend: &dyn RasterBackend,
    pts: &mut [(i16, i16)],
    count: usize,
    line_width: i16,
    line_beg: i16,
    line_end: i16,
    color: Pel,
    mode: WriteMode,
    clip: ClipRect,
    line_clip: ClipRect,
    xsize: i32,
    ysize: i32,
) {
    if count < 2 {
        return;
    }
    let (q, num_qc_lines) = cir_dda(line_width, xsize, ysize);

    // Arrows first — they shorten the polyline (EmuTOS `wideline:1407`).
    if (line_beg | line_end) & ARROWED != 0 {
        arrow(backend, pts, count, line_beg, line_end, line_width, color, mode, clip, line_clip, xsize, ysize);
    }

    let (mut wx1, mut wy1) = pts[0];
    // A closed polyline has its last vertex equal to the first.
    let closed = pts[count - 1] == (wx1, wy1);

    // Round cap at the start if the begin style is not squared, or closed.
    if line_beg != SQUARED || closed {
        do_circ(backend, wx1, wy1, &q, num_qc_lines, color, mode, clip);
    }

    for (i, &(wx2, wy2)) in pts.iter().enumerate().take(count).skip(1) {
        let vx = wx2 as i32 - wx1 as i32;
        let vy = wy2 as i32 - wy1 as i32;
        // Ignore zero-length segments.
        if vx == 0 && vy == 0 {
            continue;
        }

        // Perpendicular half-width offsets that fatten this segment.
        let (ox, oy) = if vx == 0 {
            (q[0], 0) // vertical line
        } else if vy == 0 {
            (0, num_qc_lines as i16 - 1) // horizontal line
        } else {
            let k = mul_div(-vy, ysize, xsize);
            let pvy = mul_div(vx, xsize, ysize);
            perp_off(k as i16, pvy as i16, &q, num_qc_lines)
        };

        let box_pts = [
            (wx1 + ox, wy1 + oy),
            (wx1 - ox, wy1 - oy),
            (wx2 - ox, wy2 - oy),
            (wx2 + ox, wy2 + oy),
        ];
        polygon(backend, &box_pts, 4, color, mode, clip, line_clip);

        // Round joint at an internal vertex, or a round/closed end cap.
        if i < count - 1 || line_end != SQUARED || closed {
            do_circ(backend, wx2, wy2, &q, num_qc_lines, color, mode, clip);
        }

        wx1 = wx2;
        wy1 = wy2;
    }
}

/// EmuTOS `draw_arrow` (`vdi_line.c:1503`): draw one arrowhead at the tip
/// vertex `tip`, walking toward the interior by `inc` (+1 for the begin
/// end, -1 for the end). Mutates `pts` to shorten the line behind the head.
#[allow(clippy::too_many_arguments)]
fn draw_arrow(
    backend: &dyn RasterBackend,
    pts: &mut [(i16, i16)],
    count: usize,
    tip: usize,
    inc: isize,
    line_width: i16,
    color: Pel,
    mode: WriteMode,
    clip: ClipRect,
    line_clip: ClipRect,
    xsize: i32,
    ysize: i32,
) {
    let arrow_len = if line_width < 4 { 8 } else { 3 * line_width as i32 - 1 };
    let arrow_wid = arrow_len / 2;

    let (tx, ty) = (pts[tip].0 as i32, pts[tip].1 as i32);

    // Find the first vertex far enough from the tip not to be obscured by
    // the head (distance measured in aspect-uniform space).
    let mut dx = 0i32;
    let mut dy = 0i32;
    let mut line_len2 = 0i64;
    let mut cur = tip as isize;
    for _ in 1..count {
        cur += inc;
        if cur < 0 || cur >= count as isize {
            break;
        }
        let (px, py) = pts[cur as usize];
        dx = tx - px as i32;
        dy = mul_div_round(ty - py as i32, ysize, xsize);
        line_len2 = dx as i64 * dx as i64 + dy as i64 * dy as i64;
        if line_len2 >= arrow_len as i64 * arrow_len as i64 {
            break;
        }
    }
    let xybeg = cur;
    let line_len = isqrt(line_len2 as u32) as i32;
    if line_len < arrow_len {
        return; // longest vector too short — no arrow
    }

    // Rotate the head height/base vectors (calculations in 1000x space).
    let dxfactor = mul_div_round(dx, 1000, line_len);
    let dyfactor = mul_div_round(dy, 1000, line_len);
    let ht_x = mul_div_round(arrow_len, dxfactor, 1000);
    let mut ht_y = mul_div_round(arrow_len, dyfactor, 1000);
    let base_x = mul_div_round(arrow_wid, -dyfactor, 1000);
    let mut base_y = mul_div_round(arrow_wid, dxfactor, 1000);
    // Transform the y offsets back to the real aspect-ratio space.
    ht_y = mul_div_round(ht_y, xsize, ysize);
    base_y = mul_div_round(base_y, xsize, ysize);

    let triangle = [
        ((tx + base_x - ht_x) as i16, (ty + base_y - ht_y) as i16),
        ((tx - base_x - ht_x) as i16, (ty - base_y - ht_y) as i16),
        (tx as i16, ty as i16),
    ];
    polygon(backend, &triangle, 3, color, mode, clip, line_clip);

    // Shorten: move the tip inward by the head height, and collapse every
    // skipped vertex onto it, so the line does not protrude past the head.
    let new_tip = ((tx - ht_x) as i16, (ty - ht_y) as i16);
    pts[tip] = new_tip;
    let mut p = xybeg;
    loop {
        p -= inc;
        if p == tip as isize {
            break;
        }
        pts[p as usize] = new_tip;
    }
}

/// EmuTOS `arrow` (`vdi_line.c:1597`): draw arrowheads at whichever ends of
/// the polyline have the `ARROWED` style. Mutates `pts` (see `draw_arrow`).
#[allow(clippy::too_many_arguments)]
fn arrow(
    backend: &dyn RasterBackend,
    pts: &mut [(i16, i16)],
    count: usize,
    line_beg: i16,
    line_end: i16,
    line_width: i16,
    color: Pel,
    mode: WriteMode,
    clip: ClipRect,
    line_clip: ClipRect,
    xsize: i32,
    ysize: i32,
) {
    if line_beg & ARROWED != 0 {
        draw_arrow(backend, pts, count, 0, 1, line_width, color, mode, clip, line_clip, xsize, ysize);
    }
    if line_end & ARROWED != 0 {
        draw_arrow(backend, pts, count, count - 1, -1, line_width, color, mode, clip, line_clip, xsize, ysize);
    }
}

/// Stroke a polyline honouring the current line width, reproducing EmuTOS
/// `clc_arc`'s open-arc dispatch (`vdi_gdp.c:210-218`): width 1 threads the
/// dash `mask` through `polyline`, then draws arrowheads at whichever ends
/// are `ARROWED` (`arrow(vwk, point, steps)`, `vdi_gdp.c:215-216`); width > 1
/// draws the wide line (round joins/caps, arrows handled inside `wideline`).
/// `pts[0..count]` is mutable because the arrow shortening writes back into
/// it. Used by the GDP curve / rounded-box outlines; the rounded box forces
/// `line_beg`/`line_end` to `SQUARED` (EmuTOS `vdi_v_gdp` case 8), so no
/// arrowhead fires there.
#[allow(clippy::too_many_arguments)]
pub(super) fn stroke(
    backend: &dyn RasterBackend,
    pts: &mut [(i16, i16)],
    count: usize,
    vwk: &Vwk,
    color: Pel,
    mode: WriteMode,
    clip: ClipRect,
    mask: u16,
) {
    let lclip = line_clip(vwk);
    if vwk.line_width == 1 {
        polyline(backend, count, |i| pts[i], color, mode, lclip, mask);
        // Arrowheads on the width-1 stroke (EmuTOS clc_arc calls
        // `arrow(vwk, point, steps)` after the polyline).
        if (vwk.line_beg | vwk.line_end) & ARROWED != 0 {
            let (xsize, ysize) = pixel_aspect(backend);
            arrow(
                backend, pts, count, vwk.line_beg, vwk.line_end, vwk.line_width, color, mode, clip,
                lclip, xsize, ysize,
            );
        }
    } else {
        let (xsize, ysize) = pixel_aspect(backend);
        wideline(
            backend,
            pts,
            count,
            vwk.line_width,
            vwk.line_beg,
            vwk.line_end,
            color,
            mode,
            clip,
            lclip,
            xsize,
            ysize,
        );
    }
}

/// Opcode 114 — `vr_recfl`: fill the rectangle `PTSIN[0..3]` with the
/// current fill style (interior + pattern/hatch + colour). EmuTOS
/// `vdi_vr_recfl` → `draw_rect` → `draw_rect_common` (`vdi_fill.c:293-314`).
///
/// The active pattern comes from [`fill_patterns::select`]; the writing
/// mode chooses opaque (Replace → clear bits become background pen 0) vs.
/// transparent. `HOLLOW` is **not** a no-op — its all-zero pattern erases
/// the rectangle to pen 0 in Replace mode and paints the fill colour in
/// Reverse-transparent mode (EmuTOS `st_fl_ptr` HOLLOW `patptr=&HOLLOW`,
/// `swblit_rect_common16`). The backend clips to the workstation
/// rectangle. Wide-line perimeter (`vsf_perimeter`) is handled by the GDP
/// bar, not here.
pub fn vr_recfl(pb: &Vdipb, vwk: &Vwk, backend: &dyn RasterBackend) {
    let pattern = fill_patterns::select(vwk);
    // Normalise the two corners (arb_corner); the backend clips.
    let mut x1 = pb.ptsin(0);
    let mut y1 = pb.ptsin(1);
    let mut x2 = pb.ptsin(2);
    let mut y2 = pb.ptsin(3);
    if x1 > x2 {
        core::mem::swap(&mut x1, &mut x2);
    }
    if y1 > y2 {
        core::mem::swap(&mut y1, &mut y2);
    }
    let fill = PatternFill {
        fg: vwk.fill_color,
        bg: 0, // background = colour 0
        pattern,
        mode: WriteMode::from_wrt_mode(vwk.wrt_mode),
    };
    backend.fill_pattern(x1, y1, x2 - x1 + 1, y2 - y1 + 1, fill, vwk.clip_rect());
}

/// Maximum polygon vertices / scanline edge crossings handled by the
/// shared fill buffers. This is EmuTOS' `MAX_VERTICES`, and it is the same
/// number that is advertised as `INQ_TAB[14]` (see [`super::workstation`],
/// which references this constant) and that bounds the `CONTRL[1]` vertex
/// count.
///
/// **1024, the default.** `config.h` only overrides `MAX_VERTICES` to 512
/// inside the `TARGET_192` / `TARGET_256` / `TARGET_CART` / `TARGET_FLOPPY`
/// blocks (`include/config.h:388-390,445-447,548-550,560-562`). The
/// Falcon030/TOS4 reference build — `make 512` — defines `TARGET_512`
/// (`Makefile:561`), but **`config.h` never tests `TARGET_512`**, so that
/// build takes the default `#define MAX_VERTICES 1024`
/// (`include/config.h:1652-1655`, *"TOS2 allows 512 vertices, TOS3/TOS4
/// allow 1024"*). The measured EmuTOS 1.4 reference on a TT reports 1024 in
/// `INQ_TAB[14]` (`tosdump/reference/EMUTOS14-VDI-r3.BIN`, cases 117/119),
/// which is the same value from the other direction.
pub(super) const MAX_VERTS: usize = 1024;

/// The VDI's shared polygon work area — NewTOS' equivalent of EmuTOS'
/// `VDISHARE.main` (`include/vdiext.h:47-51`), which likewise holds
/// `local_ptsin[2*MAX_VERTICES]` and `fill_buffer[MAX_VERTICES]` in one
/// static area instead of on the stack. At `MAX_VERTS = 1024` the two are
/// 4 KiB + 2 KiB; VDI traps from EL0 run on the 16 KiB `SP_EL1` exception
/// stack (`kernel/src/arch/aarch64/exception_vectors.s:80-84`), which is
/// not the place for them.
struct PolyScratch {
    /// The caller's `PTSIN` vertices, copied out because `wideline`/`arrow`
    /// need random access and mutate them (EmuTOS `local_ptsin`).
    pts: UnsafeCell<[(i16, i16); MAX_VERTS]>,
    /// `clc_flit`'s edge crossings for one scan line (EmuTOS `fill_buffer`).
    xs: UnsafeCell<[i16; MAX_VERTS]>,
}

// SAFETY: accessed only from the single-CPU, IRQ-off VDI trap path, like the
// rest of the VDI's mutable state (see `vwk::TABLE` and `contour::QUEUE`).
// `pts` and `xs` are distinct cells, so `v_fillarea` holding `pts` while
// `fill_polygon` holds `xs` is not aliasing; neither buffer is used by two
// live borrows, because no VDI entry point that takes one calls another.
unsafe impl Sync for PolyScratch {}

static POLY: PolyScratch = PolyScratch {
    pts: UnsafeCell::new([(0, 0); MAX_VERTS]),
    xs: UnsafeCell::new([0; MAX_VERTS]),
};

/// Scan-convert and fill the closed polygon `v[0..n]` with `fill`,
/// clipped to `clip`, reproducing EmuTOS `polygon` + `clc_flit`
/// (`vdi_fill.c:434-610`). Even-odd rule; each scanline span is a
/// 1-pixel-tall patterned rectangle, so the pattern stays screen-aligned.
pub(super) fn fill_polygon(
    backend: &dyn RasterBackend,
    n: usize,
    v: &[(i16, i16)],
    fill: PatternFill,
    clip: ClipRect,
) {
    if n < 3 {
        return;
    }
    let n = n.min(MAX_VERTS);
    // Vertex y-extent (EmuTOS `polygon` fill_miny/fill_maxy, `vdi_fill.c:568-580`).
    let (mut fmin, mut fmax) = (i16::MAX, i16::MIN);
    for &(_, y) in &v[..n] {
        fmin = fmin.min(y);
        fmax = fmax.max(y);
    }
    // Clip the y-extent to the workstation rectangle exactly as EmuTOS
    // `polygon` (`vdi_fill.c:584-591`): a partial top overlap lowers the
    // top bound to `ymn_clip - 1`, so `clc_flit`'s `y > end` loop still
    // fills down to `ymn_clip` inclusive (the clipped top edge agrees with
    // the unclipped case).
    if fmax < clip.ymn || fmin > clip.ymx {
        return; // polygon entirely outside the clip band
    }
    if fmin < clip.ymn {
        fmin = clip.ymn - 1;
    }
    if fmax > clip.ymx {
        fmax = clip.ymx;
    }
    // `clc_flit`'s crossings buffer, out of the shared work area.
    // SAFETY: see [`PolyScratch`] — single trap path, and no caller of
    // `fill_polygon` holds `POLY.xs` at the same time.
    let xs = unsafe { &mut *POLY.xs.get() };
    // `clc_flit`: `for (y = fmax; y > fmin; y--)` (`vdi_fill.c:441`) — the
    // topmost vertex scan line (`fmin`) is deliberately skipped, so the
    // filled band is the half-open `(fmin, fmax]`. We iterate upward over
    // the identical set of scan lines.
    for y in (fmin + 1)..=fmax {
        let yc = y as i32;
        // Collect edge crossings at scan line `y`.
        let mut cnt = 0usize;
        for i in 0..n {
            let (ex1, ey1) = v[i]; // point[i]
            let (ex2, ey2) = v[(i + 1) % n]; // point[i+1]
            let dy = ey2 as i32 - ey1 as i32;
            if dy == 0 {
                continue; // horizontal edge is ignored (`clc_flit:454`)
            }
            let dy1 = yc - ey1 as i32; // y - y1
            let dy2 = yc - ey2 as i32; // y - y2
            // Opposite signs ⇒ the scan line crosses this edge; the top
            // vertex is inclusive, the bottom exclusive (`clc_flit:470`,
            // `(dy1^dy2) < 0`).
            if (dy1 ^ dy2) < 0 {
                // `dx = (x2 - x1) << 1` so the +1/>>1 below rounds the
                // intersection (`clc_flit:475-487`); take the endpoint on
                // the side dictated by the sign of dx.
                let dx = (ex2 as i32 - ex1 as i32) << 1;
                let x = if dx < 0 {
                    ((mul_div(dy2, dx, dy) + 1) >> 1) + ex2 as i32
                } else {
                    ((mul_div(dy1, dx, dy) + 1) >> 1) + ex1 as i32
                };
                if cnt < MAX_VERTS {
                    xs[cnt] = x as i16;
                    cnt += 1;
                }
            }
        }
        // Sort the crossings ascending (EmuTOS `bub_sort`); insertion sort.
        for i in 1..cnt {
            let key = xs[i];
            let mut j = i;
            while j > 0 && xs[j - 1] > key {
                xs[j] = xs[j - 1];
                j -= 1;
            }
            xs[j] = key;
        }
        // Fill between consecutive pairs (perimeter pixels included); the
        // backend clips x to the workstation rectangle.
        let mut i = 0;
        while i + 1 < cnt {
            let xa = xs[i];
            let xb = xs[i + 1];
            if xb >= xa {
                backend.fill_pattern(xa, y, xb - xa + 1, 1, fill, clip);
            }
            i += 2;
        }
    }
}

/// Opcode 9 — `v_fillarea`: fill the closed polygon `PTSIN[0..2*n]` with
/// the current interior style. If `fill_per`, outline the perimeter.
pub fn v_fillarea(pb: &Vdipb, vwk: &Vwk, backend: &dyn RasterBackend) {
    let n = pb.contrl(1).max(0) as usize;
    if n < 3 {
        return;
    }
    let n = n.min(MAX_VERTS);
    // SAFETY: see [`PolyScratch`]. `fill_polygon` below borrows `POLY.xs`,
    // a different cell, so the two do not alias.
    let v = unsafe { &mut *POLY.pts.get() };
    for (i, slot) in v[..n].iter_mut().enumerate() {
        *slot = (pb.ptsin(2 * i), pb.ptsin(2 * i + 1));
    }
    let clip = vwk.clip_rect();
    let mode = WriteMode::from_wrt_mode(vwk.wrt_mode);
    // EmuTOS `polygon` always fills the interior (`clc_flit`), then draws
    // the perimeter when `fill_per`. HOLLOW is not skipped — its all-zero
    // pattern erases to pen 0 / paints the fill colour per the write mode.
    let fill = PatternFill {
        fg: vwk.fill_color,
        bg: 0,
        pattern: fill_patterns::select(vwk),
        mode,
    };
    fill_polygon(backend, n, v, fill, clip);
    if vwk.fill_per {
        // Outline: the polygon closed back to the first vertex, in the
        // fill colour and the workstation writing mode (EmuTOS `polygon`
        // → `polyline(vwk, ..., vwk->fill_color)`, `vdi_fill.c:608`).
        polyline(backend, n + 1, |i| v[i % n], vwk.fill_color, mode, line_clip(vwk), 0xFFFF);
    }
}

/// Opcode 6 — `v_pline`: draw the polyline through `CONTRL[1]` vertices in
/// `PTSIN`. EmuTOS `vdi_v_pline` (`vdi_line.c:1000`): reset `LN_MASK` once,
/// then width 1 → threaded `polyline` (+ arrowheads if either end is
/// `ARROWED`), width > 1 → `wideline` (perpendicular boxes, round joins/caps,
/// arrows).
pub fn v_pline(pb: &Vdipb, vwk: &Vwk, backend: &dyn RasterBackend) {
    let raw = pb.contrl(1);
    if raw < 2 {
        return;
    }
    let count = (raw as usize).min(MAX_VERTS);
    // Read the vertices into a local array: `wideline`/`arrow` mutate them
    // (the arrow shortening), and `wideline` needs random access.
    // SAFETY: see [`PolyScratch`]. `v_pline` and `v_fillarea` are separate
    // VDI entry points; neither calls the other, so `POLY.pts` has one
    // live borrow.
    let pts = unsafe { &mut *POLY.pts.get() };
    for (i, slot) in pts[..count].iter_mut().enumerate() {
        *slot = (pb.ptsin(2 * i), pb.ptsin(2 * i + 1));
    }
    let color = vwk.line_color;
    let mode = WriteMode::from_wrt_mode(vwk.wrt_mode);
    let clip = vwk.clip_rect();

    let lclip = line_clip(vwk);

    if vwk.line_width == 1 {
        polyline(backend, count, |i| pts[i], color, mode, lclip, line_mask(vwk));
        if (vwk.line_beg | vwk.line_end) & ARROWED != 0 {
            let (xsize, ysize) = pixel_aspect(backend);
            arrow(
                backend, pts, count, vwk.line_beg, vwk.line_end, vwk.line_width, color, mode, clip, lclip, xsize,
                ysize,
            );
        }
    } else {
        let (xsize, ysize) = pixel_aspect(backend);
        wideline(
            backend, pts, count, vwk.line_width, vwk.line_beg, vwk.line_end, color, mode, clip, lclip, xsize, ysize,
        );
    }
}

/// The six marker glyphs as polyline vector tables (`vdi_marker.c`):
/// `[num_polylines, (count, dx,dy …) …]`, scaled by `mark_scale`.
const M_DOT: &[i16] = &[1, 2, 0, 0, 0, 0];
const M_PLUS: &[i16] = &[2, 2, 0, -3, 0, 3, 2, -4, 0, 4, 0];
const M_STAR: &[i16] = &[3, 2, 0, -3, 0, 3, 2, 3, 2, -3, -2, 2, 3, -2, -3, 2];
const M_SQUARE: &[i16] = &[1, 5, -4, -3, 4, -3, 4, 3, -4, 3, -4, -3];
const M_CROSS: &[i16] = &[2, 2, -4, -3, 4, 3, 2, -4, 3, 4, -3];
const M_DMND: &[i16] = &[1, 5, -4, 0, 0, -3, 4, 0, 0, 3, -4, 0];
const MARKERS: [&[i16]; 6] = [M_DOT, M_PLUS, M_STAR, M_SQUARE, M_CROSS, M_DMND];

/// Opcode 7 — `v_pmarker`: draw a marker glyph at each of the `CONTRL[1]`
/// points in `PTSIN`. Markers use solid, width-1 lines in `mark_color`,
/// scaled by `mark_scale`; clipping is always on.
pub fn v_pmarker(pb: &Vdipb, vwk: &Vwk, backend: &dyn RasterBackend) {
    let count = pb.contrl(1);
    if count < 1 {
        return;
    }
    let glyph = MARKERS[(vwk.mark_index.clamp(0, 5)) as usize];
    let scale = vwk.mark_scale.max(1);
    // Markers always clip: `vdi_v_pmarker` sets `vwk->clip = 1` before its
    // polylines (`vdi_marker.c:113`), so the line clip is the rectangle —
    // the full screen when the workstation's clipping is off.
    let clip = vwk.clip_rect();
    // Markers honour the workstation writing mode (EmuTOS `v_pmarker`
    // borrows the polyline path, which draws with `vwk->wrt_mode`).
    let mode = WriteMode::from_wrt_mode(vwk.wrt_mode);
    for p in 0..count as usize {
        let (cx, cy) = (pb.ptsin(2 * p), pb.ptsin(2 * p + 1));
        let mut k = 1usize; // skip num_polylines header
        let num_lines = glyph[0];
        for _ in 0..num_lines {
            let npts = glyph[k] as usize;
            k += 1;
            polyline(
                backend,
                npts,
                |i| (cx + scale * glyph[k + 2 * i], cy + scale * glyph[k + 2 * i + 1]),
                vwk.mark_color,
                mode,
                clip,
                0xFFFF,
            );
            k += 2 * npts;
        }
    }
}
