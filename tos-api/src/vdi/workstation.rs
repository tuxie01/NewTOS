//! Workstation lifecycle — `v_opnvwk` / `v_opnwk` / `v_clsvwk` /
//! `v_clswk` / `v_clrwk` and `init_wk`.
//!
//! Spec: [`docs/api/vdi/workstation/`]. `init_wk` validates the
//! caller's `work_in` into the [`Vwk`](super::vwk::Vwk) and returns the
//! device capability tables (`DEV_TAB` → `INTOUT`, `SIZ_TAB` → `PTSOUT`).
//!
//! `DEV_TAB_ROM` / `SIZ_TAB_ROM` are the EmuTOS templates
//! (`vdi/vdi_control.c`), patched at runtime with the real screen
//! geometry and colour count from the [`RasterBackend`].

use super::backend::{ClipRect, RasterBackend};
use super::vwk::{self, validate_color_index, Vwk};
use super::{fill_patterns, font, Vdipb};

// EmuTOS `vdi_defs.h` limits used in the capability tables.
const MAX_LINE_STYLE: i16 = 7;
const MAX_LINE_WIDTH: i16 = 40;
const MAX_FILL_PATTERN: i16 = 24;
const MAX_FILL_HATCH: i16 = 12;
const FIS_PATTERN: i16 = 2; // interior style: dither pattern

/// `DEV_TAB[39]` — the DAC's RGB choice count, EmuTOS `get_palette()`
/// (`bios/screen.c:831-869`, "we return the same values as Atari TOS 4.04"),
/// derived from the workstation's plane count the way the Falcon030/TOS4
/// target profile derives it from the VIDEL mode (`bios/screen.c:840-846`):
///
/// - a 4-bit-per-pixel VIDEL mode (`(mode & VIDEL_BPPMASK) == VIDEL_4BPP`,
///   16 pens) → **4096**;
/// - any other VIDEL depth — 1, 2 or 8 bpp, and the 16-bit truecolor mode
///   this backend runs (`PLANES = 16`, `kernel/src/vdi_backend.rs`) → **0**.
///
/// The one VIDEL case a plane count cannot tell apart is the ST-compatible
/// flag (`mode & VIDEL_COMPAT`, `:843`), which returns 4096 for the
/// ST-High/ST-Medium 1- and 2-plane modes too; NewTOS has no ST-compatible
/// video mode, so a 1- or 2-plane workstation is a plain VIDEL mode here.
/// Measured on the Falcon truecolor reference: `DEV_TAB[39] = 0`
/// (`EMUTOS14-VDI-all-falcontc.BIN` cases 33/274, V03).
fn palette_precision(planes: u16) -> i16 {
    if planes == 4 {
        4096
    } else {
        0
    }
}

/// `DEV_TAB` template (45 WORDs), EmuTOS `DEV_TAB_rom`. Indices
/// `[0],[1],[3],[4],[5],[13],[35],[39]` are patched from the live device by
/// [`init_wk`] (EmuTOS `update_rez_dependent`, `vdi_control.c:203-208`).
/// `[3]/[4]` (pixel size) carry the ROM's ST-High placeholder 372 here
/// and are overwritten with [`get_std_pixel_size`] at open time.
pub(crate) const DEV_TAB_ROM: [i16; 45] = [
    639, 399, 0, 372, 372, // 0..4  res-1 x/y, precision, pixel w/h (patched)
    // [5] font heights: EmuTOS `text_init` sets DEV_TAB[5] to the number of
    // system-font (font_id==1) sizes (`vdi_text.c:661`). Patched below from
    // `font_ring_fold()`; the ring holds three (6x6/8x8/8x16).
    0, MAX_LINE_STYLE, 0, 6, 8, // 5..9  font heights, line styles, lw flag, marker types/sizes
    1, MAX_FILL_PATTERN, MAX_FILL_HATCH, 2, 10, // 10..14 fonts, fill pats, hatches, colours, #GDP
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, // 15..24 GDP ids (bar..justified text)
    3, 0, 3, 3, 3, 0, 3, 0, 3, 2, // 25..34 GDP attribute codes #1..#10
    0, 1, 1, 0, 2, // 35..39 colour cap, text rot, polyfill, cell array, palette size
    2, 1, 1, 1, 2, // 40..44 #locator/valuator/choice/string devices, ws type
];

