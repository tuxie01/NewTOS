# VDI `v_gdp` — generalized drawing primitives (Phase 00b D4)

> Source: EmuTOS @ `1a7484ab` — `vdi/vdi_gdp.c:426` (`vdi_v_gdp`),
> `:371` (`gdp_curve`), `:229` (`gdp_rbox`), `:178` (`clc_arc`);
> `vdi/vdi_text.c:1177` (`gdp_justified`). Calling convention:
> [`../_dispatcher.md`](../_dispatcher.md). Opcode **11**; the
> **sub-function is in `CONTRL[5]`**. Attribute state from the
> [`Vwk`](../../structures/vwk.md). **No output array, no error return.**
>
> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> **Contract:** opcode 11; the **GDP sub-opcode numbers** (1..10) in
> `CONTRL[5]`; each sub-op's exact `PTSIN`/`INTIN` layout; the angle units
> (tenths of a degree, CCW); which attributes each sub-op consults; the
> full-circle/full-ellipse handling; the "no output" shape. **Reference:**
> the sine table + `clc_pts`/`clc_arc` flattening, `clc_nsteps`, the
> rounded-box corner geometry, and all pixel loops (which reduce to the
> line/fill primitives) — NewTOS generates its own geometry.

`v_gdp` (opcode 11) is a single dispatcher over `CONTRL[5]`
(`vdi_gdp.c:433`). The dispatcher pre-loads `CONTRL[2]=CONTRL[4]=0`
(`jmptb1[10]={0,0}`, `vdi_main.c:42`); the sub-handlers produce no output.

