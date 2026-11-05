//! VDI inquiry opcodes — read-only queries of the workstation attribute
//! state, the palette, a pixel, and the keyboard. Spec:
//! `docs/api/vdi/inquiry/`.
//!
//! Colour queries follow the same folded-`MAP_COL` (identity) model as the
//! setters: the attribute inquiries echo the stored PEL directly, and
//! `vq_color` reads the calling workstation's private truecolor tables
//! under `TRUECOLOR_MODE` (see [`vs_color`](super::attributes::vs_color)
//! and [`VwkExt`](super::vwk::VwkExt)).

use super::backend::RasterBackend;
use super::font;
use super::req_col;
use super::text;
use super::vwk::{self, Vwk};
use super::Vdipb;

// Text effect bits queried by vqt_fontinfo (EmuTOS `F_THICKEN`/`F_SKEW`).
const F_THICKEN: i16 = 0x01;
const F_SKEW: i16 = 0x04;

/// Opcode 35 — `vql_attributes`: read back the line attributes.
pub fn vql_attributes(pb: &Vdipb, vwk: &Vwk) {
    pb.set_intout(0, vwk.line_index + 1);
    pb.set_intout(1, vwk.line_color as i16);
    pb.set_intout(2, vwk.wrt_mode + 1);
    pb.set_ptsout(0, vwk.line_width);
    pb.set_ptsout(1, 0);
    pb.set_contrl(4, 3);
    pb.set_contrl(2, 1);
}

/// Opcode 36 — `vqm_attributes`: read back the marker attributes. Note
/// `mark_index` is returned **0-based** (an EmuTOS/TOS ABI quirk).
pub fn vqm_attributes(pb: &Vdipb, vwk: &Vwk) {
    pb.set_intout(0, vwk.mark_index);
    pb.set_intout(1, vwk.mark_color as i16);
    pb.set_intout(2, vwk.wrt_mode + 1);
    pb.set_ptsout(0, 0);
    pb.set_ptsout(1, vwk.mark_height);
    pb.set_contrl(4, 3);
    pb.set_contrl(2, 1);
}

/// Opcode 37 — `vqf_attributes`: read back the fill attributes.
pub fn vqf_attributes(pb: &Vdipb, vwk: &Vwk) {
    pb.set_intout(0, vwk.fill_style);
    pb.set_intout(1, vwk.fill_color as i16);
    pb.set_intout(2, vwk.fill_index + 1);
    pb.set_intout(3, vwk.wrt_mode + 1);
    pb.set_intout(4, if vwk.fill_per { 1 } else { 0 });
    pb.set_contrl(4, 5);
}

/// Opcode 38 — `vqt_attributes`: read back the text attributes + the
/// current font's cell metrics.
///
/// The face id and the four cell metrics come from the workstation's
/// **effective** font ([`text::effective_font`]) — EmuTOS
/// `vdi_vqt_attributes` (`vdi_text.c:999-1021`) reads them all off
/// `vwk->cur_font`, so they follow `vst_font`/`vst_height`/`vst_point`
/// (including the DDA-scaled header) instead of being the 8×16 defaults.
pub fn vqt_attributes(pb: &Vdipb, vwk: &Vwk) {
    let f = text::effective_font(vwk);
    pb.set_intout(0, f.font_id);
    pb.set_intout(1, vwk.text_color as i16);
    pb.set_intout(2, vwk.chup);
    pb.set_intout(3, vwk.h_align);
    pb.set_intout(4, vwk.v_align);
    pb.set_intout(5, vwk.wrt_mode + 1);
    pb.set_ptsout(0, f.max_char_width);
    pb.set_ptsout(1, f.top);
    pb.set_ptsout(2, f.max_cell_width);
    pb.set_ptsout(3, f.top + f.bottom + 1);
    pb.set_contrl(4, 6);
    pb.set_contrl(2, 2);
}

