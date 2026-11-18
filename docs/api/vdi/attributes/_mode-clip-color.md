# VDI mode / clip / colour setters (Phase 00b D3)

> Source: EmuTOS @ `1a7484ab` — `vdi/vdi_control.c` (`vswr_mode` `:256`,
> `vs_clip` `:235`, `validate_color_index` `:224`), `vdi/vdi_col.c`
> (`vs_color` `:656`); constants in `vdi/vdi_defs.h:76-82`. Calling
> convention: [`../_dispatcher.md`](../_dispatcher.md). Setter shape as in
> [`_line-marker.md`](_line-marker.md). No error return.
>
> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> **Contract:** opcode numbers, the input slots, the writing-mode range and
> values, the clip-rectangle normalisation + screen intersection, the colour
> index/RGB validation, and what `REQ_COL`/the hardware palette store.
> **Reference:** the `Vwk`/Line-A storage and `set_color()` DAC programming.

## Trap interface (all setters below)

Native ABI: GEM trap `svc #2`, selector 115 in `x0`, `VDIPB*` in `x1`.
Per function: opcode in `CONTRL[0]`, handle in `CONTRL[6]`.

| Op | Fn | Reads | Effect → `Vwk` / device | Echoes |
|----|----|-------|-------------------------|--------|
| 32 | `vswr_mode` | `INTIN[0]` mode (1-based) | `MIN_WRT_MODE..MAX_WRT_MODE` (1..4) else `DEF_WRT_MODE` (1); `wrt_mode = mode-1` (**0-based**); sets `CONTRL[4]=1` | `INTOUT[0]` = accepted mode (1-based) |
| 129 | `vs_clip`  | `INTIN[0]` flag; `PTSIN[0..3]` rect | `clip = INTIN[0]`; set the clip rectangle (see below) | none |
| 14 | `vs_color`  | `INTIN[0]` colour idx; `INTIN[1..3]` RGB | store RGB request + program the hardware palette (see below) | none |

### Value constants (`vdi_defs.h`)

| `#define` | Value | Meaning |
|-----------|-------|---------|
| `MIN_WRT_MODE` / `MAX_WRT_MODE` / `DEF_WRT_MODE` | 1 / 4 / 1 (`MD_REPLACE`) | writing-mode range + default |
| `MD_REPLACE` / `MD_TRANS` / `MD_XOR` / `MD_ERASE` | 1 / 2 / 3 / 4 | writing-mode selector values (1-based, as passed in `INTIN[0]`) |
| `WM_REPLACE` / `WM_TRANS` / `WM_XOR` / `WM_ERASE` | 0 / 1 / 2 / 3 | the stored `wrt_mode` values (= `MD_* − 1`) |

### `vswr_mode` — writing modes (`INTIN[0]`, 1-based)

| `INTIN[0]` | Stored `wrt_mode` | Name (`MD_*`) |
|:----------:|:-----------------:|---------------|
| 1 | 0 | replace (`MD_REPLACE`) — **default** |
| 2 | 1 | transparent (`MD_TRANS`) |
| 3 | 2 | XOR (`MD_XOR`) |
| 4 | 3 | erase / reverse-transparent (`MD_ERASE`) |

`vswr_mode` clamps `INTIN[0]` to `1..4`, substituting `DEF_WRT_MODE` (1) for
anything outside, echoes the accepted **1-based** value in `INTOUT[0]`, and
stores it **0-based** as `wrt_mode = mode-1` (`vdi_control.c:261-264`). It
also contains an explicit `CONTRL[4] = 1` as its first statement
(`vdi_control.c:260`). The drawing primitives switch on the 0-based
`wrt_mode`.

**Precision note (was: "overriding the jump-table pre-load"):** this line is
**functionally redundant**, not an observable override. `jmptb1`'s entry for
op 32 already declares `nintout = 1` (`vdi_main.c:64`), which `screen()`
writes to `CONTRL[4]` *before* calling `vdi_vswr_mode` (`vdi_main.c:158-160`);
`vdi_vswr_mode` is reached **only** through this jump table (no other call
site in `vdi/`). The explicit line therefore reassigns the identical value
`1` — a caller cannot observe any difference with or without it. Corrected
per [`../_discrepancies.md`](../_discrepancies.md) local log D-VDI-16 (an
earlier revision of this doc overstated it as a behavioural override).

> Mode 4's EmuTOS constant is `MD_ERASE`; Atari docs call it
> "reverse-transparent" — same mode (draw the *background* where the pattern
> bit is clear). Named here as "erase (reverse-transparent)".

### `vs_clip` — clipping rectangle (`vdi_control.c:235-251`)

`clip = INTIN[0]` (0 = off, ≠0 = on).

