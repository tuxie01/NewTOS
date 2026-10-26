# VDI raster operations (Phase 00b D6)

> Source: EmuTOS @ `1a7484ab` — `vdi/vdi_raster.c:1470` (`vdi_vro_cpyfm`),
> `:1492` (`vdi_vrt_cpyfm`), `:269` (`vdi_vr_trnfm`), `:1330`
> (`cpy_raster`, the shared opaque/transparent driver), `:588` (`bit_blt`
> C fallback), `vdi/vdi_blit.S` (`fast_bit_blt`, the m68k fast path).
> MFDB type: `vdi_raster.c:166-176`, public ABI struct `FDB`
> `include/gsxdefs.h:65-76`. Logic-op + `PTSIN`-offset constants:
> `vdi_raster.c:94-124`. `MD_*` write modes: `vdi/vdi_defs.h:77-80`,
> `include/obdefs.h:41-44`. Dispatcher entries: `vdi/vdi_main.c:84-96`.
> Calling convention: [`../_dispatcher.md`](../_dispatcher.md).
>
> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> **Contract (must match TOS exactly):** the opcodes; the **MFDB layout**
> (`fd_addr`, `fd_w`, `fd_h`, `fd_wdwidth`, `fd_stand`, `fd_nplanes`, three
> reserved WORDs); the MFDB pointer slots in `CONTRL[7..10]`; the source/
> destination rectangle `PTSIN` layout; the **16-entry logic-op table**
> (`INTIN[0]`, esp. 3 = replace, 6 = XOR, 7 = OR) + the `PAT_FLAG` (bit 5:
> never a pattern fill — per **D-082** on the palette path it is merely
> stripped, and per **D-VDI-23** on the truecolor path it makes the whole
> call a silent no-op) and the `mode & ~0x1f` invalid rule; the `vrt_cpyfm` `MD_*` codes and its
> fg/bg colours; the standard↔device form transform direction (`vr_trnfm`
> toggles `fd_stand` per the source). **Reference:** the BitBlt pixel
> loops, skew/endmask machinery, the blitter register programming, the
> in-place transform algorithm — NewTOS reimplements the pixel work with
> its own `RasterBackend` (`tos-api/src/vdi/backend.rs`), reproducing the
> observable result.

## Opcodes (`vdi_main.c:84-96`)

| Op | Fn | `# PTSOUT` | `# INTOUT` | Effect |
|----|----|:----------:|:----------:|--------|
| 109 | `vro_cpyfm` | 0 | 0 | copy raster **opaque** (colour→colour, all planes, one of 16 logic ops) |
| 110 | `vr_trnfm`  | 0 | 0 | transform a form between **standard** and **device** layout |
| 121 | `vrt_cpyfm` | 0 | 0 | copy raster **transparent** (1-plane mono → colour, fg/bg expand) |

All three produce **no output array** and have **no error return**;
invalid parameters are a silent no-op. Native ABI (all): GEM trap
`svc #2`, selector 115 in `x0`, `VDIPB*` in `x1`; opcode in `CONTRL[0]`,
handle in `CONTRL[6]`. `v_get_pixel` (105) is in
[`../inquiry/_color-pixel-input.md`](../inquiry/_color-pixel-input.md).

> **Not a raster opcode.** `vsc_form` (111, `vdi_vsc_form`) is **not** a
> mono→colour expand — it transforms a user-defined *mouse cursor* to
> device format (`vdi_mouse.c:513`, `linea_transform_mouse`). There is no
> separate `vsc_expand` opcode in EmuTOS; monochrome→colour expansion is
> the `vrt_cpyfm` (121) transparent path below. `vsc_form` belongs to the
> cursor subsystem ([`../input/_mouse.md`](../input/_mouse.md)) and is out
> of raster scope.

---

## MFDB — memory form definition block (shared raster data structure)

The source and destination forms are described by an **MFDB** (a.k.a.
`FDB`). Its pointers are passed in the control array:
**`CONTRL[7..8]` = source MFDB, `CONTRL[9..10]` = destination MFDB**
(each a 32-bit pointer split across two WORDs on m68k;
`vdi_raster.c:278-279,915-916`). The struct
(`vdi_raster.c:166-176` = `include/gsxdefs.h:65-76`):

### m68k byte layout (the ABI a ported program builds)

| Offset | Size | Field | Meaning |
|:------:|:----:|-------|---------|
| 0 | 4 | `fd_addr` | pixel-data address; **NULL ⇒ the physical screen** |
| 4 | 2 | `fd_w` | form width in **pixels** |
| 6 | 2 | `fd_h` | form height in **pixels** |
| 8 | 2 | `fd_wdwidth` | form width in **words** = `(fd_w + 15) / 16` |
| 10 | 2 | `fd_stand` | `0` = device-specific (interleaved) form, `1` = standard (plane-sequential) form |
| 12 | 2 | `fd_nplanes` | number of bit-planes |
| 14 | 2 | `fd_r1` | reserved (0) |
| 16 | 2 | `fd_r2` | reserved (0) |
| 18 | 2 | `fd_r3` | reserved (0) |

Total 20 bytes on m68k. **Contract:** field order, sizes, and semantics.

- **Plane size in words** = `fd_h × fd_wdwidth` (used by `vr_trnfm`,
  `vdi_raster.c:300`).
- **Screen MFDB:** when `fd_addr == NULL`, the blit uses `v_bas_ad`
  (framebuffer base), `v_planes` planes, and `v_lin_wr` bytes/line
  instead of the MFDB's own geometry (`setup_info`,
  `vdi_raster.c:925-950`). Clipping is applied **only** when the
  *destination* is the screen and `vwk->clip` is set (`:947-949`).
- **`s_nxwd`/`d_nxwd`** (bytes to next word in a line) = `fd_nplanes × 2`
  for an MFDB, `v_planes × 2` for the screen (`:922,928,937,944`). Equal
  `nxwd` on both sides ⇔ equal plane counts — the `vro_cpyfm` match test.

### Native (AArch64) variant

