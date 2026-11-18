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

### Memory → memory below nine planes: the interleaved device form (V19)

When **both** MFDBs are memory forms (`fd_addr != NULL`) with the same
`fd_nplanes` = *P*, 1 ≤ *P* ≤ 8, `cpy_raster` does not take the 16-bit
branch (`vdi_raster.c:1368`, only `plane_ct > 8`) but the planar blitter
(`fast_bit_blt`, `:1457`, `vdi/vdi_blit.S:794`; the C `bit_blt`, `:588-773`,
is the same operation). No screen is involved, so the result is the same on
every machine and screen mode — measured on the oracle in profiles
falcontc and tt with 0 differences
(`tosdump/cases/vdi-v18-raster.tsv` cases 24–41,
`measurements/measurement-2026-11-05-v18-t1/RUNINFO.TXT`). The bytes of the
destination buffer are the observable result.

**Form layout** (`setup_info`, `vdi_raster.c:919-940,959-960`): a device
form of *P* planes is **interleaved**. Row `y` starts at byte
`y × fd_wdwidth × P × 2` (`s_nxln = fd_wdwidth × s_nxwd`, `:923,938`); in a
row, the 16-pixel group `g` is *P* consecutive WORDs at byte `g × P × 2`
(`s_nxwd = fd_nplanes × 2`, `:922,937`); plane `p` is WORD `p` of its group
(`s_nxpl = d_nxpl = 2`, `:959-960`). Pixel `x` is bit `15 − (x mod 16)` of
its group's WORD (the end masks `0xffff >> (d_xmin % 16)` and
`~(0x7fff >> (d_xmax % 16))`, `:651-652`). The whole form is
`fd_h × fd_wdwidth × P` WORDs. *P* = 1 is a plain monochrome bitmap.

**The blit.** For every plane, every destination pixel of the block —
`b_wd × b_ht` from the source rectangle, placed at `PTSIN[4..5]` — becomes
`op(S, D)` of the 16-entry table, bit by bit; the bits of the edge WORDs
left and right of the block are written back unchanged (`mask_out`,
`:548-552`). The op is `op_tab[0]` on every plane (`fg_col = bg_col = 0`,
`:1363-1365,744-746`). A source bit at a pixel shifted against the
destination is taken from the neighbouring source WORD (skew, `:654-656`).

**Direction** (overlap): EmuTOS compares the two start addresses
(`bit_blt :660-669`, `fast_bit_blt` `vdi/vdi_blit.S:942-946`): it copies
bottom-up and right-to-left iff the source start lies below the
destination's in memory, or both are the same WORD and the source's
`x mod 16` is the smaller; otherwise top-down, left-to-right. Within one form
that rule is the same as the coordinates' (source above, or on the same row
left of, the destination ⇒ reverse), and the copy reads every source bit
before it is overwritten — the result of copying from an unmodified source.

**`PAT_FLAG`** is stripped and no pattern is applied (D-082 above):
`setup_pattern` (`:790-800`) fills only `p_nxpl`/`p_nxln`/`p_mask`, the
pattern pointer stays `NULL` (`:1345`). The `D-VDI-23` no-op does not apply
here — it belongs to the 16-bit routine. Measured: cases 39–41 of
`vdi-v18-raster.tsv` give the plain copy.

**No clipping and no bound in EmuTOS.** The destination is not the screen,
so `dont_clip` is used (`:957`); `setup_info` reads only `fd_addr`,
`fd_wdwidth` and `fd_nplanes` (`:919-940`) — neither `fd_w` nor `fd_h`
(in `vdi_raster.c` only `vr_trnfm` reads `fd_h`, `:198,300`; nothing there
reads `fd_w`) — so a block wider than the row runs on into the next row and
a block larger than its forms is copied past them.

**NewTOS** (`tos-api/src/vdi/raster.rs`, `vro_cpyfm` → `planar_mem_to_mem`)
reproduces the layout, the bitwise ops, the edge masks, the skew and the
overlap result. Three NewTOS bounds:

- **The form is the only limit** (V26, user decision 2026-11-05). Pixels are
  addressed as EmuTOS does: group `⌊x / 16⌋` of row `y` is the linear group
  `y × fd_wdwidth + ⌊x / 16⌋` for any `x` — `fd_w` is no limit, the padding
  bits past `fd_w` are copied like any other, a column past the row is one
  of a following row, a negative column one of a preceding row. Only a
  group outside the form's `fd_h × fd_wdwidth` groups (bytes `[0, fd_h ×
  fd_wdwidth × P × 2)` from `fd_addr`) is not written and reads as 0; past
  the form EmuTOS writes foreign memory, NewTOS does not
  (`docs/COMPATIBILITY.md`). Of the form, only the groups between the
  block's first and last are checked against the caller's rights
  (`Mfdb::bind_planar_block`) and touched.
- `fd_nplanes ≤ 0` on both sides copies nothing. In EmuTOS `plane_ct = 0`
  passes `setup_info` and the plane loop counts down past zero
  (`subq.w #1,PLANE_CT` / `jeq quit_blt`, `vdi/vdi_blit.S:1102`), an
  unbounded write.
- The block is anchored at the destination upper-left (`D-VDI-25`, as for
  16 planes); a well-formed caller is unaffected.

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

### Memory destination: planar below nine planes, chunky above (V28)

`cpy_raster` takes `vrt_cpyfm16` only for `plane_ct > 8`
(`vdi_raster.c:1432-1437`). A memory destination of 1..8 planes goes to the
planar blitter (`bit_blt`/`fast_bit_blt`, `:1446-1458`) on the interleaved
device form of § "Memory → memory below nine planes" — the layout, the
linear addressing, the skew, the end masks and the direction rule are the
ones given there. Per destination plane `p` the op is
`op_tab[((fg_col >> p) & 1) << 1 | ((bg_col >> p) & 1)]` (`:744-746`) with
the table and colours of the mode table above; the source is the one mono
plane under every destination plane (`s_nxpl = 0`, `:1387`). Per plane bit
this is the expansion of the mode table: a source 1-bit makes plane `p` bit
`p` of the foreground register (`MD_REPLACE`, `MD_TRANS`) or complements it
(`MD_XOR`), a 0-bit makes it bit `p` of the background register
(`MD_REPLACE`, `MD_ERASE`); otherwise the plane bit stays.