- **On** (`INTIN[0] != 0`): read the rectangle from `PTSIN[0..3]` (as a
  `Rect` overlay), **normalise the corners** with `arb_corner` (swap so
  `x1≤x2`, `y1≤y2`; `vdi_misc.c:30`), then **intersect with the screen**:
  `xmn_clip = max(0, x1)`, `ymn_clip = max(0, y1)`,
  `xmx_clip = min(xres, x2)`, `ymx_clip = min(yres, y2)`.
- **Off** (`INTIN[0] == 0`): clip to the whole device —
  `xmn_clip=0`, `ymn_clip=0`, `xmx_clip=xres`, `ymx_clip=yres`.

Stored in `xmn_clip`/`ymn_clip`/`xmx_clip`/`ymx_clip`; every primitive clips
against this rectangle. No `INTOUT`/`PTSOUT` (counts 0/0). Note the intersect
uses `xres`/`yres` (`= DEV_TAB[0]/[1]`), the full resolution.

### `vs_color` — set palette entry (`vdi_col.c:656-704`)

```
colnum = INTIN[0]
if colnum < 0 or colnum >= numcolors:   return    # out of range: no-op
for i in 0..2:                                     # R,G,B
    REQ_COL[colnum][i] = INTIN[1+i]               # store the RAW request
    rgb[i] = clamp(INTIN[1+i], 0, 1000)           # clamp only for the DAC
set_color(colnum, rgb)                            # program the hardware palette
```

- The colour index must satisfy `0 <= colnum < numcolors`
  (`= DEV_TAB[13]`); otherwise the call is a **silent no-op**.
- **The value stored in `REQ_COL` is the raw, *unclamped* `INTIN`**
  (`vdi_col.c:690`); only the copy passed to `set_color()` is clamped to
  `[0,1000]` (`vdi_col.c:695-699`). So `vq_color` (op 26) reads back the raw
  request, which may lie outside 0..1000.
- RGB components use VDI's 0–1000 range (per mille); the driver scales to the
  hardware DAC width. Entries 0..15 go to `REQ_COL`; entries 16..255 (only on
  `EXTENDED_PALETTE` builds) go to `req_col2`. No `INTOUT`/`PTSOUT`.

`validate_color_index` (`vdi_control.c:224-230`), shared by the `vs*_color`
setters, is stricter in the *other* direction: it returns **1** (pen 1, not
0) for `colnum < 0 || colnum >= numcolors`, else `colnum`. `vs_color` does
**not** use it — it rejects out-of-range indices instead of substituting 1.

---

## Verification criteria

- **`vswr_mode` values:** `vswr_mode(3)` → `INTOUT[0]=3`, `wrt_mode=2` (XOR),
  `CONTRL[4]=1`; `vswr_mode(0)` and `vswr_mode(5)` → `INTOUT[0]=1`
  (`DEF_WRT_MODE`), `wrt_mode=0`.
- **`vs_clip` on + normalise:** `vs_clip(flag=1, PTSIN=[100,50,10,20])` →
  after `arb_corner` `(10,20)-(100,50)`, intersect screen → `xmn=10`,
  `ymn=20`, `xmx=min(xres,100)`, `ymx=min(yres,50)`.
- **`vs_clip` on + off-screen:** `PTSIN=[-5,-5, x, y]` with `x>xres` →
  `xmn=0`, `ymn=0`, `xmx=xres`, `ymx=yres` (clamped to the device).
- **`vs_clip` off:** `vs_clip(flag=0, ...)` → clip = full device
  `[0,0]..[xres,yres]`, `PTSIN` ignored.
- **`vs_color` in range:** `vs_color(2, 1000,0,0)` sets palette entry 2 red;
  `REQ_COL[2] = {1000,0,0}`, DAC programmed.
- **`vs_color` raw store:** `vs_color(2, 1200,-30,500)` → `REQ_COL[2] =
  {1200,-30,500}` (raw), but the DAC receives `{1000,0,500}` (clamped).
- **`vs_color` out of range:** `vs_color(numcolors, ...)` or a negative
  index → no change to `REQ_COL` or the palette.

---

## Edge cases discovered in source

1. **`vs_color` stores the *unclamped* request in `REQ_COL`** — only the DAC
   copy is clamped to `[0,1000]` (`vdi_col.c:690,695-699`). A round-trip
   `vs_color` → `vq_color` can therefore return values outside 0..1000.
2. **`vswr_mode` contains an explicit `CONTRL[4] = 1`** (`vdi_control.c:260`)
   that duplicates, but does not change, the jump table's own `nintout = 1`
   preload for op 32 — dead/redundant code, not an observable behaviour
   difference (see the precision note above, D-VDI-16).
3. **`vs_clip` normalises corners** (`arb_corner`) before intersecting — a
   caller may pass the rectangle in any corner order.