Per [`docs/COMPATIBILITY.md`](../../../COMPATIBILITY.md#native-vs-m68k-struct-variants):
NewTOS-recompiled C sees `fd_addr` as a **64-bit** pointer (the eight
WORD fields keep their meaning and 16-bit size). The m68k marshaling shim
(Phase 10) translates the 32-bit-pointer layout above for emulated
binaries. Both expose the same fields.

## Logic operations — `INTIN[0]` (16-entry BitBlt table)

`vro_cpyfm` takes a raster op in `INTIN[0]`. The low 4 bits select one of
16 boolean combinations of source (S) and destination (D)
(`vdi_raster.c:94-110`, applied in `do_blit`/`bit_blt`
`:495-544,746`):

| # | Name | `D' =` | # | Name | `D' =` |
|:-:|------|--------|:-:|------|--------|
| 0 | `ALL_WHITE` | `0` | 8 | `NOT_SORD` | `~(S \| D)` |
| 1 | `S_AND_D` | `S & D` | 9 | `NOT_SXORD` | `~(S ^ D)` |
| 2 | `S_AND_NOTD` | `S & ~D` | 10 | `NOT_D` | `~D` |
| **3** | **`S_ONLY`** | **`S`  (replace)** | 11 | `S_OR_NOTD` | `S \| ~D` |
| 4 | `NOTS_AND_D` | `~S & D` | 12 | `NOT_S` | `~S` |
| 5 | `D_ONLY` | `D` (no-op) | 13 | `NOTS_OR_D` | `~S \| D` |
| **6** | **`S_XOR_D`** | **`S ^ D`  (XOR)** | 14 | `NOT_SANDD` | `~S \| ~D`* |
| **7** | **`S_OR_D`** | **`S \| D`  (OR)** | 15 | `ALL_BLACK` | `1` |

*`BM_NOT_SANDD` is coded as `~S | ~D` in EmuTOS (`vdi_raster.c:539`;
De Morgan of `~(S & D)`). **Bit 5 (`0x10` = `PAT_FLAG`)** is stripped from
`mode` before the logic-op switch (`vdi_raster.c:113,1345-1349`); **any
bit above bit 5 set (`mode & ~0x1f`) is invalid → silent no-op**
(`cpy_raster:1341-1342`).

> **`PAT_FLAG` does not enable a pattern fill (D-082).** `cpy_raster`
> unconditionally sets `info->p_addr = NULL` (`vdi_raster.c:1345`)
> *before* checking the flag, and `setup_pattern` — called only when
> `PAT_FLAG` is set — fills in `p_nxpl`/`p_nxln`/`p_mask` but never
> restores `p_addr` (`:790-800`). So even though `vdi_vro_cpyfm`/
> `vdi_vrt_cpyfm` pre-load `vdi_info.p_addr = vwk->patptr` (`:1475,1497`),
> `cpy_raster` immediately discards it. `fast_bit_blt`'s only pattern
> test (`vdi_blit.S:1055-1056`, `tst.l P_ADDR(a6) / jeq next_plane`)
> therefore always takes the "no pattern" branch, and the C `bit_blt`/
> `do_blit()` fallback (`:588-773`, `424-565`) never references the
> pattern fields at all. **Net effect on the palette path
> (`plane_ct <= 8`): setting `PAT_FLAG` on `vro_cpyfm`/`vrt_cpyfm`/line-A
> raster copy is observably identical to leaving it clear** (bit 5 is
> stripped, the low 4 bits are interpreted the same, no pattern is ever
> blitted) — a genuine EmuTOS source-level gap (regression from the
> Line-A-raster refactor in `d7ac3170`, which moved the
> `p_addr = vwk->patptr` assignment out of `setup_pattern` without
> replacing it), not a spec-reading error. See global discrepancy log
> **D-082**.

> **On the truecolor path (`plane_ct > 8`, `CONF_WITH_VDI_16BIT`)
> `PAT_FLAG` is not benign — it silently kills the whole copy
> (measured for both opcodes, `D-VDI-23`).** `cpy_raster` strips bit 5
> into its *own local* `mode` (`vdi_raster.c:1347`) and then dispatches
> to the truecolor routine **without passing it** (`:1367-1372` opaque,
> `:1432-1437` transparent). Both truecolor routines re-read the raw,
> **unstripped** `INTIN[0]` (`vro_cpyfm16:981`, `vrt_cpyfm16:1232`), so a
> mode of `0x10..0x1F` matches no arm: `vro_cpyfm16`'s 16-entry switch
> has no `default:` (`:1008-1220`) and falls straight out, `vrt_cpyfm16`'s
> has an explicit `default: return` (`:1323-1324`). Either way the
> destination is left untouched. This is the same class of gap as D-082
> (a stripped value computed and then not used) but a **different,
> stronger** observable: not "the flag does nothing", but "the call does
> nothing".
>
> **Measured, `vro_cpyfm`** on a real Atari TT running EmuTOS 1.4 whose
> build has `CONF_WITH_VDI_16BIT` enabled
> (`tosdump/reference/EMUTOS14-RASTER.BIN`, 2026-09-06): case 144
> (`INTIN[0] = 0x13`, 16-plane memory forms) leaves the destination buffer
> bit-identical to its fill, while case 127 (`INTIN[0] = 3`, identical
> geometry) copies 64 words. Since a NewTOS workstation is *always*
> truecolor (`v_planes = 16`, memory MFDBs stamped 16 planes), the
> truecolor rule is the one NewTOS reproduces
> (`tos-api/src/vdi/raster.rs::vro_cpyfm`).
>
> **Measured, `vrt_cpyfm` (R05, 2026-09-21).** The TT cannot run
> `vrt_cpyfm16` at all — it dereferences `CUR_WORK->ext->palette`
> (`:1234`), and `vwk->ext` is NULL outside a truecolor mode
> (`vdi_control.c:450-452`) — so the twin was measured on EmuTOS 1.4
> (release ROM and the pinned source, 0 differences) on Hatari Falcon030
> in VIDEL truecolor (640×480, 16 planes):
> `tosdump/cases/vdi-raster-screen.tsv` (CRC `9780EC39`) case 215
> (`INTIN = 0x11,1,0` = `PAT_FLAG | MD_REPLACE`, 32×16 mono form → screen
> at (100,60), read back with `v_get_pixel`) leaves all 512 words at the
> cleared-screen value `0xFFFF`, while case 201 (`INTIN[0] = 1`, same
> geometry) expands the form to `0x0000`/`0xFFDF`
> (`tosdump/reference/EMUTOS14-RASTER-screen-falcontc.BIN`). The rule is
> therefore the same for both opcodes: **on a `> 8`-plane destination any
> `INTIN[0]` with bit 5 set draws nothing.** NewTOS' `vrt_cpyfm` now takes
> the same early return as `vro_cpyfm`
> (`tos-api/src/vdi/raster.rs::vrt_cpyfm`); host test
> `tos-api/tests/vdi_raster_reference/cases_screen.rs::case_0215_…`.

## Source / destination rectangle — `PTSIN` layout

Both rectangles are `arb_corner`-sorted to `(min,max)` order before use
(`cpy_raster:1336-1337`; `vdi_raster.c:116-124`):

| Slot | Meaning |
|:----:|---------|
| `PTSIN[0]`, `PTSIN[1]` | source rect upper-left `(x1, y1)` |
| `PTSIN[2]`, `PTSIN[3]` | source rect lower-right `(x2, y2)` |
| `PTSIN[4]`, `PTSIN[5]` | destination upper-left `(x1, y1)` |
| `PTSIN[6]`, `PTSIN[7]` | destination lower-right `(x2, y2)` |

The blit copies a source rectangle of size `(x2−x1+1)×(y2−y1+1)`; the
**size** of the copy always comes from the source rectangle, never from
`PTSIN[6..7]` (`dont_clip` sets `b_wd`/`b_ht` from `s_x/ymax − s_x/ymin`,
`vdi_raster.c:882-900`).

> **But `PTSIN[6..7]` is not ignored — it anchors the reverse-direction
> copy (measured, `D-VDI-25`).** `dont_clip` copies `PTSIN[6]`/`PTSIN[7]`
> into `info->d_xmax`/`d_ymax` **literally** (`:898-899`), and the
> overlap-safe backwards path starts there:
> `vro_cpyfm16` uses both (`:995-1001`), the palette `bit_blt` recomputes
> `d_xmax` from `d_xmin + b_wd − 1` (`:607`) but still takes `d_ymax` from
> `PTSIN[7]` (`:674-675`). A caller whose destination lower-right is
> **consistent** with the source size (`d_xmax = d_xmin + b_wd − 1`,
> `d_ymax = d_ymin + b_ht − 1`) sees no difference between the forward and
> backward path — that is every well-formed caller. A caller that passes an
> **oversized** `PTSIN[6..7]` lands the block at the *lower-right* anchor
> when the backward path is taken. Which path is taken is decided by
> comparing the two computed **start addresses** (`src < dst`,
> `vdi_raster.c:995`; `s_addr < d_addr`, `:668-669`), i.e. by where the two
> forms happen to sit in memory — so for two *distinct* forms the outcome
> is address-layout-dependent and not portable. Measured: `tosdump` case
> 157 (src rect `(0,0)-(7,7)`, dest `(0,0)-(31,15)`, 16-plane forms, form A
> below form B) lands EmuTOS' 8×8 block at rows 8..15 / cols 24..31 and
> NewTOS' at rows 0..7 / cols 0..7.

**`arb_corner` does *not* write back into the caller's `PTSIN`
(measured, `D-VDI-24`).** The m68k VDI trap glue copies the caller's
`PTSIN` into the VDI's own `lcl_ptsin` before dispatch and never copies
it back (`vdi/vdi_asm.S:79-81,99-108`); `CONTRL`, `INTIN`, `INTOUT` and
`PTSOUT` do point at the caller's arrays, `PTSIN` alone does not. So the
in-place `arb_corner((Rect*)PTSIN)` in `cpy_raster` sorts a copy: after
the call the caller's reversed corners are still reversed, and the blit
still landed at the sorted rectangle. Measured on a real TT: `tosdump`
cases 147 (`PTSIN = 7,7,0,0,…`) and 148 (`…,15,15,8,8`) come back with
`PTSIN` **unchanged** on both machines while both blit to the same 64
words. An implementation that sorts into locals (NewTOS
`raster.rs::rect_from_ptsin`) is therefore the *correct* reproduction of
the observable contract, not a deviation.

---

## `vro_cpyfm` (109) — copy raster opaque

Source: `vdi_vro_cpyfm` (`vdi_raster.c:1470-1483`) → `cpy_raster`
(`transparent = 0`).

1. `arb_corner` both rectangles; `mode = INTIN[0]`. If `mode & ~0x1f` →
   **no-op** (`:1341-1342`).
2. If bit 5 (`PAT_FLAG`) set, strip it **into `cpy_raster`'s own local
   `mode`** and call `setup_pattern`, which fills `p_nxpl`/`p_nxln`/
   `p_mask` — but the pattern pointer itself is always `NULL` at this
   point and stays `NULL` (`:1345-1349`), so no pattern is ever applied
   (D-082, see above). On the **truecolor** path the stripped `mode` is
   then discarded: `vro_cpyfm16` re-reads the raw `INTIN[0]` and
   `0x10..0x1F` copies nothing at all (`D-VDI-23`, boxed note above).
3. `setup_info` fills the blit frame from both MFDBs (screen substitution
   for `fd_addr == NULL`); returns "nothing to do" if the plane count is
   invalid (`> 8`, or `> 16` under `CONF_WITH_VDI_16BIT`) or clipping took
   everything (`:908-967`).
4. **Source and destination plane counts must match** (`s_nxwd !=
   d_nxwd` → **no-op**, `:1360-1361`).
5. The single logic op goes into `op_tab[0]` with `fg_col = bg_col = 0`
   (so only `op_tab[0]` is referenced), then all planes are blitted via
   `bit_blt`/`fast_bit_blt` (`:1363-1461`), clipped to the `Vwk` clip
   rect **iff the destination is the screen** and `vwk->clip` is set.

**Inputs:** src/dst MFDB in `CONTRL[7..10]`; `PTSIN[0..7]` (src+dst
rects); `INTIN[0]` = one of the 16 logic ops (`| 0x10` for pattern).
**Output:** the destination form is modified in place; no `INTOUT`/
`PTSOUT`.

## `vrt_cpyfm` (121) — copy raster transparent (mono → colour expand)

Source: `vdi_vrt_cpyfm` (`vdi_raster.c:1492-1505`) → `cpy_raster`
(`transparent = 1`).

Copies a **1-plane (monochrome)** source into a colour destination,
expanding 1-bits and 0-bits to colours according to a write mode:

- `INTIN[0]` = **write mode**: `MD_REPLACE` (**1**), `MD_TRANS` (**2**),
  `MD_XOR` (**3**), `MD_ERASE` (**4**); **any other value → no-op**
  (`cpy_raster:1397-1430`, `default: return`).
- `INTIN[1]` = **foreground** colour index, `INTIN[2]` = **background**
  colour index; both pass through `validate_color_index` (out of
  `[0,numcolors)` → 1) then `MAP_COL[]` to a PEL
  (`vdi_raster.c:1390-1395`).
- The **source MFDB must be one plane** (`s_nxwd != 2` → **no-op**,
  `:1384-1385`); only that one plane is read (`s_nxpl = 0`).

Each mode fills a 4-entry `op_tab` (indexed by the (fg-bit, bg-bit) of the
destination plane) with per-plane logic ops (`vdi_raster.c:1397-1430`):

| `INTIN[0]` mode | Effect of a source 1-bit | Effect of a source 0-bit | `op_tab` (fg/bg) |
|-----------------|--------------------------|--------------------------|------------------|
| `MD_REPLACE` (1) | write **fg** colour | write **bg** colour | `[00,12,03,15]`, both colours |
| `MD_TRANS` (2)   | write **fg** colour | leave destination | `[04,·,07,·]`, fg only |
| `MD_XOR` (3)     | XOR the destination | leave destination | `[06,·,·,·]`, no colour |
| `MD_ERASE` (4)   | leave destination | write **bg** colour | `[01,13,·,·]`, bg only |

> **Colour-argument order.** The variable names in EmuTOS make the intent
> explicit: `fg_col` is built from **`INTIN[1]`** and `bg_col` from
> **`INTIN[2]`** (`vdi_raster.c:1390-1395`) — `INTIN[1]` is the
> foreground (1-bit) colour, `INTIN[2]` the background (0-bit) colour.
> (The inline `/* CHECK: bug, ... colors reversed? */` comment at `:1406`
> is an EmuTOS self-note; the behaviour is preserved as-is and is the
> TOS-compatible contract.)

**Inputs:** src (mono) + dst MFDB in `CONTRL[7..10]`; `PTSIN[0..7]`;
`INTIN[0]` = `MD_*` mode; `INTIN[1]`/`INTIN[2]` = fg/bg colour indices.
**Output:** destination modified in place; no `INTOUT`/`PTSOUT`.

## `vr_trnfm` (110) — transform form (standard ↔ device)

Source: `vdi_vr_trnfm` (`vdi_raster.c:269-356`). Converts a bitmap
between **standard** (plane-sequential, machine-independent) and
**device-dependent** (plane-interleaved) layout.

- MFDBs from `CONTRL[7..8]` (source) / `CONTRL[9..10]` (destination).
- **Direction is driven by the source's `fd_stand`, and the destination's
  `fd_stand` is forced to the opposite** (`vdi_raster.c:303-314`):
  - `src->fd_stand == 1` (standard) → `dst->fd_stand = 0` (device);
    outer loop = planes, inner = plane size.
  - `src->fd_stand == 0` (device) → `dst->fd_stand = 1` (standard);
    outer loop = plane size, inner = planes.
- Plane size = `fd_h × fd_wdwidth` words (`:300`), read from the
  **source** MFDB; `fd_nplanes` also from the source (`:299`).
- **In-place** (`src->fd_addr == dst->fd_addr`) is supported via a slow
  element-rotation path (`:329-355`); for a **1-plane** form the standard
  and device layouts are identical, so an in-place mono transform is a
  **no-op** (`:330-331`).

**Inputs:** source MFDB (its `fd_stand` selects the direction) + dest
MFDB (its `fd_stand` is overwritten). **Output:** the destination pixel
buffer holds the transformed form; `dst->fd_stand` is set to `!src->fd_stand`.
No `INTOUT`/`PTSOUT`.

**Truecolor (`CONF_WITH_VDI_16BIT`, `> 8` planes):** delegates to
`vr_trnfm16` (`:190-256`) — out of the palette bring-up scope.

---

## Verification criteria

- **`vro_cpyfm` replace (op 3):** an S→D copy of a source MFDB rect
  (`PTSIN[0..3]`) to a dest rect origin (`PTSIN[4..5]`) with
  `INTIN[0]=3` reproduces the source pixels in the destination
  (matching plane counts; `s_nxwd == d_nxwd`).
- **`vro_cpyfm` XOR (op 6):** `INTIN[0]=6` XORs source into destination —
  applying it twice restores the original (rubber-band idiom).
- **`vro_cpyfm` plane-mismatch no-op:** source `fd_nplanes=1`, dest
  `fd_nplanes=4` → the call does nothing (`s_nxwd != d_nxwd`).
- **`vro_cpyfm` invalid mode no-op:** `INTIN[0]=0x20` (`& ~0x1f` set) →
  no change.
- **`vrt_cpyfm` replace (mode 1):** a 1-plane source with `INTIN=[1, fg,
  bg]` writes `fg` where the source bit is 1 and `bg` where it is 0.
- **`vrt_cpyfm` transparent (mode 2):** `INTIN[0]=2` writes `fg` on
  1-bits and leaves 0-bit pixels untouched.
- **`vrt_cpyfm` non-mono source no-op:** a source MFDB with
  `fd_nplanes != 1` → no change (`s_nxwd != 2`).
- **`vr_trnfm` toggles `fd_stand`:** given `src->fd_stand=1`, after the
  call `dst->fd_stand == 0` and the pixel buffer is the interleaved
  (device) form; given `src->fd_stand=0`, `dst->fd_stand == 1`.
- **`vr_trnfm` in-place mono:** `src==dst`, `fd_nplanes=1` → buffer
  unchanged (no-op).
- **Screen MFDB:** an MFDB with `fd_addr == NULL` targets the physical
  framebuffer (`v_bas_ad`/`v_planes`/`v_lin_wr`), and clipping applies
  only when it is the *destination*.

## Edge cases discovered in source

1. **`MD_*` codes: `MD_REPLACE = 1`, `MD_TRANS = 2`, `MD_XOR = 3`,
   `MD_ERASE = 4`** (`vdi_defs.h:77-80`). (An earlier revision of this
   spec had `MD_TRANS=1`/`MD_REPLACE=2` — **wrong**; corrected.)
2. **`vro_cpyfm` requires equal plane counts** (`s_nxwd == d_nxwd`) — an
   opaque copy between forms of different depth is a silent no-op
   (`vdi_raster.c:1360-1361`), *not* an error and *not* a depth
   conversion.
3. **`vrt_cpyfm` source must be exactly 1 plane** (`s_nxwd == 2`);
   otherwise no-op (`:1384-1385`). It is the only mono→colour expander.
4. **Only bits 0-5 of `INTIN[0]` are meaningful** for the copy ops; bit 5
   is `PAT_FLAG`, and `mode & ~0x1f` is invalid (`:1341-1349`).
5. **Overlapping src/dst is handled by direction choice.** `bit_blt`
   picks a right-to-left / bottom-to-top scan when `s_addr < d_addr`
   (or equal with non-negative skew), so an overlapping opaque copy does
   not corrupt itself (`vdi_raster.c:667-704`). NewTOS must reproduce the
   overlap-safe result.
6. **`vr_trnfm` direction comes from the *source* `fd_stand`**, and it
   **overwrites** `dst->fd_stand` — a caller cannot force a direction via
   the destination flag (`:303-314`). `fd_nplanes`/plane size are read
   from the source MFDB only.
7. **In-place `vr_trnfm` for mono is a no-op**; for multi-plane it uses a
   slow rotation that Atari TOS also has (`:329-355`).
8. **Clipping is destination-screen-only.** A blit into an off-screen
   MFDB is never clipped to the `Vwk` clip rect, even with clipping
   enabled (`setup_info:947-949`).
9. **`PAT_FLAG` (bit 5) never actually blits a pattern** through
   `vro_cpyfm`/`vrt_cpyfm`/line-A raster copy — `cpy_raster` nulls the
   pattern pointer unconditionally before the flag is even tested, and
   `setup_pattern` doesn't restore it (`vdi_raster.c:1345-1349`,
   `790-800`). Confirmed dead in every build configuration: the
   assembler `fast_bit_blt` skips its pattern branch on a null pointer
   (`vdi_blit.S:1055-1056`), and the C `bit_blt`/`do_blit` fallback used
   for the hardware blitter / ColdFire never references the pattern
   fields at all. See **D-082** (global discrepancy log).
