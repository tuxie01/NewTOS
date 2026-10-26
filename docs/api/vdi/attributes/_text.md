# VDI text attribute setters (Phase 00b D3)

> Source: EmuTOS @ `1a7484ab` — `vdi/vdi_text.c`; effect-bit and default
> constants in `vdi/vdi_defs.h:117-124`. Calling convention:
> [`../_dispatcher.md`](../_dispatcher.md). Setter shape as in
> [`_line-marker.md`](_line-marker.md). The height/point/font setters walk
> the loaded-font ring to pick the **nearest available** face/size and report
> the *actual* metrics. No error return.
>
> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> **Contract:** opcode numbers, the input slot, the clamp/normalise/nearest
> rules, and the echoed value(s). **Reference:** the font-ring walk, the
> DDA scaling (`clc_dda`/`act_siz`/`make_header`), and the `Vwk` fields —
> NewTOS reimplements the raster text path with its own font subsystem.

## Trap interface (all setters below)

Native ABI: GEM trap `svc #2`, selector 115 in `x0`, `VDIPB*` in `x1`.
Per function: opcode in `CONTRL[0]`, handle in `CONTRL[6]`.

| Op | Fn | Reads | Effect → `Vwk` | Echoes |
|----|----|-------|----------------|--------|
| 22 | `vst_color`     | `INTIN[0]` colour | `validate_color_index`; `text_color = MAP_COL[i]` | `INTOUT[0]` = validated index |
| 106 | `vst_effects`  | `INTIN[0]` style bits | `style = INTIN[0] & INQ_TAB[2]` (mask to supported) | `INTOUT[0]` = accepted bits |
| 39 | `vst_alignment` | `INTIN[0]`=horiz, `INTIN[1]`=vert | horiz `0..2` else 0 → `h_align`; vert `0..5` else 0 → `v_align` | `INTOUT[0]`/`[1]` = accepted h/v |
| 13 | `vst_rotation`  | `INTIN[0]` angle (1/10°) | normalise to `0..3599`, then round to nearest 900 (0/900/1800/2700/**3600**); `chup` | `INTOUT[0]` = rounded angle |
| 21 | `vst_font`      | `INTIN[0]` face id | walk font ring for the face; **default to system font `fon6x6` if not found**; re-apply current size | `INTOUT[0]` = selected `font_id`; also leaves `flip_y = 1` (leaked from the internal size re-drive, edge case 7 below) |
| 12 | `vst_height`    | `PTSIN[1]` = requested char height (device units); `PTSIN[0]` ignored | pick the loaded font of the same face whose `top` is nearest ≤ request; scale via DDA if `top != request`; set `cur_font` | `PTSOUT[0..3]` = actual char width, char height, cell width, cell height; also sets `flip_y = 1` (trap `d0` = 1, edge case 7 below) |
| 107 | `vst_point`    | `INTIN[0]` = requested point size | pick the font of the same face whose `point` is nearest ≤ request (may double via DDA); `pts_mode = TRUE` | `INTOUT[0]` = actual point size; `PTSOUT[0..3]` = actual sizes; also sets `flip_y = 1` (trap `d0` = 1, edge case 7 below) |
| 119 | `vst_load_fonts`| `CONTRL[7-8]`/`[9]`/`[10-11]` (GDOS ptrs) | link extra font faces (`CONF_WITH_GDOS`) | `INTOUT[0]` = count of newly added faces (0 if already loaded / no GDOS) |
| 120 | `vst_unload_fonts`| `INTIN[0]` (select, ignored) | unload all extra faces (`CONF_WITH_GDOS`) | none |

Source: `vdi_text.c:988` (`vst_color`), `:881` (`vst_effects`), `:887`
(`vst_alignment`), `:924` (`vst_rotation`), `:939` (`vst_font`), `:681`
(`vst_height`), `:820` (`vst_point`), `:1292` (`vst_load_fonts`), `:1353`
(`vst_unload_fonts`); `setup_width_height` at `:668`.

### Value constants (`vdi_defs.h`)

| `#define` | Value | Meaning |
|-----------|-------|---------|
| `F_THICKEN` | 0x01 | thickened (bold) |
| `F_LIGHT` | 0x02 | light (greyed) |
| `F_SKEW` | 0x04 | skewed (italic) |
| `F_UNDER` | 0x08 | underlined |
| `F_OUTLINE` | 0x10 | outlined |
| `F_SHADOW` | 0x20 | shadowed — **not** in the supported mask (`INQ_TAB[2] = 0x1F`) |

### Text effects (`vst_effects`, masked by `INQ_TAB[2]`)

`vst_effects` stores `INTIN[0] & INQ_TAB[2]` and echoes the result
(`vdi_text.c:883`). `INQ_TAB[2] = 0x1F` (`vq_extnd` index 2 — "text styles
supported"), so only bits `0x01..0x10` survive; bit `0x20` (`F_SHADOW`) and
any higher bits are **cleared silently**.

| Bit | Effect | In `0x1F` mask? |
|-----|--------|:---------------:|
| 0x01 | thickened (bold) | yes |
| 0x02 | light (greyed) | yes |
| 0x04 | skewed (italic) | yes |
| 0x08 | underlined | yes |
| 0x10 | outlined | yes |
| 0x20 | shadowed | **no — stripped** |

### Alignment (`vst_alignment`)

- Horizontal `INTIN[0]`: `0..2` = left / centre / right; **out of range → 0**
  (left), not clamped to nearest (`vdi_text.c:894-896`).
- Vertical `INTIN[1]`: `0..5` = baseline / half / ascent / bottom / descent /
  top; **out of range → 0** (baseline) (`vdi_text.c:898-901`).

### Rotation (`vst_rotation`)

Mimics TOS3/TOS4: the tenths-of-a-degree angle is first **normalised** to
`0..3599` (add/subtract 3600), then **rounded to the nearest multiple of 900**
via `((angle+450)/900)*900`, giving `0`, `900`, `1800`, `2700`, or `3600`
(`vdi_text.c:924-936`). On a raster screen the drawing code treats 3600 like
0, but `chup` can genuinely hold **3600** (pass 3150..3599 to store it). The
rounded value is echoed and stored in `chup`.

### Height / point size — nearest-available

`vst_height` (`PTSIN[1]` = device height) and `vst_point` (`INTIN[0]` =
point size) both walk the font ring for the current `font_id`, choose the
largest loaded font not exceeding the request, and — if it does not match
exactly — scale it via a DDA (`clc_dda`/`act_siz`/`make_header`,
`vdi_text.c:770-817,1374`). `vst_point` may pick a font at *half* the request
and double it (`dda_inc = 0xFFFF`). Both return the **actual** cell metrics
via `setup_width_height` (`PTSOUT[0]`=`max_char_width`, `PTSOUT[1]`=`top`
(char height), `PTSOUT[2]`=`max_cell_width`, `PTSOUT[3]`=`top+bottom+1`
(cell height)); `vst_point` additionally returns the actual point size in
`INTOUT[0]`. `vst_height` sets `pts_mode = FALSE`, `vst_point` sets it TRUE.

### Font selection (`vst_font`)

`vst_font` searches the font ring for `INTIN[0]` (the face id). If **no
loaded face matches, it defaults to the built-in system font `fon6x6`**
(`vdi_text.c:963-964`) — it never fails. It then re-applies the current size
by internally calling `vst_point` (if `pts_mode`) or `vst_height`, and
echoes the selected face's `font_id` in `INTOUT[0]` (which may be the system
font's id when the request was unavailable).

---

## Verification criteria

- **`vst_color`:** `vst_color(1)` → `INTOUT[0]=1`,
  `text_color = MAP_COL[1] = numcolors-1`; out-of-range index echoes 1.
- **`vst_effects` mask:** `vst_effects(0x3F)` → `INTOUT[0]=0x1F`
  (bit `0x20` stripped); `vst_effects(0x05)` → `0x05`.
- **`vst_alignment`:** `vst_alignment(h=1, v=9)` → `INTOUT=[1,0]` (v out of
  `0..5` → 0); `vst_alignment(h=5, v=3)` → `[0,3]`.
- **`vst_rotation` rounding:** `vst_rotation(500)` → `INTOUT[0]=900`;
  `vst_rotation(-100)` → normalise to 3500 → `INTOUT[0]=3600`;
  `vst_rotation(3160)` → `3600`; `vst_rotation(449)` → `0`.
- **`vst_font` fallback:** `vst_font(<unloaded id>)` → `INTOUT[0]` = the
  system font's `font_id`, `cur_font = &fon6x6`.
- **`vst_height` nearest:** requesting a height between two available sizes
  returns the actual metrics of the chosen (scaled) font in `PTSOUT[0..3]`,
  not the request.
- **`vst_load_fonts` no-GDOS / already-loaded:** returns `INTOUT[0]=0`.

---

## Edge cases discovered in source

1. **`vst_rotation` can store 3600** (`vdi_text.c:934-935`) — pass an angle
   that normalises to 3150..3599 to get `chup = 3600`; other code treats
   3600 like 0.
2. **`vst_alignment` substitutes 0, not the nearest bound** — an
   out-of-range horiz/vert value becomes left/baseline, not right/top
   (`vdi_text.c:894-901`).
3. **`vst_effects` silently strips unsupported bits** via `& INQ_TAB[2]`
   (0x1F) — `F_SHADOW` (0x20) never takes effect on the screen driver.
4. **`vst_font` never fails** — an unknown face falls back to the system
   font `fon6x6` and reports *that* font's id, so `INTOUT[0]` may differ
   from the requested face id (`vdi_text.c:963-964,984`).
5. **`vst_font` re-drives the size setter** — it calls `vst_point` or
   `vst_height` internally with the saved point size, temporarily swapping
   `INTIN`/`PTSIN`/`PTSOUT` (`vdi_text.c:969-982`), so a face change keeps
   the current size.
6. **`vst_height` reads `PTSIN[1]` (the y magnitude)**, not `PTSIN[0]` —
   `PTSIN[0]` (requested width) is ignored (`vdi_text.c:704`).
7. **`vst_height`/`vst_point` set `flip_y = 1`** via the shared
   `setup_width_height()` helper (`vdi_text.c:678`) — the global
   "coordinate vs magnitude" flag consumed by the trap return (`d0`,
   `vdi_asm.S:124-125`; see [`../_dispatcher.md`](../_dispatcher.md)).
   **`vst_font` leaks the same side effect**: it re-drives the saved size
   by internally calling `vst_point`/`vst_height` (edge case 5 above)
   against a *redirected* `PTSIN`/`PTSOUT` (a local `dummy[4]` buffer, so
   the caller's real `PTSOUT` — 0 words declared for op 21 — is never
   written), but that inner call still runs `setup_width_height()`
   against the one global `flip_y`, so `vst_font` also leaves `flip_y =
   1` on return even though it writes no `PTSOUT`. Every other setter in
   this file (`vst_color`/`effects`/`alignment`/`rotation`/
   `load_fonts`/`unload_fonts`) leaves `flip_y` at the `0` reset on entry
   to `screen()` (`vdi_main.c:135`). Not previously documented; see
   [`../_discrepancies.md`](../_discrepancies.md) D-080 (global log).

---

## Dependencies

### Requires
- An open workstation; the font ring populated by `text_init`/`text_init2`
  (`cur_font = def_font` at open); `numcolors`/`MAP_COL[]` for `vst_color`;
  `INQ_TAB[2]` for the effect mask.
- `CONF_WITH_GDOS` (default **1**) for `vst_load_fonts`/`vst_unload_fonts` to
  do anything beyond returning 0.

### Used by
- `v_gtext` (8) at draw time (consumes `cur_font`, `style`, `h_align`,
  `v_align`, `chup`, `text_color`, the scaling state).
- `vqt_attributes` (38), `vqt_extent` (116), `vqt_width` (117),
  `vqt_name` (130), `vqt_fontinfo` (131).

---

## Related data structures

- [`../../structures/vwk.md`](../../structures/vwk.md) — `cur_font`,
  `loaded_fonts`, `num_fonts`, `text_color`, `style`, `h_align`/`v_align`,
  `chup`, `pts_mode`/`scaled`/`dda_inc`/`t_sclsts` fields.
- [`../_state.md`](../_state.md) — attribute-state defaults table.

## Cross-references

### Atari Compendium / Profibuch
- Not present in `references/` (no PDFs). Cross-check of the rotation-rounding
  (esp. the 3600 case), the vertical-alignment range 0..5, the `F_SHADOW`
  strip, and the `vst_font` system-font fallback is **deferred**.

## Configuration / version notes

- **`vst_load_fonts` / `vst_unload_fonts` are gated by `CONF_WITH_GDOS`
  (default 1)** (`include/config.h:1620-1622`; `vdi_text.c:1294,1355`). Even compiled
  in, they only *link/unlink* fonts that GDOS (absent in a bare EmuTOS) would
  have supplied; without GDOS-supplied font data `vst_load_fonts` typically
  reports 0 new faces. If not compiled in, both are hard no-ops returning
  `INTOUT[0]=0`.
- **`vst_rotation` follows the TOS3/TOS4 model** (normalise-then-round). TOS1
  did **not** normalise before rounding, so it stored values from −32400 to
  +32400 and `vqt_extent` could return nothing for out-of-range angles
  (`vdi_text.c:919-923`). EmuTOS deliberately uses the TOS3/4 behaviour.

## NewTOS implementation notes (Phase 12)

- The bring-up font is a single fixed bitmap face (the framebuffer text
  path's system font); `vst_font`/`vst_load_fonts` therefore resolve to that
  face until a real font subsystem lands. `vst_height`/`vst_point` report the
  one available cell size (plus any integer doubling we support), not
  arbitrary scaling — but must still return *actual* metrics in `PTSOUT`.
- Text rotation storage must match EmuTOS exactly: `vst_rotation` echoes
  and stores one of **five** values — `0`, `900`, `1800`, `2700`, or
  `3600` (`((angle+450)/900)*900` after normalising to `0..3599`,
  `vdi_text.c:928-935`). **`3600` is genuinely stored/echoed, not folded
  to `0`** — an input that normalises to `3150..3599` rounds up to
  `3600`, and both `chup` and `INTOUT[0]` hold `3600` verbatim; there is
  **no `% 3600`** on the stored value. Do not apply one in the setter.
  What *is* true: the **rendering** switch in `output_text` (`v_gtext`'s
  glyph-placement code, `vdi_text.c:408-441`) has no explicit `case
  3600:`, so a stored `chup=3600` falls into the same `default`
  (unrotated) branch as `chup=0` — visually indistinguishable from `0`,
  but that is a coincidence of the rendering switch's case labels, not
  evidence that the stored/echoed attribute value is folded. NewTOS must
  reproduce both facts: store/echo `3600` verbatim from `vst_rotation`,
  and render `chup=3600` identically to `chup=0` in the glyph blit (four
  right-angle rendering cases: 0/900/1800/2700, with 3600 sharing the 0
  path). See [`../_discrepancies.md`](../_discrepancies.md) D-VDI-18 —
  this paragraph previously (incorrectly) said "3600 folds to 0" without
  qualification, contradicting the contract/verification-criteria above
  (`:26,76-79,113-114,127-129`); corrected.
- `style`/`h_align`/`v_align`/`chup`/`text_color` are consumed by `v_gtext`
  (opcode 8) at draw time.
