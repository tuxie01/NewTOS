# VDI area-fill output primitives (Phase 00b D4)

> Source: EmuTOS @ `1a7484ab` — `vdi/vdi_fill.c:617` (`vdi_v_fillarea`),
> `:293` (`vdi_vr_recfl`), `:1084` (`vdi_v_contourfill`); polygon scan-fill
> `:560` (`polygon`), `:948` (`contourfill`). Calling convention:
> [`../_dispatcher.md`](../_dispatcher.md). Fill attribute state from the
> [`Vwk`](../../structures/vwk.md), set via the
> [fill setters](../attributes/_fill.md) and summarised in
> [`../_state.md`](../_state.md). **No output array, no error return.**
>
> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> **Contract:** opcodes (9 / 114 / 103); the `PTSIN` vertex/coordinate
> contracts (polygon vertex count in `CONTRL[1]`; rectangle in
> `PTSIN[0..3]`; seed in `PTSIN[0..1]`); the `v_contourfill` boundary/seed
> semantics via `INTIN[0]`; which fill attributes are consulted; the
> perimeter rule; the "no output" shape. **Reference:** the scan-conversion
> (`clc_flit`), the seed-fill queue (`contourfill`), `draw_rect`, and the
> pattern/`patptr`/`patmsk` machinery — NewTOS fills its own way
> (`tos-api/src/vdi/backend.rs`).

| Op | Fn | Input | Fills |
|----|----|-------|-------|
| 9   | `v_fillarea`    | `CONTRL[1]` = vertex count; `PTSIN[]` = `(x,y)` pairs | the polygon (auto-closed) with the current interior style |
| 114 | `vr_recfl`      | `PTSIN[0..3]` = two opposite corners | the rectangle, **no perimeter** |
| 103 | `v_contourfill` | `PTSIN[0..1]` = seed `(x,y)`; `INTIN[0]` = boundary colour or −1 | the connected region around the seed |

Native ABI (all): GEM trap `svc #2`, selector 115 in `x0`, `VDIPB*` in
`x1`; opcode in `CONTRL[0]`, handle in `CONTRL[6]`. Dispatcher pre-loads
`CONTRL[2]=CONTRL[4]=0` for all three (jump-table `{0,0}`).

All three read the fill attribute state: `fill_style` (0..4 `FIS_*`),
`fill_index`, `fill_color`, `fill_per` (perimeter on/off), `ud_patrn[]`/
`multifill` (user pattern), plus `wrt_mode` and the clip rect — see
[`../attributes/_fill.md`](../attributes/_fill.md).

---

## `v_fillarea` — filled polygon (opcode 9)

### EmuTOS source

**Primary location:** `vdi/vdi_fill.c:617` — `vdi_v_fillarea(Vwk *vwk)`
(reached via `jmptb1[8]`, `vdi_main.c:40`); does the work in
`polygon(vwk, PTSIN, CONTRL[1])` (`:560`).

**Helpers called (reference):** `polygon()` (`vdi_fill.c:560`) →
`clc_flit()` (`:434`, scan-conversion) and, for the perimeter,
`polyline()` (`vdi_line.c:1101`).

**Globals read:** `CONTRL[1]` (vertex count), `PTSIN[]`, `fill_style`/
`fill_index`/`fill_color`/`fill_per`/`ud_patrn`/`multifill`, `wrt_mode`,
clip rect.

**Build configuration:** `HAVE_BEZIER` = 0 (`vdi_defs.h:18`) — the
`CONTRL[5]==13` filled-Bezier sub-case (`vdi_fill.c:622-625`) is compiled
out; `v_fillarea` always scan-fills the polygon.

### Behaviour

`polygon()` scan-converts the polygon defined by the `CONTRL[1]` `PTSIN`
vertices and fills each scanline span with the current interior style/
pattern in `fill_color`. The polygon is **automatically closed** — the
routine appends a copy of the first vertex after the last
(`vdi_fill.c:594-598`) before scan-filling, so the caller does **not** need
to repeat the first point (though doing so is harmless). It computes the
polygon's y-extent, clips it to the clip rect's y-range when `clip` is set
(`:584-592`), and returns early if the polygon is entirely outside the
clip band.

If `fill_per == TRUE`, after the fill the perimeter is stroked as a
`count+1`-point `polyline` (closing edge included) in **`fill_color`** with
`LN_MASK = 0xffff` (solid) — **not** the line attributes
(`vdi_fill.c:606-608`). So the outline uses the fill colour, solid, width
governed by the polyline path.

