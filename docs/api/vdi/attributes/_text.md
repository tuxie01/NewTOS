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
| 119 | `vst_load_fonts`| `CONTRL[7-8]` scratch-buffer ptr, `CONTRL[9]` buffer-2 offset, `CONTRL[10-11]` first-font ptr (m68k/ILP32; LP64: `[7..10]`, `[11]`, `[12..15]`, § CONTRL layout) | link the passed chain as `font_ring[2]`, byte-swap non-`F_STDFORM` data, swap in the scratch buffer (`CONF_WITH_GDOS`) | `INTOUT[0]` = number of **distinct consecutive `font_id`s in the passed chain** (not 0 for a real chain); 0 only if this workstation already has a chain loaded, or `CONF_WITH_GDOS` = 0. `CONTRL[2]`=0, `CONTRL[4]`=1 |
| 120 | `vst_unload_fonts`| none (`INTIN[0]` select is **not read**) | drop the chain, restore the default scratch buffer and face count (`CONF_WITH_GDOS`) | none (`CONTRL[2]`=`CONTRL[4]`=0) |

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

### Font chain: `vst_load_fonts` / `vst_unload_fonts` (GDOS interface)

> Added 2026-11-06 for `TASK/V33.md` (VDI-TEXT-010). Source: EmuTOS @
> `1a7484ab`, `CONF_WITH_GDOS` = 1 (`include/config.h:1620-1622`). Notes:
> *cited* = read in the file named; *inferred* = reasoned from cited code, not
> measured. Measured = probe T13A/T13B (`tosdump/probes/vdprobe.c:1213@t13`,
> oracle log `measurements/measurement-2026-11-06-v18-t2/probe/e1/VDPROBE.LOG:273-284`)
> and TX* (`tosdump/probes/vdprobe.c`, function `tx`, run
> `measurements/measurement-2026-11-06-v33/RUNINFO.TXT`).
> Analysis note with the LP64 options:
> [`docs/analysis/v33-gdos-fonts-2026-11-06.md`](../../../analysis/v33-gdos-fonts-2026-11-06.md).