10. **On the truecolor path `PAT_FLAG` makes the whole call a no-op**
   (`plane_ct > 8` + `CONF_WITH_VDI_16BIT`): `vro_cpyfm16`/`vrt_cpyfm16`
   re-read the unstripped `INTIN[0]` (`:981`, `:1232`) and `0x10..0x1F`
   matches no switch arm (`:1008-1220` no `default:`; `:1323-1324`
   `default: return`). Measured for `vro_cpyfm` on a real TT (case 144,
   `vdi-raster.tsv`) and for `vrt_cpyfm` on Hatari Falcon030 truecolor
   (case 215, `vdi-raster-screen.tsv`, R05, 2026-09-21). `D-VDI-23`.
11. **The caller's `PTSIN` is never modified.** `arb_corner` sorts the
   VDI's *local* copy (`vdi_asm.S:79-81`), so reversed corners come back
   reversed. Measured (cases 147/148). `D-VDI-24`.
12. **`PTSIN[6..7]` anchors the backwards copy.** The *size* is always the
   source's, but the reverse-direction start corner is taken literally
   from `PTSIN[6..7]` (`dont_clip:898-899`, used at `:995-1001`); a
   well-formed caller cannot observe it, an oversized destination
   rectangle can. Measured (case 157). `D-VDI-25`.

