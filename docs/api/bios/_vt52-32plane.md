# VT52 console at the 32-plane setting: the cell writer (ADR-038 point 11, V67)

Spec gate of TASK/V67 (researcher, 2026-11-18). This file is the English companion of the VT52 spec
`docs/api/bios/_vt52.md` (German, not touched here: a pointer there would require translating the whole file in the
same commit). It covers only what changes in the cell writer when `vdiplanes=32`: the row format, the colours, the
cursor, the clear, the scroll. The escape automaton, the cursor logic, the colour registers and everything else of the
console are as in `docs/api/bios/_vt52.md` and do not depend on the depth.

**Contract source.** EmuTOS has no console path above 16 planes: the Falcon truecolor cell routines work on one
`UWORD` per pixel (`references/emutos/bios/conout.c:137-180,297-331,542-575`) and are selected by
`TRUECOLOR_MODE = (v_planes > 8)` (`references/emutos/vdi/vdi_defs.h:22`). At 32 planes ADR-038 in `docs/MEMORY.md`
(point 11: raster paths work on `u32` pixels; point 4: the pen pixel `0x00RRGGBB`; X unused) is the contract and the oracle
cannot decide (ADR-038 point 12). Labels: **[C]** read in the source cited, **[I]** inferred, **[N]** NewTOS contract
(no EmuTOS rule), **[calc]** arithmetic, **[M]** measured by the V65 tester
(`measurements/measurement-2026-11-18-v65-test/RUNINFO.TXT`). **At 16 planes nothing here applies** (section 8).

## 1. What the cell writer does today

The console state lives in `tos-api/src/vdi/escape.rs`; `kernel/src/fbcon.rs` owns no pixel code (its module doc: the
VT52 automaton, the colours and "the cell rendering live in the VDI's alpha layer"), so the writer is the only
console rendering site **[C]**. Four functions put pixels on the screen, all through the 16-bit row seam:

| Function | Anchor | Today |
|---|---|---|
| colour of a register | `tos-api/src/vdi/escape.rs:234@word` | the physical workstation's pseudo-palette entry of pen `REV_MAP_COL[reg]` while that workstation is open, else `FALCON_DEFAULT_PALETTE[reg]` (`tos-api/src/vdi/escape.rs:63@FALCON_DEFAULT_PALETTE`); returns `u16`; the entry goes through `pen_word` (`tos-api/src/vdi/escape.rs:884@pen_word`), which at 32 planes is `device_to_rgb565(pen)`, the top 5 bits per gun |
| glyph cell | `tos-api/src/vdi/escape.rs:257@cell_xfer`, row store `tos-api/src/vdi/escape.rs:273@write_row_rgb565` | per glyph row 8 `u16` words (`fg` or `bg`) through `write_row_rgb565` |
| cursor | `tos-api/src/vdi/escape.rs:280@neg_cell`, `tos-api/src/vdi/escape.rs:286@read_row_rgb565`, `tos-api/src/vdi/escape.rs:290@write_row_rgb565` | per row: read 8 words, complement, write |
| clear | `tos-api/src/vdi/escape.rs:298@blank_out`, `tos-api/src/vdi/escape.rs:311@write_row_rgb565` | rows of up to 64 `u16` words of the background colour |
| scroll | `tos-api/src/vdi/escape.rs:326@copy_rect`, `tos-api/src/vdi/escape.rs:340@copy_rect` | `copy_rect` with op 3 over the whole text area, then `blank_out` of one line |

The EmuTOS counterparts **[C]**: `cell_xfer16` writes `v_cel_ht` rows of 8 `UWORD`s (`conout.c:137-180`), the colours
being `phys_work.ext->palette[fg]`/`[bg]` when the workstation exists, else `falcon_default_palette[fg & 0xf]` (`:162-170`);
`neg_cell` complements every `UWORD` of the cell, 8 per row (`:305-313`), and a second call restores it, nothing is saved
(`:284-290`); `blank_out16` fills `rows * width` words with the background colour (`:542-575`); `scroll_up` and
`scroll_down` are `memmove` of whole cell rows followed by `blank_out` of one row (`:706-725,733-752`).

At the 32 setting the 16-bit seam works, but not at 8 bits per gun: `write_row_rgb565` expands each 5-bit gun back to 8
(`kernel/src/vdi_backend.rs:979@write_row_rgb565`, `word_to_device`), so every console pixel is a 5-bit-replicated colour
and differs from the pen pixel of V64 by up to 5 per gun (`docs/api/vdi/raster/_raster.md` P9) **[C]** for the seam,
**[calc]** for the difference.

## 2. Rules at 32 planes