/// `SIZ_TAB` template (12 WORDs = 6 pairs), EmuTOS `SIZ_TAB_rom`. The
/// character-size fields `[0..3]` are placeholders here (EmuTOS' ROM
/// template is likewise `{0,7,0,7}`); [`siz_tab`] overwrites them with the
/// `text_init` fold before the table is returned.
const SIZ_TAB_ROM: [i16; 12] = [
    0, 7, 0, 7, // min/max char width/top — placeholder, filled by `siz_tab()`
    1, 0, MAX_LINE_WIDTH, 0, // min line width, reserved, max line width, reserved
    15, 11, 120, 88, // min/max marker width/height
];

/// Maximum polygon vertices (`INQ_TAB[14]`), EmuTOS `MAX_VERTICES`. Shares
/// the one definition with the real polygon cap in [`super::primitives`],
/// so the number advertised here is the number a program may actually
/// pass. See [`super::primitives::MAX_VERTS`] for why it is 1024.
const MAX_VERTICES: i16 = super::primitives::MAX_VERTS as i16;

/// `INQ_TAB` template (45 WORDs), EmuTOS `INQ_TAB_rom` — the extended
/// workstation capabilities returned by `vq_extnd` mode 1. Indices
/// `[4]` (planes), `[5]` (video LUT) and `[19]` (clip flag) are patched
/// from the live device / workstation.
const INQ_TAB_ROM: [i16; 45] = [
    4, 1, 0x1F, 0, 1, // 0..4  controller type, bg cols, text styles, scale rasters, planes
    0, 50, 1, 1, 4, // 5..9  video LUT, perf factor, contour fill, char rotation, writing modes
    2, 1, 0, 0, MAX_VERTICES, // 10..14 highest input mode, text align, inking, rubber band, max verts
    -1, 1, 0, 0, 0, // 15..19 max intin, mouse buttons, wide-line style/mode, clip flag
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 20..29 extended precision / bezier
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 30..39 raster flag / reserved
    0, 0, 0, 0, 0, // 40..44 printer borders / page size
];

/// EmuTOS' "standard" pixel size in 1/1000 mm, computed from the live
/// resolution (`bios/screen.c:872-876`, `get_std_pixel_size`, the VIDEL /
/// TT path taken by the Falcon030/TOS4 target profile — `HAS_VIDEL`).
/// Patched into `DEV_TAB[3]/[4]` by [`init_wk`] via `update_rez_dependent`
/// (`vdi_control.c:205`). For the live 1920×1080 framebuffer this yields
/// 278/278 (never the ROM template's ST-High 372).
pub(crate) fn get_std_pixel_size(xres: i16, yres: i16) -> (i16, i16) {
    let width = if xres < 640 { 556 } else { 278 };
    let height = if yres < 400 { 556 } else { 278 };
    (width, height)
}

/// The `text_init` fold over the font ring (EmuTOS `vdi_text.c:614-661`):
/// `(min_char_width, min_top, max_char_width, max_top, size_count)` across
/// every system face (`font_id == 1`) in [`font::RING`]. EmuTOS folds
/// `SIZ_TAB[0..3]` and counts `DEV_TAB[5]` in that one loop, so both are
/// taken from here — the ring is the single source, which is what keeps
/// the reported sizes and the installed faces from drifting apart.
///
/// For the three built-in faces (6×6, 8×8, 8×16, all `font_id == 1`) this
/// yields EmuTOS' `{5,4,7,13}` and a size count of 3.
fn font_ring_fold() -> (i16, i16, i16, i16, i16) {
    let (mut min_cw, mut min_top, mut max_cw, mut max_top) = (i16::MAX, i16::MAX, 0i16, 0i16);
    let mut sizes = 0i16;
    for &id in &font::RING {
        let f = font::font(id);
        if f.font_id != font::FONT_ID {
            continue; // only the system font contributes (`vdi_text.c:632`)
        }
        min_cw = min_cw.min(f.max_char_width);
        min_top = min_top.min(f.top);
        max_cw = max_cw.max(f.max_char_width);
        max_top = max_top.max(f.top);
        sizes += 1;
    }
    (min_cw, min_top, max_cw, max_top, sizes)
}

