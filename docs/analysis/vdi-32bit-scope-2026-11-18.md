# VDI at the framebuffer's real depth (32 bit): scope, contract source, measurement, programs, options (V61)

Researcher, 2026-11-18, ticket `TASK/V61.md`, main `aaed2cde0`. Research only: no product code, no ADR edit, no change
under `docs/api/`. No decision is taken here; the owner decides (V61 acceptance).

Labels: **[C]** = read in the source named, **[I]** = inferred from what was read, **[M]** = needs a measurement or a run
(none was made). EmuTOS cites are `references/emutos/<path>:<line>` written as `<path>:<line>` in tables only where the
tree is obvious from the section heading ("EmuTOS:"); every other cite is repo-relative. NewTOS code cites carry the
symbol anchor `<path>:<line>@<symbol>`.

## 0. Summary

1. **What 16 planes is today (C).** The physical store is 32-bpp XRGB8888 (`kernel/src/drivers/bcm2711/framebuffer.rs:34@DEPTH_BPP`);
   the VDI device is a 16-plane RGB565 device (`kernel/src/vdi_backend.rs:44@PLANES`). Drawing primitives already write
   the 32-bit store directly; the 16-bit word exists only at a **seam**: pen to device word, memory-MFDB blits,
   `v_get_pixel`, flood-fill reads, `bb_save`/`bb_restore`, the VT52 cell writer, `vq_scrninfo`. The 32-bit store is a
   lossless bitwise expansion of the 16-bit word including the overlay bit in the X byte
   (`kernel/src/vdi_backend.rs:200@word_to_device`).
2. **The change sites** (section 1) fall in four kinds: constants (`16`), gates written as `planes == 16`, the seam
   (`u16` words in the `RasterBackend` trait and its callers), and **format contracts** that have no EmuTOS text
   (`v_get_pixel` output, memory-MFDB layout at 32 planes, colour quantisation, `vq_scrninfo` table, `VsetMode` word).
   The NEON kernels and the line, fill and text drawing are already 32-bit and depth-agnostic (section 1.3).
3. **No EmuTOS contract exists at 32 planes, and the EmuTOS code would reject it** (section 2.1): its mode table stops at 16
   (`bios/videl.c:338-341`), `vro_cpyfm` refuses more than 16 planes (`vdi/vdi_raster.c:962-966`).
4. **Real implementations at hand:** fVDI in `references/fvdi` (32-bit driver tables, `vq_scrninfo` fill, 32-plane
   `vr_trnfm`, a different `v_get_pixel` answer than EmuTOS) and consumers in the tree that read the 32-bit layout
   from `vq_scrninfo` (HighWire, zView, DOOM). They support only the *structure* of a contract, not its values (section 2.3).
5. **Measurement:** against the pinned oracle nothing can be compared at 32 planes; the tosdump tooling itself caps at 16
   (section 3).
6. **Programs:** TeraDesk reads out of bounds at 32 (`sdk/apps/teradesk/video.c:446,592`); `present` hardwires 16; HighWire,
   zView and DOOM adapt through `vq_scrninfo`; QED and the desktop are indifferent (section 4).
7. **Options** (section 5): A stay at 16, B 32 for all, C both selectable. Compared, not decided.

## 1. Scope in NewTOS (question 1)

### 1.1 How depth enters the code (C)

| Mechanism | Where | Reading |
|---|---|---|
| One constant | `kernel/src/vdi_backend.rs:44@PLANES` (`16`), `NUM_COLORS = 256` at `:37@NUM_COLORS`; `tos-api/src/vdi/raster.rs:85@CHUNKY_PLANES`; `tos-api/src/vdi/eddi.rs:52@DEVICE_PLANES` | three independent copies of "16" |
| Gate `planes > 8` (EmuTOS `TRUECOLOR_MODE`, `vdi/vdi_defs.h:22`) | `tos-api/src/vdi/workstation.rs:250@planes`, `:431@planes`; `inquiry.rs:106@planes`, `:198@planes`; `attributes.rs:499@planes`; `contour.rs:163@planes`; `raster.rs:1523@planes` | stays true at 32 **[I]**: the predicate is `> 8` (EmuTOS has the same predicate) |
| Gate `planes == 16` | `workstation.rs:330@planes` (`inq[5]`, EmuTOS `vdi/vdi_control.c:211`); `eddi.rs:82@DEVICE_PLANES` (answers `vq_scrninfo` only at 16) | flips at 32: `INQ_TAB[5]` would become 1 (video LUT present) and EdDI would fall back to the plain `vq_extnd` answer **[I]** |
| Cap `> 16` | `raster.rs:821@CHUNKY_PLANES` and `:560@CHUNKY_PLANES` (`vro_cpyfm`/`vrt_cpyfm` return at once for a destination of more than 16 planes, EmuTOS `vdi/vdi_raster.c:962-966`) | a program passing `fd_nplanes = 32` (what `vq_extnd` would report) gets a silent no-op **[I]** |
| 16-bit word in the API | `tos-api/src/vdi/backend.rs:138@pel_to_device16`, `:159@device_to_rgb565`, `:169@rgb565_to_device`, `:424@read_row_rgb565`, `:440@write_row_rgb565` | the trait names and types the 16-bit word (`u16`); the 32-bit store is reached through `get_device_pixel`/`put_device_pixel` (`u32`) |

