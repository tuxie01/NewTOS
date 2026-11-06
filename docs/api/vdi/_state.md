# VDI shared state — workstation level (Phase 00b)

> Source: EmuTOS @ `1a7484ab`. Verified against
> `vdi/vdi_control.c:41-52` (`mcs_ptr`, `vwk_ptr[]`, `phys_work`),
> `:25-27` (handle range), `bios/lineavars.S:114-136`
> (`_INQ_TAB`/`_DEV_TAB`/`_SIZ_TAB`/`_CUR_WORK`), `vdi/vdi_defs.h:266-281`
> (extern declarations). Calling convention:
> [`_dispatcher.md`](_dispatcher.md).
>
> **Scope.** This file captures the **workstation-level** shared state
> (the handle table, capability tables, and the current-workstation
> pointer) **and** the **per-`Vwk` attribute-state model** (the
> line/marker/fill/text/mode/clip/colour fields with their open-time
> defaults and valid ranges) — the shared "current attribute state" the
> attribute setters ([`attributes/`](attributes/)) write and the output
> and inquiry handlers read. The `MAP_COL`/palette machinery detail lives
> with the colour setter/inquiry; struct layout is in
> [`../structures/vwk.md`](../structures/vwk.md).
>
> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> The **values** `DEV_TAB`/`INQ_TAB`/`SIZ_TAB` report are contract
> (programs read them via `v_opnwk`/`v_opnvwk`/`vq_extnd`). The **storage**
> (Line-A globals, the `vwk_ptr[]` array, the `next_work` chain) is
> **reference** — NewTOS keeps an equivalent registry per ADR-011.

## The handle table (`vwk_ptr[]`)

- `static Vwk *vwk_ptr[NUM_VDI_HANDLES+1]` (`vdi_control.c:48`). Entry `n`
  points at the `Vwk` for VDI handle `n`; entry 0 is unused.
- Handles run `FIRST_VDI_HANDLE = 1` (`= VDI_PHYS_HANDLE`, the physical
  workstation) through `LAST_VDI_HANDLE = NUM_VDI_HANDLES`
  (`vdi_control.c:25-27`).
- `NUM_VDI_HANDLES` is **build-config**: 64 on the small/STe target, 128
  by default (`include/config.h:391-393,1660-1662`). **Not 16.**