### PTSIN vertex contract

- `CONTRL[1]` = number of polygon vertices (= `(x,y)` pairs).
- `PTSIN[2i]`, `PTSIN[2i+1]` = vertex `i`, raster coordinates.
- **The polygon is auto-closed**: the last vertex connects back to the
  first for both the fill and (when `fill_per`) the perimeter. A caller
  that repeats the first vertex as the last gets a degenerate closing edge
  of zero length, which is harmless.
- **No minimum vertex count.** `polygon()` checks none
  (`vdi_fill.c:560-610`); `count` edges are scan-filled and `count+1`
  points stroked whatever `count` is (added 2026-11-06, TASK/V22,
  measured as VDI-DRAW-004):
  - **0 vertices** — nothing: `clc_flit` has no edge, and the perimeter is
    a 1-point `polyline`, whose loop `i = count-1; i > 0` draws no segment
    (`vdi_line.c:1106`). (`GSX_ENTRY` copies no `PTSIN` for 0,
    `vdi_asm.S:93`, so the y-extent is read from a stale `local_ptsin[0]`;
    nothing is drawn either way.)
  - **1 vertex** — the only edge (vertex → its closing copy) is
    horizontal and `clc_flit` ignores it (`:453-455`), so the fill draws
    nothing; with `fill_per` the perimeter is the dot at the vertex
    (`polyline` of 2 equal points).
  - **2 vertices** — the edge there and the edge back cross every scan
    line of the half-open band `(miny, maxy]` at the **same** x (both
    take the same endpoint and the same rounding, `:475-487`), so the
    fill alone draws one pixel per row — a dotted stroke, also with
    `fill_per` off (`vdi-v18.tsv` 51/53, probe K4C/K4D). A horizontal
    pair fills nothing. With `fill_per` the 3-point perimeter
    (there and back) is stroked over it.
- **More than `MAX_VERTICES` (1024).** `GSX_ENTRY` caps `CONTRL[1]` at
  `MAX_VERTICES` and copies that many vertices into
  `local_ptsin[2*MAX_VERTICES]` (`vdi_asm.S:95-107`); `polygon()` then
  writes the closing vertex to `ptsin[count]` (`vdi_fill.c:596`), which
  for `count == MAX_VERTICES` is past `local_ptsin`, i.e. the start of
  `fill_buffer` (`include/vdiext.h:47-51`) — which `clc_flit` overwrites
  with scan-line crossings while it still reads that vertex. EmuTOS'
  result is a buffer overrun, not a defined polygon (probe K5D: no fill,
  an outline to a wrong point). **NewTOS deviates by decision**
  (TASK/V18 decision C, `docs/COMPATIBILITY.md`): it closes the first
  `MAX_VERTICES` vertices back to vertex 0 like any other polygon.

### Inputs / Outputs

| Slot | Meaning |
|------|---------|
| `CONTRL[1]` | vertex count (clamped to `MAX_VERTICES` by the dispatcher) |
| `PTSIN[2i]`, `PTSIN[2i+1]` | vertex `i` |

**No output** (`{nptsout,nintout}={0,0}`), no error return.

---

## `vr_recfl` — filled rectangle (opcode 114)

### EmuTOS source

**Primary location:** `vdi/vdi_fill.c:293` — `vdi_vr_recfl(Vwk *vwk)`
(reached via `jmptb2[14]`, `vdi_main.c:88`).

**Helpers called (reference):** `arb_corner()` (`vdi_misc.c:30`, normalise
corners), `clipbox()` (`vdi_fill.c:252`, clip to the rect), `draw_rect()`
(the span fill).

**Globals read:** `PTSIN[0..3]`, `fill_style`/`fill_index`/`fill_color`/
`ud_patrn`/`multifill`, `wrt_mode`, clip rect. **`fill_per` is NOT read** —
`vr_recfl` draws no perimeter.

**Build configuration:** none.

### Behaviour

1. `arb_corner((Rect*)PTSIN)` normalises the two corner points to
   (lower-left, upper-right) order **in place** in `PTSIN`
   (`vdi_fill.c:300`).
2. It copies the rect into a temporary so the clip step does not corrupt
   the (now-normalised) `PTSIN` values, which a caller such as the GDP
   `v_bar` still needs for the perimeter pass (`:303-306`).
3. If `clip` is set, `clipbox()` intersects the rect with the clip
   rectangle; if the result is empty, it returns doing nothing (`:308-310`).
