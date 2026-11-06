//! VDI attribute setters needed for the first drawing primitives.
//!
//! Spec: `docs/api/vdi/attributes/`. Each validates its input into the
//! [`Vwk`](super::vwk::Vwk) and echoes the accepted value; none can
//! fail (no VDI error return). Only the setters the bring-up demo uses
//! are here (line/fill colour + fill interior); the rest land with the
//! opcodes that consume them.

use super::backend::RasterBackend;
use super::font::{self, FontRef, SysFontId};
use super::req_col;
use super::text;
use super::vwk::{self, validate_color_index, Vwk};
use super::Vdipb;

// Line / marker / text attribute bounds (EmuTOS `vdi_defs.h`).
const MAX_LINE_STYLE: i16 = 7;
const MAX_LINE_WIDTH: i16 = 40; // SIZ_TAB[6]
const MIN_END_STYLE: i16 = 0; // squared
const MAX_END_STYLE: i16 = 2; // round
const MIN_MARK_STYLE: i16 = 1;
const MAX_MARK_STYLE: i16 = 6;
const DEF_MKWD: i16 = 15; // SIZ_TAB[8] base marker width
const DEF_MKHT: i16 = 11; // SIZ_TAB[9] base marker height
const MAX_MKHT: i16 = 88; // SIZ_TAB[11]
const EFFECTS_MASK: i16 = 0x1F; // INQ_TAB[2] supported text effects

/// Writing-mode bounds (1-based API), EmuTOS `vdi_control.c`.
const MIN_WRT_MODE: i16 = 1;
const MAX_WRT_MODE: i16 = 4;
const DEF_WRT_MODE: i16 = 1; // replace

/// Opcode 32 — `vswr_mode`: set the writing/raster mode. `INTIN[0]` is
/// 1-based (1 replace, 2 transparent, 3 XOR, 4 reverse-transparent);
/// stored 0-based in `wrt_mode`. Out-of-range defaults to replace.
pub fn vswr_mode(pb: &Vdipb, vwk: &mut Vwk) {
    let mut mode = pb.intin(0);
    if !(MIN_WRT_MODE..=MAX_WRT_MODE).contains(&mode) {
        mode = DEF_WRT_MODE;
    }
    vwk.wrt_mode = mode - 1;
    pb.set_intout(0, mode);
    pb.set_contrl(4, 1);
}

/// Opcode 129 — `vs_clip`: set the clipping rectangle. `INTIN[0]` is the
/// enable flag; when set, `PTSIN[0..3]` give the rectangle (corners
/// normalised, then intersected with the screen). When clear, the clip
/// is reset to the full device. No output.
pub fn vs_clip(pb: &Vdipb, vwk: &mut Vwk, backend: &dyn RasterBackend) {
    let xres = backend.width() as i16 - 1;
    let yres = backend.height() as i16 - 1;
    vwk.clip = pb.intin(0) != 0;
    if vwk.clip {
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
        vwk.xmn_clip = x1.max(0);
        vwk.ymn_clip = y1.max(0);
        vwk.xmx_clip = x2.min(xres);
        vwk.ymx_clip = y2.min(yres);
    } else {
        vwk.xmn_clip = 0;
        vwk.ymn_clip = 0;
        vwk.xmx_clip = xres;
        vwk.ymx_clip = yres;
    }
}

/// Opcode 15 — `vsl_type`: set the line style (1..MAX, else solid).
/// Stored 0-based in `line_index`.
pub fn vsl_type(pb: &Vdipb, vwk: &mut Vwk) {
    let v = pb.intin(0);
    vwk.line_index = if (1..=MAX_LINE_STYLE).contains(&v) { v - 1 } else { 0 };
    pb.set_intout(0, vwk.line_index + 1);
    pb.set_contrl(4, 1);
}

/// Opcode 16 — `vsl_width`: set the line width from `PTSIN[0]`. Clamped
/// to `[1, MAX_LINE_WIDTH]` and forced **odd** (the line algorithm
/// centres on a pixel). Echoes the width in `PTSOUT[0]`.
pub fn vsl_width(pb: &Vdipb, vwk: &mut Vwk) {
    let mut w = pb.ptsin(0).clamp(1, MAX_LINE_WIDTH);
    if w % 2 == 0 {
        w -= 1; // force odd
    }
    vwk.line_width = w;
    pb.set_ptsout(0, w);
    pb.set_ptsout(1, 0);
    pb.set_contrl(2, 1);
}

