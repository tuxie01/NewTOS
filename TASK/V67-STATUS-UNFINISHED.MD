# V67 — receipt (developer): colour icons and the VT52 cell writer at 32 planes

Role: developer
Agent: acd789ee2164fbd81

Date 2026-11-18. Branch `v67` from `main` @ `04fe3f8f5` (worktree `target-agents/agent-acd789ee2164fbd81/wt`).
Spec: `docs/api/vdi/raster/_raster.md`, section "32-plane setting: AES colour icons, the AES inventory, the order of the
change (ADR-038 points 10 and 11, V67)" (L1-L12, I1-I11, §4 order, §5 change set) and `docs/api/bios/_vt52-32plane.md`
(C1-C7, §5 change set, §6 order); contract ADR-038 points 4, 10, 11 (`docs/MEMORY.md`); read with TASK/V67.md readings
1-6. One commit, no intermediate state (reading 6).

## What changed, by file and function

**Colour icons (L1-L11, one depth source).**

- `tos-api/src/vdi/raster/wide.rs`: new `pub fn device_form_pixel_bytes(planes)` (4 at `VDI_PLANES_WIDE`, else 2; the
  rule of `device_mem_planes`). `MemPx for u32::fetch_row`: the 16-to-32 expansion branch of V65 removed (L9); a
  `debug_assert` that a `u32` walk reads a 32-plane form; the unused `ROW_CHUNK` import dropped. Module doc (L10).
- `tos-api/src/vdi/raster.rs`: re-exports `device_form_pixel_bytes`; `blit_device_mem(backend, mem: *const u8, ...)`
  stamps the source form with `device_mem_planes(backend)` and picks the walk width from the form
  (`at_px!(src.nplanes)`, as `bb_restore`) (L8); doc comments of `CHUNKY_PLANES` and `blit_device_mem` (L10).
- `tos-api/src/vdi/vwk.rs`: free `pack_color32` beside the free `pack_color16`; `tos-api/src/vdi/mod.rs` re-exports
  it and `device_form_pixel_bytes` (L2).
- `tos-api/src/aes/resource.rs`: `CiconDevice.palette: [u32; 256]` (L1); `CiconDevice::live` packs with `pack_color32`
  at 32 and `pack_color16` at 16 (I2, I10; the no-backend fallback unchanged, I11); new `CiconDevice::pixel_bytes`
  (= `device_form_pixel_bytes(planes)`, I1); `cicon_pixel` returns the `u32` entry (L3; index, expansion, mask
  unchanged); `transform_cicon` writes to `*mut u8`, 4 bytes per pixel at 32, 2 at 16 (L4); `fix_cicons` pass 1 and 2
  `bytes = wd * 16 * rows * px` (L5; cursor and SELECTED offset follow). Section comment and doc comments.
- `tos-api/src/aes/object.rs` `draw_icon`: span `stride * 16 * px * h` with `px = device_form_pixel_bytes(
  backend.planes())` (L6), pointer `*const u8` (L7).
- Allocation (`kernel_cicon_alloc`, `cicon_alloc`): unchanged, as the spec says; only the doc of `CICON_BUF_ALIGN`.

So the loader's pixel store, the buffer bytes and cursor step, the drawer's span, the blit stamp and the palette entry
all read `backend.planes() == 32` (the loader through `CiconDevice.planes`, which `live` takes from the same backend).

**VT52 cell writer (C1-C7), `tos-api/src/vdi/escape.rs`.**

- `Con::word` returns the colour as `u32`: at 32 the phys workstation's pen entry as it is (the pen pixel, C2), before a
  workstation `rgb565_to_device(FALCON_DEFAULT_PALETTE[reg])` (C3, reading 4); at 16 the 16-bit word as before.