4. `draw_rect(vwk, &rect, fill_color)` fills the rectangle with the current
   fill pattern (`:313`).

`vr_recfl` itself draws **no** perimeter regardless of `fill_per`; the GDP
`v_bar` ([`_gdp.md`](_gdp.md#bar-sub-op-1) sub-op 1) adds the outline when
`fill_per` is set.

### PTSIN coordinate contract

- `PTSIN[0]`, `PTSIN[1]` = one corner `(x1,y1)`.
- `PTSIN[2]`, `PTSIN[3]` = the opposite corner `(x2,y2)`.
- Corners may be given in any order; `arb_corner` sorts them (and rewrites
  `PTSIN` to the sorted order).

**No output**, no error return.

---

## `v_contourfill` — seed / flood fill (opcode 103)

### EmuTOS source

**Primary location:** `vdi/vdi_fill.c:1084` — `vdi_v_contourfill(Vwk *vwk)`,
which sets the abort hook and calls `contourfill()` (`:948`) (reached via
`jmptb2[3]`, `vdi_main.c:77`).

**Helpers called (reference):** `contourfill()` (`vdi_fill.c:948`) —
scanline seed-fill with a queue in `vdishare`; `pixelread()` (read the
seed pixel); `end_pts()`/`get_seed()` (span extension); `no_abort`
(`SEEDABORT` hook).

**Globals read:** `PTSIN[0..1]` (seed), `INTIN[0]` (boundary colour /
mode), `numcolors`, `MAP_COL[]`, `fill_*` attributes, clip rect.

**Build configuration:** `CONF_WITH_VDI_16BIT` (default 1) adds a
true-colour branch that maps the search colour through the extended
palette (`vdi_fill.c:979-985`) — reference; the index/boundary contract is
unchanged for a palette screen.

### Behaviour

`contourfill()` flood-fills the connected region starting at the seed pixel
`PTSIN[0..1]`, filling with the current interior style in `fill_color`,
clipped to the `Vwk` clip rect. The `INTIN[0]` value chooses the stopping
rule:

- **`INTIN[0] < 0`** → *fill-while-same*: the search colour is read from the
  seed pixel itself (`search_color = pixelread(xleft,oldy)`, `seed_type=1`,
  `vdi_fill.c:971-973`); the fill spreads to all connected pixels equal to
  the seed colour and stops where the colour differs.
- **`INTIN[0] >= 0`** → *fill-to-boundary*: `INTIN[0]` is the boundary
  colour index. It is **range-checked against `numcolors`** — if
  `>= numcolors`, `contourfill` returns immediately (no fill,
  `vdi_fill.c:976-977`) — then mapped to a PEL via `MAP_COL[]`
  (`seed_type=0`); the fill spreads to pixels **not** equal to that
  boundary colour and stops at boundary-coloured pixels.

If the seed point lies **outside the clip rectangle**, `contourfill`
returns immediately doing nothing (`vdi_fill.c:965-967`).

### Seed queue (`vdi_fill.c:873-1062`)

The fill order and its limit are observable, so they are contract:

- **Spans are runs of one screen colour.** `end_pts` (`vdi_fill.c:836-866`,
  truecolor `end_pts16` `:746-767`) takes the colour of the pixel it is
  asked about and extends left and right over pixels of *that* colour,
  bounded by the clip's x range; the run is fillable if its colour equals
  `search_color` (`seed_type=1`) or differs from it (`seed_type=0`). A row
  outside the clip's y range has no span (`:843-844`). Truecolor compares
  the screen words with `OVERLAY_BIT` masked off (`:755`) against the
  unmasked `search_color`.
- **The screen is read live.** Each span is drawn (`draw_rect_common` on
  one row, `:1050-1057`) the moment it is taken from the queue, and later
  searches read the drawn pixels: a patterned or XOR fill changes what the
  fill sees next.
- **Queue entries** (`SEGMENT`, `include/vdiext.h:17-21`) hold a row, a
  span and a direction (`DOWN_FLAG` `0x8000` in `y`, `vdi_fill.c:30`);
  free entries are `EMPTY` (`0xffff`, `:29`). The seed span is queued going
  down (`:997-1001`), but the first pass searches **up** from it, before it
  is drawn (`oldy` still has no flag, `:1006`).
- **One pass** (`:1003-1062`): search the next row at the left end of the
  current span (`get_seed`); if that span sticks out left by two or more
  pixels and was queued, seed the spans of the *current* row from
  `oldxleft-1` down to the new span's left end, in the opposite direction
  (`:1011-1018`); seed the further spans of the next row up to `oldxright`
  (`:1019-1024`); if the last one sticks out right by two or more and was
  queued, seed the current row beyond `oldxright` in the opposite direction
  (`:1025-1032`). Then take the next non-`EMPTY` entry scanning circularly
  from `qptr` (`:1038-1048`), free it, draw it (`:1050-1057`). The fill
  ends when the queue is empty (`:1034-1036`).
- **`get_seed`** (`:888-942`) queues a fillable span — unless a queued
  entry has the same row, the opposite direction and the same left end:
  then that entry is freed and the span drawn at once (`:905-922`). A new
  seed takes the first hole, else raises `qtop`; `crunch_queue`
  (`:875-881`) lowers `qtop` over trailing holes.
- **Capacity `QSIZE`** = `sizeof(struct vsmain)/sizeof(SEGMENT)`
  (`include/vdiext.h:33`) = `3*MAX_VERTICES` WORDs / 3 WORDs =
  `MAX_VERTICES` = **1024** entries in the reference build
  (`include/config.h:1654`).
  A seed that finds neither a hole nor room **ends the whole fill**
  (`vdi_fill.c:929-932`, `:1009-1030`): what is drawn stays drawn, nothing
  else is filled, no error.
- `SEEDABORT` is `no_abort` for the VDI (`vdi_fill.c:1076-1090`), so the
  VDI fill is never aborted early.
- The queue overlays the local `PTSIN` copy in `vdishare`
  (`include/vdiext.h:47-53`, `vdi/vdi_asm.S:79-80`); the caller's `PTSIN`
  array is not touched.

### PTSIN / INTIN contract

| Slot | Meaning |
|------|---------|
| `PTSIN[0]`, `PTSIN[1]` | seed `(x,y)`, raster coordinates |
| `INTIN[0]` | boundary colour index (`>= 0`) or −1 (any negative) for fill-while-seed-colour |

**No output**, no error return. (Both the out-of-clip seed and the
`>= numcolors` boundary index are silent no-ops.)

---

## Verification criteria

- **`v_fillarea` auto-close:** `CONTRL[1]=3` with three distinct vertices
  fills the triangle **without** the caller repeating the first vertex; a
  4th vertex equal to the first changes nothing.
- **`v_fillarea` 1–2 vertices:** `CONTRL[1]=1` with `fill_per` draws the
  vertex, without it nothing; `CONTRL[1]=2` draws one fill pixel per row of
  `(miny, maxy]` even with `fill_per` off, plus the perimeter stroke when
  on (`tosdump/cases/vdi-v18.tsv` 51/53; `tosdump/probes/vdprobe.c` K4A–K4H).
- **`v_fillarea` interior:** with `fill_style=FIS_SOLID`, the polygon is
  solid `fill_color`; with `FIS_HOLLOW` and `fill_per=TRUE`, only the
  solid `fill_color` outline is drawn (interior empty).
- **`v_fillarea` perimeter colour:** with `fill_per=TRUE` the outline is
  **solid `fill_color`**, not the line attributes — a program that set a
  dashed line style still gets a solid fill-colour perimeter.
- **`vr_recfl` corners any order:** `PTSIN=[x2,y2,x1,y1]` with `x2>x1`,
  `y2>y1` fills the same rectangle as `[x1,y1,x2,y2]`.
- **`vr_recfl` no perimeter:** with `fill_per=TRUE`, `vr_recfl` alone still
  draws no outline (only `v_bar` adds it).
- **`v_contourfill` fill-while-same:** `INTIN[0]=-1`, seed on a region of
  colour C → fills the maximal connected run of colour C around the seed.
- **`v_contourfill` fill-to-boundary:** `INTIN[0]=B` (`0 <= B < numcolors`)
  → fills outward from the seed until it hits pixels of colour B.
- **`v_contourfill` guards:** seed outside the clip rect → nothing drawn;
  `INTIN[0] >= numcolors` → nothing drawn.
- **`v_contourfill` seed queue:** a dot lattice (40 rows of style
  `0x8888`, `tosdump/cases/vdi-v18.tsv` 109/154/199) is filled into every
  pocket; the fill order and the 1024-entry overflow follow the seed queue
  above.

## Edge cases discovered in source

1. **`v_fillarea` closes the polygon itself.** `polygon()` appends
   `PTSIN[0]` after the last vertex before scan-filling and stroking the
   perimeter (`vdi_fill.c:594-598,608`). Auto-close is contract — the
   caller need not repeat the first point (unlike `v_pline`, which is
   open).
2. **The fill perimeter is solid `fill_color`, not the line attributes.**
   `polygon()` sets `LN_MASK = 0xffff` and calls `polyline(..., fill_color)`
   (`vdi_fill.c:607-608`). A dashed or coloured *line* style has no effect
   on a filled-area outline. Same for the closed GDP shapes — see
   [`_gdp.md`](_gdp.md).
3. **`vr_recfl` rewrites `PTSIN` in place** via `arb_corner`
   (`vdi_fill.c:300`) — after the call, `PTSIN[0..3]` hold the *sorted*
   (ll, ur) corners, not the caller's original order. The GDP `v_bar`
   relies on this sorted form for its perimeter.
4. **`v_contourfill` reads the seed colour when `INTIN[0] < 0`**, so the
   fill mode depends on the pixel under the seed at call time — the same
   call on a differently-coloured seed fills a different region.
5. **`v_contourfill` two silent no-op guards:** seed outside the clip
   rect, and a boundary index `>= numcolors` (`vdi_fill.c:965-977`). Both
   return with nothing drawn and no error — VDI has no error channel.
6. **`v_fillarea`/`v_contourfill` honour `multifill`** (the multi-plane
   user pattern from `vsf_udpat`) only when `fill_style == FIS_USER`; the
   dispatcher clears `multifill` for any other style before the call
   (`vdi_main.c:146-147`).

## Dependencies

### Requires
- An open workstation (dispatcher resolves `CONTRL[6]`).
- The fill attribute state (`fill_style`/`fill_index`/`fill_color`/
  `fill_per`/`ud_patrn`/`multifill`) and the active `patptr`/`patmsk`
  (set by `st_fl_ptr` after `vsf_interior`/`vsf_style`) — see
  [`../attributes/_fill.md`](../attributes/_fill.md).
- `wrt_mode`, the clip rectangle, and (for `v_contourfill`) a
  pixel-readback path plus `numcolors`/`MAP_COL[]`.

### Used by
- `v_fillarea`'s `polygon()` is the fill engine for the closed GDP shapes
  (circle/ellipse/pieslice/ellpie/rfbox) and the `v_bar` fill via
  `vr_recfl` — see [`_gdp.md`](_gdp.md).

## Related data structures

- [`../../structures/vwk.md`](../../structures/vwk.md) — the `fill_*` /
  `ud_patrn` / `multifill` / clip fields.
- [`../_state.md`](../_state.md) — fill attribute defaults and ranges.
- Interior-style constants `FIS_HOLLOW`=0 … `FIS_USER`=4
  (`vdi_defs.h:60-65`); see [`../attributes/_fill.md`](../attributes/_fill.md).

## Cross-references

### Atari Compendium / Profibuch
- Not present in `references/` (no PDFs). Cross-check of the auto-close
  rule, the solid-`fill_color` perimeter, the `v_contourfill` boundary/
  seed semantics, and the `>= numcolors` guard is **deferred**.

## Configuration / version notes

- `HAVE_BEZIER` = 0 — the filled-Bezier sub-case of `v_fillarea` is
  compiled out in the default build.
- `CONF_WITH_VDI_16BIT` (default 1) adds a true-colour path to
  `v_contourfill`'s colour mapping; the palette-index/boundary contract is
  unaffected on a palette screen.

## NewTOS implementation notes (Phase 12)

- `polygon`/`draw_rect`/`contourfill` map to the framebuffer span-fill
  primitive, ANDing the active pattern (`patptr`/`patmsk`,
  [`vsf_*`](../attributes/_fill.md)) per scanline and honouring `wrt_mode`.
  Target = `vwk->device` (ADR-011).
- Keep the **auto-close** and **solid-`fill_color` perimeter** semantics —
  they are observable contract, not raster details.
- Flood-fill reads pixels back through the same `pixelread` as
  `v_get_pixel` and keeps EmuTOS' seed queue, order and `QSIZE` overflow
  as specified above (TASK/V21; a 4096-entry seed stack that dropped
  seeds left pockets unfilled, VDI-DRAW-008). `REV_MAP_COL`/`MAP_COL`
  translate between VDI colour index and PEL as elsewhere.