| `CONTRL[5]` | Primitive | Input (raster coords) | Attribute state used | Shape |
|:---:|-----------|-----------------------|----------------------|-------|
| 1  | `v_bar`      | `PTSIN[0..3]` = two rect corners | fill; **line-independent solid `fill_color` perimeter if `fill_per`** | filled rectangle |
| 2  | `v_arc`      | `PTSIN[0..1]` centre, **`PTSIN[6]`** radius; `INTIN[0]`/`INTIN[1]` begin/end angle | **line** (`line_color`/width/style) | open circular arc |
| 3  | `v_pieslice` | as arc (`PTSIN[6]` radius) | fill; solid `fill_color` perimeter if `fill_per` | filled pie wedge |
| 4  | `v_circle`   | `PTSIN[0..1]` centre, **`PTSIN[4]`** radius | fill; solid `fill_color` perimeter if `fill_per` | filled circle |
| 5  | `v_ellipse`  | `PTSIN[0..1]` centre, **`PTSIN[2]`** xrad, **`PTSIN[3]`** yrad | fill; solid `fill_color` perimeter if `fill_per` | filled ellipse |
| 6  | `v_ellarc`   | centre, `PTSIN[2]` xrad, `PTSIN[3]` yrad; `INTIN[0]`/`INTIN[1]` angles | **line** | open elliptical arc |
| 7  | `v_ellpie`   | as ellarc (`PTSIN[2/3]` radii) | fill; solid `fill_color` perimeter if `fill_per` | filled elliptical pie |
| 8  | `v_rbox`     | `PTSIN[0..3]` = two rect corners | **line** (ends forced `SQUARED`) | rounded-box outline |
| 9  | `v_rfbox`    | `PTSIN[0..3]` = two rect corners | fill | filled rounded box |
| 10 | `v_justified`| see [below](#justified-text-sub-op-10) | text | justified graphic text |

Sub-op **13** (`v_bez`, Bezier) exists only `#if HAVE_BEZIER`, which is
**hardcoded 0** (`vdi_defs.h:18`), so it is compiled out of every default
build. Any `CONTRL[5]` value the `switch` does not handle is a silent
no-op (`vdi_gdp.c:433-479`, no `default`).

Native ABI (all): GEM trap `svc #2`, selector 115 in `x0`, `VDIPB*` in
`x1`; opcode 11 in `CONTRL[0]`, sub-op in `CONTRL[5]`, handle in
`CONTRL[6]`.

---

## Bar (sub-op 1)

**Source:** `vdi_gdp.c:434-446`. Fills via `vdi_vr_recfl(vwk)`
([`_fill.md`](_fill.md#vr_recfl--filled-rectangle-opcode-114)), then, if
`fill_per == TRUE`, strokes the four edges as a **5-point `polyline` in
`fill_color`** with `LN_MASK = 0xffff` (solid).

- **PTSIN:** `PTSIN[0..1]` = one corner, `PTSIN[2..3]` = the opposite
  corner (any order; `vr_recfl`'s `arb_corner` sorts them in place).
- The perimeter's five points are derived **in place** from the (now
  sorted) `PTSIN[0..3]` (`xy[3..9]`, `vdi_gdp.c:439-442`). The outline uses
  the **fill colour**, solid — not the line attributes.

## Curves: arc / pieslice / circle / ellipse / ellarc / ellpie (sub-ops 2–7)

**Source:** `vdi_gdp.c:371` (`gdp_curve`) → `clc_arc` (`:178`) flattening.

**Centre:** `xc = PTSIN[0]`, `yc = PTSIN[1]` for all six.

**Radii (`vdi_gdp.c:378-384`):**

| Sub-op | `CONTRL[5]` | xrad source | yrad source |
|--------|:-----------:|-------------|-------------|
| `v_circle`   | 4 | `PTSIN[4]` | `mul_div(xrad, xsize, ysize)` (device aspect) |
| `v_arc`      | 2 | `PTSIN[6]` | `mul_div(xrad, xsize, ysize)` |
| `v_pieslice` | 3 | `PTSIN[6]` | `mul_div(xrad, xsize, ysize)` |
| `v_ellipse`  | 5 | `PTSIN[2]` | `PTSIN[3]` |
| `v_ellarc`   | 6 | `PTSIN[2]` | `PTSIN[3]` |
| `v_ellpie`   | 7 | `PTSIN[2]` | `PTSIN[3]` |

The circle/arc/pieslice family (`CONTRL[5] <= 4`) takes a **single**
radius: `PTSIN[4]` for the circle, `PTSIN[6]` for arc/pieslice
(`vdi_gdp.c:379`), and derives `yrad` from the device pixel aspect ratio
(`xsize`/`ysize` = `DEV_TAB[3]`/`[4]`). Negative radii are treated as
positive (`:390-393`).

**Angles** (arc/pieslice/ellarc/ellpie only): `INTIN[0]` = begin,
`INTIN[1]` = end, in **tenths of a degree** (0..3600), counter-clockwise.
`del_ang = end - begin` (`+TWOPI` if negative, `:412-414`). `v_circle`
(4) and `v_ellipse` (5) ignore `INTIN` and force `beg_ang=0`,
`end_ang=TWOPI` (3600) — a full closed shape (`:404-410`).

**Flattening:** the curve is approximated by `clc_nsteps()` line segments
(`MIN_ARC_CT`=32 … `MAX_ARC_CT`=128, chosen from the larger radius,
`vdi_gdp.c:347-363`); duplicate adjacent points are dropped
(`clc_arc:189-190`).

**Open vs closed and colour (`clc_arc:205-221`):**
- **arc (2) / ellarc (6)** — the point list is stroked as an **open
  polyline** in **`line_color`** with the current line width/style
  (`polyline`/`wideline`, plus `arrow()` if an end is `ARROWED`).
- **pieslice (3) / ellpie (7)** — a point at the centre `(xc,yc)` is
  appended (`clc_arc:199-203`), then the whole thing goes through
  `polygon()`, so it is **filled** in `fill_color`; its boundary (only
  when `fill_per`) is the **solid width-1 `fill_color` polyline**
  `polygon` draws, **not** the line attributes.
- **circle (4) / ellipse (5)** — closed and filled via `polygon()`,
  perimeter as above.

> Earlier revisions of this table listed pieslice/ellpie as "fill + line";
> that was an extraction error — the closed/filled shapes never use the
> line attributes for their boundary. Verified against
> `clc_arc`/`polygon` (`vdi_gdp.c:210-221`, `vdi_fill.c:606-608`).

**Early-out (`vdi_gdp.c:395-402`):** after the radii are made
non-negative and before the angles and `clc_nsteps`, `gdp_curve` returns
without drawing when `clip` is set and

    (xc + xrad) < xmn_clip || (xc - xrad) > xmx_clip ||
    (yc + yrad) < ymn_clip || (yc - yrad) > ymx_clip

(strict comparisons; 16-bit `WORD` sums, EmuTOS is built `-mshort`,
`Makefile:235`). The box is the **radii only**: the line width
(`wideline`) and the arrowheads (`arrow`) of an open arc/ellarc are not
counted, so a wide or arrowed stroke that would reach into the clip rect
from outside `xc±xrad`/`yc±yrad` is **not drawn at all**. With clipping off
there is no early-out. Measured against the oracle: `vdi-v18.tsv` case 59,
`vdprobe` K11A (width 9) and K11D (arrowed) draw nothing, K11C (clip cuts
the curve) draws (TASK/V25).

## Rounded boxes: rbox (sub-op 8) / rfbox (sub-op 9)

**Source:** `vdi_gdp.c:229` (`gdp_rbox`).

- **PTSIN:** `PTSIN[0..3]` = two opposite corners of the box; `arb_line`
  normalises them to (ll, ur) (`vdi_gdp.c:240`).
- The corner radius is derived from the box size: x-radius ≈ 1/64 of the
  screen width (`xres>>6`), clamped to half the box side; y-radius scaled
  by the pixel aspect and clamped likewise (`vdi_gdp.c:254-255`). Each of
  the four corners is a 5-point quarter-arc (`CORNER_POINTS`=5), giving
  `RBOX_POINTS`=21 for the outline and `RFBOX_POINTS`=20 for the fill.
- **`v_rbox` (8)** strokes the closed outline (`RBOX_POINTS` points) with
  the current line attributes (`polyline`/`wideline` in `line_color`); the
  dispatcher **forces the line ends to `SQUARED`** for the duration and
  restores them afterwards (`vdi_gdp.c:457-465`).
- **`v_rfbox` (9)** fills the shape (`RFBOX_POINTS` points) via `polygon()`
  in `fill_color` (with the solid `fill_color` perimeter when `fill_per`,
  as for any filled polygon).

## Justified text (sub-op 10)

**Source:** `vdi_text.c:1177` (`gdp_justified`) → `output_text` (the same
engine as [`v_gtext`](_text.md)).

- **Char count:** `CONTRL[3] - 2` (`vdi_text.c:1187`) — the first two
  `INTIN` slots are flags, the rest are character codes.
- **INTIN layout:** `INTIN[0]` = interword-spacing flag (nonzero = expand
  spaces), `INTIN[1]` = interchar-spacing flag (nonzero = expand between
  all characters), `INTIN[2..]` = the character codes (one ADE code per
  WORD).
- **PTSIN layout:** `PTSIN[0..1]` = the alignment `(x,y)`; **`PTSIN[2]` =
  `max_x`**, the target run width (`vdi_text.c:1205`). (`PTSIN[3]` is
  unused.)
- It distributes the slack `max_x - natural_width` across word gaps
  (interword) and/or all characters (interchar), honouring the rotation
  `chup` (0/900/1800/2700), building per-word/per-char delta values
  (`JUSTINFO`), then calls `output_text(vwk, cnt, str, max_x, &just)`.
  Interword expansion per gap is clamped to ±½ `max_cell_width` when both
  flags are set (`vdi_text.c:1222-1233`).

### Justified-text rendering (rotation, alignment)

`gdp_justified` draws nothing itself; it hands the run to the same
`output_text` as `v_gtext`, with `width = max_x` and the `JUSTINFO`
(`vdi_text.c:1288`). Consequences:

- **Alignment width:** `output_text` computes `delh` from `h_align` with
  that `width` (`vdi_text.c:354-364`), i.e. centre/right alignment of a
  justified run is against `max_x`, not the natural width. Only a
  **negative** `max_x` makes `output_text` fall back to `calc_width`
  (`if (width < 0)`, `vdi_text.c:305-306`). `delv` and the skew additives
  are as for `v_gtext`.
- **Rotation:** the anchor (`DESTX`/`DESTY` per `chup`,
  `vdi_text.c:408-440`), the per-glyph rotation and blit position and the
  base advance along the rotated baseline (`text_blt`:
  `vdi_textblit.c:1331-1342` and `upda_dst` `:1402-1432` — up for 900,
  left for 1800, down for 2700) are exactly `v_gtext`'s.
- **Deltas follow the rotation:** `gdp_justified` puts the per-gap and
  per-character deltas on the axis of the rotated baseline
  (`vdi_text.c:1235-1252`, `:1268-1285`):

  | `chup` | interword | interchar |
  |--------|-----------|-----------|
  | 0 (and any other value) | `wordx = delword`, `rmwordx = dir` | `charx = delchar`, `rmcharx = dir` |
  | 900 | `wordy = -delword`, `rmwordy = -dir` | `chary = -delchar`, `rmchary = -dir` |
  | 1800 | `wordx = -delword`, `rmwordx = -dir` | `charx = -delchar`, `rmcharx = -dir` |
  | 2700 | `wordy = delword`, `rmwordy = dir` | `chary = delchar`, `rmchary = dir` |

  After each glyph's `text_blt` (which has already advanced the pen by the
  glyph width) the apply loop adds `charx/chary`, then one remainder pixel
  `rmcharx/rmchary` while `rmchar > 0`, and — when the character code is
  `' '` — `wordx/wordy` plus one `rmwordx/rmwordy` while `rmword > 0`
  (`vdi_text.c:476-492`). So at 900 the run grows upwards, at 1800 to the
  left, at 2700 downwards, each to `max_x` pixels.
- **Underline** runs from the rotated start to the pen's final `DESTX`
  (0/1800) or `DESTY` (900/2700), as for `v_gtext` (`vdi_text.c:500-510`).
- The direct-blit fast path is never taken for a justified run
  (`ok_for_direct_blit`, `vdi_text.c:235-236`).

Measured against the oracle: `tosdump/cases/vdi-v18-t2.tsv` cases 99/102/105
(900/1800/2700) and the `vdprobe` records T1600–T1632 (TASK/V18 part 2,
VDI-TEXT-003).

### Justified-text INTIN / PTSIN table

| Slot | Meaning |
|------|---------|
| `CONTRL[3]` | flags + chars; **char count = `CONTRL[3] - 2`** |
| `INTIN[0]` | interword flag (expand spaces) |
| `INTIN[1]` | interchar flag (expand between chars) |
| `INTIN[2..]` | character codes (one per WORD) |
| `PTSIN[0]`, `PTSIN[1]` | alignment `(x,y)` |
| `PTSIN[2]` | `max_x` = target run width |

---

## Verification criteria

- **Sub-op numbers:** `CONTRL[5]` selects
  bar=1, arc=2, pieslice=3, circle=4, ellipse=5, ellarc=6, ellpie=7,
  rbox=8, rfbox=9, justified=10 (`vdi_gdp.c:433-473`).
- **`v_circle` (4):** `PTSIN=[cx,cy, _,_, r]` (radius at `PTSIN[4]`),
  fills a disc of radius `r` centred at `(cx,cy)` in `fill_color`
  (yrad = `mul_div(r, xsize, ysize)`; equal to `r` on square pixels).
- **`v_arc` (2):** `PTSIN=[cx,cy, _,_,_,_, r]` (radius at `PTSIN[6]`),
  `INTIN=[a0,a1]` → an **open** arc from `a0` to `a1` (tenths of a degree,
  CCW) stroked in **`line_color`** with the current width/style.
- **`v_ellipse` (5):** `PTSIN=[cx,cy, xr, yr]` → filled ellipse with the
  given x/y radii; `INTIN` ignored (full `0..3600`).
- **`v_pieslice` (3):** like `v_arc` but **filled** (closes to centre) in
  `fill_color`; boundary solid `fill_color` when `fill_per` — never the
  line attributes.
- **Full-shape angle override:** `v_circle`/`v_ellipse` ignore `INTIN` and
  always draw the full closed shape.
- **`v_bar` (1):** `PTSIN[0..3]` = two corners → filled rectangle; with
  `fill_per=TRUE`, a solid `fill_color` outline is added (the fill itself
  via `vr_recfl` draws none).
- **`v_rbox` (8) vs `v_rfbox` (9):** same `PTSIN[0..3]`; 8 outlines with
  line attributes (ends forced squared), 9 fills.
- **`v_justified` (10):** `PTSIN[2]=max_x` stretches the run to width
  `max_x`; char count is `CONTRL[3]-2`; `INTIN[0]`/`INTIN[1]` select
  interword/interchar expansion.

## Edge cases discovered in source

1. **Radius index differs by primitive:** circle uses `PTSIN[4]`, but
   arc/pieslice use `PTSIN[6]` (`vdi_gdp.c:379`). A program reusing the
   circle layout for an arc puts the radius in the wrong slot. The
   ellipse family uses `PTSIN[2]`/`PTSIN[3]` for the two radii.
2. **Circle/ellipse ignore `INTIN`** — they force `0..3600` regardless of
   any angle a caller supplies (`vdi_gdp.c:404-410`). Only arc/pieslice/
   ellarc/ellpie read `INTIN[0..1]`.
3. **Negative and zero radii:** negative radii are made positive
   (`vdi_gdp.c:390-393`); a zero radius degenerates to the centre point.
4. **Closed filled shapes never use the line attributes for their
   boundary.** Pieslice/ellpie/circle/ellipse/rfbox all outline (when
   applicable) in solid `fill_color` via `polygon()` — a dashed/coloured
   *line* style has no effect. Only arc/ellarc (open) and rbox use the
   line attributes.
5. **`v_bar` perimeter is fill-coloured and solid**, driven off `fill_per`
   (a *fill* attribute), not the line attributes (`vdi_gdp.c:436-444`).
6. **`del_ang` wraps:** a negative `end - begin` adds `TWOPI`
   (`vdi_gdp.c:412-414`), so `INTIN=[3000, 600]` gives `del_ang =
   (600-3000)+3600 = 1200`: the CCW sweep drawn passes through 3600/0 and
   is **1200° wide — the *shorter* of the two possible spans** (the other,
   non-wrapping, CW-only span from 3000 down to 600 would be 2400° and is
   never drawn, because `del_ang` is always forced non-negative). An
   earlier revision of this note called the un-drawn 2400° span "the short
   one" — backwards: 2400 > 1200, so the wrapped span is the shorter one.
   Corrected; see [`../_discrepancies.md`](../_discrepancies.md) D-VDI-17.
7. **`v_justified` char count is `CONTRL[3]-2`**, and the two flags occupy
   `INTIN[0..1]` — the codes start at `INTIN[2]` (`vdi_text.c:1187-1192`).
8. **`gdp_curve` early-out** on a fully-clipped bounding box
   (`vdi_gdp.c:395-402`) — no segments generated. The box is `xc±xrad`,
   `yc±yrad` without the line width or arrowheads, so a wide/arrowed arc
   whose stroke alone would reach into the clip draws nothing.
9. **`v_rbox` forces squared line ends** and restores the caller's
   `line_beg`/`line_end` afterwards (`vdi_gdp.c:457-465`) — a program that
   set arrowed ends does not get arrowheads on a rounded box.

## Dependencies

### Requires
- An open workstation (dispatcher resolves `CONTRL[6]`).
- The fill attributes (bar/pieslice/circle/ellipse/ellpie/rfbox) — see
  [`../attributes/_fill.md`](../attributes/_fill.md); the line attributes
  (arc/ellarc/rbox) — see
  [`../attributes/_line-marker.md`](../attributes/_line-marker.md); the
  text attributes (justified) — see
  [`../attributes/_text.md`](../attributes/_text.md).
- `wrt_mode`, the clip rectangle, and the device aspect `xsize`/`ysize`
  (`DEV_TAB[3]`/`[4]`) for the circle-family y-radius.

### Used by
- All sub-primitives reduce to `polyline`/`wideline`/`polygon`/
  `vr_recfl`/`output_text` — see [`_lines-markers.md`](_lines-markers.md),
  [`_fill.md`](_fill.md), [`_text.md`](_text.md).

## Related data structures

- [`../../structures/vwk.md`](../../structures/vwk.md) — line/fill/text
  attribute fields.
- [`../_state.md`](../_state.md) — attribute defaults and ranges.
- GDP constants: `CORNER_POINTS`=5, `RBOX_POINTS`=21, `RFBOX_POINTS`=20
  (`vdi_gdp.c:23-25`); `HALFPI`=900, `PI`=1800, `TWOPI`=3600
  (`vdi_gdp.c:28-30`); `MIN_ARC_CT`=32, `MAX_ARC_CT`=128
  (`vdi_defs.h:84-85`).

## Cross-references

### Atari Compendium / Profibuch
- Not present in `references/` (no PDFs). Cross-check of the per-primitive
  radius `PTSIN` indices (esp. circle `PTSIN[4]` vs arc/pieslice
  `PTSIN[6]`), the angle units, the full-shape `INTIN` override, and the
  closed-shape "solid `fill_color` perimeter, not line attributes" rule is
  **deferred**.

## Configuration / version notes

- `HAVE_BEZIER` = 0 (`vdi_defs.h:18`) — GDP sub-op 13 (Bezier) is compiled
  out of every default build.
- The circle-family y-radius uses the device pixel aspect (`xsize`/
  `ysize`); on a square-pixel framebuffer these are equal, so `yrad ==
  xrad`.

## NewTOS implementation notes (Phase 12)

- All sub-primitives reduce to the line/fill framebuffer primitives
  already used by [`_lines-markers.md`](_lines-markers.md) /
  [`_fill.md`](_fill.md) / [`_text.md`](_text.md) — `v_gdp` adds no new
  low-level op, only geometry generation. Target = `vwk->device`
  (ADR-011).
- Curve flattening (`clc_nsteps`/`clc_arc`) uses the device aspect
  (`xsize`/`ysize`); on a square-pixel framebuffer `yrad == xrad` for the
  circle case. NewTOS may generate curve geometry directly (its own
  rasteriser), keeping the sub-op numbers, PTSIN layout, angle units, and
  open/closed + colour semantics as contract.
- Angle units match [`vst_rotation`](../attributes/_text.md) (tenths of a
  degree); reuse the same normalisation helper.