- `pen_word(pen) -> u16`: the 32 arm (`device_to_rgb565`) removed, the 16 arm stays (its unused backend argument dropped).
- New private trait `CellPx` (`u16`: `read_row_rgb565`/`write_row_rgb565`; `u32`: `read_device_rect`/`write_device_rect`
  one row high) and the generic row bodies `cell_rows`, `neg_rows` (`!` on the whole word, so all 32 bits at 32, C4,
  reading 5), `blank_rows` (`[T; 64]` chunks, C5). `cell_xfer`, `neg_cell`, `blank_out` pick `u32` at 32, `u16` else;
  flushes as before (C7). Scroll unchanged (C6). The four functions changed together.

**Tests.**

- `tos-api/src/aes/resource.rs` `cicon_tests`: the helpers take the `u32` palette and a byte buffer with a 64-byte
  canary behind what `fix_cicons` asked for (checked after every load); the 16-plane expectations are unchanged. New:
  `at_32_planes_the_buffer_is_twice_the_16_plane_one_and_nothing_is_written_past_it` (sizes pinned: `need` at 32 = 3
  images of `16*16*4` = twice the 16-plane `need`; col/sel/next offsets; `num_planes` stamped 32; masks stay the
  file's), `at_32_planes_each_pixel_is_the_32_bit_palette_entry_of_its_index` (same pen at every pixel as at 16, X = 0),
  `without_a_backend_the_live_device_keeps_the_16_bit_form` (I11).
- `tos-api/tests/aes_v67_cicon32.rs` (new, 4 tests): an NRSC v11 blob with a 1-, a 4- (with SELECTED image), an 8- and a
  4+16-plane chain, loaded through `relocate_and_register` (live device = the kernel `VdiScreen` transcription at
  16/32), drawn through `render_tree_lp64` `G_CICON`, pixels read back against pixels computed from the rules
  (best_match, expand, REV_MAP_COL, P3 pen pixel, mask then AND); the frame around each icon untouched; 16 against 32
  within 5 per gun except, as reading 2 says, the exact 16-plane variant's pixels under a clear mask bit (asserted to
  exist).
- `tos-api/tests/vdi_v65_memory_forms.rs`: `the_colour_icon_blit_reads_its_16_bit_form_at_32` replaced (reading 1) by
  `the_colour_icon_blit_reads_its_32_bit_form_at_32` (raw `u32` form, stride 32, `S_AND_D` on all 32 bits, zero words
  behind the form untouched) and `the_colour_icon_blit_reads_its_16_bit_form_at_16`.
- `tos-api/tests/vt52_console_32.rs` (new, 6 tests): glyph cells in pen pixels (greys 533/733: 136/187, not 140/189),
  reverse video, cursor `!` on all 32 bits and twice = identity, clear in the background pen pixel (also under reverse
  video), scroll with a blank new row, the 16-plane words, 16 against 32 within 5 per gun.
  `tos-api/tests/vt52_console_32_default_palette.rs` (new, 2 tests, no workstation ever opened): C3, and the boot
  console bit for bit equal at 16 and 32.
- `tos-api/tests/vdi_v64_pens.rs` `the_vt52_console_takes_its_colours_from_the_pen_table`: its **32 arm** pinned V64's
  interim 5-bit projection ("until TASK/V67" in its doc) and failed by design; it now expects the pen pixels
  `pixel32([17,0,0])` / `pixel32([1000,1000,1000])` (C2). Its 16 arm is unchanged. The spec's change set does not list
  this test (see Out-of-scope observations).
- Not touched: `tos-api/tests/aes_cicon_draw.rs`, `tos-api/tests/vt52_console.rs`,
  `tos-api/tests/vt52_console_default_palette.rs` (the 16-plane bar; they pass unchanged).

Mutation checks (scratch, restored by copy): `live` forced to `pack_color16` at 32 fails 3 of 4 tests of
`aes_v67_cicon32`; `neg_cell` forced to the 16-bit seam at 32 fails the cursor test of `vt52_console_32`.

**Docs.**

- `docs/api/aes/object-form.md`: the `G_CICON` callout "NewTOS's one device form is the truecolor RGB565 one" now names
  the form of the setting's depth (reading 3); the German heading fragment "Ergänzung bei der Umsetzung" (line 360)
  translated ("Addition during implementation") in the same commit.