/// `SIZ_TAB` (12 WORDs) with the character-size fields `[0..3]` filled by
/// the `text_init` fold (EmuTOS `vdi_text.c:598-642`) rather than left at
/// the ROM template's placeholder `{0,7,0,7}`. `[4..11]` (line / marker
/// sizes) keep the ROM values. `[0]` = min char width, `[1]` = min top,
/// `[2]` = max char width, `[3]` = max top.
///
/// `SIZ_TAB[1]` is load-bearing beyond the report: it is the `DEF_CHHT`
/// that an application passes back to `vst_height` to restore the default
/// text size, and with the full ring folded in that is the 6×6 face's
/// `top` = 4, not the 8×16 face's 13.
fn siz_tab() -> [i16; 12] {
    let mut siz = SIZ_TAB_ROM;
    let (min_cw, min_top, max_cw, max_top, _) = font_ring_fold();
    siz[0] = min_cw;
    siz[1] = min_top;
    siz[2] = max_cw;
    siz[3] = max_top;
    siz
}

/// Populate a freshly-allocated `Vwk` from `work_in` (INTIN) and write
/// the device capability tables back into the parameter block. Mirrors
/// EmuTOS `init_wk`.
fn init_wk(vwk: &mut Vwk, pb: &Vdipb, backend: &dyn RasterBackend) {
    let xres = backend.width() as i16;
    let yres = backend.height() as i16;
    let ncol = backend.num_colors();
    let planes = backend.planes();

    // ── attributes from work_in (INTIN[1..10]); [0] device id, [5]
    //    font are ignored per spec. Validate-or-default each. ──
    let line_type = pb.intin(1);
    vwk.line_index = if (1..=MAX_LINE_STYLE).contains(&line_type) {
        line_type - 1
    } else {
        0 // DEF_LINE_STYLE (solid)
    };
    vwk.line_color = validate_color_index(pb.intin(2), ncol);

    let mark_type = pb.intin(3);
    vwk.mark_index = if (1..=6).contains(&mark_type) {
        mark_type - 1
    } else {
        2 // DEF_MARK_STYLE (asterisk)
    };
    vwk.mark_color = validate_color_index(pb.intin(4), ncol);

    vwk.text_color = validate_color_index(pb.intin(6), ncol);

    let fill_int = pb.intin(7);
    vwk.fill_style = if (0..=4).contains(&fill_int) { fill_int } else { 0 };
    // Clamp work_in[8] to the current interior style's index range
    // (EmuTOS `vdi_control.c:305-311`). NB: `init_wk` stores the *1-based*
    // validated value (no `-1`), unlike `vsf_style` which stores it
    // 0-based — the deliberate D-VDI-8 quirk, preserved here.
    let fi = pb.intin(8);
    let max = if vwk.fill_style == FIS_PATTERN { MAX_FILL_PATTERN } else { MAX_FILL_HATCH };
    vwk.fill_index = if (1..=max).contains(&fi) { fi } else { 1 };
    vwk.fill_color = validate_color_index(pb.intin(9), ncol);

    vwk.xfm_mode = pb.intin(10); // 0 = NDC, 2 = RC (screen is RC)

    // `text_init2(vwk)` (`vdi_control.c:333`, `vdi_text.c:572-590`): the
    // workstation starts on `def_font`, the face `text_init` chose for the
    // screen height (`vdi_text.c:612`: 8×8 under 400 lines, else 8×16). The
    // other text fields `text_init2` resets (effects, scaling, alignment,
    // `chup`, `pts_mode`, `loaded_fonts`) are `Vwk::EMPTY`'s, which every slot is
    // reset to before this runs; the scratch buffers have no NewTOS field
    // (`docs/COMPATIBILITY.md` "GDOS font chain"). Opening any workstation empties
    // the global `font_ring[2]` (`vdi_text.c:588`, R13). Evaluated for the live `V_REZ_VT` rather than cached
    // by `v_opnwk`: NewTOS changes no resolution after boot, and a
    // `v_opnvwk` without a prior `v_opnwk` (the kernel's boot screen) gets
    // the same face (`docs/api/vdi/workstation/01-v_opnwk.md` § Default font).
    vwk.cur_font = font::FontRef::System(font::text_init_def_font(yres));
    font::clear_loaded_ring();

    // Non-work_in defaults: clip off → full screen (inclusive bounds).
    vwk.clip = false;
    vwk.xmn_clip = 0;
    vwk.ymn_clip = 0;
    vwk.xmx_clip = xres - 1;
    vwk.ymx_clip = yres - 1;

    // Default user-defined fill pattern = the ROM pattern (EmuTOS
    // `init_wk`, `vdi_control.c:335-339`), so `vsf_interior(FIS_USER)`
    // before any `vsf_udpat` fills with it instead of with zeroes.
    vwk.ud_patrn = fill_patterns::ROM_UD_PATRN;

    // ── outputs: DEV_TAB → INTOUT (45), SIZ_TAB → PTSOUT (12) ──
    let (pw, ph) = get_std_pixel_size(xres, yres);
    let mut dev = DEV_TAB_ROM;
    dev[0] = xres - 1;
    dev[1] = yres - 1;
    dev[3] = pw; // pixel width  (get_pixel_size)
    dev[4] = ph; // pixel height (get_pixel_size)
    dev[5] = font_ring_fold().4; // number of system-font sizes (`text_init`)
    dev[13] = ncol as i16;
    dev[35] = if planes == 1 { 0 } else { 1 }; // colour capable? (v_planes==1)
    dev[39] = palette_precision(planes); // DAC RGB-choice count (get_palette)
    for (i, &v) in dev.iter().enumerate() {
        pb.set_intout(i, v);
    }
    for (i, &v) in siz_tab().iter().enumerate() {
        pb.set_ptsout(i, v);
    }

    // Output counts (the dispatcher pre-zeroed them).
    pb.set_contrl(4, 45);
    pb.set_contrl(2, 6);

    // ── truecolor: seed this workstation's private pseudo-palette ──
    // Under TRUECOLOR_MODE (v_planes > 8) each workstation gets its own
    // colour tables, seeded from the device-global default (EmuTOS
    // `init_wk`, `vdi_control.c:354-363`: copy `REQ_COL`/`req_col2` into
    // `ext->req_col`, then `set_color16` for each pen). NewTOS' device
    // default is the backend's 256-entry palette, exposed as the requested
    // RGB via `query_color`.
    if planes > 8 {
        // SAFETY: single trap path; `vwk.handle` is the just-opened ws.
        if let Some(ext) = unsafe { vwk::ext(vwk.handle) } {
            for (l, slot) in ext.req_col.iter_mut().enumerate() {
                let (r, g, b) = backend.query_color(l as u16);
                *slot = [r as i16, g as i16, b as i16];
            }
            // Build the pseudo-palette for pens 0..254 (EmuTOS loops
            // `l < 255`, `vdi_control.c:360`): pen 255 maps to hardware
            // register 15, which no seeded pen writes, so its slot stays 0
            // — folded, `palette[255]` likewise stays 0 (its default is
            // black anyway), so `vq_color(255, flag=1)` is `(0,0,0)` as in
            // EmuTOS. The default req_col is already in 0..1000 range.
            for l in 0..255usize {
                let rgb = ext.req_col[l];
                ext.set_color16(l, rgb);
            }
        }
    }
}