## Dependencies

### Requires
- An open workstation (dispatcher resolves `CONTRL[6]`); the clip
  rectangle + `clip` flag (for screen destinations); `MAP_COL[]` +
  `validate_color_index` + `numcolors` (for `vrt_cpyfm` colours);
  `v_bas_ad`/`v_planes`/`v_lin_wr` for screen forms.
- `CONF_WITH_VDI_16BIT` (machine-dependent, requires `CONF_WITH_VIDEL`)
  for the truecolor `vro_cpyfm16`/`vrt_cpyfm16`/`vr_trnfm16` paths.

### Used by
- AES icon/image drawing, `objc_draw` blits, screen save/restore, and any
  program moving bitmaps (`vro_cpyfm` for opaque, `vrt_cpyfm` for
  transparent/monochrome sprites, `vr_trnfm` to unpack resource images
  authored in standard form).

## Related data structures

- **MFDB** — laid out above; the fixed ABI form parsed by ported
  programs. Also referenced by
  [`../workstation/102-vq_extnd.md`](../workstation/102-vq_extnd.md)
  (`INQ_TAB[30]` = "does `vro_cpyfm` scale?", 0 on the screen).
- [`../_state.md`](../_state.md) — the `Vwk` clip rect + `clip` flag and
  `multifill` these read; the `MAP_COL[]` pen→PEL map.