/// Opcode 26 — `vq_color`: read a palette entry. `INTIN[0]` = index,
/// `INTIN[1]` = flag (0 = last *requested* value, ≠0 = *actual*). EmuTOS
/// `vdi_vq_color` (`vdi_col.c:843-936`): the range check + `INTOUT[0]`
/// default happen first, then it branches on `TRUECOLOR_MODE`
/// (`v_planes > 8`, `vdi_col.c:862-868`):
///
/// - **Truecolor** (our profile): `vq_color16` (`vdi_col.c:813-838`) reads
///   the **calling workstation's private** tables (via [`vwk::ext`]) —
///   flag 0 the raw `req_col`, flag ≠0 the 5-bit-quantised value from the
///   pseudo-palette ([`VwkExt::actual16`](super::vwk::VwkExt::actual16),
///   solving V3-010 with the truecolor quantisation formula rather than a
///   DAC read-back).
/// - **Palette** (`planes <= 8`, non-truecolor backend): flag 0 the raw
///   device-global `REQ_COL` shadow, flag ≠0 the backend read-back.
pub fn vq_color(pb: &Vdipb, vwk: &Vwk, ncol: u16, backend: &dyn RasterBackend) {
    let idx = pb.intin(0);
    let flag = pb.intin(1);
    pb.set_intout(1, 0);
    pb.set_intout(2, 0);
    pb.set_intout(3, 0);
    if idx < 0 || idx as u16 >= ncol {
        pb.set_intout(0, -1);
        pb.set_contrl(4, 4);
        return;
    }
    pb.set_intout(0, idx);

    if backend.planes() > 8 {
        // Truecolor (`vq_color16`): read the calling workstation's tables.
        // SAFETY: single trap path; `vwk.handle` is the open workstation.
        if let Some(ext) = unsafe { vwk::ext(vwk.handle) } {
            let colnum = idx as usize;
            let rgb = if flag == 0 {
                ext.req_col[colnum] // last-requested (raw)
            } else {
                ext.actual16(colnum) // current 5-bit-quantised value
            };
            pb.set_intout(1, rgb[0]);
            pb.set_intout(2, rgb[1]);
            pb.set_intout(3, rgb[2]);
        }
        pb.set_contrl(4, 4);
        return;
    }

    // Non-truecolor palette path.
    // flag 0: the raw last-requested value (REQ_COL); registers never set
    // by vs_color fall back to the live palette (the hardware default).
    // flag ≠0: the actual hardware palette read-back.
    let raw = if flag == 0 {
        // SAFETY: single trap path (see `req_col`).
        unsafe { req_col::requested(idx as u16) }
    } else {
        None
    };
    let (r, g, b) = match raw {
        Some(rgb) => rgb,
        None => {
            let (r, g, b) = backend.query_color(idx as u16);
            (r as i16, g as i16, b as i16)
        }
    };
    pb.set_intout(1, r);
    pb.set_intout(2, g);
    pb.set_intout(3, b);
    pb.set_contrl(4, 4);
}