The registers are `MAP_COL[validate_color_index(INTIN[1/2])]`
(`:1390-1395`), EmuTOS' pen → hardware register map (`vdi_col.c:28-29`,
`:736-742`): `MAP_COL_ROM = {0, 15, 1, 2, 4, 6, 3, 5, 7, 8, 9, 10, 12, 14,
11, 13}` with `MAP_COL[1] = numcolors − 1`, pens 16..254 to themselves and
pen 255 to 15 (`EXTENDED_PALETTE`, `vdi_defs.h:20`). With `numcolors = 256`
(16 planes, `vdi_control.c:206`) pen 1 is register 255. Here the register's
bits are what lands in the form, so `MAP_COL` is observable; NewTOS'
pen-ordered pseudo-palette (the `MAP_COL` fold of § Palette) does not apply
to this path (`tos-api/src/vdi/raster.rs:685@map_col`). Measured on the
oracle, falcontc: `tosdump/cases/vdi-v18-t2-raster.tsv` cases 18–26 (1, 2, 4,
8 planes, all four modes, pens 1/0, 3, 0/2, 5/10, shifted block); on profile tt
cases 21 and 25 differ, because there `numcolors` is 16 (pen 1 → 15) — a
device parameter (`measurements/measurement-2026-11-06-v18-t2/RUNINFO.TXT`).

A memory destination of 9..16 planes is `vrt_cpyfm16`'s chunky form, one
WORD per pixel with a row of `fd_wdwidth × fd_nplanes` WORDs (`dst_width =
d_nxln / 2`, `:1249`; § "NewTOS memory-MFDB format", Row geometry). Cases
28–31.

NewTOS bounds as for `vro_cpyfm` (V26): nothing past the form's `fd_h` rows
is written; `fd_nplanes ≤ 0` draws nothing (EmuTOS' plane loop would count
down past zero, `vdi/vdi_blit.S:1102`).

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
   overlap-safe result. The 16-bit `vro_cpyfm16` decides the same way on its
   own start pointers (`form + y × width + x`): backwards — last row first,
   each row from its last pixel — iff the source's lies below the
   destination's (`:989-1002`). Where rows are shorter than the block (9..15
   planes) the walk decides which value lands; NewTOS walks as EmuTOS
   (`mem_to_mem`, V28; measured `vdi-v18-t2-raster.tsv` cases 40–46).
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