- `get_vwk_by_handle(handle)` (`vdi_control.c:178`) range-checks and
  returns `vwk_ptr[handle]` (NULL if closed/out-of-range). The dispatcher
  calls it for every opcode except `v_opnwk`/`v_opnvwk`; a NULL result is
  a silent no-op (VDI's only "error").

### Lifecycle transitions

| Event | Effect on `vwk_ptr[]` / `CUR_WORK` |
|-------|-------------------------------------|
| `v_opnwk` (1) | `vwk_ptr[1] = &phys_work`; `vwk_ptr[2..] = NULL`; `CUR_WORK = &phys_work` (`vdi_control.c:542-567`) |
| `v_opnvwk` (100) | first free slot in `[2..LAST]` → new `Mxalloc`'d `Vwk`; `CUR_WORK = new` (`:420-459`) |
| `v_clsvwk` (101) | `vwk_ptr[handle] = NULL`, `Mfree`; `CUR_WORK = &phys_work` (`:462-491`) |
| `v_clswk` (2) | `Mfree` every `vwk_ptr[2..]`, set NULL; `CUR_WORK = vwk_ptr[1]` (`:573-590`) |

`build_vwk_chain()` (`:390-403`) relinks all live `Vwk`s through
`next_work` after every open/close, because some programs (Warp9) walk
the chain. **Reference** detail.

## `CUR_WORK`

- `Vwk *CUR_WORK` (Line-A global, `lineavars.S:136`; extern
  `vdi_defs.h:281`) — the currently-active virtual workstation.
- Always kept pointing at a **valid open** workstation (the physical one
  as the fallback), even on an early-exit open failure
  (`vdi_control.c:417`) and after any close.
- Used by the Line-A flood fill (which reads the fill colour from
  `CUR_WORK`) and mirrored from the resolved `vwk` at the end of every
  non-close dispatch (`vdi_main.c:183-190`).

## The capability tables — `DEV_TAB`, `INQ_TAB`, `SIZ_TAB`

Three WORD arrays living as Line-A globals, populated by `v_opnwk` from
ROM templates and patched with the live geometry. Their **contents** are
the contract; where they physically live is reference.

| Table | Size | ROM template | Returned by | Live patch |
|-------|------|--------------|-------------|------------|
| `DEV_TAB` | 45 WORDs (`lineavars.S:118`) | `DEV_TAB_rom` (`vdi_control.c:128`) | `INTOUT` of `v_opnwk`/`v_opnvwk`; `vq_extnd` mode 0 | `[0,1]` res−1, `[3,4]` pixel size, `[13]` #colours, `[35]` colour-cap, `[39]` palette (`update_rez_dependent`, `:203-208`) |
| `INQ_TAB` | 45 WORDs (`lineavars.S:114`) | `INQ_TAB_rom` (`vdi_control.c:76`) | `INTOUT` of `vq_extnd` mode 1 | `[4]` #planes, `[5]` video-LUT (`:210-213`); `[19]` clip flag patched per query in `vq_extnd` |
| `SIZ_TAB` | 12 WORDs used / 15 allocated (`lineavars.S:132`) | `SIZ_TAB_rom` (`vdi_control.c:58`) | `PTSOUT` of `v_opnwk`/`v_opnvwk`; `vq_extnd` mode 0 | `[0..3]` char metrics via `text_init` |

Full byte-index tables:
- `DEV_TAB` / `SIZ_TAB` → [`workstation/100-v_opnvwk.md`](workstation/100-v_opnvwk.md).
- `INQ_TAB` → [`workstation/102-vq_extnd.md`](workstation/102-vq_extnd.md).

The convenient aliases `xres = DEV_TAB[0]`, `yres = DEV_TAB[1]`,
`numcolors = DEV_TAB[13]` (`vdi_defs.h:94-98`) are used throughout the VDI.

## `phys_work` and `mcs_ptr`

- `Vwk phys_work` — the static attribute area for the physical
  workstation (extern `vdi_control.c:49`); never heap-allocated, so
  `v_opnwk` cannot fail for memory.
- `MCS *mcs_ptr` (`vdi_control.c:41`) — mouse-cursor save area, sized by
  `v_planes` in `update_rez_dependent`. Mouse-subsystem state; documented
  fully under [`input/_mouse.md`](input/_mouse.md).

## The per-`Vwk` attribute-state model

Every open workstation carries its own **attribute state** — the pens,
sizes, styles, writing mode, clip rectangle, and font that its drawing
calls use. The attribute setters ([`attributes/`](attributes/)) mutate
these fields; the output primitives and the `vq*_attributes` inquiries
read them. `init_wk` (`vdi_control.c:269-380`) fills them at open time;
some defaults come from the caller's `work_in`/`INTIN` open array, the
rest from fixed `#define`s and the device tables.

The **field set + valid ranges + defaults** are the contract (a program
observes them through `v_opnvwk`, the setters' echoed values, and the
inquiries). The **field storage/order in the `Vwk`** is reference
([`../structures/vwk.md`](../structures/vwk.md)); NewTOS may lay it out
freely.

### Attribute-state fields (defaults from `init_wk` / `text_init2`)

| Field | Set by | Valid range | Open default | Spec |
|-------|--------|-------------|--------------|------|
| `line_index` | `vsl_type` (15) | 0..6 (type 1..7 − 1) | 0 (`DEF_LINE_STYLE`−1) | [`attributes/_line-marker.md`](attributes/_line-marker.md) |
| `line_color` | `vsl_color` (17) | `MAP_COL[0..numcolors-1]` | `MAP_COL[INTIN[2]]` (open pen) | — |
| `line_width` | `vsl_width` (16) | odd, `1..SIZ_TAB[6]` | `DEF_LWID` (`SIZ_TAB[4]`) | — |
| `line_beg` / `line_end` | `vsl_ends` (108) | 0..2 (squared/arrow/round) | 0 / 0 (`SQUARED`) | — |
| `ud_ls` | `vsl_udsty` (113) | any 16-bit mask | `LINE_STYLE[0]` = `0xFFFF` | — |
| `mark_index` | `vsm_type` (18) | 0..5 (type 1..6 − 1) | 2 (`DEF_MARK_STYLE`−1 = asterisk) | — |
| `mark_height` | `vsm_height` (19) | `DEF_MKHT..MAX_MKHT` | `DEF_MKHT` (`SIZ_TAB[9]`) | — |
| `mark_scale` | `vsm_height` (19) | ≥1 (integer) | 1 | — |
| `mark_color` | `vsm_color` (20) | `MAP_COL[…]` | `MAP_COL[INTIN[4]]` (open pen) | — |
| `fill_style` | `vsf_interior` (23) | 0..4 (`FIS_*`) | 0 (`FIS_HOLLOW`) | [`attributes/_fill.md`](attributes/_fill.md) |
| `fill_index` | `vsf_style` (24) | pattern 1..24 or hatch 1..12 | `init_wk`: 1-based validated value; **setter: 0-based** (D-VDI-8) | — |
| `fill_color` | `vsf_color` (25) | `MAP_COL[…]` | `MAP_COL[INTIN[9]]` (open pen) | — |
| `fill_per` | `vsf_perimeter` (104) | 0 / 1 | **TRUE** (perimeter drawn) | — |
| `ud_patrn[]` | `vsf_udpat` (112) | 16 or 16·planes WORDs | `ROM_UD_PATRN` (16 WORDs) | — |
| `multifill` | `vsf_udpat` (112) | 0 / 1 | 0 | — |
| `cur_font` | `vst_font` (21), `vst_height`/`vst_point` | loaded face | `def_font` (system font: 8×8 when `V_REZ_VT < 400`, else 8×16; `text_init`, `vdi_text.c:612`, [`workstation/01-v_opnwk.md`](workstation/01-v_opnwk.md)) | [`attributes/_text.md`](attributes/_text.md) |
| `text_color` | `vst_color` (22) | `MAP_COL[…]` | `MAP_COL[INTIN[6]]` (open pen) | — |
| `style` | `vst_effects` (106) | bits & `INQ_TAB[2]` (0x1F) | 0 (no effects) | — |
| `h_align` / `v_align` | `vst_alignment` (39) | 0..2 / 0..5 | 0 / 0 (left / baseline) | — |
| `chup` | `vst_rotation` (13) | 0/900/1800/2700/3600 | 0 | — |
| `pts_mode` / `scaled` / `dda_inc` / `t_sclsts` | `vst_height`/`vst_point`/`vst_font` | scaling state | FALSE / FALSE / — / — | — |
| `num_fonts` | `vst_load_fonts`/`unload` (119/120) | ≥ `font_count` | `font_count` | — |
| `wrt_mode` | `vswr_mode` (32) | 0..3 (`WM_*` = `MD_*`−1) | 0 (`WM_REPLACE`) | [`attributes/_mode-clip-color.md`](attributes/_mode-clip-color.md) |
| `clip` | `vs_clip` (129) | 0 / 1 | FALSE (off) | — |
| `xmn_clip`/`ymn_clip`/`xmx_clip`/`ymx_clip` | `vs_clip` (129) | within `[0,0]..[xres,yres]` | `0,0,xres,yres` (full device) | — |
| `xfm_mode` | (set at open from `INTIN[10]`) | 0 = NDC, 2 = RC | `INTIN[10]` (caller) | [`workstation/100-v_opnvwk.md`](workstation/100-v_opnvwk.md) |

`flip_y` is forced to 1 by `init_wk` and by the size/inquiry handlers that
return magnitudes (`vsm_height`, `vst_height`, `vqm_attributes`, …). The
colours above are stored as **PEL** (hardware register index) via
`MAP_COL[]` — pen `n` → `MAP_COL[n]`, with `MAP_COL[1] = numcolors-1`.

### The colour-request palette (`REQ_COL` / `req_col2`)

Separate from the per-`Vwk` PEL fields is the **device palette request**:
`WORD REQ_COL[16][3]` (+ `req_col2[240][3]` on `EXTENDED_PALETTE`), the
per-entry R/G/B *as requested* by `vs_color` (op 14) in VDI's 0..1000
range. `init_colors` (`vdi_col.c:708`) seeds it from `st_palette`;
`vs_color` overwrites an entry with the **raw** (unclamped) request and
programs the DAC with a `[0,1000]`-clamped copy; `vq_color` (op 26) reads
`REQ_COL` back. This is a **device-global** (Line-A) array, not per-`Vwk`.

## Input and cursor state

> **Scope note.** These are the **device-global** input/cursor variables
> (Line-A globals in EmuTOS), *not* per-`Vwk` attribute state — the mouse
> and the logical input mode are properties of the physical device, shared
> by every workstation. Full behaviour lives in
> [`input/_input.md`](input/_input.md), [`input/_mouse.md`](input/_mouse.md)
> and [`input/_mouse-packet.md`](input/_mouse-packet.md); this section only
> catalogues the shared slots so the state map is complete.
>
> Source: `vdi/vdi_defs.h:258,272,285-294`, `bios/lineavars.S:123-173`
> (offsets), initialised by `vdimouse_init` (`vdi/vdi_mouse.c:565-615`).
> The **values/semantics** are contract (programs read them via `vq_mouse`,
> `vqin_mode`, and the locator/string terminators); the **storage** (Line-A
> addresses) is reference — NewTOS keeps equivalent per-device state.

### Input mode (set by `vsin_mode` 33, read by `vqin_mode` 115)

| Global | Device | Value |
|--------|--------|-------|
| `loc_mode` | locator | `0` = request, `1` = sample |
| `val_mode` | valuator | tracked for ABI; no handler (op 29 = nop) |
| `chc_mode` | choice | tracked; `v_choice` ignores it (dummy) |
| `str_mode` | string | `0` = request, `1` = sample |

Each holds `mode − 1` (`vdi_input.c:122-141`); all default to `0`
(request) at `vdimouse_init` (`vdi_mouse.c:575-578`), which
`vdi_v_opnwk` runs (`vdi_control.c:563`) and `vdi_v_opnvwk` does not. See
[`input/_input.md`](input/_input.md).

**The AES writes `str_mode` too.** `gsx_char()` issues `vsin_mode(4, 2)`
before every `v_string` (`aes/gemgsxif.c:459-463`), and `chkkbd()` calls it
from the dispatcher's scheduler on every pass (`aes/gemdisp.c:157-177`), so
once the AES has ticked, `vqin_mode(4)` reports **1** on any EmuTOS machine
until the next `v_opnwk`. `loc_mode` has a second writer for the same reason
— `gsx_setmousexy()` (`aes/gemgsxif.c:484-488`), reached from `appl_tplay`
alone. Measured on the reference TT: `EMUTOS14-VDI-all.BIN` records 40
(device 4 → 1) and 6/38/39 (devices 1/2/3 → 0).

### Mouse position / buttons / cursor visibility

| Global | Type | Meaning | Written by | Read by |
|--------|------|---------|-----------|---------|
| `GCURX` / `GCURY` | WORD | cursor x / y, clamped to `[0,xres]`/`[0,yres]` — and `xres`/`yres` are `DEV_TAB[0]`/`[1]`, the last coordinate, so this is `[0,width-1]`/`[0,height-1]` | packet handler (`scrn_clip`); `v_locator` seeds from `PTSIN` | `vq_mouse`, locator, cursor draw |
| `MOUSE_BT` | WORD | button bitmask: bit0 = left, bit1 = right (2-4 = ext buttons under `CONF_WITH_EXTENDED_MOUSE`) | packet handler (`mouse_int`) | `vq_mouse` |
| `cur_ms_stat` | UBYTE | status: bit5 = moved, bit6/7 = right/left changed, bit0/1 = live state | `mouse_int` | `gloc_key` (locator terminators) |
| `TERM_CH` | WORD | last input terminator/char (scancode<<8 \| ASCII) | `gloc_key`/`gchr_key`/`gchc_key` | locator/string/choice outputs |
| `HIDE_CNT` | WORD | cursor hide depth; visible only at 0; show-underflow clamped to 0 | `v_show_c`/`v_hide_c` (`dis_cur`/`hide_cur`) | VBL draw (`vb_draw`) |
| `draw_flag` / `mouse_flag` | UBYTE | "redraw pending" / "form being redefined" (VBL interlock) | `mov_cur`, `set_mouse_form`, `dis_cur`/`hide_cur` | `vb_draw` |
| `newx` / `newy` | WORD | pending cursor coords for the VBL redraw | `mov_cur` | `vb_draw` |

`GCURX`/`GCURY` init to `xres/2`, `yres/2` (`vdi_mouse.c:582-583`) — with
the aliases above that is `(width-1)/2`, `(height-1)/2`, i.e. **319/239** at
640x480, one pixel up and left of the midpoint of the pixel counts (V17);
`HIDE_CNT = 1` (hidden) at `vdimouse_init`. Full model: [`input/_mouse.md`](input/_mouse.md),
[`input/_mouse-packet.md`](input/_mouse-packet.md).

### The exchange-vector slots (`vex_*` 118, 125-127, 134)

| Global | Type | Installed by | Called on | Init |
|--------|------|--------------|-----------|------|
| `user_but` | `void(*)(void)` | `vex_butv` (125) | button change | `just_rts` |
| `user_mot` | `void(*)(void)` | `vex_motv` (126) | coordinate change | `just_rts` |
| `user_cur` | `void(*)(void)` | `vex_curv` (127) | cursor (re)draw | `mov_cur` |
| `tim_addr` | `void(*)(int)` | `vex_timv` (118) | Timer-C tick | `just_rts` |
| `user_wheel` | `PFVOID` | `vex_wheelv` (134) | wheel (`CONF_WITH_EXTENDED_MOUSE`) | `just_rts` |

Each `vex_*` returns the **old** pointer in `CONTRL[9..10]` and installs the
new one from `CONTRL[7..8]` (`vdi_mouse.c:370-418`, `vdi_misc.c:100-112`);
programs chain the previous vector. `mcs_ptr`/`mouse_cdb` (below) hold the
save-under buffer and the interleaved sprite — a **reference** rasterisation
detail. See [`input/_mouse.md`](input/_mouse.md).

## NewTOS implementation notes (Phase 12)

- The handle registry is a NewTOS structure (not a raw `Vwk *[]` at a
  Line-A address); `CUR_WORK`, the chain, and the tables are reimplemented
  per **ADR-011** as per-physical-device state (each screen device owns
  its `DEV_TAB`/`INQ_TAB`/`SIZ_TAB` + palette). The **values** returned to
  programs must match EmuTOS's for the chosen framebuffer profile.
- **Open questions** (see [`_discrepancies.md`](_discrepancies.md)):
  the `NUM_VDI_HANDLES`/`MAX_VERTICES` profile (D-VDI-3). **Resolved
  2026-09-04, D-VDI-7:** `DEV_TAB[13/35/39]` / `INQ_TAB[4/5]` for the live
  Falcon-truecolor framebuffer are `256`/`1`/`0` and `16`/`0`
  (`kernel/src/vdi_backend.rs:29-40`), not the 16-colour/4-plane figures an
  earlier revision of this note assumed for a "640×480×16-colour"
  bring-up screen — **correction (A86, 2026-09-21):** that framing is
  stale on two counts, colour depth (see D-VDI-7) and resolution (the live
  default is 1920×1080, `kernel/src/drivers/mod.rs:31-33`, not 640×480).
  `DEV_TAB[0,1]`/`INQ_TAB` derive from the live per-device geometry, so
  this model needs no change when the negotiated geometry changes — only
  the concrete numbers quoted in cross-referenced specs are geometry- (not
  colour-) dependent. `DEV_TAB[39]`'s "palette size" is also not what it
  sounds like — see
  [`workstation/100-v_opnvwk.md`](workstation/100-v_opnvwk.md#mismatches-flagged-step-2-candidates)
  mismatch #4.