- Logic-op constants `BM_*` (`vdi_raster.c:94-110`), `PAT_FLAG`
  (`:113`); `MD_*` (`vdi_defs.h:77-80`).

## Cross-references

### Atari Compendium / Profibuch
- Not present in `references/` (no PDFs). Cross-check of the MFDB byte
  layout, the 16-entry logic-op numbering, the `MD_*` code values (esp.
  the `MD_REPLACE=1`/`MD_TRANS=2` correction), and the `vrt_cpyfm`
  fg/bg (`INTIN[1]`/`INTIN[2]`) order against the documented ABI is
  **deferred**. Also unresolved: whether real Atari TOS's `vro_cpyfm`/
  `vrt_cpyfm` PAT_FLAG ever functioned (D-082 is confirmed dead in
  EmuTOS across its whole history back to the source's initial DRI
  import; no comparison possible without ROM disassembly).

### Discrepancies found by the first raster measurement (2026-09-06)

First-ever measured run of this group: `tosdump/cases/vdi-raster.tsv`
(59 cases, CRC `702EEA7D`) against a real Atari TT running EmuTOS 1.4
(`tosdump/reference/EMUTOS14-RASTER.BIN`) and a real Raspberry Pi 4 running
NewTOS (`tosdump/reference/NEWTOS-RASTER.BIN`).

- **D-VDI-23** — `PAT_FLAG` on the truecolor path aborts the copy
  (case 144). Spec corrected, NewTOS `vro_cpyfm` fixed. The `vrt_cpyfm`
  half was closed by the screen-raster measurement (R05, 2026-09-21,
  `vdi-raster-screen.tsv` case 215 on Hatari Falcon030 truecolor,
  `tosdump/reference/EMUTOS14-RASTER-screen-falcontc.BIN`): same rule, NewTOS
  `vrt_cpyfm` fixed.
- **D-VDI-24** — `arb_corner` does **not** write the sorted rectangle back
  into the caller's `PTSIN`; the trap glue works on a local copy
  (cases 147/148). Spec corrected; NewTOS was already right.
- **D-VDI-25** — `PTSIN[6..7]` anchors the reverse-direction copy (case
  157). Spec corrected; NewTOS deviates, **open** — see the discrepancy
  log for why a faithful reproduction is address-layout-dependent.