/// Opcode 108 — `vsl_ends`: set the line end styles (begin = `INTIN[0]`,
/// end = `INTIN[1]`; 0 squared / 1 arrow / 2 round, else squared).
pub fn vsl_ends(pb: &Vdipb, vwk: &mut Vwk) {
    let validate = |v: i16| if (MIN_END_STYLE..=MAX_END_STYLE).contains(&v) { v } else { MIN_END_STYLE };
    vwk.line_beg = validate(pb.intin(0));
    vwk.line_end = validate(pb.intin(1));
    pb.set_intout(0, vwk.line_beg);
    pb.set_intout(1, vwk.line_end);
    pb.set_contrl(4, 2);
}

/// Opcode 113 — `vsl_udsty`: set the 16-bit user-defined line-style mask
/// (no validation). No output.
pub fn vsl_udsty(pb: &Vdipb, vwk: &mut Vwk) {
    vwk.ud_ls = pb.intin(0) as u16;
}

/// Opcode 18 — `vsm_type`: set the marker type (1..6, else asterisk).
/// Stored 0-based in `mark_index`.
pub fn vsm_type(pb: &Vdipb, vwk: &mut Vwk) {
    let v = pb.intin(0);
    vwk.mark_index = if (MIN_MARK_STYLE..=MAX_MARK_STYLE).contains(&v) { v - 1 } else { 2 };
    pb.set_intout(0, vwk.mark_index + 1);
    pb.set_contrl(4, 1);
}

/// Opcode 19 — `vsm_height`: set the marker height from `PTSIN[1]`.
/// Clamped to `[DEF_MKHT, MAX_MKHT]`; derives an integer `mark_scale` of
/// the base marker. Echoes the actual marker width/height in `PTSOUT`.
pub fn vsm_height(pb: &Vdipb, vwk: &mut Vwk) {
    let h = pb.ptsin(1).clamp(DEF_MKHT, MAX_MKHT);
    vwk.mark_height = h;
    let scale = (h + DEF_MKHT / 2) / DEF_MKHT;
    vwk.mark_scale = scale;
    pb.set_ptsout(0, scale * DEF_MKWD);
    pb.set_ptsout(1, scale * DEF_MKHT);
    pb.set_contrl(2, 1);
}

/// Opcode 20 — `vsm_color`: set the marker colour.
pub fn vsm_color(pb: &Vdipb, vwk: &mut Vwk, ncol: u16) {
    let c = validate_color_index(pb.intin(0), ncol);
    vwk.mark_color = c;
    pb.set_intout(0, c as i16);
    pb.set_contrl(4, 1);
}

/// Opcode 106 — `vst_effects`: set the text effect bits, masked to the
/// supported set (`0x1F`: bold/light/italic/underline/outline).
pub fn vst_effects(pb: &Vdipb, vwk: &mut Vwk) {
    vwk.style = pb.intin(0) & EFFECTS_MASK;
    pb.set_intout(0, vwk.style);
    pb.set_contrl(4, 1);
}

/// Opcode 13 — `vst_rotation`: set the text baseline angle in 1/10°.
/// The angle is normalised to `[0,3599]`, then snapped to the nearest
/// right angle (`chup` ∈ {0,900,1800,2700,**3600**}). EmuTOS
/// `vdi_text.c:924-936`: an angle in 3150..3599 rounds up to 3600 (kept
/// distinct from 0 — no `% 3600`).
pub fn vst_rotation(pb: &Vdipb, vwk: &mut Vwk) {
    let mut angle = pb.intin(0) % 3600;
    if angle < 0 {
        angle += 3600;
    }
    vwk.chup = ((angle + 450) / 900) * 900;
    pb.set_intout(0, vwk.chup);
    pb.set_contrl(4, 1);
}