**Who calls the ROM like this.** The ROM VDI reads the buffer and the chain
straight from fixed `CONTRL` cells (`vdi_text.c:1308-1317`, comment: *"The
inputs to this routine are: CONTRL[7-8] = pointer to scratch buffer,
CONTRL[9] = offset to buffer 2, CONTRL[10-11] = pointer to first font"*). An
application does not supply them: GDOS (not part of EmuTOS) intercepts trap 2,
reads the font files, builds the chain and then passes the call on to the
ROM. EmuTOS itself answers the GDOS probe as "no GDOS" (`vdi/vdi_asm.S:40-49`).
The application-side binding (gemlib `vst_load_fonts.c`) leaves the chain slot
zero (`*__vdi_array_ptr(10, vdi_control) = 0`, comment "NVDI expects a FONTHDR
ptr here") — see R14.

**Data model (cited).** `font_ring` is an array of four `Fonthead *`, global
(not per workstation): `[0]` = `fon6x6`, `[1]` = `fon8x8` (+ the 8x16 chained
behind it), `[2]` = fonts loaded by GDOS, `[3]` = NULL terminator
(`include/lineavars.h:103-113`, `bios/lineavars.S:138`; filled `vdi_text.c:604-607`).
Each ring slot heads a `next_font`-linked list. Per workstation `Vwk` keeps
`loaded_fonts`, `scrtchp`, `scrpt2`, `num_fonts` (`vdi/vdi_defs.h:180-201`).
`text_init2` (called from `init_wk`, i.e. from **both** `v_opnwk` and
`v_opnvwk`: `vdi_control.c:333,457,560`) sets `loaded_fonts = NULL`,
`font_ring[2] = NULL`, default scratch buffer, `num_fonts = font_count`
(`vdi_text.c:573-590`).

#### Rules

| # | Rule | Source |
|---|------|--------|
| R1 | **One chance per workstation.** If `vwk->loaded_fonts != NULL`, `vst_load_fonts` sets `INTOUT[0] = 0` and returns; nothing else is read or changed (second call: measured `io 0`, T13A `vst_load_fonts#2`). | `vdi_text.c:1303-1306` |
| R2 | **State taken from CONTRL (not validated):** `scrpt2 = CONTRL[9]` (a signed WORD), `scrtchp = *(LONG*)&CONTRL[7]`, `first_font = *(LONG*)&CONTRL[10]`; `loaded_fonts = first_font`. | `:1314-1318` |
| R3 | **Result = number of faces.** Walk the chain through `next_font` until NULL (no cycle or length check); `count` is incremented whenever `font_id` differs from the **previous** header's id (start id `-1`), so `7,7,8` = 2 and `7,8,7` = 3 (consecutive, not distinct overall). `INTOUT[0] = count`; `vwk->num_fonts += count`. | `:1321-1346` |
| R4 | **Join the ring:** `font_ring[2] = vwk->loaded_fonts` — the *whole passed list* becomes ring slot 2, searched after the system fonts. | `:1342` |
| R5 | **Byte swap (`trnsfont`) only for headers without `F_STDFORM` (flags bit 2 = 4):** `FBASE = dat_table`, `FWIDTH = form_width` (bytes), `DELY = form_height`; then `(FWIDTH*DELY)/2` 16-bit words of the **data table** are byte-swapped in place (`ror #8`), then `flags |= F_STDFORM` is written **into the caller's header**. Only the data table is swapped — header fields, `off_table` and `hor_table` are not touched. `FWIDTH*DELY` is a 16-bit product (`-mshort`) divided as unsigned: measured 32640 bytes swap 16320 words, 32768 → 16384, 40000 → 20000, 65536 → **0**, 70000 → **2232** (probe TXF1-TXF5). No guard for two headers sharing one `dat_table`: the second swaps the data back (measured, TXE). A font with `F_STDFORM` set is not touched. **The rule is the same on AArch64**: the renderer reads a data table as a byte stream, leftmost byte first, bit 7 = leftmost pixel — which is the Motorola byte order `F_STDFORM` names, independent of the CPU — and the swap moves the same bytes in memory as `ror #8` does on the 68000 (`tos-api/src/vdi/font.rs:24@Byte`; measured: same table bytes after the load on both sides, TXD; same glyphs, TXD, T13A). | `:1331-1338`, `:553-570`, `include/asm.h:91-96`, `include/fonthdr.h:21` |
| R6 | **`vst_font(face)`** walks `font_ring[0]`, `[1]`, `[2]` in this order, each list via `next_font`, and takes the **first** header whose `font_id == face`; so a loaded font with a built-in's id (e.g. 1) is never chosen by `vst_font`. Not found → `&fon6x6`. It remembers `point` and `top` of the **previous** current font, makes the new one current, then re-applies the old size with `vst_point` (if `pts_mode`) or `vst_height` (height = old `top`) — the loaded face may therefore come out DDA-scaled (a `scratch_head` copy; the caller's header is not modified). `INTOUT[0]` = the selected header's `font_id`. T13A: `vst_font(7)` → 7 with a chain loaded, → 1 after unload. | `:939-985`, `make_header` `:770-817` |
| R7 | **`vst_height` / `vst_point`** find the first header of the current face over the same ring walk and then collect sizes across **all** ring slots (`top <= request` for height, `point <= request` and the half-size test for points), so loaded sizes of a face — including a loaded font with a built-in's id — join that face's size list. | `:681-740`, `:820-878` |
| R8 | **`vqt_name(n)`** numbers the **faces** in ring order `[0]`,`[1]`,`[2]`: the counter advances when `font_id` differs from the previous header's id (running across list boundaries; initial id `-1`). `n` out of range (0, negative, > face count) → `&fon6x6` (id 1, name `"6x6 system font"`). Output `INTOUT[0]` = id, then the name as words, NUL and zero padding (`CONTRL[4]` = 33 from the jump table). T13A: `vqt_name(2)` → id 7, `"AUDIT FONT SEVEN"`; after unload → id 1. A loaded chain starting with id 1 is *not* a new face (same id as the previous header). | `:1100-1134`, `vdi_main.c:158-159` (jump table `:93-95`) |
| R9 | **`vqt_fontinfo` / `vqt_attributes` / `vqt_width` / `vqt_extent`** read only `vwk->cur_font` — they see a loaded font only after `vst_font` made it current (T13A: `vqt_fontinfo` `io 32 95` = `first_ade`/`last_ade` of the loaded font). | `:1137-1174`, `:999-1021`, `:1062-1096` |
| R10 | **`vq_extnd(0)` does not change:** `INTOUT[10]` is `DEV_TAB[10]` (global), set only by `text_init`/`text_init2`; `vwk->num_fonts` is written by load/unload but read nowhere (all occurrences: `vdi_text.c:579,589,1345,1361`, `vdi_defs.h:201`). T13A: `vq_extnd(0) io[10]=1` after the load (face count of the built-ins; the count a GDOS reports is added by GDOS itself, not the ROM). | `vdi_control.c:626-658`, `vdi_text.c:573-590,662` |
| R11 | **`vst_unload_fonts`**: reads no input (the select word is ignored), does not check whether anything is loaded, frees nothing; sets `loaded_fonts = NULL`, `font_ring[2] = NULL`, `scrpt2 = SCRATCHBUF_OFFSET`, `scrtchp = vdishare.deftxbuf`, `num_fonts = font_count`. It does **not** reset `cur_font`: a workstation whose current font is a loaded header keeps pointing at it until the next `vst_font` (measured: TXG draws the glyphs after the unload). `CONTRL[4]` = 0. After unload `vst_load_fonts` works again (R1 guard is `loaded_fonts`). | `:1353-1363`, `vdi_main.c:95` |
| R12 | **Scratch buffer** (`scrtchp`, `scrpt2`): used by the text blitter as two halves of one buffer — `dst = (UBYTE*)SCRTCHP + buffa` with `buffa = SCRPT2 - buffa` (`scrpt2` is a **byte** offset to the second half). The ROM does not check size or alignment. The comment at `vdi_tblit.S:124-125` calls `scrpt2` a "word offset"; the **code** adds it to a `UBYTE *` (`vdi_textblit.c:396`), i.e. it is a byte offset (code wins). *Inferred* requirement on the caller: a buffer of at least `2 * scrpt2` bytes, even-aligned pointer and even offset (`vdi_tblit.S:121-127`, built-in sizing check `vdi_text.c:120`). Built-in defaults: `SCRATCHBUF_SIZE` = 2*212 bytes (`include/vdiext.h:40`), `scrpt2 = SCRATCHBUF_OFFSET` (`vdi_text.c:119`). | `vdi_text.c:324-325`, `vdi_textblit.c:395-396,451-452,482,551,681` |
| R13 | **Global/per-workstation mismatch (quirks the code has, *cited*):** the guard (R1) and the counters are per workstation, but `font_ring[2]` is global. (a) A second workstation can load its own chain and replaces `font_ring[2]` for everyone. (b) `v_opnvwk` / `v_opnwk` clear `font_ring[2]` for all workstations while the others still hold `loaded_fonts != NULL` (their next `vst_load_fonts` answers 0, R1). (c) `vst_unload_fonts` on any workstation empties the ring for all. `v_clsvwk` does not touch the ring (`vdi_control.c:462-490`). | see above |
| R14 | **NULL chain: no defined result in EmuTOS.** `first_font` is dereferenced unchecked (`first_font->font_id` and, when `F_STDFORM` is clear in whatever lives at address 0+66, the write `first_font->flags |= F_STDFORM` and a data swap). Not measured (the probe never calls without a chain). **NewTOS (decision D2, 2026-11-06): `INTOUT[0]` = 0, nothing changes**, so the guard stays open and today's gemlib callers are untouched (`docs/COMPATIBILITY.md` "GDOS font chain"). The reachability: gemlib's own binding always passes a zero chain slot (paragraph "Who calls the ROM like this") and `x_fnts_create(no_fonts == 0)` calls `vst_load_fonts` without a GDOS test (`sdk/gemlib/gemlib-0.44.0-m68k/x_fnts_create.c:8`). | `:1316-1340` |

**CONTRL layout** (decision D1, 2026-11-06). Each pointer takes the caller's
pointer width (T04), the WORD lies directly behind the first pointer (A81):

| | m68k / ILP32 (what EmuTOS reads — `ULONG_AT` = 32-bit read at the WORD address, `include/portab.h:216`) | LP64 |
|---|---|---|
| scratch pointer | `[7..8]` | `[7..10]` |
| offset | `[9]` | `[11]` |
| chain | `[10..11]` | `[12..15]` |
| length of `CONTRL` | 12 WORDs | 16 WORDs (`VDI_CNTRLMAX` 16 in `sdk/gemlib/overlay/mt_gem.h.in`) |

**`Fonthead` layout** (decision D3). The struct of `include/fonthdr.h:30-60` in the
caller's ABI, like `MFDB` and `OBJECT`. Common: `font_id` 0, `point` 2, `name` 4,
`first_ade` 36 … `flags` 66. Behind that:

| field | m68k / ILP32 | LP64 (`sdk/libc/overlay/include/mint/linea.h`, `__FONT`) |
|---|---|---|
| `hor_table` | 68 | 72 |
| `off_table` | 72 | 80 |
| `dat_table` | 76 | 88 |
| `form_width` / `form_height` | 80 / 82 | 96 / 98 |
| `next_font` | 84 | 104 |
| `reserved` | 88 | 112 |
| `sizeof` | 90 | 120 |

`off_table` and the header's words are in the CPU's order, the data table is a byte
stream (R5).


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
- **`vst_load_fonts` already-loaded / `CONF_WITH_GDOS` = 0:** returns `INTOUT[0]=0`.
  With a real chain the result is the face count (rules R1-R3 in the
  [font chain section](#font-chain-vst_load_fonts--vst_unload_fonts-gdos-interface)).

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
  (default 1)** (`include/config.h:1620-1622`; `vdi_text.c:1294,1355`; the
  `trnsfont` helper and its call in `text_init` are gated alike, `:551,647`).
  Compiled in (the pinned build), `vst_load_fonts` **does** link whatever
  chain the caller passes: the ROM does not read font files, it only accepts
  a chain that a GDOS has built, and reports the number of faces in it
  (`:1320-1346`). It is **not** "typically 0" (earlier wording of this
  file, corrected 2026-11-06, V33; measured: probe T13A, oracle returns 1 for
  a one-font chain, `measurements/measurement-2026-11-06-v18-t2/probe/e1/VDPROBE.LOG:274`).
  If not compiled in, `vst_load_fonts` is a hard no-op returning
  `INTOUT[0]=0` and `vst_unload_fonts` does nothing (`:1347-1349`). Full rules:
  [Font chain](#font-chain-vst_load_fonts--vst_unload_fonts-gdos-interface).
- **`vst_rotation` follows the TOS3/TOS4 model** (normalise-then-round). TOS1
  did **not** normalise before rounding, so it stored values from −32400 to
  +32400 and `vqt_extent` could return nothing for out-of-range angles
  (`vdi_text.c:919-923`). EmuTOS deliberately uses the TOS3/4 behaviour.

## NewTOS implementation notes (Phase 12)

- The font ring is `tos-api/src/vdi/font.rs` (`ring_slot`, `head`, `first_of_face`), the
  setters `tos-api/src/vdi/attributes.rs` (`vst_font`, `select_height`, `select_point`,
  `vst_load_fonts`, `vst_unload_fonts`); rules R1-R14 above. Deviations, each with its reason:
  `docs/COMPATIBILITY.md`, "GDOS font chain" (the scratch buffer is not used, a NULL chain,
  a current face gone from the ring, memory of an ended program, cell limits).
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