### Discrepancies found in this re-validation (2026-07-10)
- **D-082** (global log) — `PAT_FLAG` (bit 5 of `vro_cpyfm`/`vrt_cpyfm`/
  line-A raster copy's mode byte) never applies a pattern; see the boxed
  note under "Logic operations" above and
  [`../../_discrepancies.md`](../../_discrepancies.md#d-082).

## Configuration / version notes

- `CONF_WITH_VDI_16BIT` (needs `CONF_WITH_VIDEL`; 1 on Falcon/FireBee, 0
  on ST-class) adds the `*16` truecolor variants and raises the valid
  plane cap from 8 to 16. NewTOS bring-up is palette (≤ 8 planes); the
  truecolor paths are deferred.
- The fast m68k blit (`fast_bit_blt`, `vdi_blit.S`) vs the C `bit_blt`
  vs the hardware blitter (`CONF_WITH_BLITTER`) are all **reference**
  implementations of the same op table — NewTOS uses its own backend.

## NewTOS implementation notes (Phase 12)

- `bit_blt` maps to the `RasterBackend` block-copy primitive
  (`tos-api/src/vdi/backend.rs`). Implement the **full 16-entry logic-op
  table** (not just replace) — programs rely on XOR (op 6) for
  rubber-banding and OR (op 7) for overlays. Target/source is the MFDB
  buffer, or `vwk->device` when `fd_addr == NULL` (ADR-011).
- Keep `vro_cpyfm`'s equal-plane requirement, `vrt_cpyfm`'s mono-source
  requirement, and the overlap-safe copy direction as observable
  contract.
- **Do not implement pattern-fill for `PAT_FLAG`.** Per D-082, EmuTOS
  itself never blits a pattern through `vro_cpyfm`/`vrt_cpyfm` — bit 5
  is read and stripped from `mode` (so the low-4-bit logic op is
  unaffected) but no pattern is ever composited. Reproducing EmuTOS
  means treating `PAT_FLAG` as a no-op flag, not building pattern
  support into `vro_cpyfm`/`vrt_cpyfm`. (Pattern-fill for *fill* opcodes,
  e.g. `v_fillarea`/`vr_recfl`, is unrelated and out of this doc's scope.)
- **…but on the truecolor path `PAT_FLAG` must abort the call**
  (`D-VDI-23`, measured: `vro_cpyfm` case 144 on the TT, `vrt_cpyfm`
  case 215 on Hatari Falcon030 truecolor). A NewTOS workstation is always
  truecolor (`v_planes = 16`; memory MFDBs stamped 16 planes), so both
  `vro_cpyfm` and `vrt_cpyfm` gate on the *destination* plane count
  exactly as `cpy_raster` does: `plane_ct > 8` **and** bit 4 set ⇒ return
  without touching the destination; `plane_ct <= 8` keeps the strip-only
  rule. Implemented in `tos-api/src/vdi/raster.rs::vro_cpyfm` and
  `::vrt_cpyfm` (`PALETTE_PLANES_MAX`).
- `vr_trnfm` matters for unpacking icon/image data authored in standard
  form; keep both directions and the source-driven `fd_stand` toggle.
- The MFDB is a **fixed ABI** parsed by ported programs — match the field
  order/size exactly (unlike the internal `Vwk`); honour the native
  64-bit `fd_addr` variant vs the m68k 32-bit one via the header set /
  Phase-10 shim.
- Truecolor paths (`CONF_WITH_VDI_16BIT`) are out of bring-up scope
  (8-bit palette screen first).

---

## Pi / BCM2711 binding

> Re-points the *hardware reference* for the **screen MFDB** (`v_bas_ad` /
> `v_planes` / `v_lin_wr` Shifter screen vars) at the BCM2711 VideoCore
> framebuffer. The **contract is unchanged** — the MFDB layout, the
> `fd_addr == NULL ⇒ screen` rule, the 16-op logic table, the `MD_*` codes.
> Peripheral + driver: see `docs/HARDWARE-PROFILE.md` §2/§3.1.

### The screen MFDB = the VideoCore framebuffer

> **Correction (A86, 2026-09-21).** This section previously described the
> screen MFDB as a **4-plane, 16-colour VDI model** (`fd_nplanes = 4`,
> citing a since-removed `vdi_backend.rs:21`/`NUM_COLORS=16`) over a 640×480
> framebuffer. Both figures are stale: the live backend is a **Falcon
> truecolor** workstation — `NUM_COLORS = 256` / `PLANES = 16`
> (`kernel/src/vdi_backend.rs:33,40`) — matching the reference profile's
> `CONF_WITH_VIDEL` + `CONF_WITH_VDI_16BIT` gating
> (`docs/analysis/aes-audit/emutos-config-profile.md:150,314`, "VDI-16-Bit-
> Truecolor-Modi MÜSSEN real sein"). The 640×480 figures also outlived their
> citation: `framebuffer.rs` no longer has compile-time `WIDTH`/`HEIGHT`
> constants (geometry is an `AtomicU32` pair, negotiated at boot); both
> boards default to 1920×1080 (`kernel/src/drivers/mod.rs:31-33`
> `BOOT_WIDTH`/`BOOT_HEIGHT`; RPi4 falls back to it if the mailbox query
> comes up empty, `kernel/src/drivers/bcm2711/framebuffer.rs:29-33`; QEMU uses it unconditionally,
> `kernel/src/drivers/qemu_virt/ramfb.rs:80-81`). The table below reflects both corrections.
> The rewrite below could **not**, at the time of this correction, find a
> dedicated `docs/MEMORY.md` ADR for the truecolor VDI pivot to cite — the
> code itself only pointed at a placeholder, `[ADR-0xx truecolor VDI]`
> (`docs/api/vdi/workstation/102-vq_extnd.md:198`) — so this section cited
> the code and the already-resolved [D-VDI-7](../_discrepancies.md)
> discrepancy instead; flagged, not guessed. **Resolved (A101, 2026-09-21,
> same day):** `docs/MEMORY.md` **ADR-024** now records that decision; the
> placeholder is replaced.

When a program passes `fd_addr == 0` (NULL MFDB), the source/destination is
the physical screen (`tos-api/src/vdi/raster.rs:39-42,59-61`). NewTOS
substitutes the live framebuffer rather than `v_bas_ad`/`v_planes`/
`v_lin_wr`: the `RasterBackend` reads/writes the VideoCore buffer directly
(`kernel/src/vdi_backend.rs` `copy_rect`/`blit_mono`, `:736-784`/`:696-734`),
so raster ops never dereference the MFDB geometry for screen forms — they
use the backend's `width()`/`height()`/`stride_px()`. The equivalent
screen-MFDB values:

| MFDB field | Atari screen var | NewTOS value | Source |
|-----------|------------------|--------------|--------|
| `fd_addr` (screen) | `v_bas_ad` | the VideoCore fb base (CPU-visible VA) | `vdi_backend.rs:868-871` (`base_addr`) |
| `fd_w` | `V_REZ_HZ` | **1920** (negotiated; QEMU fixed, RPi4 mailbox-negotiated with this fallback) | `kernel/src/drivers/mod.rs:31-33` (`BOOT_WIDTH`), `kernel/src/drivers/bcm2711/framebuffer.rs:29-33`, `kernel/src/drivers/qemu_virt/ramfb.rs:80-81` |
| `fd_h` | `V_REZ_VT` | **1080** (same) | `kernel/src/drivers/mod.rs:31-33` (`BOOT_HEIGHT`) |
| `fd_wdwidth` | — | **120** = `(1920+15)/16` | words/row for the negotiated raster width |
| `fd_nplanes` | `v_planes` | **16** (Falcon truecolor pixel depth) | `vdi_backend.rs:35-40` (`PLANES`); matches EmuTOS `INQ_TAB[4] = v_planes` (`vdi_control.c:210`) |
| `fd_stand` | — | 0 (device) for the screen | — |

### The VDI device pixel is 16-bit RGB565; the physical store is 32-bpp XRGB8888

EmuTOS' Falcon truecolor VDI (`TRUECOLOR_MODE`: `v_planes > 8`,
`vdi/vdi_defs.h:22`, active under `CONF_WITH_VDI_16BIT`) treats **the
device pixel** — what `pixelread`/`v_get_pixel`, `vro_cpyfm16`/
`vrt_cpyfm16`, and the truecolor draw routines read and write — as a
**16-bit screen word**, `RRRRRGGGGGOBBBBB` (bit 5 = `OVERLAY_BIT`,
`vdi_fill.c:26`: 0 on every pen-drawn pixel because `set_color16` packs
`(r<<11)|(g<<6)|b`, `vdi_col.c:601-614`; 1 on every pixel of a
`v_clrwk`-cleared screen, `memset(v_bas_ad, 0xff, …)`,
`vdi_control.c:610-618`; `vdi/vdi_defs.h:153-158`, `VwkExt.palette[256]`).
This is *the* meaning of `fd_nplanes = 16` for both the screen MFDB and a
device-form memory MFDB — sixteen bits per pixel, not sixteen bit-planes.

NewTOS' physical framebuffer store is a **different, NewTOS-internal**
32-bpp XRGB8888 buffer (`kernel/src/hal/framebuffer.rs` module doc + `rgb()`;
`vdi_backend.rs` module doc) — a program never addresses this format
directly; it exists only because the BCM2711 VideoCore has no native
16-bit truecolor mode to allocate against. **The device-pixel rule
(V11/V12, 2026-09-21): the 16-bit screen word round-trips exactly through
the 32-bpp store, overlay bit included.** The store word is a pure bitwise
expansion of the screen word, one field at a time:

| screen-word field | bits | store field | bits | expansion |
|---|---|---|---|---|
| red | 15..11 | R | 23..16 | `(c5 << 3) \| (c5 >> 2)` (VIDEL DAC replication) |
| green | 10..6 | G | 15..8 | same |
| `OVERLAY_BIT` | 5 | **X** | 31..24 | replicated into all eight bits: `0xFF` set, `0x00` clear |
| blue | 4..0 | B | 7..0 | same as red |

(`kernel/src/vdi_backend.rs` `word_to_device`, inverse `device_to_word`:
the top five bits of each gun and the top bit of X.) The X byte is the
right place because the display does not show it — `ramfb` is programmed
with the DRM `XRGB8888` fourcc (X = don't care,
`kernel/src/drivers/qemu_virt/ramfb.rs`), the VideoCore with depth 32 and
alpha mode "ignored" (set-alpha-mode tag `0x0004_8007` = 2 in the allocate
message since 2026-09-21, `docs/hardware/mailbox-framebuffer.md` §3c) — and
because
replicating it like a gun keeps every bitwise operation on the 32-bit store
equal to the same operation on the 16-bit word: the sixteen `vro_cpyfm`
logic ops (`raster_op`) and the `~` of `WM_XOR` act on the overlay bit
exactly as EmuTOS' `vro_cpyfm16`/`swblit_rect_common16` do on a Falcon.
Consequently the cleared screen is the pixel `0xFF_FFFFFF` and a pen-0 fill
`0x00_FFFFFF`: identical on the display, `0xFFFF` versus `0xFFDF` to the
API — as measured on the Falcon truecolor reference
(`tosdump/reference/EMUTOS14-VDI-all-falcontc.BIN`, cases 54/160/161). Every
non-VDI writer of the framebuffer packs its colours with
`hal::framebuffer::rgb()`, i.e. with the bit clear. The two formats are
bridged at exactly two points:

- **Draw → screen.** Every VDI draw op resolves its PEL through the
  drawing workstation's pseudo-palette to a 16-bit RGB555-in-565 word, then
  expands it into the 32-bpp store (`word_to_device`, called from
  `pel_to_rgb`, `vdi_backend.rs`); a pen word never carries the overlay
  bit, so a drawn pixel has X = `0x00`. `v_clrwk` is the one writer that
  bypasses the pens: `clear_screen_word(0xFFFF)` →
  `Framebuffer::clear(word_to_device(0xFFFF))`
  ([`../workstation/03-v_clrwk.md`](../workstation/03-v_clrwk.md)).
- **Screen → 16-bit domain.** Reading the device pixel back (`v_get_pixel`,
  `bb_save`, a memory-MFDB copy, the `v_contourfill` probe) converts the raw
  32-bit store word to the 16-bit screen word via
  `device_to_rgb565`/`rgb565_to_device` (`vdi_backend.rs`, the two trait
  methods over `device_to_word`/`word_to_device`;
  `tos-api/src/vdi/raster.rs:37-44` module doc). The round-trip is lossless
  for every 16-bit word (`raster.rs:41-44`). The mouse-cursor save/restore
  (`tos-api/src/vdi/mouse.rs`) copies raw 32-bit store words and so keeps
  the bit without converting.

`fd_nplanes = 16` and `ws_ncolors`/`DEV_TAB[13] = 256` are **both**
EmuTOS-correct simultaneously, not contradictory: EmuTOS's own formula is
`DEV_TAB[13] = (v_planes<8) ? (1<<v_planes) : 256` (`vdi_control.c:206`) —
for `v_planes = 16` this is the literal constant `256`, decoupled from the
plane count (`INQ_TAB[4] = v_planes = 16`, `vdi_control.c:210`). NewTOS
reproduces exactly this: `backend.num_colors() = NUM_COLORS = 256`
(`vdi_backend.rs:29-33`), `backend.planes() = PLANES = 16` (`:35-40`), fed
into `DEV_TAB[13]`/`INQ_TAB[4]` by `init_wk`/`vq_extnd`
(`tos-api/src/vdi/workstation.rs:144-145,200,301`) — resolved and pinned
2026-09-04 as [D-VDI-7](../_discrepancies.md).

### Palette — `vs_color`/`vq_color` on 256 truecolor pens

The screen has **256 logical pens**, each holding one RGB555-in-565
pseudo-palette word — EmuTOS `VwkExt.palette[256]` (`vdi_defs.h:153-158`),
written by `set_color16` and read by `vq_color16`
(`vdi_col.c:601-614,811-838`). NewTOS mirrors the split exactly:

- **The active draw-time palette.** One 256-entry `ACTIVE_PALETTE` array
  the VDI dispatcher points at the drawing workstation before every
  colour-sensitive op (`vdi_backend.rs:129-142`, citing EmuTOS
  `CUR_WORK->ext->palette`, `vdi_line.c:459-460`), seeded from EmuTOS'
  `videl_palette1`/`videl_palette2` device defaults (`vdi_backend.rs:42-127`,
  verbatim from `vdi_col.c:130-199`).
- **`vs_color` (opcode 14)** / **`vq_color` (opcode 26)** both index pens
  `0..255` (`ncol = backend.num_colors() = 256`, range-checked against it —
  `tos-api/src/vdi/attributes.rs:389-392`, `inquiry.rs:99-103`) and, under
  `backend.planes() > 8`, read/write the **calling workstation's own**
  `req_col`/pseudo-palette (`attributes.rs:395-408`, `inquiry.rs:106-121`),
  matching EmuTOS's `vs_color16`/`vq_color16` per-`Vwk` split rather than a
  single shared hardware DAC/CLUT.
- **Index 0 = white, 1 = black** is a seeded *convention*, not a VDI
  invariant: `VIDEL_PALETTE1[0] = (1000,1000,1000)` white,
  `[1] = (0,0,0)` black (`vdi_backend.rs:52-57`, verbatim EmuTOS
  `videl_palette1`, `vdi_col.c:130-136`), and the AES's `init_3d` relies on
  it being redefinable: it picks `actbutcol = indbutcol = backgrcol =
  LWHITE` (pen 8) instead of plain `WHITE` (pen 0) whenever
  `gl_ws.ws_ncolors > LWHITE` (`aes/gemoblib.c:1327-1343`; `LWHITE = 8`,
  `WHITE = 0`, `BLACK = 1`, `include/obdefs.h:107-117`) — true for NewTOS'
  256, so dialogs get the Falcon light-grey 3D face colour
  (`VIDEL_PALETTE1[8] = (733,733,733)`, `vdi_backend.rs:55`), not plain
  white, exactly as EmuTOS does on any >16-colour Falcon screen.
  `ws_ncolors` reaches the AES as `backend.num_colors()`
  (`tos-api/src/aes/object.rs:2310-2316`, `xor_color` doc).

### XOR semantics

Two distinct EmuTOS XOR paths, both reproduced on the 32-bpp physical store:

1. **`WM_XOR` draw mode** (`wrt_mode = 3`, lines/fills/text/glyphs). EmuTOS'
   truecolor driver complements the **whole stored screen word** — colour
   guns and `OVERLAY_BIT` alike — and ignores the pen: `*work = ~temp`
   (`vdi/vdi_line.c:494,501`, `swblit_rect_common16`), `*addr = ~*addr`
   (`draw_line16`, `vdi_line.c:1670,1730`), `*q = ~*q` (the glyph blitters,
   `vdi_textblit.c:806,1120`). NewTOS' `VdiScreen::xor_pixel`
   (`vdi_backend.rs`) reproduces that on the 32-bit physical pixel by
   complementing **all 32 bits** in place — the same operation on the
   16-bit word, since the store is its bitwise expansion (the X byte flips
   with the overlay bit: `0xFFDF` ↔ `0x0020`, `0xFFFF` ↔ `0x0000`) —
   self-inverse, matching EmuTOS bit-for-bit, reached via `combine`'s
   `WriteMode::Xor` arm from every fill/line/glyph draw path. (Until
   2026-09-21 only the low 24 bits were complemented, the X byte being
   fixed at `0xFF`; corrected with V11/V12.)
2. **`vro_cpyfm` op 6** (`BM_S_XOR_D`, the 16-entry BitBlt logic-op table,
   `s ^ d` on the raw pixel word — `vdi_backend.rs:824-852` doc + table,
   citing `vdi_raster.c:495-544`/`vro_cpyfm16:1008-1220`). **Screen↔Screen**
   applies this directly to the 32-bit XRGB8888 device word (`copy_rect`,
   `vdi_backend.rs:736-784`, using the `u32` `raster_op` at `:833-852`);
   **Screen↔Memory / Memory↔Screen / Memory↔Memory** first convert the
   screen pixel into the 16-bit RGB565 domain and apply the *same* table
   there instead (`tos-api/src/vdi/raster.rs:210-229` the `u16` `raster_op`;
   `:460-513` `screen_to_mem`/`mem_to_screen`, "apply the logic op in the
   16-bit domain", `:463-465`) — because a memory MFDB's wireformat is
   16-bit, not because the screen itself changes width.

The rubber-band idiom (apply the same XOR twice, restores the original)
holds for both paths — XOR is self-inverse at any word width.
`OVERLAY_BIT` (`0x0020`, `vdi_fill.c:26`) takes part in both, through the
X byte of the store (§ "The VDI device pixel" above); the former
**VDI-OVL-001** gap (the bit had no place in the 24-bit pixel) is closed
(V11/V12, 2026-09-21).

- **Screen-to-screen ops use the backend.** `vro_cpyfm` screen→screen maps
  to `RasterBackend::copy_rect` (replace op 3 / XOR op 6, above),
  overlap-safe (`vdi_backend.rs:736-784`); the cache clean to PoC
  (`vdi_backend.rs:241-268`) makes the copy visible to the VideoCore
  display DMA on real hardware.
- **Mono→colour `vrt_cpyfm`** expands a 1-plane source to fg/bg PEL via
  `blit_mono` (`vdi_backend.rs:696-734`); the mono-source (`s_nxwd==2`)
  requirement is a contract kept above the backend.
- **Native 64-bit MFDB pointer.** NewTOS carries the source MFDB in
  `CONTRL[7..10]` and the destination in `CONTRL[11..14]` (four WORDs each)
  because AArch64 pointers are 64-bit (`raster.rs:4-9`) — a documented
  NewTOS-native ABI extension of the 32-bit m68k `CONTRL[7..8]`/`[9..10]`
  layout; the field *meanings* are unchanged.

### NewTOS memory-MFDB format (device form, `fd_stand = 0`) — task #51

> **Correction (2026-09-20, A72):** this section previously described the
> format as **32-bpp chunky** with **one `u32` (XRGB8888) per pixel** and
> `fd_nplanes = 32`. That was stale even before A57: the memory-MFDB
> wireformat has been **16-bit RGB565-chunky** (`fd_nplanes = 16`) since
> the truecolor unification ("the unified 16-bit memory format (TC3b)",
> `tos-api/src/vdi/raster.rs:634`), and the code never carried a 32-bpp
> memory MFDB. The paragraph below is rewritten to the current source and
> now agrees with § "AES consumer" below, which already documented
> RGB565 words off the screen.

A memory MFDB (`fd_addr != 0`) in NewTOS is a **16-bit RGB565-chunky
truecolor** buffer: **one `UWORD` (`RRRRRGGGGGOBBBBB`, bit 5 =
`OVERLAY_BIT`: 0 on a pen-drawn pixel, 1 on a `v_clrwk`-cleared one)
per pixel**, not a 32-bpp `u32`/pixel bitmap and not a planar bitmap
(`tos-api/src/vdi/raster.rs:21-35`, module doc). This is the app-visible
wireformat EmuTOS' own `vro_cpyfm16`/`vrt_cpyfm16` operate on raw
(`vdi_raster.c:975-1221,1226-1326`) — observable, not an internal detail
(`docs/COMPATIBILITY.md`). The **physical** framebuffer store stays 32-bpp
XRGB8888; the 16↔32 conversion happens only at the screen boundary, inside
`screen_to_mem`/`mem_to_screen` via `RasterBackend::device_to_rgb565`/
`rgb565_to_device` (`raster.rs:37-44`). Because the screen is
5-bit-quantised and carries the overlay bit in the X byte, that round-trip
is lossless for every word, so `bb_save`/`bb_restore` reproduce the screen
pixels bit-for-bit, overlay bit included (`raster.rs:550-554`; a
`bb_save` over a `v_clrwk`-cleared area stores `0xFFFF` words, as on a
Falcon).

| MFDB field | NewTOS memory value | Meaning |
|-----------|---------------------|---------|
| `fd_addr` | caller-owned buffer | base of the `UWORD` (RGB565) pixel array; `bb_save`/`bb_restore` take it as a `*mut`/`*const u32` for the caller's convenience — two 16-bit pixels per `u32` word (`raster.rs:559-560,589-590`) |
| `fd_w` / `fd_h` | region size (px) | extent; every access is bounds-checked against it |
| `fd_wdwidth` | `(fd_w + 15) / 16` | 16-px words (`mem_wdwidth`, `raster.rs:193-198`); **row stride in pixels = `fd_wdwidth × 16`** (padded up, mirroring EmuTOS `bb_set`'s word alignment) |
| `fd_stand` | `0` (device) | chunky, not plane-sequential |
| `fd_nplanes` | `16` | truecolor depth (`CHUNKY_PLANES`, `raster.rs:61-67`); the interleaved-stride identity `bytes/line = fd_wdwidth × fd_nplanes / 8 × 16 = fd_wdwidth × 32` holds (`raster.rs:32-35`) |

- **Pixel offset** of `(col, row)` = `row × (fd_wdwidth × 16) + col` `UWORD`s
  — computed from the MFDB's *own* `fd_wdwidth`, **never** the screen
  stride. Reads/writes outside `fd_w × fd_h` (or the buffer) are ignored.
- `bb_save_words(w, h) = mem_wdwidth(w) * 8 * h` (`raster.rs:546-548`)
  counts `u32` words as a counting convenience while the pixels are
  16-bit; the byte size is `bb_save_words(w, h) * 4` = `stride16(w) * h *
  2` (`stride16(w) = ((w + 15) / 16) * 16`).
- This differs from the m68k planar `bb_save`, which uses `fd_stand = TRUE`
  (plane-sequential, 1 bit per plane word); on NewTOS the memory store is
  chunky RGB565, so a device-form chunky buffer is the natural, lossless
  save target.

### `vro_cpyfm` off-screen coverage (implemented, task #51)

All four source/destination combinations are implemented
(`tos-api/src/vdi/raster.rs`): **Screen↔Screen** (backend block-copy),
**Screen→Memory** (`bb_save`), **Memory→Screen** (`bb_restore`), and
**Memory↔Memory**, each with the full 16-op `raster_op` table on the raw
pixel word. Clipping applies **only** when the destination is the screen
(edge case 8). The Memory→Screen write region is flushed to the point of
coherency (`RasterBackend::flush_rect`) so the display DMA sees it. The
screen side reads/writes raw device words via
`RasterBackend::get_device_pixel` / `put_device_pixel` (added for this
task; distinct from the lossy PEL `read_pixel`). `vrt_cpyfm`'s
memory-mono-source → screen expand was already in place and is unchanged.

### AES consumer — `bb_save` / `bb_restore`

`tos_api::vdi::bb_save` / `bb_restore` are the Screen↔Memory copies with
`S_ONLY` (op 3) the AES uses (EmuTOS `gemgsxif.c` `bb_set`). The buffer
they copy into is the AES's (`tos-api/src/aes/gsxif.rs`, A57): **one**
persistent screen buffer, EmuTOS' `gl_tmp`/`gl_mlen`, taken once from the
BDOS pool when the AES goes graphic (`gsx_malloc`, `gemgsxif.c:108-114`,
from `sh_tographic`) and shared by the alert (`fm_alert`, `gemfmalt.c:363,
390`) and the drop-down (`menu_sr`, `gemmnlib.c:208-223`) — the same block
`wind_get(WF_SCREEN)` hands out. Its length is one screen in device pixels
(`bb_save_words(W, H) * 4`, `gsx_mcalc`), so `bb_set`'s "buffer too small"
height clip (`:334-343`) cannot trigger; a pop-up or nested submenu takes a
transient pool block per opening instead (`popup_blit`,
`gemmnext.c:136-215`). The desktop under a dismissed alert or a closed
drop-down is reinstated without a full repaint, device-exact (16-bit
RGB565 words off the screen, no pen lookup — A71).

**The copy runs with the mouse cursor hidden.** `bb_save`/`bb_restore` go
through `bb_set` (`gemgsxif.c:320-380`), which brackets the blit itself —
`gsx_moff()` at `:367`, `gsx_mon()` at `:378`, on the counted `gl_moff`
(`:440-455`) — so the save-under sprite never enters the saved image and is
re-shown with a fresh save-under after the restore. `fm_alert`
(`gemfmalt.c:363,390`) relies on this and carries no bracket of its own.
NewTOS places the bracket at the same level, inside
`tos_api::vdi::bb_save`/`bb_restore` (A66, 2026-09-18); a caller's own
bracket nests on the same counter.

### 1080p later

`fd_w`/`fd_h`/`fd_wdwidth` for the screen MFDB derive from the live
framebuffer geometry via the `RasterBackend`; a larger or smaller negotiated
mode flows through without changing the raster-op code — already true at
today's 1920×1080 default (`kernel/src/drivers/mod.rs:31-33`).
**Correction (A86, 2026-09-21):** `fd_nplanes = 16` and `ws_ncolors = 256`
are tied to the Falcon-truecolor plane/colour model
(`vdi_backend.rs:33,40`), independent of resolution — not the "16-colour
CLUT model" this note previously named (see the corrected "screen MFDB"
table above).