/// Opcode 21 — `vst_font`: select a text face (EmuTOS `vdi_vst_font`,
/// `vdi_text.c:939-985`). The first header of the face over the ring, `fon6x6`
/// when there is none, then the size of the previous current font — its effective
/// `point`/`top`, scaled header included — re-applied through `vst_point` or
/// `vst_height`, whose `PTSOUT` go to a local there and so nowhere here. `INTOUT[0]`
/// = the selected header's id. Leaves the trap's `flip_y` at 1 (`mod.rs`).
pub fn vst_font(pb: &Vdipb, vwk: &mut Vwk) {
    let old = text::effective_font(pb, vwk);
    let face = pb.intin(0);
    vwk.cur_font = match font::first_of_face(pb, face) {
        Some((r, _, _)) => r,
        None => FontRef::System(SysFontId::F6x6),
    };
    if vwk.pts_mode {
        select_point(pb, vwk, old.point);
    } else {
        select_height(pb, vwk, old.top as u16);
    }
    pb.set_intout(0, text::effective_font(pb, vwk).font_id);
    pb.set_contrl(4, 1);
}

/// The four metrics `setup_width_height` reports for the chosen (possibly
/// DDA-scaled) font: `max_char_width`, `top`, `max_cell_width`, `bottom`.
struct SizeReport {
    max_char_width: i16,
    top: i16,
    max_cell_width: i16,
    bottom: i16,
}

/// Fill `PTSOUT[0..3]` from a [`SizeReport`] (EmuTOS `setup_width_height`,
/// `vdi_text.c:668-679`): char width, `top`, cell width, `top+bottom+1`.
fn setup_width_height(pb: &Vdipb, r: &SizeReport) {
    pb.set_ptsout(0, r.max_char_width);
    pb.set_ptsout(1, r.top);
    pb.set_ptsout(2, r.max_cell_width);
    pb.set_ptsout(3, r.top.wrapping_add(r.bottom).wrapping_add(1));
    pb.set_contrl(2, 2);
}

/// `clc_dda` (EmuTOS `vdi_text.c:1374-1391`): the DDA fraction to scale a
/// font from `act` to `req` units. Returns `(dda_inc, t_sclsts)` where
/// `t_sclsts` is `true` when enlarging; `0xFFFF` means "exactly double".
fn clc_dda(act: u16, req: u16) -> (u16, bool) {
    // `act == 0` (a font's `top`, the program's to choose) cannot divide: `req`
    // is never below it, and `req >= act` doubles.
    if req < act {
        let req = if req == 0 { 1 } else { req }; // scale down; min size 1
        ((((req as u32) << 16) / act as u32) as u16, false)
    } else {
        let req = req - act; // scale up
        if req >= act {
            (0xFFFF, true) // larger than 2x → double
        } else {
            ((((req as u32) << 16) / act as u32) as u16, true)
        }
    }
}

/// `act_siz` (EmuTOS `vdi_text.c:743-767`): the actual scaled size of a
/// `top`-unit dimension under the DDA `(dda_inc, t_sclsts)`.
fn act_siz(dda_inc: u16, t_sclsts: bool, top: u16) -> u16 {
    if dda_inc == 0xFFFF {
        return top.wrapping_shl(1); // exact double
    }
    let mut accu: u16 = 0x7FFF;
    let mut retval: u16 = if t_sclsts { top } else { 0 };
    for _ in 0..top {
        accu = accu.wrapping_add(dda_inc);
        if accu < dda_inc {
            retval = retval.wrapping_add(1); // carry out of 16 bits
        }
    }
    if top != 0 && retval == 0 {
        retval = 1; // non-zero input → at least 1
    }
    retval
}

/// The four report metrics for `font` at its natural (unscaled) size.
fn base_report(f: &font::SysFont) -> SizeReport {
    SizeReport {
        max_char_width: f.max_char_width,
        top: f.top,
        max_cell_width: f.max_cell_width,
        bottom: f.bottom,
    }
}

/// The four report metrics for `font` scaled by the DDA `(dda_inc,
/// t_sclsts)` (EmuTOS `make_header`, `vdi_text.c:770-817`). The exact-double
/// case gives `top` a `*2+1` (the header quirk); everything else is
/// `act_siz`.
fn scaled_report(f: &font::SysFont, dda_inc: u16, t_sclsts: bool) -> SizeReport {
    let sz = |v: i16| act_siz(dda_inc, t_sclsts, v as u16) as i16;
    let top = if dda_inc == 0xFFFF { f.top.wrapping_mul(2).wrapping_add(1) } else { sz(f.top) };
    SizeReport {
        max_char_width: sz(f.max_char_width),
        top,
        max_cell_width: sz(f.max_cell_width),
        bottom: sz(f.bottom),
    }
}