/// Opcode 105 — `v_get_pixel`: read the pixel at `PTSIN[0..1]`.
///
/// EmuTOS `vdi_v_get_pixel` (`vdi_fill.c:1095-1115`) fetches the pixel with
/// `pixelread` and then branches on `TRUECOLOR_MODE` (`v_planes > 8`,
/// `vdi_defs.h:22`) — the same predicate as `vs_color`/`vq_color`, here
/// `backend.planes() > 8`:
///
/// - **Truecolor** (our Falcon profile, `planes == 16`): `pixelread` returns
///   the raw 16-bit screen word (`vdi_fill.c:677-681`) and the reply is
///   `INTOUT[0] = 0`, `INTOUT[1] = pel` (`vdi_fill.c:1104-1110`) — a 16-bit
///   RGB colour, **not** a pen index, and there is no `REV_MAP_COL` lookup
///   because a truecolor screen holds no palette indices to reverse-map.
///   NewTOS' framebuffer is 32-bpp XRGB8888 internally
///   (`docs/COMPATIBILITY.md`), so the app-visible 16-bit word is recovered
///   with [`device_to_rgb565`](RasterBackend::device_to_rgb565) over
///   [`get_device_pixel`](RasterBackend::get_device_pixel) — the same
///   screen→16-bit boundary a memory MFDB crosses. That round-trip is exact
///   for every *drawn* pixel: every VDI draw resolves its PEL through the
///   5-bit-quantised pseudo-palette (`(r << 11) | (g << 6) | b`,
///   `vdi_col.c:601-614`), whose word layout `RRRRRGGGGG0BBBBB` is what
///   `device_to_rgb565` produces. Reading the stored colour — not
///   `read_pixel`'s nearest-CLUT reverse map — also keeps pixels that are
///   *not* a pen (a blitted truecolor bitmap) readable, as on a Falcon.
///
///   The round trip is exact for the one word no pen produces as well: the
///   whole screen after `v_clrwk` (`memset(v_bas_ad, 0xff, …)`,
///   `vdi_control.c:610-618`) has `OVERLAY_BIT` (`0x0020`, `vdi_fill.c:26`)
///   set, `pixelread` returns it unmasked (`vdi_fill.c:671-681`), and EmuTOS
///   answers `0xFFFF` (measured, `EMUTOS14-VDI-all-falcontc.BIN` cases
///   54/160/161). The kernel keeps that bit in the device pixel's X byte,
///   so NewTOS answers the same (VDI-OVL-001, closed by V11/V12,
///   2026-09-21; `tos-api/tests/vdi_reference_overlay_bit.rs`).
/// - **Palette** (`planes <= 8`, a non-truecolor backend): `INTOUT[0] = pel`,
///   the raw device pixel value, and `INTOUT[1] = REV_MAP_COL[pel]`, the VDI
///   pen (`vdi_fill.c:1113-1114`) — identity under NewTOS' folded `MAP_COL`
///   ([`VwkExt`](super::vwk::VwkExt)), so both slots carry the PEL.
pub fn v_get_pixel(pb: &Vdipb, backend: &dyn RasterBackend) {
    let x = pb.ptsin(0);
    let y = pb.ptsin(1);
    pb.set_contrl(4, 2);

    // `pel = (WORD)pixelread(x,y)` (`vdi_fill.c:1102`) — the same read
    // `v_contourfill` uses (`super::contour::pixelread`).
    let pel = super::contour::pixelread(backend, x, y);
    if backend.planes() > 8 {
        // TRUECOLOR_MODE: the 16-bit screen word, cast to a WORD.
        pb.set_intout(0, 0);
        pb.set_intout(1, pel as i16);
        return;
    }

    pb.set_intout(0, pel as i16);
    pb.set_intout(1, pel as i16);
}

/// Opcode 128 — `vq_key_s`: the current keyboard shift-state bitmask
/// (`MODE_*`), from the IKBD decoder via the host. EmuTOS `vdi_vq_key_s`
/// returns `gshift_s()` = `Kbshift(-1) & 0x000f` (`vdi_input.c:112,187`),
/// so only the four modifier bits (right-shift/left-shift/ctrl/alt) are
/// reported — the Caps-Lock bit (0x10) that the IKBD decoder also tracks
/// is masked off (AUD-V1-007).
pub fn vq_key_s(pb: &Vdipb) {
    let state = crate::host::host().map(|h| h.keyboard_shift_state()).unwrap_or(0);
    pb.set_intout(0, (state as i16) & 0x0f);
    pb.set_contrl(4, 1);
}

