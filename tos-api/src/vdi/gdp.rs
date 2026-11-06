//! VDI generalized drawing primitives — `v_gdp` (opcode 11). Spec:
//! `docs/api/vdi/output/_gdp.md`. The sub-function is in `CONTRL[5]`.
//!
//! Every sub-primitive reduces to the line / fill primitives in
//! [`primitives`](super::primitives); `v_gdp` only generates geometry.

use super::backend::{PatternFill, RasterBackend, WriteMode};
use super::fill_patterns;
use super::primitives::{fill_polygon, line_clip, line_mask, mul_div, polyline, stroke};
use super::trig::{icos, isin, umul_shift, HALFPI, MAX_TABLE_ANGLE};
use super::vwk::Vwk;
use super::Vdipb;

// GDP sub-function codes (`CONTRL[5]`).
const BAR: i16 = 1;
const ARC: i16 = 2;
const PIESLICE: i16 = 3;
const CIRCLE: i16 = 4;
const ELLIPSE: i16 = 5;
const ELLARC: i16 = 6;
const ELLPIE: i16 = 7;
const RBOX: i16 = 8;
const RFBOX: i16 = 9;
const JUSTIFIED: i16 = 10;

// Line end style (EmuTOS `vdi_defs.h:89`). The rounded box forces both
// ends to `SQUARED` for the duration of the outline (`vdi_v_gdp` case 8).
const SQUARED: i16 = 0;

// Arc segment-count clamp (EmuTOS `vdi_defs.h:84-85`).
const MIN_ARC_CT: i32 = 32;
const MAX_ARC_CT: i32 = 128;

// Angle constants in tenths of a degree (EmuTOS `vdi_gdp.c:29-30`).
const PI: i16 = 1800;
const TWOPI: i16 = 3600;

/// Opcode 11 — `v_gdp`: dispatch on `CONTRL[5]`.
pub fn v_gdp(pb: &Vdipb, vwk: &Vwk, backend: &dyn RasterBackend) {
    match pb.contrl(5) {
        BAR => bar(pb, vwk, backend),
        CIRCLE | ELLIPSE => curve(pb, vwk, backend, true, false),
        PIESLICE | ELLPIE => curve(pb, vwk, backend, true, true),
        ARC | ELLARC => curve(pb, vwk, backend, false, false),
        RBOX => rbox(pb, vwk, backend, false),
        RFBOX => rbox(pb, vwk, backend, true),
        JUSTIFIED => super::text::v_justified(pb, vwk, backend),
        _ => {} // unknown sub-op → no-op (VDI has no error return)
    }
}

/// Normalised inclusive rectangle from `PTSIN[0..3]`.
fn rect(pb: &Vdipb) -> (i16, i16, i16, i16) {
    let (mut x1, mut y1) = (pb.ptsin(0), pb.ptsin(1));
    let (mut x2, mut y2) = (pb.ptsin(2), pb.ptsin(3));
    if x1 > x2 {
        core::mem::swap(&mut x1, &mut x2);
    }
    if y1 > y2 {
        core::mem::swap(&mut y1, &mut y2);
    }
    (x1, y1, x2, y2)
}

/// The current interior fill descriptor. `HOLLOW` is not a no-op: its
/// all-zero pattern erases the interior to pen 0 in Replace mode and
/// paints the fill colour in Reverse-transparent mode (EmuTOS
/// `st_fl_ptr` HOLLOW `patptr=&HOLLOW`, `swblit_rect_common16`).
fn fill_of(vwk: &Vwk) -> PatternFill<'_> {
    PatternFill {
        fg: vwk.fill_color,
        bg: 0,
        pattern: fill_patterns::select(vwk),
        mode: WriteMode::from_wrt_mode(vwk.wrt_mode),
    }
}

/// Sub-op 1 — `v_bar`: a filled rectangle plus, if `fill_per`, a solid
/// perimeter in the fill colour.
fn bar(pb: &Vdipb, vwk: &Vwk, backend: &dyn RasterBackend) {
    let (x1, y1, x2, y2) = rect(pb);
    let clip = vwk.clip_rect();
    let mode = WriteMode::from_wrt_mode(vwk.wrt_mode);
    backend.fill_pattern(x1, y1, x2 - x1 + 1, y2 - y1 + 1, fill_of(vwk), clip);
    if vwk.fill_per {
        let pts = [(x1, y1), (x2, y1), (x2, y2), (x1, y2), (x1, y1)];
        polyline(backend, 5, |i| pts[i], vwk.fill_color, mode, line_clip(vwk), 0xFFFF);
    }
}