/// A font of the current face to start the size walk from, and the ring slot to
/// go on with. EmuTOS dereferences a NULL `test_font` when the face is gone from the
/// ring — another workstation's `v_opnvwk` or unload while this one's `cur_font` is
/// a loaded header (R13; `vdi_text.c:692-704` reads low memory). No defined result;
/// here the walk starts on `fon6x6` and ends there (`docs/COMPATIBILITY.md` "GDOS
/// font chain").
fn start_of_face(pb: &Vdipb, face: i16) -> (FontRef, font::SysFont, usize) {
    font::first_of_face(pb, face).unwrap_or((FontRef::System(SysFontId::F6x6), font::FON_6X6, 3))
}

/// The traversal of `vdi_vst_height`/`vdi_vst_point` (`vdi_text.c:706-713`,
/// `:845-855`): from the first header of the face along its list while `fits` holds,
/// then from the head of each later ring slot the same way. A list stops at the first
/// header that does not fit or has another id.
fn walk_sizes(
    pb: &Vdipb,
    start: (FontRef, font::SysFont, usize),
    face: i16,
    mut fits: impl FnMut(FontRef, &font::SysFont) -> bool,
) {
    let (r, h, mut slot) = start;
    let mut test = Some((r, h));
    let mut steps = 0;
    loop {
        while let Some((r, h)) = test {
            if !(h.font_id == face && fits(r, &h)) {
                break;
            }
            steps += 1;
            if steps >= font::MAX_CHAIN {
                return;
            }
            test = font::next_font(&h).and_then(|n| font::head(pb, n).map(|nh| (n, nh)));
        }
        let Some(n) = font::ring_slot(slot) else { return };
        slot += 1;
        test = font::head(pb, n).map(|nh| (n, nh));
    }
}

/// `vdi_vst_height` from its size on (`vdi_text.c:681-727`): the last header of the
/// face whose `top` fits (UWORD compare), scaled by DDA when not exact.
fn select_height(pb: &Vdipb, vwk: &mut Vwk, test_height: u16) -> SizeReport {
    let face = font::head_or_system(pb, vwk.cur_font).font_id;
    vwk.pts_mode = false;
    let start = start_of_face(pb, face);
    let (mut single_ref, mut single) = (start.0, start.1);
    walk_sizes(pb, start, face, |r, h| {
        let ok = (h.top as u16) <= test_height;
        if ok {
            (single_ref, single) = (r, *h);
        }
        ok
    });
    vwk.cur_font = single_ref;
    // Reset to the non-scaled state (`vwk->scaled = FALSE`), overridden below
    // when the size is not an exact match.
    vwk.scaled = false;
    if (single.top as u16) != test_height {
        let (dda_inc, t_sclsts) = clc_dda(single.top as u16, test_height);
        // Remember the DDA so the render path (`v_gtext`) actually scales the
        // glyph, not just the reported metrics (EmuTOS stores these on the
        // Vwk for `output_text`/`text_blt`).
        vwk.dda_inc = dda_inc;
        vwk.t_sclsts = t_sclsts;
        vwk.scaled = true;
        scaled_report(&single, dda_inc, t_sclsts)
    } else {
        base_report(&single)
    }
}

/// Opcode 12 — `vst_height`: request a text height in device units (`PTSIN[1]`).
/// `PTSOUT[0..3]` = the actual char/cell metrics.
pub fn vst_height(pb: &Vdipb, vwk: &mut Vwk) {
    let report = select_height(pb, vwk, pb.ptsin(1) as u16);
    setup_width_height(pb, &report);
}

