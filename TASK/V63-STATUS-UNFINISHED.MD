# V63 — receipt (developer): the 32-plane device pixel, X byte unused, reads masked at 32 planes

Role: developer
Agent: agent-a0892229b99b3d8ff

Date 2026-11-18. Branch `v63` from `main` @ `fd0b002ec` (worktree `target-agents/agent-a0892229b99b3d8ff/wt`).
Spec: `docs/api/vdi/raster/_raster.md` § "32-plane setting: the device pixel (ADR-038 point 4 and 11, V63)"
(rules R1-R6, site table §2, §3, boundary table §4), `docs/api/vdi/workstation/03-v_clrwk.md`,
`docs/api/vdi/inquiry/_color-pixel-input.md`, `docs/api/vdi/output/_fill.md` (their "32-plane setting" sections);
contract ADR-038 points 4 and 11 (`docs/MEMORY.md`). Read with TASK/V63.md "Spec gate ... and the lead's reading of
its open points" (1: the seed read is masked at 32; 2: `v_get_pixel`'s answer shape stays; 3: the cleared pixel
`0xFFFFFFFF` stands, tests never assert its X byte).

## Changes

- `tos-api/src/vdi/contour.rs:164@pixelread` (the one site the spec marks **change**): for `planes() > 8` the raw
  store word from `get_device_pixel_wrapped` is masked with `0x00FF_FFFF` before `device_to_rgb565` **only when
  `planes() == 32`** (R5, gated as R6 asks); at 16 planes the word goes through unmasked as before. This one read
  serves `v_get_pixel` (`tos-api/src/vdi/inquiry.rs:190@v_get_pixel`), the flood-fill seed read
  (`tos-api/src/vdi/contour.rs:477@pixelread`) and every span probe (`tos-api/src/vdi/contour.rs:255@color`). Doc
  comment of `pixelread` and the seed comment in `v_contourfill` name the 32-plane mask.
- `kernel/src/hal/framebuffer.rs` module doc (comment only, site table row "comment only"): X carries
  `OVERLAY_BIT` at 16 planes and is unused at 32 (pens write 0, every VDI read masks it). Same line count.
- `tos-api/tests/vdi_v63_device_pixel.rs` (new, 7 tests, own binary because `install_host` takes one host per
  process): the backend is the kernel's `VdiScreen` over a host store with a pitch wider than the width — the
  kernel's draw kernels are compiled in by path (`kernel/src/raster_kernel.rs`, `raster_line.rs`, `raster_expand.rs`,
  `raster_copy.rs`, as the `raster_kernel_*` tests do) and called as `kernel/src/vdi_backend.rs` calls them; the
  pixel conversions (`word_to_device`, `device_to_word`, `pel_to_rgb`, `xor_pixel` via `combine_index`,
  `clear_screen_word`, `read_pixel`) are transcribed with anchors (the precedent is
  `tos-api/tests/vdi_reference/overlay_bit.rs`). `planes()` is settable, so each test runs both settings.
- `docs/analysis/deps/mapping.tsv`: the note of the `pixelread → pixelread` row names the 32-plane mask (ratchet).

The sites the spec marks **keep** were not edited: `pel_to_rgb`, `xor_pixel`, `raster_op`, `v_clrwk` /
`clear_screen_word`, `color`, `palette`, `v_get_pixel`, `read_pixel`, `get_device_pixel(_wrapped)`,
`word_to_device`/`device_to_word`. Each one that is reachable by test meets its rule at 32 planes (table below);
no stop condition was hit.

## Rules against tests