- `docs/api/aes/resource-fileselector.md`: **not touched** (reading 3; it is German from `:1843`; its §5.7/§5.9 text
  describes the 16-plane path, which stays). The spec section stands as the 32-plane statement.
- `docs/api/vdi/raster/_raster.md`, `docs/api/bios/_vt52-32plane.md`: anchor line numbers only (the moved code lines;
  checked: the files differ from `HEAD` only in `.rs:<n>@` numbers). `cargo xtask doc-check --fix-anchors` does not exist
  yet (TASK/TOOL49.md OPEN); remapped by a diff line map, each target read: `escape.rs:241@word` is `fn word`,
  `resource.rs:3069@palette` the field, `resource.rs:3357@bytes`/`:3384@bytes` pass 1/pass 2, `wide.rs:204@fetch_row`
  the `u32` impl. `vdi_v65_memory_forms.rs:709@the_colour_icon_blit_reads_its_16_bit_form_at_32` now lands on the doc
  comment of the replacing test that names it.
- `docs/analysis/deps/mapping.tsv` (ratchet): notes updated for `blank_out16`, `cell_xfer16`, `neg_cell`,
  `expand_cicondata`, `transform_cicon`, `transform_all_cicons`, `vro_cpyfm16`→`mem_to_screen`/`fetch_row`; new rows
  `blank_out16`→`blank_rows`, `cell_xfer16`→`cell_rows`, `cell_xfer16`→`word`, `neg_cell`→`neg_rows`,
  `transform_cicon`→`live`, `transform_all_cicons`→`device_form_pixel_bytes`, `gr_colourblit`→`blit_device_mem`,
  `set_color16`→`pack_color32`. `cargo xtask deps --check-mapping`: 1990 rows, all resolve.

## Statements the spec marks inferred, checked

- I1 "depth fixed before any AES call": `kernel/src/lib.rs:1393@set_planes` runs `vdi_backend::set_planes` from the boot
  arguments; its doc says before `init_framebuffer`. Read, not run.
- I3/I4 "n equals the 16-plane n", "same result as 16 destination planes": confirmed by host test (every pixel of the
  4- and 8-plane variants and the SELECTED image resolves to the same pen at 16 and 32).
- I5 consequence (exact 16-plane variant differs under a clear mask bit): confirmed by host test (differences found,
  only there).
- C3 "the 16 default words have bit 5 clear, X = 0": confirmed (test asserts `word & 0x20 == 0` for all 16).
- C4 complement X: as reading 5; the test sees `X = 0xFF` after one toggle, the identity after two.
- C6 "copy_rect op 3 is a raw 32-bit copy at 32": run on the host through the fixture, which compiles
  `kernel/src/raster_copy.rs` by path: the scrolled row equals the drawn raw words.
- §6 "the raw seam panics, not overruns": every slice handed to `read_device_rect`/`write_device_rect` is exactly
  `w * 1` words long.
- §4 order table (mismatch effects): not run; the change is one commit, the unit test's canary pins "no write past".

## Build, clippy, tests

- Baseline in this worktree at `04fe3f8f5` before any edit; after the change the same commands.
- `cargo xtask build --board rpi4`: OK; `--board qemu-virt`: OK (rpi4 recompiled `tos-api` and `kernel`, virt `kernel`).
  rustc warnings `kernel` 93 / `tos-api` 64, as the baseline. Built without `CARGO_TARGET_DIR`.
- `cargo clippy -p kernel --features board-rpi4` and `board-qemu-virt`: kernel 121 / tos-api 73 before and after.
  `cargo clippy -p tos-api --all-targets`: 118 warning lines before and after, per-file counts identical. **Delta 0.**
- Full `cargo xtask test` at the final code (worktree with `worktree-setup` links): exit 0, **2570 passed / 0 failed /
  7 ignored** (sum of the `test result:` lines; the baseline 2554 plus 16 new: 3 `cicon_tests`, 4 `aes_v67_cicon32`, 6
  `vt52_console_32`, 2 `vt52_console_32_default_palette`, and the V65 blit test replaced by two), no SKIPPED.