### 1.2 Change list by file and function

Kinds: **K** constant; **G** gate; **S** seam (convert word and store); **F** format contract, no EmuTOS value, needs the
section 2 decision; **N** no change expected **[I]**.

**VDI (`tos-api/src/vdi/`)**

| File : function | Dependence on the 16-plane pixel (C) | Kind |
|---|---|---|
| `workstation.rs` `init_wk` (`:156@init_wk`), `vq_extnd` (`:298@vq_extnd`) | `dev[35]`, `dev[39]` via `palette_precision` (`:39@palette_precision`), `inq[4] = planes` (`:325@inq`), `inq[5]` gate (`:330@planes`); workstation open seeds a per-workstation 256-entry pseudo-palette (`:250@planes`) | G, F |
| `attributes.rs` `vs_color` (`:492@vs_color`), `vq_color` (`inquiry.rs:93@vq_color`) | truecolor path stores 5-bit-quantised requests (`attributes.rs:499@planes`); EmuTOS `vs_color16` packs `(r<<11)\|(g<<6)\|b` from 5-bit values (`vdi/vdi_col.c:594-616`) | F |
| `vwk.rs` `set_color16`, `pack_color16` (`:256@set_color16`, `:262@pack_color16`) | the 16-bit pseudo-palette `palette: [u16; 256]` (`:219@palette`) | F, S |
| `attributes.rs` `vsf_udpat` (`:616@vsf_udpat`) | accepts `count == planes * 16`; takes `planes` as a parameter | N |
| `inquiry.rs` `v_get_pixel` (`:190@v_get_pixel`) | truecolor answer `INTOUT[0]=0`, `INTOUT[1]=` 16-bit word (EmuTOS `vdi/vdi_fill.c:1104-1110`): one WORD has no room for a 24 or 32-bit pixel | F |
| `contour.rs` `pixelread` (`:163@planes`), flood-fill compare (`:228@truecolor`) | compares 16-bit screen words through `device_to_rgb565` | S |
| `raster.rs` `vro_cpyfm` (`:796@vro_cpyfm`), `vrt_cpyfm` (`:544@vrt_cpyfm`), `vr_trnfm` (`:1497@vr_trnfm`), `vr_trnfm16` (`:1393@vr_trnfm16`) | `CHUNKY_PLANES`, `device_mem` stamps 16 (`:212@device_mem`), word-wide memory access (`u16`), `plane_ct <= 16` cap, `vr_trnfm16` is a 16-plane-sequential standard form | K, G, F, S |
| `raster.rs` `screen_to_mem` (`:904@screen_to_mem`), `mem_to_screen` (`:956@mem_to_screen`) | row conversion through `read_row_rgb565`/`write_row_rgb565`; at 32 planes a memory MFDB word would equal the store word and these shrink to copies **[I]** | S |
| `raster.rs` `bb_save_words` (`:1296@bb_save_words`), `bb_save`/`bb_restore`/`blit_device_mem` (`:1324@bb_save`, `:1340@bb_restore`, `:1369@blit_device_mem`) | save buffer sized and typed as 16-bit pixels (`bb_save_words = wdwidth * 8 * h` `u32` words) | K, S |
| `escape.rs` VT52 cell writer (`:257@cell_xfer`, `:273@write_row_rgb565`, `:280@neg_cell`, `:298@blank_out`) | writes cell rows as 16-bit words | S |
| `eddi.rs` `scrninfo_table` (`:112@scrninfo_table`), gate (`:82@DEVICE_PLANES`), consts (`:52@DEVICE_PLANES`, `:64@GUN_BITS`) | table derived for exactly the 16-bit device: format 2, CLUT 2, 16 bits, 32768 colours, R 11..15, G 6..10, B 0..4, genlock bit 5, order `0x02` | F |
| `mouse.rs` cursor save/restore (`:141@XRGB8888`) | holds raw XRGB8888 words | N |
| `text.rs`, `line.rs`, fills, `backend.rs` `fill_rect`/`abline`/`blit_mono`/`copy_rect` | take `Pel` and clip; no 16-bit word in their path (the only `rgb565` hits in `text.rs` are test doubles, `:1521@pel_to_device16`) | N |

**Kernel raster backend and NEON kernels (ADR-036)**