| Rule | Test (`tos-api/tests/vdi_v63_device_pixel.rs`) | At 32 planes | At 16 planes |
|---|---|---|---|
| R1, R2 | `fills_lines_and_text_draw_the_pen_pixel_with_x_zero` | `vr_recfl` with pens 0-16, 100, 255, `v_pline`, `v_gtext` (replace): every drawn pixel = `word_to_device(pen word)`, X = 0 | same pixels; the whole scene equals the 32-plane one (colour bits for every pixel, the full word for every drawn pixel) |
| R3, R5 | `a_cleared_screen_reads_as_the_white_pen_at_32_planes` | after `v_clrwk` the colour is `0x00FFFFFF` and `v_get_pixel` answers `(0, 0xFFDF)` = a pen-0 pixel (interim answer, `_color-pixel-input.md`) | `(0, 0xFFFF)`, one `OVERLAY_BIT` from pen 0 `0xFFDF` (EmuTOS) |
| R5 | `v_get_pixel_ignores_the_x_byte_at_32_planes` | for 7 colours × 8 X values the answer equals the one with X = 0 | the top bit of X comes back as `OVERLAY_BIT` (unmasked read) |
| R5 (span probes, seed) | `the_flood_fill_ignores_x_noise_at_32_planes` | a white region with random X noise and a seed whose X is `0xA5`, inside a black frame: fill-while-same and fill-to-boundary both fill exactly the region | fill-while-same draws nothing (unmasked seed, `vdi_fill.c:972`), fill-to-boundary fills the region |
| R5 (seed, lead point 1) | `a_seed_fill_on_a_cleared_screen_fills_at_32_and_draws_nothing_at_16` | `v_contourfill(-1)` on a cleared screen fills the clip rectangle | draws nothing (EmuTOS) |
| R4 (`WM_XOR`) | `wm_xor_complements_all_32_bits_and_ignores_the_pen` | `vr_recfl` and `v_pline` in XOR over random 32-bit words: every pixel `!before`, pen ignored; twice restores | same |
| R4 (16 ops) | `the_sixteen_logic_operations_are_bitwise_on_32_bits` | the kernel's `raster_op` against an independent bit-by-bit table for all 16 ops on random words, `ALL_BLACK` = `0xFFFFFFFF`, `ALL_WHITE` = 0; the kernel's `copy_rect` for all 16 ops over a store with X noise | (the kernel op does not depend on the setting) |

Mutation check (scratch edit, restored by copy): with the mask disabled (`planes == 0` instead of `== 32`) the four
read tests fail (`a_cleared_screen…`, `v_get_pixel_ignores…`, `the_flood_fill_ignores…`, `a_seed_fill…`) and the
three keep-site tests stay green, so the tests catch the change and pin the keep sites independently.

## What a `vdiplanes=32` boot does now (interim, before V64/V65)

- Pens draw the same 24 colour bits as at 16 planes with X = 0; `v_clrwk` still writes `0xFFFFFFFF` (R3, allowed).
- `v_get_pixel` answers `INTOUT[0] = 0`, `INTOUT[1] =` the 16-bit word of the masked pixel: a cleared screen `0xFFDF`
  (not `0xFFFF`), any VDI-drawn pixel losslessly, other pixels with the low 3 bits of each gun lost (lead point 2;
  V65 replaces the shape).
- `v_contourfill` treats a cleared background as the white pen: a fill-while-same seeded on it fills.
- `vro_cpyfm` / `vrt_cpyfm` still refuse a 32-plane screen (`tos-api/src/vdi/raster.rs:85@CHUNKY_PLANES`, V65): R4 for
  the sixteen operations is therefore pinned on the kernel's `raster_op`/`copy_rect` (also the AES window-move path,
  `tos-api/src/aes/window.rs:2085@copy_rect`), not through the VDI call.

## Build, clippy, tests

- `cargo xtask build --board rpi4`: OK, `kernel` 93 rustc warnings (as V62). `cargo xtask build --board qemu-virt`:
  OK, 93. Each after `touch kernel/src/lib.rs` ("Compiling kernel" seen), with the final code.
- Clippy, baseline at `fd0b002ec` in this worktree (changed files replaced by their `HEAD` copies, the new test moved
  aside, then put back): `cargo clippy -p kernel --features board-rpi4` kernel 121 / tos-api 73, `--features
  board-qemu-virt` 121 / 73, before and after; `cargo clippy -p tos-api --lib --tests` 261 warnings over the same
  set of targets before and after; the new test binary has none. **Delta 0.**