- `cargo xtask doc-check` on the three edited docs: 0 failures. Over all 75 docs (outside `docs/archive/`) citing a changed file: no failure into
  a changed file except the pre-existing `docs/analysis/v33-gdos-fonts-2026-11-06.md:36,60` (the `set_contrl_ptr` anchor at line 262 of `tos-api/src/vdi/mod.rs`,
  the symbol was at 285 at `HEAD` already).
- `cargo xtask status-check --new-only`: 0 rejected.

## Tester handoff

- **Host:** `cargo xtask test`; specifically `aes_v67_cicon32` (4), `vt52_console_32` (6),
  `vt52_console_32_default_palette` (2), `vdi_v65_memory_forms` (18), `vdi_v64_pens` (13), the `cicon_tests` of
  `tos-api` lib (11); the 16-plane bar `aes_cicon_draw`, `vt52_console`, `vt52_console_default_palette` unchanged.
- **Default boot (16 planes), nothing may change:** the V65 bar (`measurements/measurement-2026-11-18-v65-test/RUNINFO.TXT`):
  boot screendump hashes (rpi4 prefix `a3bd102e7f28`), the apitest console screendump (prefix `f79b85387b0e`),
  tosdump tables at default and `vdiplanes=16` against `main` without a changed verdict.
- **`vdiplanes=32` boot:** screendump pairs against the 16-plane boot of the same build, per gun within 5 (V64): the
  desktop, a dialog opened and closed (save and restore), the console (the apitest console screendump at default
  colours should equal the V65 hash `f79b85387b0e`: registers 0/15 are pens 0/255, identical pixels at both settings),
  and colour icons: launch TeraDesk (`DESKTOP.NAP`, its `cicons.rsc`, the only colour-icon resource on the image).
  State whether `cicons.rsc` holds 16-plane variants (reading 2). `tosdump/probes/icprobe.c` cannot read the 32-bit form
  (V69). Expected boot log `gsx_malloc ... len=8294400` at 1080p unchanged (V65).
- **Real-Pi4-only:** nothing specific; the console at 32 now goes through the kernel's raw rectangle seam (the NEON row
  copy of V59/V65) instead of the 16-bit row seam, so console speed at 1080p/32 is worth a look on the board.

## Proposed `docs/STATUS.md` line

2026-11-18 — V67: colour icons and the VT52 console at 32 planes. At `vdiplanes=32` the AES colour-icon device form is
the 32-plane form: the loader packs the palette to pen pixels (`pack_color32`) and stores 4 bytes a pixel, the buffer,
the cursor step, `draw_icon`'s span and the `blit_device_mem` stamp follow the same depth (one commit, no overrun);
V65's 16-to-32 expansion at the blit is gone. The VT52 cell writer writes raw 32-bit pen pixels through the raw
rectangle seam, the cursor complements all 32 bits, the boot console's default words are gun-replicated (same picture
at 16 and 32). 16 planes unchanged. Commit <hash>.

## Out-of-scope observations

- Spec inventory gap: `tos-api/tests/vdi_v64_pens.rs` `the_vt52_console_takes_its_colours_from_the_pen_table` (32 arm
  pinned the V64 interim) is not in `_vt52-32plane.md` §5; its 32 arm was moved to C2 here.
- `kernel/src/vdi_backend.rs:1127@set_planes` doc still says the device pixel stays 16-bit "until TASK/V63-V67".
- `tos-api/tests/aes_cicon_draw.rs` module doc names "NewTOS's 16-bit RGB565 memory-MFDB form" (true for its 16-plane double).
- `docs/api/aes/resource-fileselector.md` §5.7/§5.9 describe only the 16-plane icon path (file partly German).
- `vdi_planes32_common`: a test binary that switches the setting keeps the physical workstation's pen table of the depth
  it was opened at until the next `v_opnwk`; the new icon tests make one VDI call on their own workstation before drawing.
- `escape.rs` `pen_word` is now a plain truncation of the 16-plane entry; kept because the spec keeps its 16 arm.
- `cargo xtask build` fails when `CARGO_TARGET_DIR` is set (known); built without it.