| File : function | Dependence | Kind |
|---|---|---|
| `kernel/src/vdi_backend.rs` `word_to_device`/`device_to_word` (`:200@word_to_device`, `:213@device_to_word`) | 5-bit gun replication plus the overlay bit replicated into the X byte (`OVERLAY_X`, `:192@OVERLAY_X`) | S, F |
| `vdi_backend.rs` `pel_to_device16` (`:501`), `device_to_rgb565` (`:510`), `rgb565_to_device` (`:524`), `clear_screen_word` (`:558`), `read_row_rgb565` (`:945`), `write_row_rgb565` (`:979`) (`@pel_to_device16`, `@device_to_rgb565`, `@rgb565_to_device`, `@clear_screen_word`, `@read_row_rgb565`, `@write_row_rgb565`) | the seam | S |
| `vdi_backend.rs` `set_color` (`:469@set_color`), `ACTIVE_PALETTE` (`:145@ACTIVE_PALETTE`), `read_pixel` (`:531@read_pixel`) | palette of 16-bit words (`pack555`); `read_pixel` reverse-maps to the nearest pen | F |
| `vdi_backend.rs` `xor_pixel` (`:408@xor_pixel`) | complements all 32 bits, which equals the 16-bit `~word` only because the store is a bitwise expansion | F |
| `kernel/src/raster_kernel.rs` (`fill_span` `:486@fill_span`), `raster_copy.rs` (`raster_op` `:92@raster_op`), `raster_expand.rs`, `raster_line.rs` | operate on `u32` pixels and bitwise ops; the `u16` in them are pattern, dash and glyph bit rows, not pixels | N **[I]** (bitwise ops hold for any 32-bit pixel; the *meaning* of `ALL_BLACK = !0` and of the X byte at 32 planes is F) |
| `kernel/src/hal/framebuffer.rs` (`:61@pixels`), `drivers/bcm2711/framebuffer.rs` (`:34@DEPTH_BPP`, `:60@ALPHA_MODE_IGNORED`), `drivers/qemu_virt/ramfb.rs` | the store is 32-bpp XRGB; the VideoCore is told alpha "ignored" because the X byte carries the overlay bit | F: the X byte meaning |

**AES (`tos-api/src/aes/`)**

| File : function | Dependence | Kind |
|---|---|---|
| `appl.rs` `screen_planes` (`:369@screen_planes`) | `ap_planes` = `planes()` (EmuTOS `aes/gemsuper.c:97`); a WORD, holds 32 | N |
| `init.rs` (`:355@planes`) | `n = (gl_nplanes > 2) ? 2 : gl_nplanes - 1` picks the colour pair (EmuTOS `gemgraf.c`-style, see the comment at `:347`); the `> 2` arm is depth-agnostic | N |
| `gsxif.rs` `gsx_mcalc` (`:359@gsx_mcalc`), `bb_save`/`Saved` (`:528@bb_save`) | one screen buffer sized by `bb_save_words`: 614 400 bytes at 640x480 and 4 147 200 at 1920x1080 as 16-bit pixels (`:354-355`); doubles at 32 **[I]** (arithmetic) | K, S |
| `menu.rs` drop-down save (`:1032@device-exact`), `form.rs`, `window.rs` | go through `gsxif::bb_save`; the `rgb565` hits in `window.rs` and `menu.rs` tests are mocks | N, via `gsxif` |
| `resource.rs` colour icons (`:1227@planes`, `:3029@NewTOS`, `CiconDevice` `:3058@CiconDevice`), `object.rs` (`:835@num_planes`) | the transformed `CICON` device form is "the 16-bit RGB565-chunky memory MFDB"; file-side `num_planes` accepted 1..16 (`:1227`) is the resource *file's* depth, not the device's | S, F (device form) |
| `window.rs`, `graf.rs`, 3D drawing | pens and `fill_rect`; no 16-bit word | N **[I]** (not individually read) |

**Programs in the repository:** `programs/desktop/src/global.rs:184@patcol_index` (`_ => 2` for any depth above 2),
`gsx.rs:125@gsx_nplanes` (reads `vq_extnd` `intout[4]`), `inf.rs:3874@nplanes` (`_ => 16` is a colour *index*, not a depth):
no change **[I]**.

### 1.3 XBIOS, cookies, Line-A, screendump (C)