4. **`vs_color` and `validate_color_index` disagree on out-of-range** —
   `vs_color` (op 14) *rejects* a bad index (no-op), while the `vs*_color`
   attribute setters *substitute pen 1* via `validate_color_index`
   (`vdi_control.c:226`). Same "index" concept, two different fallbacks.
5. **The clip intersect uses `xres`/`yres`, i.e. `DEV_TAB[0]/[1]`** (the
   full device resolution), not the current window — clipping is against the
   whole screen (`vdi_control.c:243-244`).

---

## Dependencies

### Requires
- An open workstation; `xres`/`yres` (`DEV_TAB[0]/[1]`) for the clip
  intersect; `numcolors` (`DEV_TAB[13]`) for the `vs_color` range check;
  `REQ_COL`/`req_col2` palette-request arrays; `set_color()` (BIOS/hardware
  palette programming).

### Used by
- Every drawing primitive consults `wrt_mode` and the clip rectangle;
  `vs_color` feeds the palette the pens resolve through.
- `vq_color` (26) reads `REQ_COL` back; `vq_extnd` mode 1 reports the clip
  rect + flag; the `vq*_attributes` inquiries return `wrt_mode+1`.

---

## Related data structures

- [`../../structures/vwk.md`](../../structures/vwk.md) — `wrt_mode`, `clip`,
  `xmn_clip`/`ymn_clip`/`xmx_clip`/`ymx_clip`.
- [`../_state.md`](../_state.md) — attribute-state defaults table + the
  `REQ_COL` palette model.
- [`../inquiry/_color-pixel-input.md`](../inquiry/_color-pixel-input.md) —
  `vq_color` reads the `REQ_COL` request back.

## Cross-references

### Atari Compendium / Profibuch
- Not present in `references/` (no PDFs). Cross-check of the writing-mode 4
  name ("reverse-transparent" vs `MD_ERASE`), the `vs_color` raw-store, and
  the clip screen-intersection is **deferred**.

## Configuration / version notes

- `MAXCOLOURS` (the `REQ_COL`/`MAP_COL` size) is 16 unless `EXTENDED_PALETTE`
  (`CONF_WITH_VIDEL || CONF_WITH_TT_SHIFTER`), both **0** by default
  (`vdi_col.c:18-22`). On the default build only pens 0..15 exist and
  `req_col2` (16..255) is unused.
- On `CONF_WITH_VDI_16BIT` truecolor builds, `vs_color` early-dispatches to
  `vs_color16` (`vdi_col.c:661-666`) — **out of bring-up scope**.
- On `CONF_WITH_TT_SHIFTER` builds with the TT palette-bank feature active
  at runtime (`has_tt_shifter`), `vs_color` remaps `colnum` via
  `adjust_tt_colnum()` before storing/programming it (`vdi_col.c:678-681`).
  Doubly gated (build config **and** runtime hardware detection), both
  **0**/false in the default build — out of bring-up scope, noted here only
  for completeness of the `vs_color` CONF-gating picture.
- `vswr_mode`/`vs_clip` are configuration-independent.

## NewTOS implementation notes (Phase 12)

- `wrt_mode` maps to the framebuffer blit op: replace = store, transparent =
  store where pattern set, XOR = `^=`, erase/reverse-transparent = store
  background where pattern clear.
- `set_color` writes the per-device palette (ADR-011); for a palette-based
  bring-up framebuffer this is a direct DAC write. Keep `REQ_COL` holding the
  **raw** request so `vq_color` round-trips exactly like EmuTOS.
- Clip bounds come from the device's `xres`/`yres` (`vwk->device`), not a
  global — multi-screen safe (ADR-011).

## 32-plane setting: `vs_color` (ADR-038 point 4, V64)

Spec gate of TASK/V64 (researcher, 2026-11-18). Everything above is the 16-plane contract
and stays. At 32 planes ADR-038 is the contract; the rules P1-P9, the default palette and
the full change set are in [`../raster/_raster.md`](../raster/_raster.md) § "32-plane setting:
pens, and every place a pen becomes a pixel (ADR-038 point 4, V64)". For `vs_color`:

- The index check (`0 <= colnum < numcolors`, silent no-op outside) and the raw, unclamped
  store of the request in the calling workstation's `req_col` are the 16-plane rule
  (`references/emutos/vdi/vdi_col.c:634-646`).
- The clamped copy (0..1000) becomes the pen's pixel `0x00RRGGBB` with `q = (c * 255 + 500) / 1000`
  per gun (ADR-038 point 4, the EmuTOS rule `vdi_col.c:588-591` with 255 for 31).
- The 16-plane path (`vs_color16`, `references/emutos/vdi/vdi_col.c:620-649`; NewTOS
  `tos-api/src/vdi/attributes.rs:492@vs_color`) is not changed.

### Open points

- Inferred: the formula is ADR-038's own ([I] there); see the Open points of the section in `_raster.md`.