| # | Rule | EmuTOS rule it generalises | Label |
|---|---|---|---|
| C1 | **Row format.** A console cell row is 8 raw 32-bit store words `0xXXRRGGBB` (the cell is 8 pixels wide at both fonts, `[bg; 8]` in `tos-api/src/vdi/escape.rs:257@cell_xfer`; EmuTOS `:137-180`), written through the raw rectangle seam `write_device_rect(x, y, w, h, &[u32])` (`tos-api/src/vdi/backend.rs:495@write_device_rect`: raw words, off-screen dropped, no flush, panics if `src` holds fewer than `w * h` words) and read through `read_device_rect` (`tos-api/src/vdi/backend.rs:469@read_device_rect`). No 16-bit seam is used at 32. One row per call or one cell per call is the developer's | `UWORD` rows | [N] |
| C2 | **Colour.** `fg` and `bg` are the registers `ch & 0x0f` as today (reverse video swaps them, `M_REVID`); each resolves to the **pen pixel** `0x00RRGGBB` of pen `REV_MAP_COL[reg]` in the physical workstation's table once it is open (`VwkExt.palette` holds the pixel at 32, `tos-api/src/vdi/vwk.rs:219@palette`; V64 P3, X = 0). No `pen_word`: the 5-bit projection disappears at 32 | `palette[fg]` (`conout.c:162-166`) | [N] |
| C3 | **Colour before the workstation exists** (the boot console, `kernel/src/fbcon.rs:34@init`): `FALCON_DEFAULT_PALETTE[reg]` expanded to a pixel by `rgb565_to_device`, i.e. each 5-bit gun replicated to 8 bits, X = 0 (the 16 words all have bit 5 clear **[calc]**, so X = 0). That is the pixel the 16-plane screen shows for the same word, so the boot console is the same picture at both settings | `falcon_default_palette[fg & 0xf]` (`conout.c:169-170`) | [I] (the choice; the alternative is in Question 1) |
| C4 | **Cursor.** `neg_cell` complements every raw word of the cell, all 32 bits (`!word`), 8 words per row, `cell height` rows; a second call restores the cell bit for bit; nothing is saved. EmuTOS complements all 16 bits of each `UWORD`, the overlay bit included (`conout.c:305-313`); the 32-bit analogue complements X too, which is harmless because every read masks X (V63 R5) | `*addr = ~*addr` | [I] |
| C5 | **Clear.** `blank_out` writes the background register's pixel (C2/C3) into the rectangle, never the reverse-video foreground (as at 16, `conout.c:557-565`), in chunks of at most the row buffer's length | `blank_out16` | [N] |
| C6 | **Scroll** is unchanged: `copy_rect` with op 3 is a raw-word copy of the text area at both settings (`kernel/src/raster_copy.rs:92@raster_op`, V63 spec section 2), followed by `blank_out` of one row | `memmove` + `blank_out` | [C] for EmuTOS, [I] for the copy at 32 (read, not run) |
| C7 | **Flush** after each cell or rectangle is `flush_cells` as today (`tos-api/src/vdi/escape.rs:247@flush_cells`); the raw seam does not flush | `conout.c` writes the screen memory | [C] |

**Why C4 must change together with C1/C2/C5.** If `cell_xfer` wrote 8-bit pen pixels but `neg_cell` stayed on the
16-bit seam, one cursor toggle would read the cell as 5-bit words and write it back replicated: a gun byte `0xBB`
(pen 8, `q(733) = 187 = 0xBB`) would come back as `0xBD`, and after two toggles the cell would not hold the colour the
glyph was drawn with **[calc: top 5 bits `10111`, replicated `(23 << 3) | (23 >> 2) = 189`]**. Today the cell holds only
5-bit-replicated values, so today's double toggle is the identity for console-written cells.

## 3. Sizes and the one source of the depth

- The cell is 8 pixels wide (`cw = 8` at both console fonts: 8x8 under 400 lines, 8x16 above, `tos-api/src/vdi/escape.rs:173@con_font`
  and `kernel/src/fbcon.rs` module doc), so a row buffer is `[u32; 8]` and a cell `8 * cell height` words (at most 128).
- `blank_out`'s row chunk is `[u32; 64]` at 32 (256 bytes), `[u16; 64]` at 16.
- The depth is read from `backend.planes() == 32` (`VDI_PLANES_WIDE`), the same source as every other 32-plane rule; the
  depth is fixed at boot (`kernel/src/lib.rs:1393@set_planes`), so the arm cannot change between a draw and its cursor
  toggle.

## 4. The 16-plane paths this slice does not touch (ADR-038 point 11)