/// `vdi_vst_point` from its size on (`vdi_text.c:820-878`): the last header of the
/// face whose `point` fits (WORD compare) and the last whose doubled `point` does;
/// if the exact size is missing but the double fits, the doubled font
/// (`dda_inc = 0xFFFF`). Returns the report and the point size actually set.
fn select_point(pb: &Vdipb, vwk: &mut Vwk, test_height: i16) -> (SizeReport, i16) {
    let face = font::head_or_system(pb, vwk.cur_font).font_id;
    vwk.pts_mode = true;
    let start = start_of_face(pb, face);
    let (mut single_ref, mut single) = (start.0, start.1);
    let (mut double_ref, mut double) = (start.0, start.1);
    walk_sizes(pb, start, face, |r, f| {
        let ok = f.point <= test_height;
        if ok {
            (single_ref, single) = (r, *f);
            if f.point.wrapping_mul(2) <= test_height {
                (double_ref, double) = (r, *f);
            }
        }
        ok
    });
    vwk.cur_font = single_ref;
    // Reset to the non-scaled state (`vwk->scaled = FALSE`), overridden below
    // when the doubled font is selected.
    vwk.scaled = false;
    if single.point != test_height {
        let h = double.point.wrapping_mul(2);
        if h > single.point && h <= test_height {
            // The exact size is absent but the doubled font fits: use it and
            // record the exact-double DDA so the render path scales it 2×
            // (EmuTOS sets `dda_inc = 0xFFFF; t_sclsts = 1` + `make_header`).
            vwk.cur_font = double_ref;
            vwk.dda_inc = 0xFFFF;
            vwk.t_sclsts = true;
            vwk.scaled = true;
            return (scaled_report(&double, 0xFFFF, true), h);
        }
    }
    (base_report(&single), single.point)
}

/// Opcode 107 — `vst_point`: request a text height in points (`INTIN[0]`).
/// `INTOUT[0]` = the point size actually set; `PTSOUT[0..3]` = its char/cell
/// metrics.
pub fn vst_point(pb: &Vdipb, vwk: &mut Vwk) {
    let (report, point_out) = select_point(pb, vwk, pb.intin(0));
    setup_width_height(pb, &report);
    pb.set_intout(0, point_out); // point size actually set
    pb.set_contrl(4, 1);
}

/// Opcode 119 — `vst_load_fonts`: link the chain the caller (a GDOS) built
/// (EmuTOS `vdi_vst_load_fonts`, `vdi_text.c:1292-1350`; rules R1-R5 of
/// `docs/api/vdi/attributes/_text.md`). One chance per workstation: with a chain
/// linked the answer is 0 and nothing is read. Otherwise the walk counts the faces
/// (`font_id` differing from the *previous* header's, start id -1), swaps the data
/// table of every header without `F_STDFORM` and sets that flag in the caller's
/// header, then the chain becomes `font_ring[2]`.
///
/// A NULL chain — gemlib's binding always passes one — answers 0 and changes
/// nothing (decision D2; EmuTOS dereferences it, no defined result). The scratch
/// pointer and offset in `CONTRL` are not read: NewTOS' text path has its own
/// working store (`docs/COMPATIBILITY.md` "GDOS font chain").
pub fn vst_load_fonts(pb: &Vdipb, vwk: &mut Vwk) {
    pb.set_contrl(4, 1);
    if vwk.loaded_fonts != 0 {
        pb.set_intout(0, 0);
        return;
    }
    let first = pb.contrl_font_chain();
    if first == 0 {
        pb.set_intout(0, 0);
        return;
    }
    vwk.loaded_fonts = first;
    let owner = font::owner_of_caller(pb);
    let mut id: i16 = -1;
    let mut count: i16 = 0;
    let mut cur = first;
    for _ in 0..font::MAX_CHAIN {
        let at = font::LoadedRef { addr: cur, owner };
        let Some(h) = font::head(pb, FontRef::Loaded(at)) else { return };
        let Some(t) = h.loaded else { return };
        if h.font_id != id {
            id = h.font_id;
            count = count.wrapping_add(1);
        }
        if t.flags & font::F_STDFORM == 0 {
            font::trnsfont(pb, t.dat_table, t.form_width, h.form_height);
            font::set_stdform(pb, cur, t.flags);
        }
        if pb.faulted() {
            return;
        }
        cur = t.next_font;
        if cur == 0 {
            break;
        }
    }
    font::set_loaded_ring(font::LoadedRef { addr: first, owner });
    pb.set_intout(0, count);
}

/// Opcode 120 — `vst_unload_fonts` (EmuTOS `vdi_text.c:1353-1363`, R11): reads no
/// input, frees nothing, checks nothing. `cur_font` stays: a workstation drawing in
/// a loaded font keeps it until the next `vst_font` (measured, probe TXG).
pub fn vst_unload_fonts(vwk: &mut Vwk) {
    vwk.loaded_fonts = 0;
    font::clear_loaded_ring();
}

/// Opcode 104 — `vsf_perimeter`: enable/disable the fill-area outline.
pub fn vsf_perimeter(pb: &Vdipb, vwk: &mut Vwk) {
    vwk.fill_per = pb.intin(0) != 0;
    pb.set_intout(0, if vwk.fill_per { 1 } else { 0 });
    pb.set_contrl(4, 1);
}