/// `clc_pts` (EmuTOS `vdi_gdp.c:126-170`): the raster endpoint on the
/// ellipse centred `(xc,yc)` with radii `(xrad,yrad)` at `angle` (tenths of
/// a degree, CCW). Reduces the angle to the first quadrant with sign flags,
/// snaps onto the pure axis near 0°/90° (the table has no exact-90° entry),
/// and otherwise scales the Q16 `icos`/`isin` by the radius via
/// `umul_shift`. Screen y grows downward, so the (already sign-adjusted)
/// y offset is *added* to `yc` exactly as EmuTOS.
fn clc_pts(xc: i16, yc: i16, xrad: i16, yrad: i16, mut angle: i16) -> (i16, i16) {
    const X_NEGATIVE: u8 = 0x02;
    const Y_NEGATIVE: u8 = 0x01;
    let mut negative = Y_NEGATIVE; /* default for first quadrant */

    while angle >= TWOPI {
        // normalise angle to 0-3599 inclusive
        angle -= TWOPI;
    }

    if angle > 3 * HALFPI {
        // fourth quadrant
        angle = TWOPI - angle;
        negative = 0;
    } else if angle > PI {
        // third quadrant
        angle -= PI;
        negative = X_NEGATIVE;
    } else if angle > HALFPI {
        // second quadrant
        angle = PI - angle;
        negative = X_NEGATIVE | Y_NEGATIVE;
    }

    // handle the values not handled by the table
    let (mut xdiff, mut ydiff) = if angle > MAX_TABLE_ANGLE {
        (0i16, yrad)
    } else if angle < HALFPI - MAX_TABLE_ANGLE {
        (xrad, 0i16)
    } else {
        (
            umul_shift(icos(angle), xrad as u16) as i16,
            umul_shift(isin(angle), yrad as u16) as i16,
        )
    };

    if negative & X_NEGATIVE != 0 {
        xdiff = -xdiff;
    }
    if negative & Y_NEGATIVE != 0 {
        ydiff = -ydiff;
    }

    // i32 intermediates avoid a debug overflow panic; the truncating cast
    // matches EmuTOS' 16-bit WORD wrap (coordinates are on-screen anyway).
    (
        (xc as i32 + xdiff as i32) as i16,
        (yc as i32 + ydiff as i32) as i16,
    )
}

/// `clc_nsteps` (EmuTOS `vdi_gdp.c:347-363`): the number of line segments
/// for a circle/ellipse/arc, based on the larger of the two radii and
/// clamped to `MIN_ARC_CT..=MAX_ARC_CT`.
fn clc_nsteps(xrad: i16, yrad: i16) -> usize {
    let mut steps = if xrad > yrad { xrad } else { yrad } as i32;
    steps >>= 2;
    steps.clamp(MIN_ARC_CT, MAX_ARC_CT) as usize
}