A memory MFDB (`fd_addr != 0`) with `fd_nplanes = 16` in NewTOS is a
**16-bit RGB565-chunky truecolor** buffer: **one `UWORD`
(`RRRRRGGGGGOBBBBB`, bit 5 =
`OVERLAY_BIT`: 0 on a pen-drawn pixel, 1 on a `v_clrwk`-cleared one)
per pixel**, not a 32-bpp `u32`/pixel bitmap and not a planar bitmap
(`tos-api/src/vdi/raster.rs:21-35@CHUNKY_PLANES`, module doc). (A memory form with
`fd_nplanes` 1..8 is the interleaved planar device form, copied by
`vro_cpyfm` memory → memory as EmuTOS does — § "Memory → memory below nine
planes" above, V19.) This is the app-visible
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
| `fd_w` / `fd_h` | region size (px) | `fd_w` is **not** a limit (EmuTOS does not read it); `fd_h` bounds the form: no access outside its `fd_h` rows of `fd_wdwidth × fd_nplanes` words (V26, V28, below) |
| `fd_wdwidth` | `(fd_w + 15) / 16` | 16-px words (`mem_wdwidth`, `raster.rs:193-198`); **row stride in pixels = `fd_wdwidth × 16`** (padded up, mirroring EmuTOS `bb_set`'s word alignment) |
| `fd_stand` | `0` (device) | chunky, not plane-sequential |
| `fd_nplanes` | `16` | truecolor depth (`CHUNKY_PLANES`, `raster.rs:61-67`); the interleaved-stride identity `bytes/line = fd_wdwidth × fd_nplanes / 8 × 16 = fd_wdwidth × 32` holds (`raster.rs:32-35`) |

- **Pixel offset** of `(col, row)` = `row × (fd_wdwidth × fd_nplanes) + col`
  `UWORD`s — `row × fd_wdwidth × 16 + col` at the 16 planes of this format —
  computed from the MFDB's *own* `fd_wdwidth` and `fd_nplanes`, **never** the
  screen stride, for any `col` (also past the row or negative). A pixel whose
  offset lies outside the form (below 0 or at `fd_h × fd_wdwidth ×
  fd_nplanes` and above) reads as 0 and is not written.
- **Row geometry and bound (V26, user decision 2026-11-05 "ja a": memory-form
  geometry like EmuTOS).** EmuTOS derives a memory form's row from
  `fd_wdwidth` alone: `setup_info` sets `s_nxln = fd_wdwidth × s_nxwd` and
  `d_nxln = fd_wdwidth × d_nxwd` with `s_nxwd`/`d_nxwd = fd_nplanes × 2`
  (`vdi_raster.c:919-923,933-938`), i.e. `fd_wdwidth × 32` bytes at 16
  planes and `fd_wdwidth × fd_nplanes × 2` bytes — fewer than the row's
  `fd_wdwidth × 16` pixels — for a chunky form of 9..15 planes (V28). `vro_cpyfm16` addresses pixel `(x, y)` as `form + y × width + x`
  and steps rows by `s_nxln / 2` / `d_nxln / 2` WORDs (`:986-990`,
  `src += src_width` / `dst += dst_width` per row, `:1016-1216`);
  `vrt_cpyfm16` its destination the same way (`:1249-1250`, `:1265-1318`)
  and its 1-plane source as WORD `y × src_width + (x >> 4)`, bit
  `0x8000 >> (x & 15)` (`:1241-1244`, `p++` on each 16th pixel). Neither
  `setup_info` nor the loops read `fd_w` or `fd_h` (in `vdi_raster.c` only
  `vr_trnfm` reads `fd_h`, `:198,300`). So a block wider than
  `fd_wdwidth × 16` runs on into the next row's WORDs, the padding columns
  between `fd_w` and `fd_wdwidth × 16` are read and written, and a block
  past `fd_h` runs past the form. NewTOS addresses every memory form that
  way (`vro_cpyfm` chunky paths, `vrt_cpyfm` into memory, and the planar
  form, § "Memory → memory below nine planes"; a chunky memory form of
  9..15 planes takes its row of `fd_wdwidth × fd_nplanes` WORDs as EmuTOS
  since V28, `tos-api/src/vdi/raster.rs:231@mem_stride_px`; measured
  `vdi-v18-t2-raster.tsv` cases 28–30, 33–37) with **one** limit: nothing
  outside the form's `fd_h` rows — bytes `[0, fd_h × fd_wdwidth ×
  fd_nplanes × 2)` from `fd_addr` — is read or written. (A 16-pixel row of
  a 9..15-plane form is longer than the form's row, so a block over the
  whole form runs past its end: EmuTOS writes the words there, NewTOS does
  not — cases 28, 29, 33, 34, 35, 37 differ only in those words.) Inside the form, only the pixels between
  the block's first and its last are checked against the caller's rights
  (`tos-api/src/vdi/raster.rs:187@bind_block`, hull
  `chunky_hull`) and touched, so the kernel never touches memory it has not
  checked. EmuTOS writes past the form there; NewTOS cuts
  (`docs/COMPATIBILITY.md`, kernel memory safety). Before V26 NewTOS stopped
  a row at `fd_w` and, with `fd_w > fd_wdwidth × 16`, ran into a row it had
  not checked (host tests `v26_row_bound_tests` in the same file).
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

---

## 32-plane setting: the device pixel (ADR-038 point 4 and 11, V63)

Spec gate of TASK/V63 (researcher, 2026-11-18). **Contract source:** ADR-038 in
`docs/MEMORY.md`; where EmuTOS has a 16-plane rule the rule is cited and the 32-plane
rule is its mechanical generalisation. EmuTOS has no path above 16 planes
(`references/emutos/vdi/vdi_defs.h:22`), so at 32 planes the ADR is the contract
(ADR-038 point 12) and the oracle cannot decide. At 16 planes nothing in this file above
changes. Labels: **[C]** read in the source cited, **[I]** inferred, **[D]** ADR-038
design choice. This section covers what V63 owns (the pixel word, its X byte, the clear
value, the logic operations, the read masks); the boundary to V64, V65 and V66 is
§4.

### 1. The rules V63 implements

| # | Rule at 32 planes | EmuTOS 16-plane rule it generalises | ADR-038 |
|---|---|---|---|
| R1 | The device pixel is one 32-bit integer `0xXXRRGGBB`, 8 bits per gun, **equal to the store word** (little-endian bytes B, G, R, X). **The X byte (bits 31..24) is unused.** | The device pixel is the 16-bit store word `RRRRRGGGGGOBBBBB`; bit 5 is the one bit that is no gun (`references/emutos/vdi/vdi_fill.c:26`, `references/emutos/vdi/vdi_defs.h:153-158`) **[C]** | point 4, amends ADR-024 point 3 (point 12) |
| R2 | **A pen writes X = 0:** a drawn pixel is `0x00RRGGBB`. | A pen word never carries `OVERLAY_BIT`: `(r<<11)\|(g<<6)\|b`, `references/emutos/vdi/vdi_col.c:601-614` **[C]**; fills, lines, text, marks and `vrt_cpyfm` read `palette[color]` (`references/emutos/vdi/vdi_line.c:460`, `references/emutos/vdi/vdi_textblit.c:765,1043`, `references/emutos/vdi/vdi_raster.c:1235`) **[C]** | point 4 ("pens write it 0") |
| R3 | **`v_clrwk` writes a constant, not a pen:** every byte `0xff` is allowed (the pixel `0xFFFFFFFF`), clip ignored, the whole visible screen. Because X is unused, `0x00FFFFFF` is equally conforming (§3). | `memset(v_bas_ad, 0xff, size)` under `TRUECOLOR_MODE`, the word `0xFFFF` (`references/emutos/vdi/vdi_control.c:610-618`) **[C]** | point 4 ("`v_clrwk` may fill every byte `0xff`") |
| R4 | **Logic operations act bitwise on all 32 bits**, X included: `WM_XOR` complements the whole stored pixel and ignores the pen; the sixteen `vro_cpyfm` operations are the bitwise table on 32 bits (`ALL_BLACK` writes `!0`, `ALL_WHITE` writes 0). X after an operation is noise (R5). | `*work = ~temp`, `*addr = ~*addr` (`references/emutos/vdi/vdi_line.c:494,501,1670,1730`), `*q = ~*q` (`references/emutos/vdi/vdi_textblit.c:806,1120`); the sixteen operations on `UWORD` (`references/emutos/vdi/vdi_raster.c:1007-1215`, `ALL_BLACK` `*q = 0xffff` at `:1213`) **[C]** | points 4, 5, 11 ("bitwise on all 32 bits") |
| R5 | **Every read ignores X.** The value a read path works with is `store & 0x00FFFFFF`. A cleared screen therefore reads as the pixel of a white pen (`0x00FFFFFF`): through `v_get_pixel` and through the flood fill, seed read included. | EmuTOS masks the overlay bit on the screen side of the flood fill (`references/emutos/vdi/vdi_fill.c:702-709,755`) and returns it unmasked from `pixelread` (`references/emutos/vdi/vdi_fill.c:671-681`) **[C]** | point 4 ("every read masks it off ... so a cleared screen reads as the white pen"; "X noise is ignored on read") |
| R6 | **At 16 planes none of R1-R5 applies**: the overlay bit stays, in the X byte as today, and the reads stay unmasked as above. Every mask of R5 is gated on the 32-plane setting (`backend.planes() == 32`). | the whole of ADR-024 | point 1, acceptance of TASK/V63 |

R5's seed-fill clause is **inferred [I]** and differs from the 16-plane behaviour:
EmuTOS reads the seed pixel unmasked (`search_color = pixelread(xleft, oldy)`,
`references/emutos/vdi/vdi_fill.c:972`) and compares it with masked screen words
(`:755,763`), so a seed pixel with the overlay bit set (a cleared background) never
equals and the fill draws nothing; NewTOS reproduces that at 16 planes
(`tos-api/src/vdi/contour.rs:474@pixelread`). The ADR says X noise is "ignored on read"
and a cleared screen "reads as the white pen", which taken for every read gives a seed
read that is masked too. ADR-038 does not name the seed read separately (Open points).

### 2. Where V63 acts: the sites, by file and function

Verdict per site: **change** = V63 edits it; **keep** = the code already meets the rule
(read in this run, not executed) and V63 adds only tests and comment text; **other
slice** = see §4.

| Site | Rule | Verdict |
|---|---|---|
| `kernel/src/vdi_backend.rs:282@pel_to_rgb`: pen to pixel for every draw path (`:606`, `:625`, `:629`, `:670`, `:704`, `:736`, `:775`, `:901`, all `Self::pel_to_rgb`): `word_to_device(ACTIVE_PALETTE[pen])` | R2 | **keep.** The palette words never have bit 5 set (`pack555`, `kernel/src/vdi_backend.rs:168@pack555`; the workstation side `tos-api/src/vdi/vwk.rs:262@pack_color16`), so `word_to_device` writes X = 0 **[C]**. The pen's 5-bit quantisation is V64's. |
| `kernel/src/vdi_backend.rs:408@xor_pixel`: `!v` on the whole `u32` | R4 | **keep.** Complements all 32 bits, X included **[C]**. |
| `kernel/src/raster_copy.rs:92@raster_op`: the sixteen operations on `u32`, used by screen-to-screen `copy_rect` | R4 | **keep** **[C]** (`:109`: `ALL_BLACK` is `!0`). |
| `tos-api/src/vdi/workstation.rs:431@v_clrwk` with `backend.clear_screen_word(0xFFFF)` (`:433`) and `kernel/src/vdi_backend.rs:558@clear_screen_word` (`fb.clear(self.rgb565_to_device(word))`) | R3 | **keep.** `word_to_device(0xFFFF)` is `0xFFFFFFFF` (`kernel/src/vdi_backend.rs:200@word_to_device`: X `0xFF` because bit 5 is set, guns `0xFF`) **[C]**, i.e. the "every byte `0xff`" fill of R3 with no change. The `u16` argument stays (ADR-038 point 11: the `u16` seam stays for the 16-plane setting). |
| `tos-api/src/vdi/contour.rs:162@pixelread`: `device_to_rgb565(get_device_pixel_wrapped(x, y))` for `planes() > 8` | R5, R6 | **change.** At 32 planes mask the raw store word with `0x00FFFFFF` before it is converted; at 16 planes it stays as it is (unmasked: `v_get_pixel` after `v_clrwk` is `0xFFFF`, `docs/api/vdi/workstation/03-v_clrwk.md`). This one function is the read of `v_get_pixel` (`tos-api/src/vdi/inquiry.rs:190@v_get_pixel`), of the flood-fill seed (`tos-api/src/vdi/contour.rs:474@pixelread`) and of every span probe (`tos-api/src/vdi/contour.rs:252@color`), so one mask covers all three. Whether the mask lives here or in a raster-backend method is the developer's (internal). |
| `tos-api/src/vdi/contour.rs:252@color` (`c & !OVERLAY_BIT`) | R5 | **keep.** After the change above bit 5 of the word is 0 at 32 planes; the mask is harmless there and needed at 16. |
| `tos-api/src/vdi/contour.rs:489@palette` (`search_color` from the physical workstation's pen word) | R2 | **keep** (pen word, X-free). |
| `tos-api/src/vdi/inquiry.rs:190@v_get_pixel` | R5 | **keep for V63** (it only forwards `pixelread`); its answer shape is V65 (§3, §4). |
| `kernel/src/vdi_backend.rs:531@read_pixel` (palette modes; reached only at `planes() <= 8` by `pixelread`, `tos-api/src/vdi/contour.rs:164@pixelread`, and directly by `tos-api/src/aes/menu.rs:2729@read_pixel`) | R5 | **keep.** It already masks `& 0x00FFFFFF` and compares through `word_to_device(..) & 0x00FFFFFF` **[C]**. |
| `tos-api/src/vdi/backend.rs:393@get_device_pixel_wrapped`, `kernel/src/vdi_backend.rs:910@get_device_pixel` | R1, R5 | **keep raw.** They return the whole store word including X; the mouse save-under holds raw words (`tos-api/src/vdi/mouse.rs`) and needs them. The mask is applied by the readers (`pixelread`), not here. |
| `kernel/src/hal/framebuffer.rs:61@pixels` and the module doc above it (`:14-27`): "X is owned by the VDI backend ... `OVERLAY_BIT`" | R1 | **comment only.** No code: the store is XRGB8888 with alpha "ignored" (`kernel/src/drivers/bcm2711/framebuffer.rs:60@ALPHA_MODE_IGNORED`) at both settings. The doc must say X carries the overlay bit at 16 planes and is unused at 32. |
| `kernel/src/vdi_backend.rs:200@word_to_device`, `:213@device_to_word` | R1 | **keep** (16-bit seam, §4). |

**Consequence for the size of the slice [I]:** apart from the read mask at 32 planes
(`pixelread`) and the comment text, the pixel-write sites already produce the V63
pixel, because the store at 16 planes is a bitwise expansion whose pen pixels have X = 0
and whose clear pixel has X = `0xFF`. The slice is chiefly the read mask plus the tests that pin R1-R6
at 32 planes (a test backend at `planes() == 32` whose store holds X noise).

### 3. What a pen's pixel is in V63, before V64, and what V63 leaves open

- **A pen's pixel before V64.** Pens still hold 5-bit pseudo-palette words
  (`kernel/src/vdi_backend.rs:145@ACTIVE_PALETTE`, `tos-api/src/vdi/vwk.rs:219@palette`) and
  `pel_to_rgb` expands them by gun replication `c8 = (c5 << 3) | (c5 >> 2)`
  (`kernel/src/vdi_backend.rs:200@word_to_device`, `expand5to8`). So in V63 a pen's pixel at 32
  planes is **`0x00` followed by the 16-plane pen's expanded guns**: the same 24 colour
  bits as at 16 planes, X = 0 **[C]**. White is `0x00FFFFFF` (5-bit 31 expands to
  `0xFF`) and equals the masked cleared screen **[C]**. V64 replaces the quantisation
  (`q = (c * 255 + 500) / 1000`, ADR-038 point 4) and nothing in V63 pre-empts it.
- **The clear value.** R3 allows `0xFFFFFFFF` (what the unchanged path writes) and
  `0x00FFFFFF`. No V63 read path can tell them apart (R5); the display shows white for
  both. The developer's test must not assert the X byte of a cleared pixel; it asserts what
  a masked read returns and the colour. A raw read of the store (for instance through
  `Physbase`) is outside the contract of X (R1).
- **`v_get_pixel` between V63 and V65 [I].** The answer shape is V65's (ADR-038
  point 7: `INTOUT[0]` high word, `INTOUT[1]` low word). V63 does not change it, so until V65 the
  call at 32 planes returns what the unchanged code returns for the masked read:
  `INTOUT[0] = 0`, `INTOUT[1] =` the 16-bit word `device_to_rgb565` makes of the masked pixel
  (`tos-api/src/vdi/inquiry.rs:201@set_intout`: truecolor branch for `planes() > 8`). In V63 that word is
  lossless for every pixel a VDI path wrote (5-bit-replicated guns, first bullet) and
  loses the low 3 bits of a gun otherwise. With the mask, a cleared screen answers `0xFFDF` =
  the pen-0 pixel of the default palette (not `0xFFFF` as at 16 planes). This interim answer
  is not a contract and V65 replaces it.

### 4. The boundary to V64, V65 and V66

| Item | Slice | Why not V63 |
|---|---|---|
| Pen quantisation with 8 bits per gun: `ACTIVE_PALETTE` (`kernel/src/vdi_backend.rs:145@ACTIVE_PALETTE`, `u16`), `vdi2fivebits`, `pack555` (`kernel/src/vdi_backend.rs:157@vdi2fivebits`, `:168@pack555`), `set_color` (`kernel/src/vdi_backend.rs:469@set_color`), `set_active_palette`, the workstation pseudo-palette `palette: [u16; 256]`, `set_color16`, `pack_color16` (`tos-api/src/vdi/vwk.rs:256@set_color16`), `vq_color` | **V64** | ADR-038 point 4 pen rule; TASK/V64.md scope |
| Width of the flood-fill compare and of `search_color` (`u16`, `tos-api/src/vdi/contour.rs:199@search_color`) and of `pixelread`'s return | **V64** at the latest | While every pixel is 5-bit-replicated, comparing the 16-bit projection equals comparing 24 bits. Once pens carry 8 bits per gun, two pens that differ in the low 3 bits of a gun project to the same word, so the compare must be on the masked 24-bit pixel (R5: the value a read works with is `store & 0x00FFFFFF`). TASK/V64.md does not name this; the lead decides (Open points). V63 may widen it already; nothing in V63 requires it. |
| `RasterBackend::read_pixel`'s reverse map through the 5-bit palette | **V64** | it follows the pens |
| `pel_to_device16` (`tos-api/src/vdi/backend.rs:138@pel_to_device16`, used by `vrt_cpyfm`'s memory destination, `tos-api/src/vdi/raster.rs:671@pel_to_device16`): the pen as a memory-MFDB word | **V65** (value from V64) | a memory-MFDB pixel; stays the 16-bit word at the 16-plane setting (ADR-038 point 11) |
| `device_to_rgb565`, `rgb565_to_device`, `read_row_rgb565`, `write_row_rgb565` and their users in `screen_to_mem`/`mem_to_screen`, `bb_save`/`bb_restore`, `blit_device_mem` (`tos-api/src/vdi/raster.rs:904@screen_to_mem`, `:956@mem_to_screen`, `:1324@bb_save`) | **V65** | memory MFDB, save buffers. Until V65 they still turn bit 31 of a store word into bit 5 of the 16-bit word, so a memory copy of a cleared screen still carries the overlay bit; whether a 32-plane read masks X there is V65's |
| `v_get_pixel` answer shape (high word first) | **V65** | ADR-038 point 7 |
| `CHUNKY_PLANES`, the plane cap of `vro_cpyfm`/`vrt_cpyfm`, `vr_trnfm` standard form, 16-plane memory MFDB refused at 32 | **V65** | ADR-038 points 5, 6; `docs/api/vdi/workstation/100-v_opnvwk.md` §2 |
| The `vq_scrninfo` table and `[13] = 8`, `[96..103] = 24..31` (X unused as seen by a program) | **V66** | ADR-038 point 8 |
| VT52 cell writer (`tos-api/src/vdi/escape.rs:273@write_row_rgb565`) | not named by V64/V65/V66 | uses the 16-bit row seam; out of scope here (Open points) |

**Can V63 be built without a part of V64 or V65?** Yes. The one place where it touches
a V65 item is `v_get_pixel`, and there the slice changes only the read mask under
`pixelread` and leaves the answer shape as it is (§3, interim). It touches no V64 item.

### Open points

- **Inferred (R5, seed read):** the flood-fill seed read is masked at 32 planes although
  EmuTOS reads it unmasked at 16 planes (`references/emutos/vdi/vdi_fill.c:972`), so a
  `v_contourfill` with `INTIN[0] < 0` on a freshly cleared screen fills at 32 planes and
  draws nothing at 16. ADR-038 point 4 names `v_get_pixel` and "flood fill" as reads that
  mask, without separating the seed. The lead confirms or reverses; the code change is the
  same one `pixelread` mask either way unless the seed is read raw.
- **Inferred (§3):** the interim answer of `v_get_pixel` at 32 planes between V63 and V65.
  ADR-038 gives the final shape only. If the lead prefers the final shape in V63, point 7 moves
  from V65 into this slice.
- **Inferred (R3):** `0xFFFFFFFF` or `0x00FFFFFF` as the cleared pixel is left to the
  developer by the ADR ("may fill"); both read as the white pen.
- **For V64's ticket:** the flood-fill compare width (§4) is not in `TASK/V64.md`.
- **Out of scope observation:** the VT52 cell writer (`tos-api/src/vdi/escape.rs:273@write_row_rgb565`)
  uses the 16-bit seam and has no slice in V63-V66; the AES `tos-api/src/aes/menu.rs:2729@read_pixel`
  call reverse-maps through the palette (affected by V64's palette width).
- **Not executed:** all "keep" verdicts are reading of the code, not a run.

---

## 32-plane setting: pens, and every place a pen becomes a pixel (ADR-038 point 4, V64)

Spec gate of TASK/V64 (researcher, 2026-11-18). **Contract source:** ADR-038 in
`docs/MEMORY.md` (points 4 and 11); at 32 planes the ADR is the contract, the oracle
cannot decide (ADR-038 point 12). Each rule below names the EmuTOS 16-plane rule it
generalises. Labels: **[C]** read in the source cited, **[I]** inferred, **[D]** ADR-038
design choice, **[calc]** computed in this run by arithmetic over the formula (not a
run of the kernel). The pixel word, X byte, clear value and read mask are V63's
(section "32-plane setting: the device pixel" above, R1-R6); the memory-MFDB forms and
the answer shape of `v_get_pixel` are V65.

### 1. The 16-plane path, which this slice does not touch (ADR-038 point 11)

| What | EmuTOS (16 planes) | NewTOS (stays as it is under `planes() == 16`) |
|---|---|---|
| pen to 5-bit gun, `(c*31+500)/1000` | `vdi2fivebits`, `references/emutos/vdi/vdi_col.c:588-591` [C] | `tos-api/src/vdi/vwk.rs:239@vdi2fivebits`; `kernel/src/vdi_backend.rs:161@vdi2fivebits` |
| pen to pseudo-palette word `(r<<11)\|(g<<6)\|b` | `set_color16`, `references/emutos/vdi/vdi_col.c:601-614` [C] | `tos-api/src/vdi/vwk.rs:256@set_color16`, `:262@pack_color16`; `kernel/src/vdi_backend.rs:168@pack555` |
| `vs_color` truecolor path | `vs_color16`, `references/emutos/vdi/vdi_col.c:620-649`, selected by `TRUECOLOR_MODE` at `:661-666` [C] | `tos-api/src/vdi/attributes.rs:492@vs_color` |
| 5-bit gun to `vq_color` value `((c&0x1f)*1000+16)/31` | `fivebits2vdi`, `references/emutos/vdi/vdi_col.c:804-807` [C] | `tos-api/src/vdi/vwk.rs:246@fivebits2vdi`, `:273@actual16` |
| `vq_color` truecolor path | `vq_color16`, `references/emutos/vdi/vdi_col.c:813-838`, selected at `:862-868`, after the range check `:854-860` [C] | `tos-api/src/vdi/inquiry.rs:93@vq_color` |
| per-workstation seed of the tables | `init_wk`, `references/emutos/vdi/vdi_control.c:354-363` (`l < 255`) [C] | `tos-api/src/vdi/workstation.rs:250@planes` |

At 16 planes every line of the right column keeps its behaviour: the 5-bit functions
are not edited, only a 32-plane branch is added beside them **[I]**. The truecolor gate
`planes() > 8` stays true at 32 (ADR-038 point 3; `references/emutos/vdi/vdi_defs.h:22`),
so the 32-plane rule is selected by `planes() == 32` inside that branch, as V63's
rule R6 does for the masks.

### 2. The rules at 32 planes

| # | Rule | EmuTOS rule it generalises | ADR-038 |
|---|---|---|---|
| P1 | **`vs_color` request.** Index range, silent no-op outside `0 <= idx < numcolors` (256), is unchanged. The request `(INTIN[1..3])` is stored **raw and unclamped** in the calling workstation's `req_col`; only the copy that makes the pixel is clamped to 0..1000. | `vs_color16` stores `*intin` into `req_col` before the clamp, `references/emutos/vdi/vdi_col.c:634-646` [C] | point 4 ("last-requested values") |
| P2 | **Quantisation.** `q = (c * 255 + 500) / 1000` per gun, `c` clamped to 0..1000, integer (truncating) division. [calc] `q(0) = 0`, `q(1000) = 255`, `q(1) = 0`, `q(2) = 1`, `q(500) = 128`, `q(999) = 255`; `q` is monotone non-decreasing over 0..1000. | `divu(col*31+500, 1000)`, `references/emutos/vdi/vdi_col.c:588-591` [C], with 255 for 31 | point 4 **[I]** in the ADR |
| P3 | **The pen's pixel** is `0x00RRGGBB` with `R = q(r)`, `G = q(g)`, `B = q(b)`; X = 0 (V63 R2). There is no green LSB and no overlay position: all 24 bits are colour. | `(r<<11)\|(g<<6)\|b`, bit 5 never set, `references/emutos/vdi/vdi_col.c:613` and the comment at `:599` [C] | point 4 |
| P4 | **`vq_color`, "set" value (`INTIN[1] == 0`).** The last-requested triple of the calling workstation, raw and unclamped, exactly as at 16 (a request of `-50, 1500, 0` comes back unchanged). Range check as today: an index outside 0..255 gives `INTOUT[0] = -1` and `INTOUT[1..3] = 0`. | `vq_color16` flag 0, `references/emutos/vdi/vdi_col.c:821-826` [C]; range check `:854-860` [C] | point 4 |
| P5 | **`vq_color`, "realised" value (`INTIN[1] != 0`).** Per gun `a = (q * 1000 + 127) / 255`, `q` the gun byte of the pen's stored pixel. [calc] `a(0) = 0`, `a(255) = 1000`, `a(1) = 4`, `a(128) = 502`, `a(254) = 996`; `q(a(q)) = q` for every `q` 0..255; `\|a(q(c)) - c\| <= 2` for every `c` 0..1000. | `fivebits2vdi`, `references/emutos/vdi/vdi_col.c:804-807`, and the read at `:832-837` [C] | point 4 **[I]** |
| P6 | **The default palette at 32 planes.** The 256 pens, their default *requested* triples (VDI 0..1000) and the default pen set are those of 16 planes: pens 0..15 `VIDEL_PALETTE1`, pens 16..255 `VIDEL_PALETTE2` (`kernel/src/vdi_backend.rs:56@VIDEL_PALETTE1`, `:70@VIDEL_PALETTE2`, seeded `:236@ensure_palette_init`), verbatim EmuTOS `videl_palette1`/`videl_palette2` (`references/emutos/vdi/vdi_col.c:130-199`) [C]. Each workstation copies them at open and builds the pens of 0..254 from them, **pen 255 left at pixel 0** (`tos-api/src/vdi/workstation.rs:250@planes`, EmuTOS `l < 255`, `references/emutos/vdi/vdi_control.c:360`) [C]. The pixel of a default pen is `q` of the triple: pen 0 `0x00FFFFFF`, pen 1 `0x00000000`, pen 2 `0x00FF0000`, pen 8 (733) `0x00BBBBBB`, pen 9 (533) `0x00888888`, pen 10 (667,0,0) `0x00AA0000` [calc]. | `init_wk` seeding, `references/emutos/vdi/vdi_control.c:354-363` [C] | point 3 ("the 256 pens, their default values and the default pen set stay") |
| P7 | **Default pens are exactly representable.** Every component of the 256 default triples is one of the 16 values 0, 67, 133, 200, 267, 333, 400, 467, 533, 600, 667, 733, 800, 867, 933, 1000, and [calc] `a(q(c)) = c` for each. So at 32 planes `vq_color(flag != 0)` of an untouched pen equals its flag 0 answer, pen 255 included (requested `(0,0,0)`, pixel 0). At 16 planes it does not (`a5(q5(67)) = 65`, `a5(q5(133)) = 129`, ...) [calc]. | the 16-plane values are the existing behaviour | point 4 |
| P8 | **Which workstation.** A draw resolves a pen through the *drawing* workstation's table (`tos-api/src/vdi/mod.rs:497@set_active_palette`); `vq_color` and `vs_color` read and write the *calling* workstation's own table; the flood-fill search colour for a pen index reads the *physical* workstation's table (F3 below). All three as at 16. | `references/emutos/vdi/vdi_line.c:459-460`; `vdi_col.c:620-649,813-838`; `vdi_fill.c:978-985` [C] | point 4 |
| P9 | **Against 16 planes (acceptance (ii)).** Project a 32-plane pen pixel to 16 bits by the top 5 bits of each gun (`kernel/src/vdi_backend.rs:213@device_to_word`). [calc] For every request `c` 0..1000, `\|(c*31+500)/1000 - ((c*255+500)/1000 >> 3)\| <= 1`; the maximum 1 occurs (`c = 17`: 1 against 0). Compared as 8-bit values, a 16-plane pen pixel (5-bit replicated) and the 32-plane one differ by at most 5 [calc]. The overlay bit (word bit 5) is not a gun and differs on a cleared screen by V63's design (R5/R6): compare guns, not words. | none (own design) | point 13 (ii) |

### 3. Where a pen index becomes a device pixel: the change set

Verdict: **V64** = this slice edits it; **V65** = another slice; **decide** = no slice
is named for it (Open points).

| Site | Today (read, not run) | At 32 planes | Slice |
|---|---|---|---|
| `tos-api/src/vdi/vwk.rs:219@palette` `VwkExt.palette: [u16; 256]` | 5-bit pseudo-palette word per pen | must hold the 24-bit pen pixel (or three gun bytes); the width is the developer's | V64 |
| `tos-api/src/vdi/vwk.rs:239@vdi2fivebits`, `:246@fivebits2vdi`, `:256@set_color16`, `:262@pack_color16`, `:273@actual16`, free `:287@pack_color16` | 5-bit quantise / pack / read back | 8-bit counterparts per P2, P3, P5; the 16-plane ones stay | V64 |
| `tos-api/src/vdi/attributes.rs:492@vs_color` | raw into `req_col`, clamped copy into `set_color16` | P1 | V64 |
| `tos-api/src/vdi/inquiry.rs:93@vq_color` | flag 0 `req_col`, flag != 0 `actual16` | P4, P5 | V64 |
| `tos-api/src/vdi/workstation.rs:250@planes` `init_wk` seed | `set_color16` for pens 0..254 from `query_color` | seed with the 8-bit rule, pen 255 stays 0 (P6) | V64 |
| `tos-api/src/vdi/backend.rs:126@set_active_palette` (`&[u16; 256]`) and its caller `tos-api/src/vdi/mod.rs:497@set_active_palette` | hands the drawing workstation's table to the backend before each op | the signature cannot carry 8-bit pens; six test doubles implement it too (`tos-api/src/vdi/contour.rs:587@set_active_palette`, `tos-api/src/vdi/raster.rs:1801@set_active_palette`, `tos-api/src/vdi/text.rs:1520@set_active_palette`, `tos-api/src/aes/window.rs:6546@set_active_palette`, `:6908@set_active_palette`, `tos-api/src/aes/menu.rs:2815@set_active_palette`) | V64 |
| `kernel/src/vdi_backend.rs:145@ACTIVE_PALETTE` (`AtomicU16`), `:236@ensure_palette_init`, `:161@vdi2fivebits`, `:168@pack555` | active table and lazily seeded default, 5-bit | 8-bit pen pixels, same defaults (P6) | V64 |
| `kernel/src/vdi_backend.rs:282@pel_to_rgb` (every fill, line, text, mark path) | `word_to_device(ACTIVE_PALETTE[pen])` (5-bit guns replicated) | the pen's stored pixel, X = 0 | V64 |
| `kernel/src/vdi_backend.rs:469@set_color` (device-global `REQ_COL` and `ACTIVE_PALETTE`) | `pack555` | only caller: the palette-mode branch of `vs_color` (`tos-api/src/vdi/attributes.rs:521@set_color`), unreachable at `planes() > 8`; keep the two tables consistent | V64 |
| `kernel/src/vdi_backend.rs:485@query_color`, `:153@REQ_COL` (10 bits per gun, `r<<20\|g<<10\|b`) | requested triples, 0..1000 | **no change** (a request needs no more than 10 bits) | none |
| `kernel/src/vdi_backend.rs:531@read_pixel` and trait `tos-api/src/vdi/backend.rs:177@read_pixel` | masks the pixel to 24 bits, then the nearest active pen by squared RGB distance, first minimum wins (`d < best_d`) | compare against the 24-bit pen pixels; same tie rule | V64 ("follows the pens", TASK/V64.md) |
| `tos-api/src/vdi/contour.rs:164@pixelread`, `:202@search_color`, `:255@color`, `:477@pixelread`, `:492@palette` | `u16` words | section 4 | V64 |
| `tos-api/src/vdi/escape.rs:234@word` (VT52 cell colours: `phys.ext.palette[...]`) with `:257@cell_xfer` and the 16-bit row writer | the console's two colours are 16-bit pseudo-palette words | at 32 the cell colour is a pen's pixel and `ext.palette` is no longer a `u16` table, so this site must change or break | **decide** |
| `kernel/src/vdi_backend.rs:501@pel_to_device16` and its user `tos-api/src/vdi/raster.rs:671@pel_to_device16` (`vrt_cpyfm` into a memory MFDB) | the pen as a 16-bit memory word | the pen's 32-bit pixel as a memory pixel; the value comes from V64 | V65 |
| `tos-api/src/aes/resource.rs:3075@live` (`CiconDevice` palette built by `pack_color16`) | colour-icon device form from 16-bit pen words | follows the MFDB form of ADR-038 point 10 | V65 |
| `tos-api/src/aes/menu.rs:2729@read_pixel` (`selftest_menu_sr_residue`), `kernel/src/selftest_raster.rs:38@read_pixel` | background compare through the reverse map | only self-tests; no production caller found by grep | none |
| `v_clrwk` (`kernel/src/vdi_backend.rs:558@clear_screen_word`), `xor_pixel` | constants / bitwise, no pen | V63 | V63 |

**Pixel to pen.** EmuTOS has none in truecolor: `v_get_pixel` answers `INTOUT[0] = 0`,
`INTOUT[1] =` the word, and `REV_MAP_COL` is used only on the palette path
(`references/emutos/vdi/vdi_fill.c:1104-1114`) [C]. So at 16 and 32 planes `v_get_pixel`
returns no pen index; the only reverse map is `read_pixel` above, reached at more than
8 planes only by the self-tests (grep over `tos-api/src`, `kernel/src`, `programs`).
The flood fill's search colour does not use it either (section 4).

### 4. Flood fill at 32 planes (the items moved from V63)

Sites: `tos-api/src/vdi/contour.rs:164@pixelread`, `:202@search_color`, `:255@color`,
`:477@pixelread`, `:492@palette`. EmuTOS reference: `pixelread` `references/emutos/vdi/vdi_fill.c:671-681`,
`end_pts16` `:746-767`, `contourfill` `:948-1070` [C].

| # | Rule at 32 planes | At 16 planes (unchanged) |
|---|---|---|
| F1 | `pixelread` returns the **masked 24-bit pixel** `store & 0x00FFFFFF` as a 32-bit value; no 16-bit projection on this path. This is V63 R5 with the width widened. | the 16-bit word, whole (`vdi_fill.c:671-681`), `u16` |
| F2 | The compare in `end_pts`/`search_to_*` is **equality of those 24-bit values**. The `& !OVERLAY_BIT` of `tos-api/src/vdi/contour.rs:255@color` (`OVERLAY_BIT = 0x0020`, `:80@OVERLAY_BIT`) **must not be applied**: in a 24-bit pixel `0x20` is a bit of blue, and masking it would make two pens that differ only in blue bit 5 compare equal **[I]**. | the mask stays (`vdi_fill.c:709,733,755`) |
| F3 | `search_color`: for `INTIN[0] < 0` the masked pixel at the seed (`pixelread`, V63 R5); for a pen `INTIN[0] >= 0` the **pen's pixel `0x00RRGGBB` from the physical workstation's table** (`MAP_COL` folded, `tos-api/src/vdi/contour.rs:492@palette`); `INTIN[0] >= numcolors` and a seed outside the clip stay silent no-ops. | `phys_work.ext->palette[search_color]`, `vdi_fill.c:978-985`; guards `:965-967,976-977` [C] |
| F4 | The span draw is unchanged: pen `fill_color` through `pel_to_rgb`. The queue sentinels (`EMPTY = 0xFFFF`, `DOWN_FLAG = 0x8000`, `tos-api/src/vdi/contour.rs:85@EMPTY`) are `y` values of the seed queue, not pixels, and stay 16-bit (`vdi_fill.c:29-30`). | same |
| F5 | `v_get_pixel` in V64 keeps the interim of V63 (`_color-pixel-input.md`): `INTOUT[0] = 0`, `INTOUT[1] =` the 16-bit projection (P9) of the masked pixel, now computed from the 24-bit value. The final shape (high word, low word) is V65's **[I]**. | `INTOUT[0] = 0`, `INTOUT[1] = pel` (`vdi_fill.c:1104-1110`) |

**Test pens for "stops at a pen that differs only in the low 3 bits of a gun" [calc].**
Requests `(17, 0, 0)` and `(25, 0, 0)` give the same 16-plane word (5-bit red 1 for both:
`(17*31+500)/1000 = 1`, `(25*31+500)/1000 = 1`), so at 16 planes the two pens are one
colour; at 32 planes the pixels are `0x00040000` and `0x00060000` (`q(17) = 4`, `q(25) = 6`),
which differ in bits 0..2 of red only. A boundary fill with the first pen as boundary
(`INTIN[0] = pen`) and the second as the interior, or a fill-while-same
(`INTIN[0] = -1`) over a region of the first pen bordered by the second, separates them
at 32 planes and does not at 16.

### Open points

- **Question (V64 or other slice): VT52 console colours.** `tos-api/src/vdi/escape.rs:234@word`
  reads `phys.ext.palette[...]` as a 16-bit word and writes it through the 16-bit row seam
  (`tos-api/src/vdi/escape.rs:257@cell_xfer`). It is a pen-to-pixel site, so changing
  `VwkExt.palette` breaks it, but no ticket V63-V66 names it (V63's section 4 lists it as "not
  named"). My reading: V64 carries the colour source (the 8-bit pen pixel), V65 the row writer.
  Needs the lead's decision.
- **Inferred:** P2/P5 are ADR-038's own formulas, labelled **[I]** by the ADR. The `+127` of
  P5 is `floor(255/2)`; EmuTOS's `+16` is `ceil(31/2)`; fVDI's cited line
  (`references/fvdi/fvdi/drivers/aranym/palette.c:195`) uses `1 << (bits-1)` = 128. [calc] `+127`
  and `+128` give identical results for every `q` 0..255, so the contract is the same either way.
- **Inferred:** F2 (the overlay-bit mask must not run on a 24-bit pixel) and F5 (the interim
  `v_get_pixel` answer stays in V64) are my readings of ADR-038 point 4 and of V63's split;
  the lead confirms F5 (the alternative is to move ADR-038 point 7 into V64).
- **Not executed:** every "today" column is reading of the code; every **[calc]** is arithmetic
  over the formulas, not a run of NewTOS. Nothing was measured; at 32 planes the oracle cannot
  measure.
- **Not traced:** whether any XBIOS palette call (`Setcolor`, `Setpalette`, `VsetRGB`) reaches the
  VDI palette; the grep over `tos-api/src`, `kernel/src` and `programs` found no caller of
  `RasterBackend::set_color` other than `vs_color`'s palette branch.
