# `vq_scrninfo` and the `EdDI` cookie at the 32-plane setting (ADR-038 point 8, V66)

Spec gate of TASK/V66 (researcher, 2026-11-18). Companion of
[`eddi-vq_scrninfo.md`](eddi-vq_scrninfo.md) (the 16-plane spec, approved 2026-10-02, still
German and untouched here: it is translated when it is next edited, TASK/V62.md open point).
This file is English and holds **only** what the 32-plane setting adds or changes; everything
not stated here is as in that file.

**Contract source.** ADR-038 in `docs/MEMORY.md` (accepted 2026-11-18), point 8 for the
table, points 3, 4, 7, 9, 11, 12 and 13 (i) around it. EmuTOS has **no EdDI at all**:
`grep -rIn "EdDI\|scrninfo\|0x45644449" references/emutos/` finds nothing (checked in this
run, pin `1a7484ab`), so there is no EmuTOS rule for the table at any depth, 16 included. At 32
planes the ADR is the contract (ADR-038 point 12); the oracle cannot decide. Labels: **[C]**
read in the source cited in this run, **[I]** inferred, **[D]** ADR-038 design choice (no
source defines the value), **[N]** NewTOS contract per ADR-038 where EmuTOS has no rule.

## 1. Where the EdDI word layout comes from (item 1, "say which source you used")

| Source | In the repository? | Used for |
|---|---|---|
| gemlib `n_vq_scrninfo.c`, Doxygen comment: the word layout `work_out[0..14]`, the table ranges 16..271, the bit-order text per depth, three worked examples | the file `sdk/gemlib/gemlib-0.44.0/n_vq_scrninfo.c` is extracted from the tracked tarball `sdk/gemlib/gemlib-0.44.0.tar.xz` into a gitignored directory (present in the main tree, read there this run) | the layout of every word below **[C]** |
| tos.hyp `vq_scrninfo.ui`, NVDI 2.50 `Neue_Fkt.txt` | web page / `Applications/z-tools/...` (gitignored, local); read in the 2026-10-02 research, **not re-read in this run** | cross-reference only; `eddi-vq_scrninfo.md` §1 and §3.4 record that they agree with gemlib |
| fVDI `references/fvdi/fvdi/drivers/aranym/spec.c`, `references/fvdi/fvdi/drivers/common/init.c` | yes (`references/fvdi`) | plausibility witness for the 32-bit values (ADR-038 point 12) |

What the gemlib text says for **32 planes** [C]
(`sdk/gemlib/gemlib-0.44.0/n_vq_scrninfo.c:70-80`): "16777216 colors (32 Planes)"; bit order
`0` = "usual bit order, 8 bits overlay or unused, 8 bits red, 8 bits green, 8 bits blue
(`xxxxxxxx-rrrrrrrr-gggggggg-bbbbbbbb`)"; bit order `7` = "swapped bytes (intel order)
(`bbbbbbbb-gggggggg-rrrrrrrr-xxxxxxxx`)". The worked examples are only 256 colours (Falcon),
HiColor (Falcon) and Crazy Dots HiColor (`sdk/gemlib/gemlib-0.44.0/n_vq_scrninfo.c:101-241`); **there is no worked
example for 24 or 32 planes** [C], and no real NVDI or Nova table is available
(`eddi-vq_scrninfo.md` §6.1). So the 32-plane values below are the ADR's choice (point 8),
with fVDI's `32` "usual" row as witness: red 16..23, green 8..15, blue 0..7, `org 1`, no
alpha, genlock or unused bits reported (`references/fvdi/fvdi/drivers/aranym/spec.c:25-27,75`)
and the colour count `1 << (r+g+b bits)` as a high/low word pair
(`references/fvdi/fvdi/drivers/common/init.c:123-128`) [C].

**What is not sourced:** the meaning of word `[15]` (undocumented; gemlib lists nothing;
fVDI sets `dummy2 = 0`, `references/fvdi/fvdi/drivers/common/init.c:134`) is treated as 0
as at 16 planes; the behaviour of a program that reads `[5..7]` as 0 (open point 2).

## 2. The full answer at 32 planes, `INTOUT[0..=272]` (item 1)

