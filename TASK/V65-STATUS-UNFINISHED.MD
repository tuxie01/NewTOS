# V65 — receipt (developer): 32-plane memory forms, copies, vr_trnfm, wide v_get_pixel, save buffers

Role: developer
Agent: afb2dd3e5eb949ba1

Date 2026-11-18. Branch `v65` from `main` @ `e225d1780` (worktree `target-agents/agent-afb2dd3e5eb949ba1/wt`).
Spec: `docs/api/vdi/raster/_raster.md` § "32-plane setting: memory forms, copies, `vr_trnfm`, `v_get_pixel`, save
buffers (ADR-038 points 5-7 and 10, V65)" §1-9, and § "32-plane setting: the wide `v_get_pixel` answer" of
`docs/api/vdi/inquiry/_color-pixel-input.md`; contract ADR-038 points 5, 6, 7, 10, 11 (`docs/MEMORY.md`); read with
TASK/V65.md "Spec gate ... and the lead's reading of its open points", readings 1-6.

## What changed, by file and function

- `tos-api/src/vdi/raster/wide.rs` (new submodule of `raster`): the 32-plane device form.
  `WIDE_PLANES`; `device_mem_planes` (**the one depth source**: 32 at `vdiplanes=32`, else `CHUNKY_PLANES` = 16);
  `refused_at_depth` (reading 1: at 32 a form of 9..31 planes, 16 included, is refused); `save_form` (the `bb_save`
  buffer form, stamped from `device_mem_planes`); `Mfdb::px_bytes`, `Mfdb::save_words`; the trait `MemPx` over `u16`
  and `u32` (`read_screen_row`/`write_screen_row`: the 16-bit row seam for `u16`, the raw `read_device_rect` /
  `write_device_rect` one row high for `u32` — the kernel's NEON row copy, ADR-036; `fetch_row`: for `u32` a 32-plane
  form as it is, a 16-bit form expanded per pixel with `rgb565_to_device`, which is only the colour-icon form);
  the macro `at_px!` (picks `u16`/`u32` by plane count at a call site); `vr_trnfm32` (§5).
- `tos-api/src/vdi/raster.rs`: the chunky paths are generic over `MemPx` and at 16 planes are the `u16` instances of
  the code before (word for word; the GUI07, V19, V26, V28 unit tests unchanged apart from three `::<u16>` and two
  `: u16` annotations): `Mfdb::get_mem`/`put_mem`/`read_row`/`combine_row`, `apply_row(_op)`, `raster_op`,
  `mono_to_mem`, `screen_to_mem`, `mem_to_screen`, `mem_to_mem`; `Mfdb::mem_stride_px` (32-plane row `fd_wdwidth *
  16`, reading 6), `bind_block` and `chunky_reverse` (4-byte pixels); `device_mem` takes the plane count;
  `vrt_cpyfm`/`vro_cpyfm` plane gates (cap = `device_mem_planes`, plus `refused_at_depth`, both before any bind);
  `vrt_cpyfm` memory destination at 32 with `pel_to_device32`; `bb_save_words(backend, w, h)` = `save_form(..)
  .save_words()`, `bb_save`/`bb_restore` through `save_form` (reading 5: stamp and size in this one commit, one
  source); `blit_device_mem` keeps `CHUNKY_PLANES` (reading 4) and at 32 runs the 32-bit screen walk with the
  expanded icon pixels; `vr_trnfm` gate (reading 2: at 32 only `fd_nplanes == 32` goes to `vr_trnfm32`, 9..31 a
  silent no-op before the binds; at 16 unchanged).