/// Opcode 100 — open a virtual workstation.
///
/// Returns whether `init_wk` ran, i.e. whether the workstation was actually
/// opened. EmuTOS `vdi_v_opnvwk` bails out before `init_wk` when no handle
/// (`vdi_control.c:425-427`) or no memory (`:444-446`) is left, and it is
/// `init_wk` that raises `flip_y` (`:379`) — so the trap's `d0` is 1 only on
/// the successful path (see [`crate::vdi::FLIP_Y_MAGNITUDES`]).
pub fn v_opnvwk(pb: &Vdipb, backend: &dyn RasterBackend) -> bool {
    // SAFETY: single trap path.
    match unsafe { vwk::alloc_handle() } {
        Some(vwk) => {
            let handle = vwk.handle;
            init_wk(vwk, pb, backend);
            pb.set_contrl(6, handle);
            true
        }
        None => {
            pb.set_contrl(6, 0); // no free handle → fail (VDI: no error code)
            false
        }
    }
}

/// Opcode 102 — `vq_extnd`: extended workstation inquiry (EmuTOS
/// `vdi_vq_extnd`). Mode 0 (`INTIN[0] == 0`) re-returns the standard open
/// tables (`DEV_TAB` → `INTOUT`, `SIZ_TAB` → `PTSOUT`); mode ≠ 0 returns the
/// extended `INQ_TAB` plus the workstation's current clipping rectangle.
pub fn vq_extnd(pb: &Vdipb, vwk: &crate::vdi::vwk::Vwk, backend: &dyn RasterBackend) {
    let xres = backend.width() as i16;
    let yres = backend.height() as i16;
    let ncol = backend.num_colors();
    let planes = backend.planes();

    if pb.intin(0) == 0 {
        // Mode 0: identical to v_opnvwk's device tables.
        let (pw, ph) = get_std_pixel_size(xres, yres);
        let mut dev = DEV_TAB_ROM;
        dev[0] = xres - 1;
        dev[1] = yres - 1;
        dev[3] = pw; // pixel width  (get_pixel_size)
        dev[4] = ph; // pixel height (get_pixel_size)
        dev[5] = font_ring_fold().4; // number of system-font sizes (`text_init`)
        dev[13] = ncol as i16;
        dev[35] = if planes == 1 { 0 } else { 1 }; // colour capable? (v_planes==1)
        dev[39] = palette_precision(planes); // DAC RGB-choice count (get_palette)
        for (i, &v) in dev.iter().enumerate() {
            pb.set_intout(i, v);
        }
        for (i, &v) in siz_tab().iter().enumerate() {
            pb.set_ptsout(i, v);
        }
    } else {
        // Mode 1: the extended table + the live clipping rectangle.
        let mut inq = INQ_TAB_ROM;
        inq[4] = planes as i16; // number of planes = v_planes (vdi_control.c:210)
        // Video lookup table present: 0 for truecolor (v_planes==16) or a
        // mono monitor, 1 otherwise (vdi_control.c:211-213). The colour-only
        // framebuffer never reports MON_MONO, so this reduces to the
        // truecolor test.
        inq[5] = if planes == 16 { 0 } else { 1 };
        inq[19] = i16::from(vwk.clip); // live clip flag
        for (i, &v) in inq.iter().enumerate() {
            pb.set_intout(i, v);
        }
        pb.set_ptsout(0, vwk.xmn_clip);
        pb.set_ptsout(1, vwk.ymn_clip);
        pb.set_ptsout(2, vwk.xmx_clip);
        pb.set_ptsout(3, vwk.ymx_clip);
        for i in 4..12 {
            pb.set_ptsout(i, 0);
        }
    }
    pb.set_contrl(4, 45); // # INTOUT
    pb.set_contrl(2, 6); // # PTSOUT (6 pairs)
}