/// Opcode 116 — `vqt_extent`: the bounding box of the `CONTRL[3]`-char
/// string in `INTIN`, as four corner points in `PTSOUT[0..7]`, rotated
/// per `chup` (90° snaps).
///
/// EmuTOS `vdi_vqt_extent` (`vdi_text.c:1024-1059`) sizes the box with the
/// same `calc_width`/`calc_height` the render path uses, both reading
/// `vwk->cur_font` — so the extent tracks the selected face, its DDA
/// scaling and the active effects, not a fixed 8×16 cell.
pub fn vqt_extent(pb: &Vdipb, vwk: &Vwk) {
    let count = pb.contrl(3).max(0);
    let base = font::font(vwk.cur_font);
    let f = text::effective_font(vwk);
    let width = text::calc_width(base, &f, count, vwk);
    let height = text::calc_height(&f, vwk);
    for i in 0..8 {
        pb.set_ptsout(i, 0);
    }
    match vwk.chup {
        900 => {
            pb.set_ptsout(0, height);
            pb.set_ptsout(2, height);
            pb.set_ptsout(3, width);
            pb.set_ptsout(5, width);
        }
        1800 => {
            pb.set_ptsout(0, width);
            pb.set_ptsout(1, height);
            pb.set_ptsout(3, height);
            pb.set_ptsout(6, width);
        }
        2700 => {
            pb.set_ptsout(1, width);
            pb.set_ptsout(4, height);
            pb.set_ptsout(6, height);
            pb.set_ptsout(7, width);
        }
        _ => {
            // 0 / 3600: P0=(0,0) P1=(w,0) P2=(w,h) P3=(0,h)
            pb.set_ptsout(2, width);
            pb.set_ptsout(4, width);
            pb.set_ptsout(5, height);
            pb.set_ptsout(7, height);
        }
    }
    pb.set_contrl(2, 4);
}

/// Opcode 117 — `vqt_width`: the cell + horizontal offsets of char
/// `INTIN[0]`. `INTOUT[0]` = char (or −1 if outside the font range);
/// `PTSOUT[0]` = cell width, `PTSOUT[2]`/`[4]` = left/right offsets (0
/// for the monospace system font).
///
/// All of it is read off `vwk->cur_font` in EmuTOS (`vdi_text.c:1066`):
/// the ADE range that decides the −1, and the advance width. The advance
/// is `off_table[k+1] - off_table[k]`, i.e. the **unscaled** cell width
/// even on a scaled workstation (`make_header` copies the `off_table`
/// pointer verbatim, `vdi_text.c:810`), which the DDA is then applied to
/// (`:1082-1087`). No built-in face carries `F_HORZ_OFF`, so the two
/// offset slots stay at the zeroes EmuTOS pre-writes.
///
/// **Only the x components are written.** EmuTOS `vdi_vqt_width`
/// (`vdi_text.c:1062-1096`) pre-zeroes `PTSOUT[2]`/`PTSOUT[4]` (`:1072-1073`),
/// assigns `PTSOUT[0]` on the in-range path (`:1081`), and never touches the
/// y components `PTSOUT[1]`/`[3]`/`[5]` — unlike `vqt_extent`, which bzeroes
/// its eight slots first (`:1031`). Writing them would clobber caller memory
/// the VDI has no claim on; the reference dump leaves them at the poison
/// pattern (`tosdump/reference/FINDINGS-r2.md` R2-2).
pub fn vqt_width(pb: &Vdipb, vwk: &Vwk) {
    let base = font::font(vwk.cur_font);
    let f = text::effective_font(vwk);
    let k = pb.intin(0);
    pb.set_ptsout(2, 0);
    pb.set_ptsout(4, 0);
    if !(f.first_ade..=f.last_ade).contains(&k) {
        pb.set_intout(0, -1);
    } else {
        pb.set_intout(0, k);
        // Monospace: every `off_table` delta is the base cell width.
        let mut w = base.max_cell_width;
        if vwk.scaled {
            w = text::act_siz(w.max(0) as u16, vwk.dda_inc, vwk.t_sclsts) as i16;
        }
        pb.set_ptsout(0, w);
    }
    pb.set_contrl(4, 1);
    pb.set_contrl(2, 3);
}