/// Sub-ops 2–7 — arc / pieslice / circle / ellipse / ellarc / ellpie.
/// Reproduces EmuTOS `gdp_curve` (`vdi_gdp.c:371-419`) + `clc_arc`
/// (`vdi_gdp.c:178-222`). `closed` fills (circle/ellipse/pie); open arcs
/// are stroked. `to_centre` closes the wedge to the centre (pieslice/ellpie).
fn curve(pb: &Vdipb, vwk: &Vwk, backend: &dyn RasterBackend, closed: bool, to_centre: bool) {
    let sub = pb.contrl(5);
    let xc = pb.ptsin(0);
    let yc = pb.ptsin(1);
    let (xsize, ysize) = super::workstation::get_std_pixel_size(backend.width() as i16, backend.height() as i16);
    // Radii (EmuTOS `gdp_curve:378-384`): the ellipse family carries
    // xrad/yrad in PTSIN[2..3]; the round family a single radius (circle
    // PTSIN[4], arc/pie PTSIN[6]) with yrad scaled by the pixel aspect.
    let (mut xrad, mut yrad) = match sub {
        ELLIPSE | ELLARC | ELLPIE => (pb.ptsin(2), pb.ptsin(3)),
        CIRCLE => {
            let r = pb.ptsin(4);
            (r, mul_div(r as i32, xsize as i32, ysize as i32) as i16)
        }
        _ => {
            let r = pb.ptsin(6);
            (r, mul_div(r as i32, xsize as i32, ysize as i32) as i16)
        }
    };
    // Negative radii are treated like positive ones (EmuTOS `:390-393`).
    if xrad < 0 {
        xrad = xrad.wrapping_neg();
    }
    if yrad < 0 {
        yrad = yrad.wrapping_neg();
    }

    // Quit if clipping excludes the entire curve (EmuTOS `gdp_curve:395-402`):
    // only with clipping on, and against the radii alone — the line width
    // and the arrowheads are not counted, so a wide or arrowed stroke that
    // would reach into the clip from outside `xc ± xrad`/`yc ± yrad` is not
    // drawn. Strict comparisons, 16-bit WORD sums as in EmuTOS (`-mshort`).
    if vwk.clip
        && (xc.wrapping_add(xrad) < vwk.xmn_clip
            || xc.wrapping_sub(xrad) > vwk.xmx_clip
            || yc.wrapping_add(yrad) < vwk.ymn_clip
            || yc.wrapping_sub(yrad) > vwk.ymx_clip)
    {
        return;
    }

    // Angles in tenths of a degree (CCW). Full shapes force 0..3600
    // (`gdp_curve:404-410`).
    let full = matches!(sub, CIRCLE | ELLIPSE);
    let (beg_ang, end_ang) = if full { (0i16, TWOPI) } else { (pb.intin(0), pb.intin(1)) };
    // EmuTOS `del_ang = end - begin; if (del_ang < 0) del_ang += TWOPI`
    // (`:412-414`). Only a *negative* sweep wraps — `beg == end` yields a
    // zero sweep (a degenerate point), NOT a full circle. `wrapping_*` keeps
    // the m68k 16-bit WORD wrap for out-of-range INTIN angles (and so avoids
    // a debug overflow panic on adversarial input).
    let mut del_ang = end_ang.wrapping_sub(beg_ang);
    if del_ang < 0 {
        del_ang = del_ang.wrapping_add(TWOPI);
    }

    let steps = clc_nsteps(xrad, yrad);

    // clc_arc: the start point, then `steps-1` interior points spaced by
    // `mul_div(del_ang, i, steps)` (duplicates skipped), then the exact end
    // point. Room for MAX_ARC_CT+1 arc points plus the pie centre.
    let mut pts = [(0i16, 0i16); MAX_ARC_CT as usize + 2];
    let mut n = 0usize;
    pts[n] = clc_pts(xc, yc, xrad, yrad, beg_ang);
    n += 1;
    for i in 1..steps {
        let angle = (mul_div(del_ang as i32, i as i32, steps as i32) as i16).wrapping_add(beg_ang);
        let p = clc_pts(xc, yc, xrad, yrad, angle);
        if p != pts[n - 1] {
            // ignore duplicates (EmuTOS `:189`)
            pts[n] = p;
            n += 1;
        }
    }
    pts[n] = clc_pts(xc, yc, xrad, yrad, end_ang);
    n += 1;
    // Pie wedge: draw to centre (EmuTOS `:199-203`).
    if to_centre {
        pts[n] = (xc, yc);
        n += 1;
    }

    let clip = vwk.clip_rect();
    let mode = WriteMode::from_wrt_mode(vwk.wrt_mode);
    if !closed && !to_centre {
        // Open arc/ellarc: EmuTOS `clc_arc:210-218` strokes with the line
        // attributes — width 1 threads `set_LN_MASK`/`line_mask` and adds
        // arrowheads on `ARROWED` ends; width > 1 draws the wide line.
        stroke(backend, &mut pts, n, vwk, vwk.line_color, mode, clip, line_mask(vwk));
    } else {
        // Closed circle/ellipse/pieslice/ellpie: EmuTOS `polygon()` fills
        // the interior and (only when `fill_per`) draws the boundary as a
        // solid width-1 polyline in the **fill colour** (`vdi_fill.c:606-608`).
        fill_polygon(backend, n, &pts, fill_of(vwk), clip);
        if vwk.fill_per {
            polyline(backend, n + 1, |i| pts[i % n], vwk.fill_color, mode, line_clip(vwk), 0xFFFF);
        }
    }
}

/// Corner points per rounded-box corner (EmuTOS `CORNER_POINTS`,
/// `vdi_gdp.c:23`); 5 for TOS visual compatibility.
const CORNER_POINTS: usize = 5;
/// Point counts for `polyline()` (outline, with the closing vertex) and
/// `polygon()` (fill) — EmuTOS `RBOX_POINTS`/`RFBOX_POINTS` (`:24-25`).
const RBOX_POINTS: usize = 4 * CORNER_POINTS + 1; // 21
const RFBOX_POINTS: usize = 4 * CORNER_POINTS; // 20

// Precalculated sine/cosine used for the rounded-box corners, scaled to a
// max value of **32767** (EmuTOS `vdi_gdp.c:79-84`). The five fixed corner
// angles are 0/22.5/45/67.5/90°.
const ISIN225: i32 = 12539;
const ISIN450: i32 = 23170;
const ISIN675: i32 = 30273;
const ICOS225: i32 = ISIN675;
const ICOS450: i32 = ISIN450;
const ICOS675: i32 = ISIN225;