/// Opcode 1 — open the physical workstation (the screen device).
///
/// Unlike `v_opnvwk` this also runs `timer_init()` and `vdimouse_init()`
/// (`vdi/vdi_control.c:562-563`) — on **every** call, not once: the mouse
/// comes back to its open-time state whatever the previous session or
/// program left (centre, buttons 0, hide count 1, the default arrow, the
/// `vex_*` defaults, packets on again; [`super::mouse::vdimouse_init`], V53).
/// Its first act is to put all four logical input devices back into REQUEST
/// mode (`vdi/vdi_mouse.c:575-578`). That reset is observable through
/// `vqin_mode`, and it matters because the AES leaves the string device in
/// SAMPLE mode on every keyboard poll (`input::set_input_mode`): a program
/// that opens the physical workstation itself sees 0 again, one that opens
/// only a virtual workstation on the AES's physical handle sees the AES's 1.
pub fn v_opnwk(pb: &Vdipb, backend: &dyn RasterBackend) {
    // SAFETY: single trap path.
    let vwk = unsafe { vwk::open_physical() };
    let handle = vwk.handle;
    init_wk(vwk, pb, backend);
    super::mouse::timer_init();
    super::mouse::vdimouse_init(backend);
    super::input::reset_input_modes();
    // `esc_init(vwk)` (`vdi_control.c:564`) — "enter graphics mode": the
    // alpha console's `escfn2` (home, cursor off, clear), CON04.
    super::escape::esc_init(backend);
    pb.set_contrl(6, handle);
}