| Item | State | Effect at 32 |
|---|---|---|
| `VsetMode(-1)` | `0x001C` = `VIDEL_VGA\|VIDEL_80COL\|VIDEL_TRUECOLOR` (`tos-api/src/xbios/screen.rs:442@FALCON_MODE_WORD`) | the Falcon mode word's bpp field is 3 bits with `VIDEL_TRUECOLOR = 4` as the largest value (`include/biosdefs.h:146-154`): **no encoding for 24 or 32 bit** exists in the word; `0x001C` can stay as "a Falcon word" or the answer must leave the Falcon word space (F) |
| `Getrez` | 2 for every non-compat Falcon word (`screen.rs:180@getrez`) | unchanged while the word is non-compat **[I]** |
| `VgetSize` | already the **32-bpp** screen size (`screen.rs:482@vgetsize`; EmuTOS computes `width * 2 * height`, i.e. 16-bit, `bios/videl.c:756`) | already contradicts the VDI's 16 planes today; closes at 32 **[I]** |
| `VgetRGB` | first 16 entries of the Falcon palette, limit 16 (`screen.rs:492@VGETRGB_LIMIT`) | unchanged **[I]** |
| `Physbase`/`Logbase` | hand out the real framebuffer (`screen.rs:134@physbase`, `:105@caller_view`); the example program documents the buffer as linear XRGB8888 (`sdk/examples/physbase.c:19-25@XRGB8888`) | **a program writing the screen directly already sees 32 bpp while `vq_extnd` says 16 planes (C)**; at 32 the two agree |
| `_VDO` | `VDO_FALCON` (`tos-api/src/cookie.rs:266@VDO_FALCON`) | unchanged |
| `EdDI` | published with a marker value (`cookie.rs:37@EDDI`) | the table behind it changes (F) |
| Line-A | absent by decision (`docs/COMPATIBILITY.md:1649-1661`) | no LINEA variables to change |
| Screendump | QMP `screendump` of the QEMU display shows the 32-bpp store (`kernel/src/drivers/qemu_virt/ramfb.rs` header, `tools/xtask/src/main.rs:953@screendump`) | none; at 16 planes the picture is 5-bit-quantised per gun, at 32 it would carry what is drawn **[I]** |

### 1.4 Cost shape (I)

Counting the table: roughly 6 constant/gate sites, one trait (5 methods) with ~11 implementing or calling files, one format
decision per F row (8 rows). The drawing kernels need no change. The large sites are `raster.rs` (MFDB/`vr_trnfm` forms)
and the `vq_scrninfo` table; the others are mechanical once the section 2 questions are answered. Not estimated in hours:
no code was read for effort.

## 2. Source of the contract (question 2)

### 2.1 What EmuTOS says (C)

- EmuTOS derives depth from the Videl registers and knows `1, 2, 4, 8, 16` bits only (`bios/videl.c:336-341`, function
  `get_videl_bpp` from `:320`). `VgetSize` clamps any bpp field above truecolor to truecolor and returns `width * 2 * height`
  (`bios/videl.c:734-738,756`).
- Truecolor is `v_planes > 8` (`vdi/vdi_defs.h:22`), and 16 is the only value fed to it (ADR-024). Every truecolor routine is
  word-wide: `vro_cpyfm16` works on `UWORD*` (`vdi/vdi_raster.c:975-985`), `set_color16` packs 5-bit guns (`vdi/vdi_col.c:594-616`),
  `v_get_pixel` returns one WORD (`vdi/vdi_fill.c:1104-1110`), and the plane cap refuses more than 16
  (`vdi/vdi_raster.c:962-966`).