- `tos-api/src/vdi/backend.rs`: trait method `pel_to_device32` (the pen's 32-bit pixel, X = 0), default = the
  16-plane rule `rgb565_to_device(pel_to_device16(pel))` with a `debug_assert` that a backend reporting 32 planes
  overrides it; so the ~70 test doubles needed no edit (spec §4/§8 left name and signature to the developer).
- `kernel/src/vdi_backend.rs`: `VdiScreen::pel_to_device32` = `pel_to_rgb` (at 32 the entry `0x00RRGGBB`, at 16 the
  expanded word); `pen_word16` doc.
- `tos-api/src/vdi/inquiry.rs`: `v_get_pixel` at 32 answers `{high, low}` of the masked pixel (§6); 16 unchanged.
- `tos-api/src/aes/gsxif.rs`: `bb_save_words` callers pass the backend; `gsx_mcalc` doc (sizes derived, §7).
- Tests: new `tos-api/tests/vdi_v65_memory_forms.rs` (17); `vdi_planes32_common` gets `pel_to_device32` and the
  kernel's `read_device_rect`/`write_device_rect` (through `raster_copy`, as the kernel); `vdi_v63_device_pixel` and
  `vdi_v64_pens`: only their **32-plane** `v_get_pixel` expectations move from the interim answer to the V65 answer
  (the spec says V65 replaces it; one test renamed `v_get_pixel_answers_the_wide_pixel_at_32_planes`), every 16-plane
  expectation untouched; three AES tests pass the backend to `bb_save_words`.
- `docs/analysis/deps/mapping.tsv`: rows for the touched functions (ratchet): new `device_mem_planes`,
  `refused_at_depth`, `px_bytes`, `save_form`, `read_screen_row`, `fetch_row`, `vr_trnfm32`, `pel_to_device32`,
  `bb_save_words`; notes of `mem_stride_px`, `screen_to_mem`, `mem_to_screen`, `mem_to_mem`, `raster_op`,
  `mono_to_mem`, `v_get_pixel`, `gsx_mcalc` updated. `cargo xtask deps --check-mapping`: 1981 rows, all resolve.
- `TASK/V64-STATUS-UNFINISHED.MD`: one anchor line number only (its `VDI_PLANES_WIDE` cite into `tos-api/src/vdi/backend.rs`, 720 → 736, shifted by
  the new trait method).

**Layout for the anchors.** `doc-check --fix-anchors` (TASK/TOOL49) does not exist yet, and `docs/MEMORY.md` (ADR-038
cites `raster.rs:821@CHUNKY_PLANES` and `:1523@planes`) and a DONE receipt (`TASK/BIOS10-STATUS-DONE.MD`, cites
`inquiry.rs:208@vq_key_s`) may not be edited. A first layout broke 138 anchors; the new code was moved into
`tos-api/src/vdi/raster/wide.rs` and the edits in `raster.rs`, `inquiry.rs`, `backend.rs`, `vdi_backend.rs` kept line-neutral, so
the gates still sit at the cited lines. `doc-check --staged`: 0 broken by this commit.

## Spec inferences relied on, checked against the source

- §3 row `fd_wdwidth * 16` pixels = `s_nxln / 4`: `setup_info` sets `s_nxln = fd_wdwidth * fd_nplanes * 2` bytes
  (`references/emutos/vdi/vdi_raster.c:921-923`, `:937-938`), 64 bytes per word at 32 planes — confirmed.
- §3 overlap direction on start addresses: `vro_cpyfm16` `if (src < dst)` on `form + y * width + x`
  (`vdi_raster.c:986-1001`) — implemented with byte addresses, 4 bytes a pixel; pinned by a host test.
- §4 `MD_XOR` `*q = ~*q` (`vdi_raster.c:1274`) — all 32 bits; pinned.
- §4 the pen's memory pixel through the pen-ordered table (V5-008 fold for 8-bit pens): `pel_to_device32` is the
  kernel's `pel_to_rgb`, the same lookup every screen draw uses, so the fold holds per pen as at 16.
- §5 `vr_trnfm16` reads `planesize` from the source and never its plane count (`vdi_raster.c:197-199`) — generalised
  to 32; output zeroed, scratch in place, `fd_stand` untouched; identity pinned both ways, in place too.
- §2 refusal before any bind: the plane gates precede every `bind`/`bind_block` in `vro_cpyfm`, `vrt_cpyfm`,
  `vr_trnfm` (reading of the code); pinned by the 9..31 test (forms, screen and `INTOUT` untouched).
- §7 the depth is fixed before the AES takes its buffer: `vdi_backend::set_planes` runs in `kernel_main_rust`
  (`kernel/src/lib.rs`) before the framebuffer and the AES (reading of the code).

## Build, clippy, tests

- `cargo xtask build --board rpi4`: OK; `--board qemu-virt`: OK (each after `touch kernel/src/lib.rs
  tos-api/src/lib.rs`, "Compiling kernel"/"Compiling tos-api" seen). rustc warnings `kernel` 93 / `tos-api` 64, as the
  baseline. Built without `CARGO_TARGET_DIR`.
- Clippy, baseline in a detached worktree at `e225d1780` (`target-agents/agent-afb2dd3e5eb949ba1/base`):
  `cargo clippy -p kernel --features board-rpi4` and `board-qemu-virt`: kernel 121 / tos-api 73 before and after, the
  per-file counts identical. `cargo clippy -p tos-api --all-targets`: 93 warning lines before, 93 after, the per-kind
  counts identical (the one `raster.rs` warning, a complex type in the GUI07 test module, existed before). **Delta 0.**
- Full `cargo xtask test` (worktree with `worktree-setup` links, final code): exit 0, **2544 passed / 0 failed / 7
  ignored** (sum of the `test result:` lines; the baseline 2527 plus the 17 new), no SKIPPED.
- `cargo xtask doc-check --staged`: 0 broken by this commit. `cargo xtask status-check --new-only`: see below.

## Tester handoff

- **Host:** `cargo xtask test`; the new `vdi_v65_memory_forms` (17: round trip with `vro_cpyfm` + `v_get_pixel`;
  row stride; the sixteen ops memory↔memory, screen→memory, memory→screen at 32 bits; overlap direction; the four
  `vrt_cpyfm` modes with 8-bit pens; 9..31 planes refused for `vro_cpyfm`/`vrt_cpyfm`/`vr_trnfm`; planar 1..8 alike at
  both settings; `vr_trnfm` identity both ways and in place, plane order; `v_get_pixel` shape; `bb_save_words` sizes
  at both settings and `bb_save` writing nothing past it, exact restore; the colour-icon blit on its 16-bit form),
  plus `vdi_v63_device_pixel`, `vdi_v64_pens`, the GUI07/V19/V26/V28 unit tests in `raster.rs`
  (the 16-plane bar), `aes_screen_buffer_save_under`, `aes_form_alert_cursor_bracket`, `aes_window_screen_buffer`,
  `aes_cicon_draw`.
- **Default boot (16 planes):** nothing may change. Expected: no changed verdict in the VDI raster case runs against
  `falcontc` (`vro_cpyfm`/`vrt_cpyfm`/`vr_trnfm`/`v_get_pixel` cases, `tosdump/cases/vdi-v18-t2-raster.tsv` among
  them), and screendumps of a default boot (desktop, a drop-down menu opened and closed, an alert) byte-identical to
  `main`.
- **`vdiplanes=32`** (QEMU `-append`, `cmdline.txt`): boot to the desktop; open and close a drop-down menu, a pop-up
  and an alert: the area under them comes back exactly (before V65 the save went through the 16-bit seam at 32 and
  came back 5-bit). Colour icons draw (5-bit colour until TASK/V67; the `S_AND_D` background keeps 8 bits per gun).
  `PRESENT.NAP` builds 16-plane forms: at 32 its copies are refused by design (O5, TASK/V69). The AES screen buffer
  is twice the size at 32 (`gsx_mcalc`; 1920×1080 → 8 294 400 bytes) — the boot log line of `gsx_malloc` shows it.
- **Real-Pi4-only:** the speed criterion, the copies against today's seam (ADR-036 point 5): QEMU gives no speed
  figure. Compare a `bb_save`/`bb_restore` or `vro_cpyfm` screen↔memory loop at `vdiplanes=16` (16-bit seam, one
  conversion per pixel) against `vdiplanes=32` (raw row copy through `raster_copy::read_rect`/`write_rect`).

## Proposed `docs/STATUS.md` line

2026-11-18 — V65: memory forms at 32 planes. At `vdiplanes=32` the device memory form is 32-bit (`fd_nplanes = 32`,
row `fd_wdwidth * 16` pixels); `vro_cpyfm`/`vrt_cpyfm` take it with the cap raised to the setting's depth and the
sixteen ops on all 32 bits, forms of 9..31 planes (16 included) are silent no-ops; `vr_trnfm` transforms 32-plane
standard ↔ device forms; `v_get_pixel` answers the high and the low word of the masked pixel; `bb_save`/`bb_restore`
and their buffer size follow one depth source, so the AES screen buffer doubles at 32 and restores exactly; the
colour-icon blit keeps its 16-bit form until V67. 16 planes unchanged. New `tos-api/src/vdi/raster/wide.rs`, host
test `vdi_v65_memory_forms`. Commit <hash>.

## Out-of-scope observations

- `cargo xtask doc-check --fix-anchors` (TASK/TOOL49) is still open; the anchors were kept by layout instead.
- `cargo xtask build` fails when `CARGO_TARGET_DIR` is set (known); built without it.
- The `_raster.md` V65 change-set table (§8) names the pre-V65 sites; the new helpers live in
  `tos-api/src/vdi/raster/wide.rs` (the researcher may add them).
- The 32-bit copy paths double the stack buffers of one copy (two `[u32; 256]`, plus `[u16; 256]` on the icon path,
  about 2.5 KiB) on the 64 KiB kernel stack.