/// Opcode 14 — `vs_color`: set a palette entry. `INTIN[0]` = colour
/// index, `INTIN[1..3]` = R/G/B. Out-of-range indices are ignored. The
/// **raw** request is stored (so `vq_color` flag 0 round-trips it verbatim,
/// possibly outside 0..1000); only the copy that programmes the palette is
/// clamped to 0..=1000. No output. EmuTOS `vdi_vs_color` (`vdi_col.c:656-704`).
///
/// Two paths, gated on `TRUECOLOR_MODE` (`v_planes > 8`, `vdi_col.c:661-666`):
///
/// - **Truecolor** (our Falcon profile, `planes == 16`): `vs_color16`
///   (`vdi_col.c:620-648`) writes **only** the calling workstation's private
///   5-bit pseudo-palette + `req_col` (via [`vwk::ext`]), so two open
///   workstations keep independent colours and `vq_color(flag=1)` reads
///   back the 5-bit-quantised value. The draw path resolves each PEL through
///   this per-`Vwk` pseudo-palette at the write point (the active-palette
///   boundary the VDI dispatcher sets, `mod.rs`), exactly as EmuTOS' truecolor
///   VDI reads `CUR_WORK->ext->palette` (`vdi_line.c:459-460`) — so **no
///   separate backend CLUT is programmed** (the TC2 interim bridge is gone).
/// - **Palette** (`planes <= 8`, unreachable in the truecolor profile but
///   kept for a non-truecolor backend): the original hardware-CLUT path —
///   raw request into the device-global `REQ_COL` shadow, clamped copy into
///   the backend CLUT.
pub fn vs_color(pb: &Vdipb, vwk: &Vwk, ncol: u16, backend: &dyn RasterBackend) {
    let idx = pb.intin(0);
    if idx < 0 || idx as u16 >= ncol {
        return;
    }
    let (r, g, b) = (pb.intin(1), pb.intin(2), pb.intin(3));

    if backend.planes() > 8 {
        // Truecolor (`vs_color16`): write the calling workstation's own 5-bit
        // query/draw tables and nothing else — the draw path resolves through
        // this per-Vwk pseudo-palette at the write point. SAFETY: single trap
        // path; `vwk.handle` is the open ws.
        if let Some(ext) = unsafe { vwk::ext(vwk.handle) } {
            let colnum = idx as usize;
            // Raw, unclamped request (EmuTOS `req_col[colnum][i] = *intin`).
            ext.req_col[colnum] = [r, g, b];
            // Clamp only the copy that builds the pseudo-palette.
            let clamp = |v: i16| v.clamp(0, 1000);
            ext.set_color16(colnum, [clamp(r), clamp(g), clamp(b)]);
        }
        return;
    }

    // Non-truecolor hardware CLUT path.
    // Store the raw, unclamped request (EmuTOS `REQ_COL[colnum][i] = *intin`).
    // SAFETY: single trap path (see `req_col`).
    unsafe { req_col::store_request(idx as u16, r, g, b) };
    // Clamp only the DAC copy to the VDI 0..1000 range.
    let clamp = |v: i16| v.clamp(0, 1000) as u16;
    backend.set_color(idx as u16, clamp(r), clamp(g), clamp(b));
}

/// Opcode 17 — `vsl_color`: set the polyline colour.
pub fn vsl_color(pb: &Vdipb, vwk: &mut Vwk, ncol: u16) {
    let c = validate_color_index(pb.intin(0), ncol);
    vwk.line_color = c;
    pb.set_intout(0, c as i16);
    pb.set_contrl(4, 1);
}

/// Opcode 25 — `vsf_color`: set the fill-area colour.
pub fn vsf_color(pb: &Vdipb, vwk: &mut Vwk, ncol: u16) {
    let c = validate_color_index(pb.intin(0), ncol);
    vwk.fill_color = c;
    pb.set_intout(0, c as i16);
    pb.set_contrl(4, 1);
}

/// Opcode 22 — `vst_color`: set the text colour.
pub fn vst_color(pb: &Vdipb, vwk: &mut Vwk, ncol: u16) {
    let c = validate_color_index(pb.intin(0), ncol);
    vwk.text_color = c;
    pb.set_intout(0, c as i16);
    pb.set_contrl(4, 1);
}