The call is unchanged: opcode 102, `CONTRL[5] == 1`, `INTIN[0] == 2`
(`tos-api/src/vdi/eddi.rs:79@is_scrninfo`); `CONTRL[4] = 272`, `CONTRL[2] = 0`, PTSOUT
untouched (`tos-api/src/vdi/eddi.rs:156@vq_scrninfo`) **[C]**. The 273 words:

| Index | At 32 planes | At 16 planes (unchanged, `tos-api/src/vdi/eddi.rs:110@scrninfo_table`) | Basis of the 32-plane value |
|---|---|---|---|
| `[0]` format | `2` packed pixels | 2 | ADR-038 point 8; gemlib `:13-17` for the meaning **[D]** |
| `[1]` CLUT | `2` software ("pseudo") CLUT | 2 | ADR-038 point 8; agrees with `INQ_TAB[5] = 0` (section 4) **[D]** |
| `[2]` planes | `32` | 16 | ADR-038 point 8; equals `backend.planes()` and `INQ_TAB[4]` (section 4) **[D]** |
| `[3]` colours, high word | `0x0100` (256) | 0 | ADR-038 point 8: 2^24 as a LONG, high word first; the word order is the existing convention (`tos-api/src/vdi/eddi.rs:110@scrninfo_table`, `w[3] = colours >> 16`), fVDI `references/fvdi/fvdi/drivers/common/init.c:127-128` **[D]** |
| `[4]` colours, low word | `0` | `0x8000` | same; LONG `[3]:[4]` = `0x01000000` = 16 777 216 = `1 << (8+8+8)` |
| `[5]` line width in bytes | `0` | 0 | ADR-038 point 8 "as at 16" (E5 of `eddi-vq_scrninfo.md` §7: no accessible line width) **[D]**; see open point 2 |
| `[6]`, `[7]` bitmap address | `0`, `0` | 0, 0 | same; a 64-bit address does not fit two words **[D]** |
| `[8]`, `[9]`, `[10]` bits red, green, blue | `8`, `8`, `8` | 5, 5, 5 | ADR-038 point 4 (8 bits per gun, owner choice O2) |
| `[11]` alpha bits | `0` | 0 | ADR-038 point 4 (X unused, O1); the VideoCore is told alpha is ignored (`kernel/src/drivers/bcm2711/framebuffer.rs:60@ALPHA_MODE_IGNORED`) **[C for the firmware mode, D for the word]** |
| `[12]` genlock bits | `0` | 1 | ADR-038 point 8 / V61 O1 option A: X is unused, not an overlay carrier (V63 masks X on every read) **[D]** |
| `[13]` unused bits | `8` | 0 | ADR-038 point 8: the X byte **[D]** |
| `[14]` bit order | `1` (usual) | `2` (Falcon order) | ADR-038 point 8; number convention in section 3 **[D]** |
| `[15]` | `0` | 0 | undocumented, as at 16 |
| `[16..23]` red bit numbers, LSB first | `16, 17, 18, 19, 20, 21, 22, 23` | `11..15` in `[16..20]` | ADR-038 point 8 **[D]**; fVDI `r_32` |
| `[24..31]` | `-1` | -1 | not used |
| `[32..39]` green | `8, 9, 10, 11, 12, 13, 14, 15` | `6..10` in `[32..36]` | ADR-038 point 8; fVDI `g_32` |
| `[40..47]` | `-1` | -1 | |
| `[48..55]` blue | `0, 1, 2, 3, 4, 5, 6, 7` | `0..4` in `[48..52]` | ADR-038 point 8; fVDI `b_32` |
| `[56..63]` | `-1` | -1 | |
| `[64..79]` alpha | all `-1` | all -1 | `[11] = 0` |
| `[80..95]` genlock | all `-1` | `[80] = 5`, rest -1 | `[12] = 0` |
| `[96..103]` unused bits | `24, 25, 26, 27, 28, 29, 30, 31` | all -1 | ADR-038 point 8: the X byte |
| `[104..127]` | `-1` | -1 | |
| `[128..272]` | `0` | 0 | reserved (gemlib `:95`) |

That is 4 tables of 8 numbered entries = 32 numbers, 80 `-1` entries in `[16..127]`.