- `cargo test -p tos-api --test vdi_v63_device_pixel`: 7 passed; `--test vdi_reference_overlay_bit` (the 16-plane
  V11/V12 contract): 4 passed.
- Full `cargo xtask test` (worktree with `worktree-setup` links, final code): exit 0, 2514 passed / 0 failed /
  7 ignored (sum of the `test result:` lines; V62's 2507 plus the 7 new), no SKIPPED.
- `cargo xtask doc-check --staged`: 0 broken by this commit (277 cites of the 3 changed paths in docs the commit
  leaves alone). The edit to `contour.rs` is kept inside the ±5-line anchor tolerance of the cites in
  `docs/api/vdi/**`, `docs/COMPATIBILITY.md` and `TASK/V21.md`/`V38.md`/`TOOL32.md`, so no doc needed an anchor
  change (`--fix-anchors`, TASK/TOOL49.md, does not exist yet). `cargo xtask deps --check-mapping --staged`: 0
  problems.

## Tester handoff

- Host: `cargo xtask test`; the new `vdi_v63_device_pixel` (7) and the unchanged `vdi_reference_overlay_bit` (4),
  `contour.rs` unit tests (brake, V38).
- Default unchanged (acceptance 2, "same bar as V62"): the code path at 16 planes differs only by one extra
  `planes()` call in `pixelread`. Expected: no changed verdict in the VDI case runs against `falcontc`
  (`cargo xtask tosdump …` with the tables of the last VDI acceptance, among them `vdi-all.tsv` cases 54/160/161 =
  `v_get_pixel` after `v_clrwk` `0xFFFF`, and the contour cases of `tosdump/cases/vdi-v18.tsv`), and screendumps of a
  default boot byte-identical to `main`.
- 32 planes (`vdiplanes=32` in the boot arguments, QEMU `-append` / `cmdline.txt`): boot to the desktop; a screendump
  equal to the default boot's (pen pixels have the same 24 colour bits; X is not displayed). The tosdump caps at
  16 planes are not lifted (ADR-038 point 13 iv, later slice), so a case table at 32 is not expected to run as
  at 16; if a `v_get_pixel` after `v_clrwk` is observed, the expected interim answer is `0, 0xFFDF`.
- Real-Pi4-only: nothing specific to this slice; both boards ignore the X byte (`ALPHA_MODE_IGNORED`, ramfb
  `XRGB8888`).

## Proposed `docs/STATUS.md` line

2026-11-18 — V63: the 32-plane device pixel. At `vdiplanes=32` every VDI read (`v_get_pixel`, flood-fill seed and
span probes) masks the unused X byte of the store word in `pixelread`, so a cleared screen reads as the white pen;
pens already write X = 0, `v_clrwk` keeps `0xFFFFFFFF`, logic operations act on all 32 bits. 16 planes unchanged.
New host test `vdi_v63_device_pixel` (kernel draw kernels by path). Commit <hash>.

## Out-of-scope observations

- `vro_cpyfm`/`vrt_cpyfm` are silent no-ops at 32 planes until V65 (`tos-api/src/vdi/raster.rs:85@CHUNKY_PLANES`);
  acceptance 1 "logic operations are bitwise on 32 bits" is met at the kernel ops and `WM_XOR`, not through those calls.
- `cargo xtask build` fails when `CARGO_TARGET_DIR` is set (`programs/build.sh` looks for the archives under
  `<tree>/target/programs`); the builds therefore used the worktree's own target directory inside the agent scratch directory.
- CLAUDE.md names `cargo xtask doc-check --fix-anchors`, which does not exist yet (TASK/TOOL49.md is OPEN).
- The repository is not rustfmt-clean (`kernel/src/raster_copy.rs`, `tos-api/src/vdi/contour.rs` differ from default
  rustfmt); left alone.