/// Opcode 2 — `v_clswk`: close the physical workstation. EmuTOS
/// `vdi_v_clswk` (`vdi_control.c:573-590`) closes **all** open virtual
/// workstations (handles 2..=`NUM_VDI_HANDLES`) and leaves the physical
/// workstation (handle 1) open, making it line-A's current workstation
/// again. This is the opposite of the previous NewTOS behaviour, which
/// aliased opcode 2 onto `v_clsvwk` and so closed the *physical* screen
/// device on the standard standalone-GEM call (AUD-V1-001).
///
/// Then `timer_exit()` and `vdimouse_exit()` (`vdi_control.c:587-588`): the
/// mouse is switched off until the next `v_opnwk` — no packet moves or draws
/// the pointer, the mouse vectors are `just_rts`
/// ([`super::mouse::vdimouse_exit`], V53); `timer_exit` restores the
/// `etv_timer` chain NewTOS never built (ADR-031) and leaves `tim_addr`.
/// Last, `esc_exit(vwk)` (`vdi_control.c:589`) — "back to console mode":
/// the alpha console's `escfn3` (clear, cursor on, CR), CON04. NewTOS does
/// not model line-A's `CUR_WORK` (documented acceptable deviation
/// AUD-V1-010).
pub fn v_clswk(backend: &dyn RasterBackend) {
    // SAFETY: single trap path.
    unsafe { vwk::close_all_virtual() };
    super::mouse::vdimouse_exit();
    super::escape::esc_exit(backend);
}

/// Opcode 101 — `v_clsvwk`: close a single virtual workstation. EmuTOS
/// `vdi_v_clsvwk` (`vdi_control.c:462-491`) refuses to close the physical
/// workstation this way: `if (handle == VDI_PHYS_HANDLE) return;`
/// (AUD-V1-002). Closing an already-closed handle is a no-op (the
/// dispatcher already resolved it to an open workstation).
pub fn v_clsvwk(handle: i16) {
    if handle == vwk::PHYS_HANDLE {
        return; // can't close the physical workstation this way
    }
    // SAFETY: single trap path.
    unsafe { vwk::free_handle(handle) };
}

/// Opcode 3 — `v_clrwk`: clear the workstation. EmuTOS `vdi_v_clrwk`
/// (`vdi_control.c:600-618`) memsets the whole screen from `v_bas_ad`
/// (the clip rectangle is not consulted), with a fill **byte** that depends
/// on the colour model:
///
/// - **Truecolor** (`TRUECOLOR_MODE`, `v_planes > 8`, `vdi_defs.h:22`):
///   `fill = 0xff`, so every 16-bit screen word becomes `0xFFFF`
///   ([`clear_screen_word`](RasterBackend::clear_screen_word)). That word is
///   *not* pen 0: the pseudo-palette's white is `(31<<11)|(31<<6)|31` =
///   `0xFFDF`, because `set_color16` never sets the green LSB
///   (`vdi_col.c:601-614`). The two differ by exactly `OVERLAY_BIT`
///   (`0x0020`, `vdi_fill.c:26`), which the truecolor seed fill masks out of
///   the screen word so a cleared background and a pen-0 fill compare equal
///   (`vdi_fill.c:702,709,726,733`). Clearing through PEL 0 instead would
///   also make the cleared colour follow the palette, where EmuTOS clears to
///   this constant however pen 0 has been redefined.
/// - **Palette** (`v_planes <= 8`): `fill = 0x00`, i.e. every plane bit 0 =
///   colour index 0, which under NewTOS' folded `MAP_COL`
///   ([`VwkExt`](super::vwk::VwkExt)) is PEL 0.
pub fn v_clrwk(backend: &dyn RasterBackend) {
    if backend.planes() > 8 {
        backend.clear_screen_word(0xFFFF);
        return;
    }
    let (w, h) = (backend.width(), backend.height());
    backend.fill_rect(0, 0, w as i16, h as i16, 0, ClipRect::full(w, h));
}

#[cfg(test)]
mod tests {
    use super::palette_precision;

    /// V03: `DEV_TAB[39]` follows EmuTOS `get_palette()` on a VIDEL
    /// (`bios/screen.c:840-846`) — 4096 only for the 4-bpp (16-pen) mode,
    /// 0 for every other depth, the 16-bit truecolor profile included
    /// (measured 0 on the Falcon truecolor reference, cases 33/274).
    #[test]
    fn dev_tab_39_is_4096_for_four_planes_and_0_for_every_other_videl_depth() {
        assert_eq!(palette_precision(16), 0, "VIDEL_TRUECOLOR: return 0");
        assert_eq!(palette_precision(8), 0, "VIDEL_8BPP: neither COMPAT nor 4BPP");
        assert_eq!(palette_precision(4), 4096, "VIDEL_4BPP: return 4096");
        assert_eq!(palette_precision(2), 0, "VIDEL_2BPP, not ST-compatible");
        assert_eq!(palette_precision(1), 0, "VIDEL_1BPP, not ST-compatible");
    }
}
