# V64 — receipt (developer): pens with 8 bits per gun at 32 planes, flood-fill compare on the masked 24-bit pixel

Role: developer
Agent: agent-a483993e3a366927a

Date 2026-11-18. Branch `v64` from `main` @ `77ebc02b2` (worktree `target-agents/agent-a483993e3a366927a/wt`).
Spec: `docs/api/vdi/raster/_raster.md` § "32-plane setting: pens, and every place a pen becomes a pixel (ADR-038
point 4, V64)" (rules P1-P9, the change-set table §3, flood fill F1-F5 §4), with the pointers in
`docs/api/vdi/attributes/_mode-clip-color.md`, `docs/api/vdi/inquiry/_color-pixel-input.md`,
`docs/api/vdi/output/_fill.md`; contract ADR-038 points 4 and 11 (`docs/MEMORY.md`); read with TASK/V64.md "Spec gate
... and the lead's reading of its open points" (slice cut points 1-4).

## Changes (the change-set table's V64 rows)

- **Workstation pen table** `tos-api/src/vdi/vwk.rs:219@palette`: `VwkExt.palette` is `[u32; 256]`, each entry in the
  form of the device depth: the 16-bit word `RRRRRGGGGG0BBBBB` at 16 planes (unchanged value), the pixel `0x00RRGGBB`
  at 32. The 16-plane functions (`vdi2fivebits`, `fivebits2vdi`, `set_color16`, `pack_color16`, `actual16`, free
  `pack_color16`) are unchanged in behaviour (two lines touched for the wider slot). New, in a second `impl VwkExt`
  at the end of the file: `tos-api/src/vdi/vwk.rs:443@vdi2eightbits` (P2), `:451@eightbits2vdi` (P5),
  `:458@pack_color32` (P3, clamps to 0..1000), `:468@set_color32`, `:475@actual32`, and the depth dispatchers
  `:483@set_color` / `:494@actual` (`planes == 32` → 8-bit, else 5-bit; R6's gate).
- **Trait** `tos-api/src/vdi/backend.rs:126@set_active_palette` takes `&[u32; 256]` (slice cut point 1); new constant
  `tos-api/src/vdi/backend.rs:736@VDI_PLANES_WIDE` (= 32). The dispatcher call site (`tos-api/src/vdi/mod.rs`
  `set_active_palette(&ext.palette)`) is unchanged code. `pel_to_device16`'s doc names the 32-plane interim.
- **`vs_color`** `tos-api/src/vdi/attributes.rs:511@set_color`: the clamped copy goes through `VwkExt::set_color`
  (P1-P3); raw `req_col` store and index check unchanged.
- **`vq_color`** `tos-api/src/vdi/inquiry.rs:114@actual`: flag != 0 through `VwkExt::actual` (P5); flag 0 and the range
  check unchanged (P4).
- **`init_wk`** `tos-api/src/vdi/workstation.rs:265@set_color`: pens 0..254 through `VwkExt::set_color`, pen 255 left 0 (P6).
- **Flood fill** `tos-api/src/vdi/contour.rs:164@pixelread` returns `u32`: at 32 planes the masked 24-bit pixel
  `store & 0x00FF_FFFF` (F1), at 16 the 16-bit word as before, palette mode the PEL. `Flood.search_color` is `u32`;
  `Flood::color` (`tos-api/src/vdi/contour.rs:257@color`) masks `OVERLAY_BIT` only when `mask_overlay`
  (`:232@mask_overlay`: truecolor and not 32 planes; F2). The pen search colour is the physical workstation's
  entry, which at 32 is the pen pixel (F3, `:493@truecolor`). The queue sentinels stay 16-bit (F4).
- **`v_get_pixel`** `tos-api/src/vdi/inquiry.rs:190@v_get_pixel`: at 32 planes `INTOUT[1] = device_to_rgb565(masked
  pixel)`, i.e. V63's interim answer, now from the 24-bit value (F5; slice cut point 3). 16 planes unchanged.
- **VT52 colour source** (slice cut point 2) `tos-api/src/vdi/escape.rs:234@word` takes the backend and reads the pen
  entry through `tos-api/src/vdi/escape.rs:884@pen_word`: at 16 planes the word as before, at 32 the pen pixel's top 5
  bits per gun for the unchanged 16-bit cell writer (V67).
- **Kernel** `kernel/src/vdi_backend.rs`: `ACTIVE_PALETTE` is `[AtomicU32; 256]`; new at the end of the file
  `vdi2eightbits`, `pack888`, `pen_entry` (the entry at the setting's depth, used by `ensure_palette_init` and
  `set_color`: "keep the two tables consistent"), `pen_word16` (`pel_to_device16`: the word at 16, at 32 the pixel's
  top 5 bits per gun until V65) and `pen_pixel` (the device pixel of an entry: `word_to_device` at 16, the entry itself
  at 32), used by `pel_to_rgb` and `read_pixel` (the reverse map compares with the 24-bit pen pixels, same tie rule).
  At 16 planes every value is the one before (`pack555` widened, `entry as u16` back).
- **Test doubles** of the trait (6 in `tos-api/src`, 69 in `tos-api/tests`): the signature only; the 44 that store the
  table store `palette.map(|p| p as u16)` (they run at 16 planes, where every entry is a 16-bit word). No expectation
  of any of them changed.
- **Tests**: new `tos-api/tests/vdi_v64_pens.rs` (13 tests). The V63 fixture (the kernel's `VdiScreen` over a host
  store, the transcription of the kernel's pen step) moved unchanged into `tos-api/tests/vdi_planes32_common/mod.rs`,
  shared by both binaries, with the V64 transcription (`pen_pixel`, `pel_to_device16`/`pen_word16`, `read_pixel`) and
  the spec formulas written independently of the code (`q8`, `a8`, `q5`, `a5`, `pixel32`, `word16`).
- **V63 test, one 32-plane expectation changed by the spec:** `fills_lines_and_text_draw_the_pen_pixel_with_x_zero`
  asserted that the 32-plane pen scene equals the 16-plane one. That was V63's interim ("V64 replaces the quantisation",
  `_raster.md` V63 §3) and contradicts P3/P9 (pen 8 is `0x00BBBBBB` at 32, `0x00BDBDBD` at 16). It now asserts the
  32-plane pixel by the 8-bit rule and P9 against 16 (within one 5-bit step per gun, exact where a gun is 0 or 1000).
  Its 16-plane expectations are unchanged; the other six V63 tests are unchanged and pass.
- `docs/analysis/deps/mapping.tsv` (ratchet): new rows `vdi2fivebits → vwk.rs vdi2eightbits`, `fivebits2vdi →
  eightbits2vdi`, `set_color16 → vwk.rs set_color32`, `set_color16 → kernel pen_entry`; notes of `vs_color16`,
  `vq_color16`, `init_colors` (both rows), `set_color`, `pixelread`, `end_pts16`, `vdi_v_get_pixel`, `cell_xfer16`
  name the 32-plane rule.

## Slice cut point 4: the overlay-bit mask at 32 planes (researcher's inference F2) — checked

- EmuTOS masks `OVERLAY_BIT` (`0x0020`, `references/emutos/vdi/vdi_fill.c:26`) on 16-bit `UWORD`s only: the search
  colour and each screen word in `search_to_right16`/`search_to_left16` (`references/emutos/vdi/vdi_fill.c:702-709`,
  `:726-733`) and the start word in `end_pts16` (`:755`), whose result is compared with the unmasked `search_color`
  (`:763`). All of it sits under `TRUECOLOR_MODE` with a 16-bit word; there is no path above 16 planes
  (`references/emutos/vdi/vdi_defs.h:22`).
- In NewTOS's 32-plane pixel the layout is `(r << 16) | (g << 8) | b` (`kernel/src/hal/framebuffer.rs:36@rgb`), so
  `0x20` is bit 5 of blue. The overlay information of 16 planes lives in the X byte, which `pixelread` already masks at
  32 (V63 R5); nothing at 32 carries an overlay bit to mask.
- So the inference holds: applied at 32 the mask would make pens that differ only in blue bit 5 compare equal (`(0,0,0)`
  = `0x000000` and `(0,0,125)` = `0x000020`, `q(125) = 32`). The code gates it off at 32 (`mask_overlay`) and keeps it
  at 16. Test `the_overlay_bit_mask_does_not_run_at_32_planes` pins it; with the mask on at 32 (mutation) that test fails.
- No spec error found anywhere in the slice; no stop condition hit.

## Rules against tests (`tos-api/tests/vdi_v64_pens.rs`)

| Rule | Test | At 32 planes | At 16 planes |
|---|---|---|---|
| P2, P5 documented values | `the_documented_values_of_both_formulas_at_32_planes` | `q(0,1,2,500,999,1000) = 0,0,1,128,255,255` (drawn pixel); `a(0,1,128,254,255) = 0,4,502,996,1000` (`vq_color` flag 1) | — |
| P1-P5 over the request range | `vs_color_and_vq_color_over_the_request_range` | every `c` 0..1000 of each gun: pixel = `pixel32`, flag 0 = request, flag 1 = `a8(q8(c))`, `\|a(q(c)) - c\| <= 2` | the 5-bit rule of EmuTOS for the same 3003 requests (regression) |
| P5 `q(a(q)) = q` | `the_realised_value_round_trips_every_gun_byte_at_32_planes` | all 256 bytes | — |
| P1, P4 raw request | `an_unclamped_request_is_answered_raw_and_drawn_clamped` | `(-50,1500,0)` → flag 0 raw, flag 1 `(0,1000,0)`, pixel `0x0000FF00`; `i16` extremes | same |
| error input | `the_error_input_of_vs_color_and_vq_color_is_unchanged` | `vs_color(-1/256/MIN/MAX)` changes no request and no pen; `vq_color` there `(-1,0,0,0)`, 4 outputs, both flags | same |
| P6, P7 | `the_default_pens_at_32_planes` | pens 0, 1, 2, 8, 9, 10 = `0x00FFFFFF`, 0, `0x00FF0000`, `0x00BBBBBB`, `0x00888888`, `0x00AA0000`; all 256 default pens of the fixture, pen 255 = 0; realised = requested for every pen and for the 16 component values | pen 8 reads back `742` (5-bit) |
| P8 | `vs_color_changes_only_the_calling_workstation_at_32_planes` | the other workstation keeps its pen | — |
| P9, F5 | `pens_at_16_and_32_planes_agree_within_one_5_bit_step` | every request of every gun: top 5 bits per gun within 1 of 16 planes, bound 1 reached at `c = 17`; `v_get_pixel` = `device_to_word(pixel)` | `v_get_pixel` = the word |
| test pens, `read_pixel` | `the_test_pens_are_one_colour_at_16_and_two_at_32_planes` | `(17,0,0)`, `(25,0,0)` → `0x00040000`, `0x00060000`; reverse map gives the right pen | both `0x0800`; reverse map gives the first |
| F1-F3, ticket acceptance | `a_contour_fill_stops_at_a_pen_differing_in_the_low_3_bits_at_32_planes` | boundary fill (ring pen as `INTIN[0]`, physical workstation's pixel) and fill-while-same fill exactly the inside | boundary fill draws nothing, fill-while-same runs over the ring to the cleared screen |
| F2 | `the_overlay_bit_mask_does_not_run_at_32_planes` | `0x000000` ring, `0x000020` inside: both fill modes fill exactly the inside | — |
| F5 | `v_get_pixel_keeps_the_interim_answer_at_32_planes` | both test pens answer `(0, 0x0000)`, X noise masked | both `(0, 0x0800)` |
| slice point 2 | `the_vt52_console_takes_its_colours_from_the_pen_table` | glyph pixels = pen 255's pixel through its top 5 bits per gun (`(17,0,0)` → 0) | pen 255's word (`0x0800` → `0x00080000`) |

Mutation check (scratch edits, each restored from a copy; the restore verified by `git diff --stat`): the 8-bit branch
of `VwkExt::set_color` off → 11 of 13 fail; the `OVERLAY_BIT` mask on at 32 → `the_overlay_bit_mask_does_not_run…`
fails; `pixelread` at 32 back to the 16-bit projection → the contour, `v_get_pixel` and P9 tests fail; `vq_color` back
to `actual16` → 5 fail; `init_wk` back to `set_color16` → `the_default_pens…` fails; `pen_word` without its 32-plane
branch → the VT52 test fails.

## Build, clippy, tests

- `cargo xtask build --board rpi4`: OK, `kernel` 93 / `tos-api` 64 rustc warnings (baseline the same, built at
  `77ebc02b2` in this worktree). `cargo xtask build --board qemu-virt`: OK, 93 / 64. Each after `touch
  kernel/src/lib.rs` ("Compiling kernel" seen once per build), with the final code. Built without `CARGO_TARGET_DIR`.
- Clippy, baseline at `77ebc02b2` in this worktree before any edit: `cargo clippy -p kernel --features board-rpi4`
  kernel 121 / tos-api 73, `--features board-qemu-virt` 121 / 73; after: the same, and the per-file warning locations
  (file, count) identical. `cargo clippy -p tos-api --lib --tests`: 261 before, 261 after, the per-target counts
  identical; the new binary `vdi_v64_pens` and the fixture module have none. **Delta 0.**
- `cargo test -p tos-api --test vdi_v64_pens`: 13 passed; `--test vdi_v63_device_pixel`: 7 passed.
- Full `cargo xtask test` (worktree with `worktree-setup` links, final code): exit 0, **2527 passed / 0 failed /
  7 ignored** (sum of the `test result:` lines; the baseline 2514 plus the 13 new), no SKIPPED.
- `cargo xtask doc-check --staged`: 0 broken by this commit. The new code sits at the ends of `vwk.rs`, `backend.rs`,
  `escape.rs` and `vdi_backend.rs` and the edited blocks keep their line counts, so the ±5 anchors into these files in
  `docs/`, `TASK/` and `docs/audits/` still hold (a first layout broke 36 and was rearranged, no doc edited for it).
  `cargo xtask deps --check-mapping`: 1972 rows, all resolve. `cargo xtask status-check --new-only`: 1 checked, 0 rejected.

## Tester handoff

- **Host:** `cargo xtask test`; the new `vdi_v64_pens` (13) and `vdi_v63_device_pixel` (7, one 32-plane expectation
  updated as above), `vdi_reference_overlay_bit` (the 16-plane V11/V12 contract), the `contour.rs` unit tests.
- **Default boot (16 planes), "at 16 planes nothing changes":** every 16-plane value is the one before (the entry is the
  same 16-bit word in a `u32`). Expected: no changed verdict in the VDI case runs against `falcontc` (the tables of the
  last VDI acceptance, among them `vdi-all.tsv` `vs_color`/`vq_color`/`v_get_pixel`/`v_contourfill` cases and
  `tosdump/cases/vdi-v18.tsv`), and screendumps of a default boot byte-identical to `main`.
- **`vdiplanes=32`** (QEMU `-append`, `cmdline.txt`): boot to the desktop. Acceptance (ii), 16 against 32 on the same
  scenes: compare per gun on the top 5 bits, not whole words (bit 5 of the projected word differs on a cleared screen
  by V63's design). [calc] For all 16 component values of the default palette `q8(c) >> 3 == q5(c)`, so every default
  pen projects exactly (difference 0); the 8-bit values differ (e.g. pen 8 `0xBB` at 32 against `0xBD` at 16). A pen
  set by a program can differ by one step (bound 1, reached at `c = 17`); record the measured tolerance.
- **Contour fill at 32 through tosdump:** the DUMPVDI table cannot show a region (every case starts with `v_clrwk`,
  `docs/analysis/v64-contourfill-dumper-extent-and-acceptance-2026-11-18.md` §1); the region is pinned by the host
  tests above, or by the `vdprobe` K8 cases with a grabbed rectangle. The tosdump caps at 16 planes are not lifted
  (later slice).
- `v_get_pixel` at 32 planes after `v_clrwk` still answers `0, 0xFFDF` (interim, V65 replaces it).
- **Real-Pi4-only:** nothing specific to this slice (both boards ignore the X byte; the pen pixel is the store word).

## Proposed `docs/STATUS.md` line

2026-11-18 — V64: pens with 8 bits per gun at 32 planes. At `vdiplanes=32` `vs_color` builds the pen pixel
`0x00RRGGBB` with `(c*255+500)/1000`, `vq_color` realised values are `(q*1000+127)/255`, the 256 default pens read back
exactly; the workstation pen table and `set_active_palette` are 32-bit, the kernel's active palette follows the
setting; `v_contourfill` compares the masked 24-bit pixel without the 16-plane overlay mask, so it separates pens that
differ only in the low bits of a gun; `v_get_pixel` keeps V63's interim answer; the VT52 console takes the pen pixel's
top 5 bits until V67. 16 planes unchanged. New host test `vdi_v64_pens`, shared fixture `vdi_planes32_common`.
Commit <hash>.

## Out-of-scope observations

- `cargo clippy -p kernel --features board-…` in a fresh worktree fails in the kernel build script until `cargo xtask
  build-programs --board …` (or `cargo xtask build`) has run once (`kernel/build.rs` panics on the missing
  `NEWDESK.NAP`); the baseline was therefore taken after the baseline builds.
- `cargo xtask build` fails when `CARGO_TARGET_DIR` is set (known, as in V63's receipt); built without it.
- The `_raster.md` change-set table's "Today" column (§3) now describes the code before V64; the researcher may move it.
- The kernel's device default for pens 16..254 at 32 planes (`ensure_palette_init` over `VIDEL_PALETTE2`) is not
  host-testable (the kernel is not host-linkable); P7 for all 256 pens is pinned through the 16 component values and
  the fixture's pens 0..15.
- `tos-api/src/aes/resource.rs` `CiconDevice::live` still builds its device palette with `pack_color16` (V65 row of the
  change-set table); untouched.
- Several test files and the repository in general are not rustfmt-clean (long lines); left alone.