The 16-plane column is the code as it stands (`eddi.rs:110@scrninfo_table`, read this run),
not a claim about the Enhancer; the measurement against the real EdDI at 16 planes is
`measurements/measurement-2026-10-02-scrninfo/hatari-falcon-tc-enhancer/SCRNINFO.TXT`
(cited by `eddi.rs`'s header), outside this slice.

## 3. The pixel format as the store really is (item 1)

- **The store.** The framebuffer is 32 bpp, the pixel `0xXX_RR_GG_BB` packed by
  `hal::framebuffer::rgb`, "i.e. little-endian memory bytes `[B, G, R, X]` — blue at the
  lowest address" (`kernel/src/drivers/bcm2711/framebuffer.rs:34-38@DEPTH_BPP`; one `u32`
  per pixel, `kernel/src/hal/framebuffer.rs:61@pixels`) **[C]**. AArch64 is little-endian
  (`.cargo/config.toml` builds `aarch64-unknown-none-softfloat`) **[C]**.
- **The device pixel.** At the 32 setting the device pixel is the store word, one 32-bit
  integer `0xXXRRGGBB`: R bits 23..16, G 15..8, B 7..0, X 31..24 unused, pens write X as 0
  (ADR-038 point 4; the memory form of a 32-plane MFDB holds the same integer, native
  endian, ADR-038 point 5, `docs/api/vdi/raster/_raster.md` §"32-plane setting: memory
  forms ..." section 3) **[D]**.
- **How the bit numbers count.** The numbers count the bits **of that native 32-bit integer
  as the CPU reads it from the memory form**, bit 0 the least significant. This is the
  convention of the existing 16-plane table (E3 of the 2026-10-02 approval: a program and
  `vro_cpyfm` read the same native word, "no byte-order difference to report and bit 7
  stays clear", `tos-api/src/vdi/eddi.rs:1-30@E3`, module header) **[C]**; at 32 planes it is
  applied the same way **[D]**, so `[14] = 1` and **not** `0x81`.
- **Why that is not `0x81` [I].** Read literally as bytes at increasing addresses, the memory
  of this machine is `B, G, R, X`; NVDI's `bit 7` row for 32 planes spells the *pixel* as
  `bbbbbbbb-gggggggg-rrrrrrrr-xxxxxxxx` (`sdk/gemlib/gemlib-0.44.0/n_vq_scrninfo.c:78-80`), which is that byte order
  written leftmost-byte-first. The 16-plane decision E3 (approved) already chose the native
  word over the byte-wise reading ("Fall B" of `eddi-vq_scrninfo.md` §6.3 against its "Fall A"
  `0x82`), and ADR-038 point 8 carries it to 32. The consumers cited in section 9 count the
  bits as native values and look at the numbers, not at `[14]` (HighWire
  `sdk/apps/highwire/src/raster.c:1713-1716`; zView
  `Applications/z-tools/zview/zvdi/vdi.c:38-41`; DOOM prints `[14]` but does not use it,
  `sdk/apps/doom/doomgeneric_gem.c:85-89@EdDI` comment). fVDI's own `0x81` marks ARAnyM's host
  framebuffer and carries its own `FIXME: big and little endian differences`
  (`references/fvdi/fvdi/drivers/aranym/spec.c:64`) **[C]**; it is not adopted (TASK/V61.md
  point 2).
- **Pixel-format flags.** `[14] = 1` only: bit 0 set (usual order), bit 1 (Falcon) clear, bit
  7 (swapped bytes) clear **[D]**. A future m68k thunk reading the form as big-endian longs
  would see swapped bytes; that is a decision of that thunk (TASK/V61.md point 2).

## 4. Consistency table (item 2)

What must agree with the `vq_scrninfo` answer at the 32 setting. "Anchor" is the NewTOS site
that produces the value; the setting is `backend.planes()`, set once at boot
(`kernel/src/vdi_backend.rs:44@PLANES`, `kernel/src/vdi_backend.rs:1125@set_planes`;
`kernel/src/bootargs.rs:165@VDI_PLANES_WIDE`) **[C]**.

| Observable | Value at 32 | Agrees with `vq_scrninfo`? | Where it comes from |
|---|---|---|---|
| `EdDI` cookie | `EDDI_MARKER = 0x00000001`, seeded at every boot, **independent of the setting** | presence only: a program that finds it calls `vq_scrninfo` | `tos-api/src/cookie.rs:58@EDDI_MARKER`, `tos-api/src/cookie.rs:228@EDDI` **[C]** |
| `vq_extnd` mode 1 `INTOUT[4]` = `INQ_TAB[4]` | 32 | **must equal `[2]`**: DOOM refuses the description when `si[2] != nplanes` (`sdk/apps/doom/doomgeneric_gem.c:1109@nplanes`) | `inq[4] = planes` (`tos-api/src/vdi/workstation.rs:325@inq`); EmuTOS `INQ_TAB[4] = v_planes` (`references/emutos/vdi/vdi_control.c:210`) **[C]** |
| `vq_extnd` mode 1 `INTOUT[5]` = `INQ_TAB[5]` (video LUT) | 0 | **must say "no LUT" when `[1] = 2`**, and does | `tos-api/src/vdi/workstation.rs:331@planes` (the gate already includes 32, V62); ADR-038 point 3 **[C]** |
| `vq_extnd` mode 0 / `v_opnwk` / `v_opnvwk` `work_out[13]` = `DEV_TAB[13]` | 256 (the pens) | **not the colour count**: `[3]:[4]` is 2^24, `DEV_TAB[13]` is 256; the same relation as at 16 planes (32768 against 256). EmuTOS: `(v_planes<8) ? (1<<v_planes) : 256` (`references/emutos/vdi/vdi_control.c:206`) | `dev[13] = ncol` (`tos-api/src/vdi/workstation.rs:229@dev`, `:313@dev`), `ncol = backend.num_colors() = 256` (`kernel/src/vdi_backend.rs:37@NUM_COLORS`) **[C]** |
| `work_out[35]` colour capable | 1 | consistent (colour device) | `tos-api/src/vdi/workstation.rs:230@dev`; EmuTOS `vdi_control.c:207` **[C]** |
| `work_out[39]` | 0 | consistent (no palette precision to report) | `palette_precision(32) = 0` (`tos-api/src/vdi/workstation.rs:39@palette_precision`) **[C]** |
| AES `ap_planes` (`global[10]`) | 32 | equals `[2]` | `screen_planes()` returns `planes()` (`tos-api/src/aes/appl.rs:369@screen_planes`) **[C]** |
| `v_get_pixel` word layout | `INTOUT[0] = (pixel >> 16) & 0xFFFF`, `INTOUT[1] = pixel & 0xFFFF`, X masked | **must equal the pixel the tables describe**, masked to the table bits (`0x00FFFFFF`) | `tos-api/src/vdi/inquiry.rs:201@VDI_PLANES_WIDE`, mask `tos-api/src/vdi/contour.rs:167@VDI_PLANES_WIDE` (V63/V65) **[C]** |
| Memory MFDB the tables describe | `fd_stand = 0`, `fd_nplanes = [2] = 32`, row `fd_wdwidth * 16` pixels, native `u32` | **must equal the form `vro_cpyfm` reads** | ADR-038 point 5; `_raster.md` memory-forms section 3 **[D]** |
| Pen pixel | `0x00RRGGBB`, 8 bits per gun | **must equal the table-built pixel of the pen's 8-bit colour** | V64 P1-P3 in `docs/api/vdi/raster/_raster.md` §"32-plane setting: pens ..." **[D]** |
| `VsetMode(-1)` | `0x001C`: "16-bit truecolor" | **disagrees by decision**, `[2] = 32` against a Falcon word that has no 32-bit value | ADR-038 point 9 / O4 (`tos-api/src/xbios/screen.rs:442@FALCON_MODE_WORD`); V68 and `docs/COMPATIBILITY.md` own it **[C]** |
| `VgetSize` | the 32-bpp store | agrees (at 16 it disagrees, `docs/DEVIATIONS.md`) | `tos-api/src/xbios/screen.rs:482@vgetsize`; ADR-038 point 9 **[C]** |
| `Physbase` memory | 32-bpp XRGB8888, linear | consistent with the pixel; **`[5..7] = 0` still says "no accessible screen"** | `sdk/examples/physbase.c:19-25@XRGB8888`; open point 2 |

Nothing else in the tree reads the plane count from a second place that this slice changes:
the three constants of the 16 (`PLANES`, `CHUNKY_PLANES`, `DEVICE_PLANES`) are closed by V62,
V65 and now V66 (`docs/api/vdi/workstation/100-v_opnvwk.md` §2).

## 5. The 16-plane answer (item 3)

**Unchanged** (ADR-038 point 11 and acceptance of TASK/V66: "at 16 planes the answer is
unchanged"). The path that stays as it is: `scrninfo_table()` at 16 (the column in section 2),
the gate `backend.planes() == DEVICE_PLANES` for `planes() == 16`
(`tos-api/src/vdi/eddi.rs:79@is_scrninfo`), the cookie, the 16-plane tests
`tos-api/tests/vdi_eddi_scrninfo.rs` (the whole-table comparison against the Enhancer dump at
`:123`, `fn vq_scrninfo_answers_the_falcon_hicolor_table_word_for_word`) **[C]**.
The seam of the 16-bit word (`device_to_rgb565`, `rgb565_to_device`, ...) is not touched by
this slice (ADR-038 point 11).

**What the developer must not change at 16:** any value of the 16 column of section 2; the
fall-through to `INQ_TAB` for a backend that is neither 16 nor 32 (host test doubles, `eddi.rs`
doc of `is_scrninfo`) stays **[I]**: ADR-038 widens the gate to "both depths", not to every
count.

## 6. The structural self-test of ADR-038 point 13 (i), as testable statements (item 4)

Let `T` be the 273-word answer at the 32 setting, `T16` the same call at 16. The statements
are over the answer alone (S1-S5) and over the answer plus the drawing path (S6-S7). Statement
S5 is the colour-count check; S6 is the round trip the owner closed for V65 and V66
(`TASK/V65.md` "Owner decisions 2026-11-18").

- **S1, shape.** `T[0] = 2`, `T[1] = 2`, `T[2] = 32`; `T[2]` equals `vq_extnd(1)` `INTOUT[4]`
  and `INTOUT[4]` of the same call at 16 equals `T16[2]`.
- **S2, counts match the tables.** The number of entries `!= -1` in `T[16..32]`, `T[32..48]`,
  `T[48..64]`, `T[64..80]`, `T[80..96]`, `T[96..128]` equals `T[8]`, `T[9]`, `T[10]`, `T[11]`,
  `T[12]`, `T[13]` respectively; the entries `!= -1` of each table are its first `n` entries
  (a `-1` is never followed by a number). At 32: 8, 8, 8, 0, 0, 8.
- **S3, the tables partition the pixel.** The union of the numbers in the six tables is exactly
  `{0 .. T[2]-1}`, and the six sets are pairwise disjoint (no number appears twice). At 32:
  `{16..23} + {8..15} + {0..7} + {24..31}` = `{0..31}`; `8+8+8+0+0+8 = 32 = T[2]`. The same
  statement holds for `T16` (`{11..15} + {6..10} + {0..4} + {5}` = `{0..15}`) **[C]** for 16 by
  the code, **[D]** for 32 by the ADR.
- **S4, gun bits ascending.** Within each of R, G, B the numbers rise by 1 from entry 0
  (entry `k` = least significant bit + `k`), so a program building a gun value `q` sets pixel
  bit `table[k]` from bit `k` of `q` (the use DOOM makes of the table,
  `sdk/apps/doom/doomgeneric_gem.c:1021-1036@encode_rgb`).
- **S5, colour count.** `(T[3] as u16 << 16) | T[4] as u16 == 1 << (T[8] + T[9] + T[10])`: at 32
  `0x01000000`; at 16 `0x00008000` (32768, E4 of the 16-plane spec).
- **S6, round trip (V65's wide `v_get_pixel`).** For a set of 8-bit `(r, g, b)` triples
  including `(0,0,0)`, `(255,255,255)`, each single-gun maximum, and one triple per table bit:
  build `p = sum over c in {R,G,B}, k in 0..T[8+c] of ((q_c >> k) & 1) << T[16 + 16*c + k]`
  with `q_c` the 8-bit value; write `p` (X = 0) into a 32-plane memory MFDB (`fd_stand = 0`,
  `fd_nplanes = T[2]`, `fd_wdwidth * 16` pixels a row); `vro_cpyfm` it to the screen with
  `S_ONLY`; read the pixel with `v_get_pixel`. Then
  `((INTOUT[0] as u16 as u32) << 16) | (INTOUT[1] as u16 as u32) == p & M` with
  `M` = the OR of `1 << n` over every number `n` in the R, G, B tables (= `0x00FFFFFF`).
  The mask is needed because the read masks X (ADR-038 point 4; `_raster.md` memory-forms
  section 6) and a screen-derived X is never asserted (`_raster.md` memory-forms section 3,
  "X byte under the copies") **[I]**.
- **S7, the other direction and the pen.** `vro_cpyfm` from the screen into a memory MFDB
  after a pen was drawn (`vs_color` with `c = (q * 1000 + 127) / 255` as DOOM sets pens,
  `sdk/apps/doom/doomgeneric_gem.c:1176-1185@set_pen`, then `vr_recfl`) gives a word `w` with
  `w & M == p(q_r, q_g, q_b)` for the table-built `p`, for `q` within the pen rule
  `(c * 255 + 500) / 1000` of ADR-038 point 4 (that rule reproduces `q` for this `c`: calculated over all `q` = 0..255 with
  `((q*1000+127)/255*255+500)/1000 == q`, integer division, no exception, a host calculation
  and not a run of NewTOS) **[calc]**.
  This is the check DOOM performs on the first frame (`doomgeneric_gem.c:1192-1250@eddi_check`); it is
  not asked by ADR-038 point 13 (i) and is listed as an addition.

Statements that need V65 for their drawing half: S6, S7 (V65 is landed, `2ef2d6c6e`); S1-S5
need only the table and the backend double (the `harness::PLANES` of the existing test file
must take 32, `tos-api/tests/vdi_eddi_scrninfo.rs:237@PLANES`).

## 7. Change set for the developer (item 5)

| Function / site | Anchor | What changes | Slice note |
|---|---|---|---|
| `DEVICE_PLANES` constant | `tos-api/src/vdi/eddi.rs:52@DEVICE_PLANES` | the table no longer depends on one constant; 16 and 32 each have their own gun layout (`GUN_BITS`, `RED_LSB`, `GREEN_LSB`, `BLUE_LSB`, `GENLOCK_BIT`, `ORDER_FALCON`, `eddi.rs:57-66@GUN_BITS`: 16-plane constants stay, 32-plane ones are added: 8, 16, 8, 0, none, `1`) | the `u32` pixel layout for 32 can reuse the pixel-word layout of V63 (R1) |
| gate | `tos-api/src/vdi/eddi.rs:79@is_scrninfo` (line 82 `backend.planes() == DEVICE_PLANES`) | true for 16 **and** `VDI_PLANES_WIDE` (`tos-api/src/vdi/backend.rs:736@VDI_PLANES_WIDE`); any other count keeps falling through to `vq_extnd` **[I]** | after V66 a 32-plane boot no longer answers this call with `INQ_TAB` (`100-v_opnvwk.md` §4) |
| `scrninfo_table` | `tos-api/src/vdi/eddi.rs:110@scrninfo_table` | builds the 273 words of section 2 for the setting; today it takes no argument and the test calls it so (`tos-api/tests/vdi_eddi_scrninfo.rs:154@scrninfo_table`); an argument or a second function is the developer's | the 16 output is bit-for-bit today's |
| `vq_scrninfo` | `tos-api/src/vdi/eddi.rs:156@vq_scrninfo` | gets the plane count from the backend (it receives only `pb` today) | output counts `272` / `0` unchanged |
| dispatch | `tos-api/src/vdi/mod.rs:504@scrninfo`, `:521@scrninfo` | passes the backend to `vq_scrninfo` if the signature needs it | |
| module header and doc comments of `eddi.rs` | `tos-api/src/vdi/eddi.rs:1-30@E3`, `:44-52`, `:68-78` | state the 32 answer; they now say "only 16" | doc-check anchors |
| cookie | `tos-api/src/cookie.rs:228@EDDI` | **none**: seeded unconditionally | |
| `vq_extnd`, `v_opnwk`, `v_opnvwk` | `tos-api/src/vdi/workstation.rs:325@inq`, `:331@planes` | **none**: V62 already made `INQ_TAB[4] = 32`, `INQ_TAB[5] = 0`, `DEV_TAB[13] = 256`, `DEV_TAB[35] = 1`, `DEV_TAB[39] = 0` | verify only |
| tests | `tos-api/tests/vdi_eddi_scrninfo.rs` (whole-table test `:123`, bit-table test `:162`, fall-through test `:196`) | add the 32 table word for word, S1-S7 of section 6; the 16 test stays; the fall-through test must stop expecting `INQ_TAB` at 32 | `harness::PLANES` must be settable to 32 |
| docs | `docs/COMPATIBILITY.md` EdDI section (item 4, "The answer is the Falcon HiColor one", `:602-610` region) | one row for the 32 setting (ADR-038 point 12 asks for the section); `docs/api/vdi/workstation/100-v_opnvwk.md` §4 and `102-vq_extnd.md` open point lose the V62 interim text; `eddi-vq_scrninfo.md` §6 gets a pointer to this file and is translated in the same commit (CLAUDE.md language rule, V62 open point) | the lead edits ticket and STATUS |
| 16-plane paths | section 5 | none | |

## 8. Acceptance criteria of TASK/V66.md, per criterion (item 6; not reworded)

| Criterion (the ticket's words) | Testable with this spec? |
|---|---|
| "The structural self-test of ADR-038 point 13 (i): the bit tables partition the pixel, the colour count matches, the round trip of V65 holds once V65 is in." | **Yes.** Partition: S3; colour count: S5; round trip: S6 (V65 is landed). All three are over values this spec fixes (section 2) and the drawing path V63-V65 spec'd. S3 and S5 need no run beyond a host test of `scrninfo_table`. |
| "At 16 planes the answer is unchanged." | **Yes.** The 16 column of section 2 is the code's own table; the existing whole-table test against the Enhancer dump (`tos-api/tests/vdi_eddi_scrninfo.rs:123@vq_scrninfo_answers_the_falcon_hicolor_table_word_for_word`) is the regression bar. |
| "`cargo xtask test` green; both board builds; warnings delta 0; doc-check of the edited docs 0 failures." | Mechanical; no spec content. |

## 9. Program survey (item 7, facts only)

Searched `sdk/apps/` (through the symlinked app directories, `grep -R`), `sdk/overlays/`,
`sdk/examples/`, `programs/`, `tosdump/` for `vq_scrninfo`, `C_EdDI`, `EdDI`, `0x45644449`,
plus `Applications/z-tools/zview` (a local, gitignored tree). `programs/` and `sdk/examples/`:
no hit. Every executable of the standard image maps to a local source by the build recipe
`tools/make-sd-image.sh`: **not checked in this run** (open point 4); the survey covers local
sources only.

| Program | Reads | Does with a 32-plane answer (from the code) |
|---|---|---|
| DOOM `sdk/apps/doom/doomgeneric_gem.c` | `Getcookie(C_EdDI)` `:1096`, `vq_scrninfo` `:1102`, `[0]`, `[1]`, `[2]`, `[8..10]`, `[14]` (printed only), tables `[16 + 16*c + k]` | requires `si[2] == nplanes` (`:1109`, so 32 against `INQ_TAB[4] = 32`); for `[0] = 2` and `[1] = 2` takes the route `EDDI_TRUE`: each channel width `1..16`, every table bit `0 <= b < dev_bpp * 8` (= 32, `dev_bpp = (nplanes + 7) / 8 = 4`, `:1513`) and **no bit twice** (`:1120-1135`), `cmp_mask` = the OR of those bits (`:1137`, so X is ignored in the comparison). `[5..7]`, `[11..13]` are not read; alpha, genlock and unused bits stay 0 (`:63` comment). Then verifies with 256 pens read back by `vro_cpyfm` (`:1192-1250`) |
| HighWire `sdk/apps/highwire/src/romvdi.c:70,318-346`, `sdk/apps/highwire/src/raster.c:1711-1716@hw_vq_scrninfo`, `:1794-1806@raster_32` | `getcookie(EdDI)`, `vq_scrninfo`; accepts the answer if `out[0]` in 0..2 and `out[2]` in 1..32 (`sdk/apps/highwire/src/romvdi.c:331@out`) | `sdepth = (UWORD)out[4] == 0x8000 ? 15 : out[2]` = 32 (`out[4] = 0`); `reverse = out[16] < out[48]` = false (16 < 0 false), `z_trail = out[48] > 0` = false; so `raster_32` (`:1794-1806`), which stores `(r << 16 \| g << 8 \| b)` as one `ULONG` (`:1392-1405` area). It copies `out[16..272]` into `pixel_val` (`:1712`) and does not read `[14]` for 32. At 16 planes today it takes the 15-bit case and falls through to `raster_16` because `[14] & 2` (`:1757-1770`) |
| zView `Applications/z-tools/zview/zvdi/vdi.c:28-41,164-175` (local, gitignored) | cookie, `vq_scrninfo`, `[4]`, `[2]`, `[16]`, `[48]` | same selection as HighWire: `case 32` with `reverse`/`z_trail` false takes the plain 32-bit raster (`:164-180`) |
| TeraDesk `sdk/overlays/teradesk/tests/cookie-host.c:75@EdDI`, `sdk/overlays/teradesk/tests/cookie-probe.c:63@EdDI` | list `EdDI` as a cookie present in the jar (`0x00000001`) | not a `vq_scrninfo` reader; the cookie is presence-only |
| `tosdump/core/scrninfo.c` | cookie, `vq_extnd(0/1)`, `vq_scrninfo` into 512 words pre-filled with a poison value (`:208-218`), builds a pixel test with `planes` up to 32 (`:220-221,520-522`: refuses `planes > 32`) | a 32-plane run would write the table and probe pixels; it carries the 16/32 caps of ADR-038 point 13 (iv) elsewhere (`tosdump/core/cases.c`) |
| `present`, QED, desktop, `sdk/apps/apitest`, `sdk/apps/gemtest` | no `vq_scrninfo` or `EdDI` reference | none |

## Open points

- **STOP-block 1, `[5..7]` at 32 (needs a decision).** ADR-038 point 8 gives `[5..7] = 0`
  "as at 16". At 32 a linear 32-bpp screen exists and `Physbase` hands it out
  (`sdk/examples/physbase.c:19-25@XRGB8888`), so a program reading `[5]` could now be told the
  line width in bytes (a 16-bit value) while `[6/7]` cannot hold a 64-bit address. My reading
  **[I]**: keep `0` (the ADR; the gemlib text gives no "inaccessible" value, and no
  in-tree consumer reads `[5..7]`: DOOM, HighWire and zView do not). Question to the lead: is
  that final?
- **Question 2, `[14] = 1` against a byte-wise reading.** The bit numbers count the native
  32-bit integer; NVDI's text for `bit 7` describes the pixel with the lowest-addressed byte
  leftmost, which on this little-endian store is `bbbbbbbb-gggggggg-rrrrrrrr-xxxxxxxx`
  (section 3). A program that applies NVDI's byte-wise reading to `[14] = 1` and builds the
  pixel by bytes would get swapped colours; the three consumers read here use the bit
  numbers, so they do not. My reading **[I]**: `[14] = 1` as the ADR says, consistent with the
  approved 16-plane E3; no source decides the byte-wise reading. A measurement on a real
  EdDI at 32 planes (NVDI, Nova) is not possible here.
- **Question 3, X in `[12]`/`[13]`.** ADR-038 point 8 gives `[12] = 0`, `[13] = 8` (O1 option
  A). If a program sets a table-listed unused bit (`[96..103]`) in a pixel it writes, V63's
  read mask drops it; the spec states the masks, not what a program should do with unused
  bits (E9 of the 16-plane spec says leave them 0) **[I]**.
- **Question 4, image survey.** The "every executable on the image maps to a local source"
  check (`tools/make-sd-image.sh`, `<image>.manifest`) was not run in this run; the survey in
  section 9 is of the source trees only.
- **Inferred:** the gate widening in section 7 for counts other
  than 16 and 32; that `[15]` stays 0; every **[I]** line above.
- **Not read in this run:** tos.hyp `vq_scrninfo.ui` and NVDI 2.50 `Neue_Fkt.txt` (the gemlib
  Doxygen text, which `eddi-vq_scrninfo.md` §3.4 records as agreeing with them, is the layout
  source used); the 16-plane measurement `SCRNINFO.TXT`.
- **Out-of-scope observations:** (a) `tos-api/src/cookie.rs:48@Doom` still says "Doom does not
  look yet", which is stale (DOOM reads the cookie, `doomgeneric_gem.c:1096@Getcookie`); (b)
  `eddi-vq_scrninfo.md` is German and carries the 16-plane-only §6.3; (c) `VsetMode(-1)` at 32
  says 16-bit truecolor against `[2] = 32`, by ADR-038 point 9 (V68); (d) the colour-icon and
  VT52 paths stay 16-bit until V67 (`_raster.md` memory-forms section 7); a pen drawn into
  those is out of S7.