/// Opcode 130 — `vqt_name`: the `INTIN[0]`-th (1-based) font face.
///
/// EmuTOS `vdi_vqt_name` (`vdi_text.c:1100-1134`) walks `font_ring` counting
/// **distinct `font_id`s** and stops on the `element`-th one; an element that
/// the walk never reaches — including element ≤ 0 — falls back to the
/// built-in `fon6x6` (`:1125-1126`). It reports the *ring*, not `cur_font`,
/// and does not change it (`docs/api/vdi/inquiry/_text.md#vqt_name-130`).
///
/// All three built-in system faces carry `font_id = 1`
/// (`bios/fnt_st_6x6.c:91`, `fnt_st_8x8.c:147`, `fnt_st_8x16.c:275`, mirrored
/// in [`font::RING`]), so the count reaches 1 on the first ring member and never
/// advances: element 1 selects `fon6x6` and every other element falls back to
/// `fon6x6` too. The result is therefore the same for every index — which is
/// exactly what the reference dump records for `INTIN[0]` = 0..3
/// (`tosdump/reference/EMUTOS14-VDI-r2.BIN`, cases 54-57). The walk is coded
/// out rather than short-circuited because a loaded face with a different
/// `font_id` (`vst_load_fonts`, EmuTOS `font_ring[2]`) would make it bite.
///
/// `INTOUT[0]` = the selected face's `font_id`; `INTOUT[1..]` = its name, one
/// character per WORD, NUL-terminated and zero-padded to `NAME_LEN` (32), for
/// the 33 output WORDs the jump table declares (`vdi_main.c:105`).
/// Closes AUD-V1-011.
pub fn vqt_name(pb: &Vdipb) {
    let element = pb.intin(0);

    let mut count: i16 = 0;
    let mut current_font_id: i16 = -1;
    let mut found: Option<&'static font::SysFont> = None;
    for id in font::RING {
        let f = font::font(id);
        if f.font_id != current_font_id {
            current_font_id = f.font_id;
            count += 1;
            if count == element {
                found = Some(f);
                break;
            }
        }
    }
    // Out of range → the system font (EmuTOS `tmp_font = &fon6x6`).
    let f = found.unwrap_or(&font::FON_6X6);

    pb.set_intout(0, f.font_id);
    // EmuTOS writes the name one WORD per character *including* its NUL and
    // then pads to `FONT_NAME_LEN` counting only the characters it copied
    // (`vdi_text.c:1130-1133`), so the last slot it touches is always
    // `INTOUT[33]` — one WORD past the 33 the jump table declares
    // (`vdi_main.c:105`). The extra WORD is deterministically 0 and the TT
    // reference records it (`tosdump/reference/EMUTOS14-VDI-all.BIN`, cases
    // 80-83, `intout[33] = 0`), so it is reproduced rather than rounded off;
    // every GEM `intout` is the standard 45 WORDs, so the slot exists.
    let mut out = 1usize;
    let mut i = 0usize;
    loop {
        let ch = f.name.get(i).copied().unwrap_or(0);
        pb.set_intout(out, ch as i16);
        out += 1;
        if ch == 0 {
            break;
        }
        i += 1;
    }
    while i < font::NAME_LEN {
        pb.set_intout(out, 0);
        out += 1;
        i += 1;
    }
    pb.set_contrl(4, 1 + font::NAME_LEN as i16);
}

/// Opcode 131 — `vqt_fontinfo`: the current font's metrics, with the
/// thicken/skew slots reflecting the active text `style`.
///
/// Every slot is a field of `vwk->cur_font` in EmuTOS
/// (`vdi_vqt_fontinfo`, `vdi_text.c:1137-1174`), so the report follows the
/// face `vst_font`/`vst_height`/`vst_point` selected — including the
/// DDA-scaled scratch header — rather than the 8×16 module constants.
pub fn vqt_fontinfo(pb: &Vdipb, vwk: &Vwk) {
    let f = text::effective_font(vwk);
    pb.set_intout(0, f.first_ade);
    pb.set_intout(1, f.last_ade);
    pb.set_ptsout(0, f.max_cell_width);
    pb.set_ptsout(1, f.bottom);
    pb.set_ptsout(2, if vwk.style & F_THICKEN != 0 { f.thicken } else { 0 });
    pb.set_ptsout(3, f.descent);
    if vwk.style & F_SKEW != 0 {
        pb.set_ptsout(4, f.left_offset);
        pb.set_ptsout(5, f.half);
        pb.set_ptsout(6, f.right_offset);
    } else {
        pb.set_ptsout(4, 0);
        pb.set_ptsout(5, f.half);
        pb.set_ptsout(6, 0);
    }
    pb.set_ptsout(7, f.ascent);
    pb.set_ptsout(8, 0);
    pb.set_ptsout(9, f.top);
    pb.set_contrl(4, 2);
    pb.set_contrl(2, 5);
}
