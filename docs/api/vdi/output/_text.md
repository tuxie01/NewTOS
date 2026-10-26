# VDI text output — `v_gtext` (Phase 00b D4)

> Source: EmuTOS @ `1a7484ab` — `vdi/vdi_text.c:546` (`vdi_v_gtext`),
> `:288` (`output_text`), `:176` (`calc_width`); glyph blit in
> `vdi/vdi_textblit.c`. Calling convention:
> [`../_dispatcher.md`](../_dispatcher.md). Text attribute state from the
> [`Vwk`](../../structures/vwk.md), set via the
> [text attribute setters](../attributes/_text.md) and summarised in
> [`../_state.md`](../_state.md). **No output array, no error return.**
>
> The **justified** text variant (`v_justified`) is **GDP sub-opcode 10 of
> opcode 11 (`v_gdp`)**, not opcode 11 itself — it is documented in
> [`_gdp.md`](_gdp.md#justified-text-sub-op-10). This file covers the
> plain graphic-text output `v_gtext` (opcode 8).
>
> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> **Contract:** opcode 8; the `INTIN[]` character-code array with count in
> `CONTRL[3]` (one ADE code per WORD); `PTSIN[0..1]` as the single
> alignment point; which text attributes are consulted; the "no output"
> shape. **Reference:** `calc_width`, the alignment arithmetic in
> `output_text`, the DDA scaling, and the glyph blit (`vdi_textblit.c`) —
> NewTOS reimplements the raster text path with its own font subsystem.

| Op | Fn | Input | Draws |
|----|----|-------|-------|
| 8 | `v_gtext` | `CONTRL[3]` = char count; `INTIN[i]` = char code (one WORD each); `PTSIN[0..1]` = alignment `(x,y)` | the string in the current font, size, effects, rotation, colour and writing mode |

Native ABI: GEM trap `svc #2`, selector 115 in `x0`, `VDIPB*` in `x1`;
opcode 8 in `CONTRL[0]`, handle in `CONTRL[6]`, char count in `CONTRL[3]`,
codes in `INTIN[]`, position in `PTSIN[0..1]`. Dispatcher pre-loads
`CONTRL[2]=CONTRL[4]=0` (no outputs; `jmptb1[7]={0,0}`, `vdi_main.c:39`).

## EmuTOS source

**Primary location:** `vdi/vdi_text.c:546` — `vdi_v_gtext(Vwk *vwk)`, which
calls `output_text(vwk, CONTRL[3], INTIN, -1, NULL)` (`:548`). The width
argument `-1` means "compute the natural width" (via `calc_width`); the
`NULL` `justified` argument means "no justification" (that is the
distinction from GDP sub-op 10).

**Helpers called (reference):**
- `vdi_text.c:288` — `output_text()` — snapshots the draw state, applies
  alignment, walks the string, blits each glyph.
- `vdi_text.c:176` — `calc_width()` — computes the run width for
  horizontal alignment (accounts for monospace/proportional, scaling,
  thicken/skew/outline).
- `vdi_textblit.c` — the glyph bit-blit (clipping, effects, rotation).

**Globals read (attribute state, from `Vwk`):** `cur_font` (face + metrics
+ `dat_table`/`form_width`), `dda_inc`/`t_sclsts`/`scaled` (size scaling),
`wrt_mode`, `style` (effect bits `F_THICKEN`/`F_LIGHT`/`F_SKEW`/
`F_UNDER`/`F_OUTLINE`), `chup` (rotation 0/900/1800/2700), `h_align`/
`v_align`, `text_color` (applied by the blit), the clip rect. Reads
`CONTRL[3]` (char count), `INTIN[]` (codes), `PTSIN[0..1]` (position).

**Globals written (reference):** many Line-A blit globals (`DDAINC`,
`WRT_MODE`, `CLIP`, `XMINCL..YMAXCL`, `STYLE`, `CHUP`, `WEIGHT`,
`LITEMASK`, `SKEWMASK`, `LOFF`/`ROFF`, `FBASE`, `FWIDTH`, `DESTX`/`DESTY`,
…) — the assembler-support mirror; NewTOS replaces these with backend
parameters.

**Build configuration:**
- `CONF_WITH_VDI_TEXT_SPEEDUP` (default **1**, `include/config.h:1636-1638`) enables
  a direct-blit fast path (`ok_for_direct_blit`, `vdi_text.c:227`) taken
  only when there are no effects, no rotation, and left alignment. A pure
  raster optimisation — **reference**, no observable effect on the output.
- `HAVE_BEZIER` = 0 (`vdi_defs.h:18`) — not relevant to `v_gtext`.

## Source extraction summary

`v_gtext` draws a run of characters at a single anchor point using the
current text attributes. The character codes arrive **one ADE code per
`INTIN` WORD** (not packed bytes); `CONTRL[3]` is the character count.
`PTSIN[0..1]` is the *alignment point* — the glyph origin is derived from
it by the current horizontal/vertical alignment, not a fixed corner.

`output_text` first quick-outs if `count <= 0`. It computes the run width
(`calc_width`) to place the alignment point, snapshots the `Vwk` draw
state (font base/width, scaling DDA, writing mode, clip rect, effect
masks, rotation) into the blit globals, then converts `PTSIN[0..1]` into
the glyph origin using `h_align` (0=left, 1=centre, 2=right) and `v_align`
(0=baseline, 1=half, 2=ascent, 3=bottom, 4=descent, 5=top). Skew and
outline shift the origin slightly (outlined text starts one pixel earlier,
`vdi_text.c:347-352`). It then blits each glyph from the font bitmap,
advancing by the per-character cell width and honouring `wrt_mode` and the
clip rectangle.

Effects (`style`) drive the blit: `F_THICKEN` (bold), `F_LIGHT` (grey via
`LITEMASK`), `F_SKEW` (italic via `SKEWMASK` + left/right offsets),
`F_UNDER` (underline placed inside the cell, `:399-405`), `F_OUTLINE`.
Rotation `chup` is one of 0/900/1800/2700 on the screen driver (3600 folds
to 0). The size scaling state (`scaled`/`dda_inc`/`t_sclsts`, set by
`vst_height`/`vst_point`) may double or DDA-scale the glyphs.

## Trap interface

| | |
|--|--|
| Opcode | `CONTRL[0] = 8` |
| Handle | `CONTRL[6]` (resolved to the drawing `Vwk`) |
| `# INTIN` (`CONTRL[3]`) | character count — `INTIN[i]` = char `i`'s ADE code (one WORD) |
| `# PTSIN` (`CONTRL[1]`) | 1 — `PTSIN[0..1]` = alignment `(x,y)` |
| `# INTOUT` / `# PTSOUT` | 0 / 0 |

Native ABI: GEM trap `svc #2`, selector 115 in `x0`, `VDIPB*` in `x1`.

## Inputs

| Slot | Name | Meaning |
|------|------|---------|
| `CONTRL[3]` | char count | number of character codes (one WORD each) in `INTIN`; **the string length is `CONTRL[3]`, there is no NUL terminator** |
| `INTIN[i]` | char code | ADE (Atari Device Encoding) code of character `i`; used to index the font's `off_table`/`dat_table` |
| `PTSIN[0]`, `PTSIN[1]` | `x`, `y` | the alignment point (raster coordinates); the glyph origin is derived from it via `h_align`/`v_align` |

Each `INTIN[i]` is a full WORD holding one character code — the GEM-side
binding passes the string as an array of WORDs, not packed bytes.

## Outputs

**None.** No `INTOUT`, no `PTSOUT` (`{nptsout,nintout}={0,0}`). No error
return — an invalid handle is a dispatcher no-op; `count <= 0` draws
nothing.

## Verification criteria

- **Nominal:** `CONTRL[3]=N`, `INTIN[0..N-1]` = the ADE codes,
  `PTSIN=[x,y]`, default attributes (left/baseline, `cur_font`=system
  font, no effects, `chup=0`) → `N` glyphs placed left-to-right with the
  baseline through `y` and the first glyph's left edge at `x`, in
  `text_color`, clipped to the `Vwk` clip rect when `clip` is on.
- **Length via `CONTRL[3]`:** with `CONTRL[3]=3` only three glyphs draw,
  regardless of any following `INTIN` slots — the count, not a terminator,
  bounds the string.
- **Empty string:** `CONTRL[3]=0` (or negative) draws nothing (no error).
- **Centre alignment:** `h_align=1` with the same input places the run so
  its width straddles `x` (origin shifted left by `width/2`).
- **Effects/rotation are read here:** setting `style`/`chup` via
  `vst_effects`/`vst_rotation` before the call changes the rendered glyphs
  (bold/italic/underline/outline, right-angle rotation).

## Edge cases discovered in source

1. **The character count is `CONTRL[3]` (the `INTIN` count), not a
   terminator.** `v_gtext` passes `CONTRL[3]` straight to `output_text` as
   `count` (`vdi_text.c:548`); there is no scan for a NUL. A caller sets
   `CONTRL[3]` to the exact number of glyphs.
2. **Codes are one-per-WORD.** `calc_width` and the blit both step `str`
   one WORD per character (`vdi_text.c:189`); a program must not pack two
   chars into one WORD.
3. **Alignment substitutes 0 for out-of-range values.** `output_text`'s
   `h_align`/`v_align` `switch` use the `default` (left/baseline) for any
   value outside the documented range (`vdi_text.c:354-390`), matching the
   `vst_alignment` setter which already clamps to 0.
4. **Outlined text shifts the origin one pixel earlier** and widens the
   run (`+2px` per char in `calc_width`, `vdi_text.c:207-208,352`), which
   moves the centre/right alignment point too.
5. **`v_gtext` never fails on a missing font** — `cur_font` always points
   at a loaded face (the system font at open, per
   [`../attributes/_text.md`](../attributes/_text.md)); an unmapped char
   code indexes the font's fallback glyph, not an error.

## Dependencies

### Requires
- An open workstation with a valid `cur_font` (set at open to the system
  font; changed by `vst_font`/`vst_height`/`vst_point`).
- The text attribute state (`style`, `chup`, `h_align`/`v_align`,
  `text_color`, the scaling state) — see
  [`../attributes/_text.md`](../attributes/_text.md).
- `wrt_mode` and the clip rectangle.

### Used by
- `gdp_justified` (GDP sub-op 10) calls `output_text` with the same path
  plus per-char/word deltas — see [`_gdp.md`](_gdp.md#justified-text-sub-op-10).

## Related data structures

- [`../../structures/vwk.md`](../../structures/vwk.md) — `cur_font`,
  `style`, `chup`, `h_align`/`v_align`, `text_color`, scaling fields.
- [`../_state.md`](../_state.md) — text attribute defaults and ranges.
- The `Fonthead` metrics (`top`/`half`/`ascent`/`bottom`/`descent`,
  `off_table`, `dat_table`, `form_width`) drive alignment and glyph
  placement — struct spec lives with the font subsystem.

## Cross-references

### Atari Compendium / Profibuch
- Not present in `references/` (no PDFs). Cross-check of the "count via
  `CONTRL[3]`, one code per WORD" ABI and the six vertical-alignment
  reference points is **deferred**.

## Configuration / version notes

- `CONF_WITH_VDI_TEXT_SPEEDUP` (default 1) adds a direct-blit fast path
  that only triggers for unrotated, unstyled, left-aligned text
  (`vdi_text.c:227-286`) — a raster optimisation with no observable effect
  on the drawn output.
- Rotation is limited to right angles on the screen driver (`chup` ∈
  {0,900,1800,2700}; 3600 folds to 0), matching `vst_rotation`.

## NewTOS implementation notes (Phase 12)

- The bring-up font is the single fixed system bitmap face used by the
  framebuffer text path; `cur_font` resolves to it until a real font
  subsystem lands, so size scaling is limited to integer doubling — but
  `v_gtext` must still honour the alignment/effect/rotation attributes.
- Char codes arrive **one per `INTIN` WORD**; the binding does not pack
  bytes. Length is `CONTRL[3]`, not a NUL.
- Glyph blit honours `wrt_mode` and clips to `vwk->{xmn..ymx}_clip`;
  target framebuffer = `vwk->device` (ADR-011). Effects (`style`) and
  rotation (`chup`) are read here, set by
  [`vst_effects`](../attributes/_text.md) / `vst_rotation`.