- Result: at 32 planes EmuTOS offers neither behaviour nor an error to copy (ADR-024's statement is right, checked here).

### 2.2 What would define the contract: candidates

A contract for a 32-plane device has five parts. For each, what is readable:

| Part | Candidate source | State |
|---|---|---|
| (a) The answer shape of `vq_scrninfo` (273 words, bit-number tables, format, CLUT, order flag) | `docs/api/vdi/eddi-vq_scrninfo.md` (the existing NewTOS spec; sources: NVDI documentation, the in-tree NVDI 2.50 text, fVDI) | **structure documented; 24/32-bit values not documented**: the spec states NVDI documents 24/32 only "as an ordering scheme without a worked example" and that no real NVDI/Nova table is available (`docs/api/vdi/eddi-vq_scrninfo.md` §6.1, lines 264-269) |
| (b) The 32-bit values an implementation fills | fVDI `references/fvdi/fvdi/drivers/aranym/spec.c` (read here: 32-bit usual: R bits 16..23, G 8..15, B 0..7, format 2, CLUT 2, org `1`; 24-bit "usual" uses the `_32f` tables R 8..15, G 16..23, B 24..31; the "fb layout" rows use org `0x81`) `references/fvdi/fvdi/drivers/aranym/spec.c:21-31,71-80`; the structure is filled in `references/fvdi/fvdi/drivers/common/init.c:102-175` | readable; one implementation, and the existing spec already warns that fVDI's 24-bit row contradicts the NVDI description (`eddi-vq_scrninfo.md` §6.1). The mode-table comment names `0x80` as "Intel byte order" (`references/fvdi/fvdi/drivers/aranym/spec.c:62`); what that means for an AArch64 little-endian store is **not decided by the source** (flag) |
| (c) `vq_extnd` planes and colour count | fVDI reports the physical depth: `INQ_TAB[4] = wk->screen.mfdb.bitplanes` (`references/fvdi/fvdi/engine/workstn.c:591`), set from the driver resolution `bpp` (`references/fvdi/fvdi/drivers/aranym/spec.c:376`); the colour count in the `vq_scrninfo` structure is `1 << (r+g+b bits)` as a high/low pair (`references/fvdi/fvdi/drivers/common/init.c:123-128`) | readable |
| (d) MFDB device form and `vr_trnfm` | fVDI `lib_vr_trnfm` has 16- and 32-bit chunky branches (`references/fvdi/fvdi/engine/blit.s:1084-1086,1188,1462-1477`; 16-bit branch `:1409-1417`); the standard form of a 32-bit transform is the fVDI routine's, not read in full here | **partly read**: the 32-bit branch bodies (`to_standard_32`, `references/fvdi/fvdi/engine/blit.s:1188` ff.) were not read to the end: needs deeper analysis |
| (e) `v_get_pixel` answer | fVDI returns the pixel in `intout[0]` (low word) and, above 16 bits, the high word in `intout[1]`, else `-1` (`references/fvdi/fvdi/engine/blit.s:44-96`, the `swap` and `cmp.w #16` at `:82-86`, m68k source read, not run) | readable; **differs from EmuTOS at 16 bit already** (EmuTOS: `INTOUT[0]=0`, `INTOUT[1]=pel`, `vdi/vdi_fill.c:1104-1110`) |

Real consumers show what programs *do* with such a table (C): HighWire selects `raster_32`, `raster_32r` or `raster_32z` from
`out[2]` (bits), `reverse = out[16] < out[48]`, `z_trail = out[48] > 0`
(`sdk/apps/highwire/src/raster.c:1713-1721,1783-1800`); zView does the same selection with `case 32` at
`Applications/z-tools/zview/zvdi/vdi.c:39-41,164-175`. So both consumers would work against *any* internally consistent 32-bit
table, **provided the program's blit path accepts a 32-plane device-form MFDB** (section 4).

Other implementations named by the ticket: MagiC has NVDI-line screen drivers per colour depth (ADR-035 context); I did not
read them because the ticket names fVDI only and ADR-035 limits MagiC to raster technique. Graphics-card VDIs (NVDI, Nova)
have no source at hand (`eddi-vq_scrninfo.md` §6.1). FreeMiNT (`references/freemint`) has no `vq_scrninfo` text (grep over the tree
found none).

### 2.3 What would have to change in ADR-024 and ADR-035 (C for the wording, I for the consequence)

- **ADR-035 point 1** says EmuTOS stays the specification of every pixel; points 2-3 allow fVDI and MagiC for *technique*
  only, nothing copied, and point 4 forbids a technique that would change a pixel against EmuTOS (`docs/MEMORY.md:3376-3384`).
  For fVDI to count as the specification of a 32-plane device, point 1 would need an exception ("for a depth EmuTOS does
  not have, the contract is taken from X"), point 2 would need to allow reading *values* (a table) rather than technique, and
  point 4 would need a carve-out (the oracle cannot decide at that depth, section 3). The licence limit of point 3 stays: a
  table of numbers read from fVDI would have to be re-derived from a description, not copied.
- **ADR-024** decision points 1, 4 and 5 fix 16 planes, the 16-bit memory-MFDB wire format ("this is contract, not
  internal") and `ap_planes = 16` (`docs/MEMORY.md:2256-2290`). A 32-plane device replaces them (B) or adds a second mode (C).
  Point 3 (the 32-bpp store is an internal detail) would stop being true as far as the X-byte and the quantisation are visible.
- **COMPATIBILITY.md** names `VgetSize`, `VsetMode(-1)` and the XBIOS23 `Getrez` answers as the current Falcon-video contract
  (`docs/COMPATIBILITY.md:1909-1933`); each would need a revised row.

Not decided here: whether "behaves like fVDI at 32" or "behaves like a documented NVDI at 32" is wanted; section 5 lists it as a
sub-choice of options B and C.

## 3. Measurement (question 3)

### 3.1 Lost at 32 planes (C)

- **The oracle has no 32-plane mode.** Both references are 16-bit or lower: `tt` 4 planes, `falcontc` 16-bit truecolor
  (`docs/analysis/ORACLE.md:61`); EmuTOS cannot be set to a deeper mode (section 2.1).
- **The tosdump tooling itself caps at 16.** The raster table directive rejects `planes` above 16 (`tosdump/core/cases.c:2548@planes`),
  an MFDB column with `fd_nplanes > 16` is refused because "the call could only ever be a no-op" (`tosdump/core/cases.c:2176@nplanes`),
  and the raster dumper refuses to run a table whose declared depth differs from the workstation's
  (`tosdump/core/dumprast.c:128@planes`). A 32-plane run needs those changed before any table runs.
- Cases that read depth-dependent values at 16 planes (`vq_extnd[4]`, `v_get_pixel`, MFDB readbacks, `vr_trnfm`) have no
  oracle value at 32.

### 3.2 Still comparable

- **Everything depth-independent**: geometry, clipping, attribute state, AES object layout, GEMDOS/BIOS/XBIOS answers that do
  not depend on the mode word: the existing tables, unchanged **[I]** (they do not read depth; a run would confirm: [M]).
- **Pen-to-colour behaviour projected to 16 bit** **[I]**: if the 32-plane colour model keeps EmuTOS' 5-bit quantisation of the
  requested colour (section 1.2, `vs_color`) and replicates guns as the store does today, then reducing a 32-bit read-back to the
  16-bit word reproduces the oracle's word, and the existing raster comparisons could be run through a "reduce to RGB565" step on
  the NewTOS side. This holds only for pen-drawn content; content blitted at full depth has no oracle word. [M] whether it holds
  for text, fills and lines: not run.
- **Option C keeps a 16-plane mode** which can still be run against `tt`/`falcontc` unchanged; the 16-plane regression bar
  would remain the existing tables (ORACLE.md profiles). [I]

### 3.3 What would replace the pixel comparison where it is lost (proposals, not decisions)

1. A **structural check** of the `vq_scrninfo` table against its own definition: each bit-number table is a consistent
   partition of the 32-bit word, colour count equals `1 << sum(bits)`, round trip `put_device_pixel -> get_device_pixel` per
   pen. These are properties, not oracle values. [I]
2. A **projection comparison** (3.2) for pen-drawn scenes, if the colour model is the projected one.
3. **Cross-implementation reading**: fVDI's tables as a plausibility check, labelled as such (not oracle). The existing
   measured counterpart for EdDI is the VDI Enhancer on EmuTOS at 16 bit (`tos-api/src/vdi/eddi.rs:23@Enhancer`), so even EdDI's measured
   evidence stops at 16. [C]
4. **Program-level acceptance** (HighWire, zView, DOOM display an image correctly): [M], by run, no oracle.

## 4. Programs (question 4)

Standard-image executables: the REQUIRED list and `build_example` calls of `tools/make-sd-image.sh:68@REQUIRED` map every program to a
local recipe or source (argvtest, libtest, clocktest, setjmptst, MXPROBE, mathtst, EX_LIB/LDGTEST, ZVIEW, CORE/COREVAL,
DUMPVDI, VDIDRAW, QED, TeraDesk, PRESENT, plus the freestanding `sdk/examples/*.c` programs). The **gap "no local source" is
empty by construction for these**; no `<image>.manifest` was present in the worktree to check the file list against, so this
rests on the script, not on a manifest. HighWire, hypview and DOOM are in `sdk/apps/` but not in that REQUIRED list (grep over
`tools/make-sd-image.sh`, no hit for them), so they are not on the standard image.

| Program | Reads | At 32 planes (source reading) |
|---|---|---|
| **TeraDesk** (`sdk/apps/teradesk/`) | `xd_nplanes = work_out[4]` from `vq_extnd(handle, 1, …)` (`sdk/apps/teradesk/library/xdialog/xdialog.c:2350-2351`); `Getrez`, `VsetMode(-1)` (`sdk/apps/teradesk/video.c:150,175`) | **out of bounds**: `npc[]` has 17 entries (`sdk/apps/teradesk/video.c:446`) and is indexed with `np = xd_nplanes` (`sdk/apps/teradesk/video.c:592`, `np` at `:454`); at 32 it reads `npc[32]` (I: a read of C, not a run). The same `xd_nplanes` is stamped into an MFDB (`sdk/apps/teradesk/library/xdialog/xdutil.c:103-105`, `sdk/apps/teradesk/library/xdialog/xddraw.c:1283-1285`) and the dialog save-under copies the screen into and out of that MFDB with `vro_cpyfm` (`sdk/apps/teradesk/library/xdialog/xdialog.c:193,217`): with the plane cap (`raster.rs:821@CHUNKY_PLANES`) a 32-plane MFDB blit returns without drawing **[I]**; [M] on a run |
| **QED** (`sdk/apps/qed/`) | `gl_planes` only for the colour-config file name: `min(gl_planes, 8)` (`sdk/apps/qed/src/options.c:986`), `gl_planes > 1` (`sdk/apps/qed/src/rsc.c:193`) | none (indifferent) |
| **HighWire** (`sdk/apps/highwire/`; not on the standard image) | `vq_scrninfo` through `hw_vq_scrninfo`, which uses the EdDI answer when `out[0]` is 0..2 and `out[2]` 1..32 (`sdk/apps/highwire/src/romvdi.c:330-336`) else describes the screen itself with `out[2] = planes`, packed above 8 planes (`sdk/apps/highwire/src/romvdi.c:338-342`); selects `raster_32`/`r`/`z` (`sdk/apps/highwire/src/raster.c:1783-1800`); builds image MFDBs with `fd_nplanes = planes` (`sdk/apps/highwire/src/image.c:904,933`, `sdk/apps/highwire/src/hwWind.c:1767`) | adapts to whatever the table says **if** a 32-plane MFDB blit works; with the cap at `raster.rs:821@CHUNKY_PLANES` the picture would not appear **[I]**, [M] |
| **zView** (`sdk/zview`, source in `Applications/z-tools/zview/`) | requires the `EdDI` cookie (`Applications/z-tools/zview/zvdi/vdi.c:27`), `vq_scrninfo` (`Applications/z-tools/zview/zvdi/vdi.c:31`), depth/`reverse`/`z_trail` (`:39-41`), `case 32` (`:164-175`); `max_color = 1 << app.nplanes` (`Applications/z-tools/zview/zvdi/color.c:75`) | adapts via the table; `1 << 32` in an `int16`/int context at `Applications/z-tools/zview/zvdi/color.c:75` is evaluated only if that code path runs at 32 planes (I; the path was not traced, [M]) |
| **hypview** (`sdk/apps/hypview/`; not on the standard image) | `vq_extnd` `ext_workout[4]` as plane count (`sdk/apps/hypview/dl_init.c:190`, `sdk/apps/hypview/hyp/prepare.c:336,375,384-402`), builds `pic->mfdb.fd_nplanes = ext_workout[4]` and calls `vr_trnfm` (`sdk/apps/hypview/hyp/prepare.c:412`); `TrueColors` for hypview pictures (`sdk/apps/hypview/hyp/prepare.c:217-300`) | memory sizes scale with the plane count (`sdk/apps/hypview/hyp/prepare.c:336`); transform at 32 planes depends on `vr_trnfm` accepting `nplanes = 32` (today: only the 16-bit standard form above 8 planes, `raster.rs:1523@planes`) [M] |
| **present** (`sdk/apps/present/present.c`; on the standard image) | hardwires `fd_nplanes = 16` in two MFDBs (`sdk/apps/present/present.c:620@fd_nplanes`, `:1098`) | **breaks at 32 if the device form changes**: it assumes the 16-bit memory form regardless of the workstation; it needs a patch (overlay) or the 16-plane mode |
| **DOOM** (`sdk/apps/doom/`; not on the standard image) | `nplanes = INQ_TAB[4]`, `vq_scrninfo` route, `C2P_MAX_PLANES = 32` (`sdk/apps/doom/doomgeneric_gem.c:340@C2P_MAX_PLANES`, `:514@nplanes`), tests 1..32 planes (`sdk/apps/doom/tests/c2p-host.c:17@planes`, `:128@planes`) | designed for 32; still goes through `vr_trnfm`/`vro_cpyfm` with a device MFDB (`sdk/apps/doom/doomgeneric_gem.c:25-40@vr_trnfm`) |
| **desktop** (`programs/desktop/`) | `gl_nplanes` / `gsx_nplanes()` (`programs/desktop/src/gsx.rs:125@gsx_nplanes`) | none (`_ => 2` pair; `inf.rs:3874@nplanes`) |
| `sdk/examples` `physbase`, `scrflip` | direct screen as 32-bit words, no plane query (`sdk/examples/physbase.c:19-25@XRGB8888`; `sdk/examples/scrflip.c:138-146@Logbase`) | none; they already see 32 bpp |
| **tosdump** dumpers | cap 16 (section 3.1) | need a change for any 32-plane run |

All consumers that use `vq_scrninfo` (HighWire, zView, DOOM) depend on `vro_cpyfm` and `vr_trnfm` handling a device-form
MFDB of the reported depth: that is the same site (`raster.rs`) in all three, and it is the load-bearing one for programs. [I]

## 5. Options (question 5)

Cost is in change sites from section 1.2; "gains" are what a program can observe; none of this is measured speed (all speed
statements are [I] or [M]).

| | **A. Stay at 16 planes** | **B. 32 planes for all** | **C. Both selectable** |
|---|---|---|---|
| What changes | nothing | all K, G, S and F rows of section 1.2; XBIOS `VsetMode`/`VgetSize` coherence; AES screen buffer sizes | everything in B plus a mode switch and both paths kept |
| What programs gain | nothing new; keeps what exists: Falcon-profile programs and the 16-bit memory-MFDB contract (ADR-024 point 4) | programs see the depth the framebuffer has: 24-bit colour fidelity, `vq_extnd` and `Physbase` agree; HighWire, zView, DOOM take their 32-bit branches | the 16-plane programs run unchanged **and** 32-capable programs get 32; direct-screen programs see one consistent mode per setting |
| Fidelity vs EmuTOS | stays measurable, equal to `falcontc` where measured (ADR-024); what is **not** faithful: 16 planes reported over a 32-bit store, `VgetSize` 32-bpp already, `Physbase` memory 32-bpp | no EmuTOS counterpart at all; the whole VDI depth contract is NewTOS's own; pixel comparison lost (section 3.1) | 16-plane mode keeps the oracle; the 32-plane mode has none |
| Speed | status quo: every memory-MFDB blit, `v_get_pixel`, flood-fill read and `bb_save`/`bb_restore` converts word to store word per pixel (`raster.rs:904@screen_to_mem`, `:956@mem_to_screen`, `vdi_backend.rs:945@read_row_rgb565`) | those copies could become plain 32-bit copies; save buffers double in size (1920x1080: 4 147 200 bytes as 16-bit, `tos-api/src/aes/gsxif.rs:354-355@bb_save_words`) [I]; [M] Fbench per ADR-036 point 5 | both costs |
| Thrown away | nothing | ADR-024 points 1, 4, 5; the 16-bit memory-MFDB contract; `eddi.rs` table; `present`'s hardwired 16 needs a patch | nothing is dropped; a second code path stays alive |
| Programs at risk | TeraDesk and friends keep working (they work today); the 17-entry `npc[]` is not hit | TeraDesk (`sdk/apps/teradesk/video.c:592`), `present`, hypview and any program that stamps `vq_extnd[4]` into an MFDB (the cap at `raster.rs:821@CHUNKY_PLANES`) | same, but only in the 32 setting |
| Roadmap fit (1.0 = the base like EmuTOS on the Pi, extensions after; `docs/MEMORY.md`, memory note, version roadmap) | fits 1.0: EmuTOS' truecolor is the 16-bit one | an extension beyond EmuTOS; against the 1.0 rule unless the owner calls it part of the base (the owner's 2026-11-18 statement in `TASK/V61.md`: "64K colours is not right", implement it "the way it really is") | an extension, with a 1.0-compatible default available |
| Open sub-choices | none | which 32-bit layout (fVDI's R 16..23/G 8..15/B 0..7 or another), what the X byte means, `v_get_pixel` answer shape (EmuTOS WORD vs fVDI two words), whether pens stay 5-bit quantised, what `VsetMode(-1)` answers | all of B plus how the mode is chosen (boot setting `cmdline.txt`/`config.txt`, a program call, at run time) |

Cross-effects worth stating once:

- The XBIOS23/XBIOS22 answers (`VsetMode(-1) = 0x001C`, `Getrez = 2`) are consistent with A and break nothing under B/C only if the
  mode word stays a Falcon word; the Falcon word has no 32-bit value (`include/biosdefs.h:146-154`). [C]
- Under B and C the F rows make NewTOS's VDI depth contract its own; the `COMPATIBILITY.md` rule "what a program observes
  must match" then has no oracle for it, so the document would carry the decision (section 2.3).
- TeraDesk's video dialog is tied to the Falcon mode word; see `docs/analysis/teradesk-video-pi-mode-2026-11-18.md` for that
  discussion (not repeated). [C]

## Open points

STOP-blocks (needs deeper analysis):

1. fVDI's 32-bit `vr_trnfm` bodies (`to_standard_32`, `references/fvdi/fvdi/engine/blit.s:1188` ff., and `:1477` ff.) were not read to the end; the 32-plane standard form (plane order, word order) is not established. Needed before any `vr_trnfm` spec at 32.
2. What the fVDI mode flag `0x80` ("Intel byte order", `references/fvdi/fvdi/drivers/aranym/spec.c:62`) means for a little-endian AArch64 store, and which of org `1` and `0x81` the NewTOS store would announce: not decided by the sources read.
3. Whether the X byte at 32 planes may remain the overlay-bit carrier (ADR-024 point 3, `kernel/src/hal/framebuffer.rs` header) or must become unused/alpha: a design choice with no source.

Inferred statements (labelled [I] above): gates `planes > 8` stay true at 32; the NEON kernels need no change; AES drawing is pen-based; `screen_to_mem`/`mem_to_screen` shrink to copies; save buffers double; TeraDesk out-of-bounds read and the blit no-op at the plane cap (read, not run); the projection comparison in 3.2.

To be measured [M]: no-op of 32-plane `vro_cpyfm` for TeraDesk, HighWire, hypview; whether the 16-bit projection reproduces oracle words for text, fills and lines; Fbench difference between the 16-bit seam and a plain 32-bit copy; zView `Applications/z-tools/zview/zvdi/color.c:75` path at 32 planes.

Out-of-scope observations:

- Today's `vq_extnd` says 16 planes while `Physbase` memory is XRGB8888 (`sdk/examples/physbase.c:19-25@XRGB8888`): a program writing the screen directly must already know it is on NewTOS; `docs/DEVIATIONS.md` has a row for `VgetSize` (line 48) but none for the 16-plane versus 32-bpp `Physbase` memory (grep for physbase, 16 planes, 32-bpp found only that row); a candidate row for the lead, not written here.
- `docs/api/vdi/eddi-vq_scrninfo.md` §6.1 cites fVDI's aranym driver (`spec.c`) as lines 11-86; the mode table read here is at `references/fvdi/fvdi/drivers/aranym/spec.c:71-80`; the content matches (R/G/B tables and org values), only the line range differs.
- HighWire and DOOM are not on the standard image; no manifest was available to confirm other cards.