/// Opcode 39 — `vst_alignment`: set the text alignment.
/// `INTIN[0]` = horizontal (0 left, 1 centre, 2 right), `INTIN[1]` =
/// vertical (0 baseline … 5 top). Out-of-range clamps to 0.
pub fn vst_alignment(pb: &Vdipb, vwk: &mut Vwk) {
    let h = pb.intin(0);
    let v = pb.intin(1);
    vwk.h_align = if (0..=2).contains(&h) { h } else { 0 };
    vwk.v_align = if (0..=5).contains(&v) { v } else { 0 };
    pb.set_intout(0, vwk.h_align);
    pb.set_intout(1, vwk.v_align);
    pb.set_contrl(4, 2);
}

/// Opcode 23 — `vsf_interior`: set the fill interior style
/// (0 = hollow, 1 = solid, 2 = pattern, 3 = hatch, 4 = user).
pub fn vsf_interior(pb: &Vdipb, vwk: &mut Vwk) {
    let s = pb.intin(0);
    vwk.fill_style = if (0..=4).contains(&s) { s } else { 0 };
    pb.set_intout(0, vwk.fill_style);
    pb.set_contrl(4, 1);
}

const FIS_PATTERN: i16 = 2;
const MAX_FILL_PATTERN: i16 = 24;
const MAX_FILL_HATCH: i16 = 12;

/// Opcode 24 — `vsf_style`: set the pattern/hatch index (1-based in the
/// API), validated against the current interior style's range. Stored
/// 0-based in `fill_index`.
pub fn vsf_style(pb: &Vdipb, vwk: &mut Vwk) {
    let mut fi = pb.intin(0);
    let max = if vwk.fill_style == FIS_PATTERN {
        MAX_FILL_PATTERN
    } else {
        MAX_FILL_HATCH
    };
    if !(1..=max).contains(&fi) {
        fi = 1; // default
    }
    vwk.fill_index = fi - 1;
    pb.set_intout(0, fi);
    pb.set_contrl(4, 1);
}

/// Opcode 112 — `vsf_udpat`: set the user-defined fill pattern from
/// `INTIN` (`CONTRL[3]` words). EmuTOS (`vdi_fill.c:149-166`) accepts
/// **only** a 16-word single-plane pattern (`multifill = 0`) or a full
/// `INQ_TAB[4]*16`-word multi-plane pattern (`multifill = 1`); any other
/// count is invalid and leaves the state unchanged.
///
/// **Truecolor multi-plane reality (AUD-V2-009).** Under our Falcon
/// truecolor profile EmuTOS renders user fill patterns **monochrome**:
/// `swblit_rect_common16` (`vdi_line.c:450-551`) indexes `patptr[patmsk &
/// y]` and — unlike the planar `swblit_rect_common` — never advances the
/// pattern by 16 per plane (no `if (multifill) patind += 16`). It uses
/// only the first 16 pattern words, painting `fgcol`/`bgcol` (the fill
/// colour / pen 0). So storing plane 0 and rendering it monochrome is
/// **1:1 with EmuTOS in truecolor**; colour multi-plane patterns are a
/// planar-mode-only feature, unreachable on this device — the `multifill`
/// flag is kept for a future planar backend.
///
/// The multi-plane count is validated against the true `INQ_TAB[4] =
/// v_planes` (`planes`, threaded from `backend.planes()` = 16 in
/// truecolor), so a correctly-sized 16-plane pattern (256 words) is
/// accepted exactly as EmuTOS (`vdi_fill.c:157`: the count must equal
/// `INQ_TAB[4]` times 16). Only the first 16 words (plane 0) are stored —
/// the truecolor render uses only those (monochrome, see above); a future
/// planar backend would extend the store, not the acceptance test.
pub fn vsf_udpat(pb: &Vdipb, vwk: &mut Vwk, planes: u16) {
    let count = pb.contrl(3);
    // EmuTOS `vdi_vsf_udpat` (`vdi_fill.c:155-160`): 16 words = single
    // plane, `INQ_TAB[4] * 16` words = valid multi-plane, else invalid.
    if count == 16 {
        vwk.multifill = false;
    } else if count == planes as i16 * 16 {
        vwk.multifill = true;
    } else {
        return; // invalid count: no state change
    }
    for i in 0..16 {
        vwk.ud_patrn[i] = pb.intin(i) as u16;
    }
}