| What | Why it stays |
|---|---|
| the `u16` rows, `write_row_rgb565`/`read_row_rgb565`, `FALCON_DEFAULT_PALETTE` as `u16` words, `pen_word`'s 16 arm | the default setting; the V65 console screendump (prefix `f79b85387b0e`) and the 16-plane console host tests (`tos-api/tests/vt52_console.rs`) are the bar |
| `scroll_up`/`scroll_down`, the automaton, the colour registers, `ESC b/c`, `M_REVID` logic | not depth dependent |
| `kernel/src/fbcon.rs` | owns no state or pixel code |

## 5. The change set for the developer

| Function | Anchor | What changes at 32 | Slice |
|---|---|---|---|
| `Con::word` | `tos-api/src/vdi/escape.rs:234@word` | returns the colour as a pixel (`u32`): at 32 the pen pixel or the expanded default word (C2, C3), at 16 the word as today; no `pen_word` at 32 | V67 |
| `pen_word` | `tos-api/src/vdi/escape.rs:884@pen_word` | its 32 arm (`device_to_rgb565`) goes away; the 16 arm stays | V67 |
| `cell_xfer` | `tos-api/src/vdi/escape.rs:257@cell_xfer` | `[u32; 8]` rows through `write_device_rect` at 32 | V67 |
| `neg_cell` | `tos-api/src/vdi/escape.rs:280@neg_cell` | raw read, `!`, raw write at 32 | V67 |
| `blank_out` | `tos-api/src/vdi/escape.rs:298@blank_out` | `[u32; 64]` chunks through `write_device_rect` at 32 | V67 |
| tests | `tos-api/tests/vt52_console.rs`, the 32-plane fixtures of `tos-api/tests/vdi_planes32_common/mod.rs:167@Screen` | a 32-plane console test: glyph pixels equal the pen pixels, a double cursor toggle is the identity over non-5-bit colours, blank and scroll; the 16-plane tests stay | V67 |

## 6. Order of the change

`Con::word`, `cell_xfer`, `neg_cell` and `blank_out` change in **one commit** (section 2, "Why C4 must change
together"). There is no buffer shared with another site: the row arrays are local to each function, and the raw seam
panics, not overruns, when a slice is shorter than `w * h` (`tos-api/src/vdi/backend.rs:469@read_device_rect`); the V62
overrun hazard does not exist here. The change is independent of the colour-icon commit
(`docs/api/vdi/raster/_raster.md`, section "32-plane setting: AES colour icons ... (V67)" §4) and needs only what V64
(pen pixel) and V59 (the raw rectangle seam, `read_device_rect`/`write_device_rect`) already provide.

## 7. Acceptance of TASK/V67, the console part (criteria not reworded)

| Criterion (the console clause of "the desktop, a dialog ..., colour icons and the console show the same picture as at 16 (screendump pairs, within the pen tolerance of V64)") | Testable with this spec? |
|---|---|
| console picture at 32 equals 16 within the pen tolerance | **Yes.** By construction the default colours are identical: register 15 is pen 255, whose pixel is 0, and register 0 is pen 0, `0x00FFFFFF` at 32 and `0xFFDF` expanded to `0x00FFFFFF` at 16 (`tos-api/src/vdi/escape.rs:74@REV_MAP_COL`, V64 P6), so the apitest console screendump hash of V65 (prefix `f79b85387b0e`, equal at four configurations **[M]**) is the bar for default colours; other registers (`ESC b/c`) differ by at most 5 per gun (P9). A host test pins C1-C5 at 32 planes |
| the cursor | Yes: C4, a double toggle is the identity (host test with a non-5-bit colour) |
| "at 16 planes nothing changes" | Yes: section 4 |

## Open points

- **Question 1 (colour before the workstation exists, C3).** My reading **[I]**: the 16 default words are expanded by gun
  replication (`rgb565_to_device`), so the boot console shows the same pixels at both settings. The alternative is to derive
  an 8-bit pixel from the default pen triples by the P3 rule; ADR-038 names no rule for `falcon_default_palette`. The
  difference is at most 5 per gun and exists only until the first workstation opens. The lead confirms.
- **Question 2 (X byte under the cursor, C4).** My reading **[I]**: complement all 32 bits. Alternative: complement the 24
  colour bits only. No read path can tell them apart (V63 R5); a raw read of the store through `Physbase` could.
- **Inferred:** every **[I]** line; that `copy_rect` op 3 is a raw 32-bit copy at 32 is read from the V63 spec and
  `kernel/src/raster_copy.rs`, not run; the pixel colours of the default screen are arithmetic over V64 P3/P6.
- **Not executed:** every "today" statement is reading; the V65 run (item 7 there) is the only measurement quoted.
- **Out-of-scope observations:** `docs/api/bios/_vt52.md` section 5 and decision D1 describe the 16-bit path only; they are
  German and untouched. The console font, blink clock and the other deliberate deviations (`tos-api/src/vdi/escape.rs`
  module doc) do not depend on the depth.