/// Sub-ops 8/9 — rounded box (EmuTOS `gdp_rbox`, `vdi_gdp.c:229-338`).
/// `filled` chooses `v_rfbox` (9) vs `v_rbox` (8). The corner radius is
/// derived from the screen width (`xres>>6`), clamped to half the box side.
fn rbox(pb: &Vdipb, vwk: &Vwk, backend: &dyn RasterBackend, filled: bool) {
    // arb_line: (x1,y1) = LL corner, (x2,y2) = UR corner, so after this
    // x1 <= x2 and y1 >= y2 (screen y grows downward). EmuTOS `:240-244`.
    let (mut x1, mut y1) = (pb.ptsin(0), pb.ptsin(1));
    let (mut x2, mut y2) = (pb.ptsin(2), pb.ptsin(3));
    if x1 > x2 {
        core::mem::swap(&mut x1, &mut x2);
    }
    if y1 < y2 {
        core::mem::swap(&mut y1, &mut y2);
    }

    // xradius ~ 1/64 of the screen width (`xres` = DEV_TAB[0] = width-1),
    // clamped to half the box side; yradius scaled by the pixel aspect and
    // clamped likewise (EmuTOS `:254-255`).
    let xres = backend.width() as i32 - 1;
    let (xsize, ysize) = super::workstation::get_std_pixel_size(backend.width() as i16, backend.height() as i16);
    let xradius = (xres >> 6).min((x2 as i32 - x1 as i32) / 2);
    let yradius = mul_div(xradius, xsize as i32, ysize as i32).min((y1 as i32 - y2 as i32) / 2);

    // Unsigned offsets of the 5 corner points from the quadrant centre
    // (EmuTOS `:262-271`).
    let xoff = [
        0,
        mul_div(ICOS675, xradius, 32767),
        mul_div(ICOS450, xradius, 32767),
        mul_div(ICOS225, xradius, 32767),
        xradius,
    ];
    let yoff = [
        yradius,
        mul_div(ISIN675, yradius, 32767),
        mul_div(ISIN450, yradius, 32767),
        mul_div(ISIN225, yradius, 32767),
        0,
    ];

    // Fill PTSIN starting with the UR corner, then LR, LL, UL. For the LR
    // and UL corners the offset sequence is consumed in reverse (EmuTOS
    // `:274-321`). One extra slot holds the polyline closing vertex.
    let mut pts = [(0i16, 0i16); RBOX_POINTS + 1];
    let mut n = 0usize;

    // UR corner.
    let xcentre_r = x2 as i32 - xradius;
    let mut ycentre = y2 as i32 + yradius;
    for i in 0..CORNER_POINTS {
        pts[n] = ((xcentre_r + xoff[i]) as i16, (ycentre - yoff[i]) as i16);
        n += 1;
    }
    // LR corner (offsets reversed); xcentre unchanged.
    ycentre = y1 as i32 - yradius;
    for i in 0..CORNER_POINTS {
        let j = CORNER_POINTS - 1 - i;
        pts[n] = ((xcentre_r + xoff[j]) as i16, (ycentre + yoff[j]) as i16);
        n += 1;
    }
    // LL corner; ycentre unchanged.
    let xcentre_l = x1 as i32 + xradius;
    for i in 0..CORNER_POINTS {
        pts[n] = ((xcentre_l - xoff[i]) as i16, (ycentre + yoff[i]) as i16);
        n += 1;
    }
    // UL corner (offsets reversed); xcentre unchanged.
    ycentre = y2 as i32 + yradius;
    for i in 0..CORNER_POINTS {
        let j = CORNER_POINTS - 1 - i;
        pts[n] = ((xcentre_l - xoff[j]) as i16, (ycentre - yoff[j]) as i16);
        n += 1;
    }
    debug_assert_eq!(n, RFBOX_POINTS);

    let clip = vwk.clip_rect();
    let mode = WriteMode::from_wrt_mode(vwk.wrt_mode);
    if filled {
        // v_rfbox: polygon() fills, plus the solid fill-colour perimeter
        // when fill_per (EmuTOS `:334-337`, `vdi_fill.c:606-608`).
        fill_polygon(backend, RFBOX_POINTS, &pts, fill_of(vwk), clip);
        if vwk.fill_per {
            polyline(
                backend,
                RFBOX_POINTS + 1,
                |i| pts[i % RFBOX_POINTS],
                vwk.fill_color,
                mode,
                line_clip(vwk),
                0xFFFF,
            );
        }
    } else {
        // v_rbox: join up the polyline (append PTSIN[0]) and stroke it with
        // the line attributes; the dispatcher forces the ends to SQUARED
        // (EmuTOS `vdi_v_gdp` case 8, `:457-465`), so no arrowhead fires.
        pts[n] = pts[0];
        let mut v = *vwk;
        v.line_beg = SQUARED;
        v.line_end = SQUARED;
        stroke(backend, &mut pts, RBOX_POINTS, &v, vwk.line_color, mode, clip, line_mask(vwk));
    }
}
