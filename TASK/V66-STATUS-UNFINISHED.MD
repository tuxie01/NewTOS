# V66 — receipt (developer): the `vq_scrninfo` / EdDI answer at 32 planes

Role: developer
Agent: a7bc92cb2885c6517

Date 2026-11-18. Branch `v66` from `main` @ `445eb69fa` (worktree `target-agents/agent-a7bc92cb2885c6517/wt`).
Spec: `docs/api/vdi/eddi-vq_scrninfo-32.md` (§2 the full table, §4 consistency, §5 the 16-plane answer, §6 S1-S7,
§7 change set); contract ADR-038 points 8, 11 and 13 (i) (`docs/MEMORY.md`); read with TASK/V66.md "Spec gate ...
and the lead's reading of its open points", readings 1-6.

## What changed, by file and function

- `tos-api/src/vdi/eddi.rs`: the table is built from a per-depth `Layout` (planes, gun bits, R/G/B LSB, genlock
  bits/LSB, unused bits/LSB, bit order). `LAYOUT_16` is the existing 16-plane constants (`DEVICE_PLANES`, `GUN_BITS`,
  `RED_LSB`, `GREEN_LSB`, `BLUE_LSB`, `GENLOCK_BIT`, `ORDER_FALCON`, unchanged); `LAYOUT_32` the ADR-038 point 8
  values (8/8/8, red 16, green 8, blue 0, no genlock, 8 unused bits from 24, `ORDER_USUAL = 0x01`). New private
  `layout(planes)` (16 or `VDI_PLANES_WIDE`, else `None`) and `build(&Layout)`; `scrninfo_table()` keeps its
  signature and returns the 16-plane table (`build(&LAYOUT_16)`); new `pub fn scrninfo_table_for(planes) ->
  Option<[i16; 273]>`. `is_scrninfo` admits 16 and 32 (reading 4), any other count falls through to `vq_extnd` as
  before; `vq_scrninfo(pb, backend)` writes the table of the backend's depth, output counts `272`/`0` unchanged.
  Module header and doc comments state both depths; the German quote at the `SCRNINFO_NINTOUT` doc translated.
- `tos-api/src/vdi/mod.rs` (`dispatch`): passes the backend to `eddi::vq_scrninfo`.
- `tos-api/tests/vdi_v66_scrninfo32.rs` (new, 10 tests, fixture `vdi_planes32_common`): the 32-plane table word for
  word against a table written from spec §2 (virtual and physical handle, frame, nothing past `INTOUT[272]`,
  PTSOUT untouched, `d0 = 0`); at 16 the dispatched answer equals `scrninfo_table()`; `scrninfo_table_for` is `None`
  for 0, 1, 2, 4, 8, 15, 17, 24, 31, 33, 64; S1-S6 each at 16 and at 32; S7 (optional, reading 5) at both (at 16 with
  EmuTOS's 5-bit `fivebits2vdi` rule as the pen value).
- `tos-api/tests/vdi_v62_planes_setting.rs`: the V62 interim test `vq_scrninfo_at_32_planes_falls_through_to_vq_extnd`
  (which expected `INQ_TAB` at 32, spec §7 "the fall-through test must stop expecting `INQ_TAB` at 32") becomes
  `vq_scrninfo_follows_the_setting_and_falls_through_for_other_depths`: 16 and 32 answer 272 words with `[2]` =
  the depth; 1, 4, 8, 24 still answer `vq_extnd` mode 1 word for word. Its 16-plane expectations are unchanged.
- `tos-api/tests/vdi_eddi_scrninfo.rs`: **not touched** (ADR-038 point 11; the whole-table test passes as it is).
- `docs/api/vdi/workstation/100-v_opnvwk.md` §2 row, §4 and open points; `docs/api/vdi/workstation/102-vq_extnd.md`
  EdDI paragraph and open point: the V62 interim text closed. `docs/COMPATIBILITY.md` "The VDI speaks EdDI": point 5
  with the 16/32 table rows. `docs/api/vdi/eddi-vq_scrninfo.md` not touched (ticket).
- `docs/analysis/deps/mapping.tsv`: new row `vdi_vq_extnd` → `tos-api/src/vdi/eddi.rs` `vq_scrninfo` (ratchet).
  `cargo xtask deps --check-mapping`: 1982 rows, all resolve.

**Anchors kept by layout.** `docs/MEMORY.md` (ADR-038 point 8: `eddi.rs:82@DEVICE_PLANES`, `:112@scrninfo_table`),
TASK/V61.md, the spec and two dated snapshots cite `eddi.rs` lines; the new code is ordered so that every
cited symbol still stands within ±5 lines (`DEVICE_PLANES` 53 and 77, `GUN_BITS` 58, `is_scrninfo` 81,
`scrninfo_table` 113, `w` 119-125, `vq_scrninfo` 160, `E3`, `Enhancer`). doc-check over every file citing `eddi.rs`:
no anchor failure.

## Spec and readings, as implemented

- Values: spec §2 column "At 32 planes" exactly; `[5..7] = 0` (reading 1), `[14] = 1` (reading 2), unused table
  `[96..103] = 24..31`, the read mask drops X (reading 3, V63/V65 code, tested by S6/S7 with `p & M`).
- 16-plane table: built by the same `build` from the unchanged constants; `vdi_eddi_scrninfo.rs`
  `vq_scrninfo_answers_the_falcon_hicolor_table_word_for_word` and `the_bit_tables_rebuild_the_device_word` pass
  unchanged.
- Consistency (spec §4): `vq_extnd`, `v_opnwk`/`v_opnvwk` and the cookie needed no change (verified: S1 compares
  `T[2]` with `vq_extnd(1)` `INTOUT[4]` at both depths).

## Build, clippy, tests

- `cargo xtask build --board rpi4`: OK; `--board qemu-virt`: OK (each after `touch tos-api/src/lib.rs`, "Compiling
  tos-api" seen once per build). rustc warnings `kernel` 93 / `tos-api` 64, as the baseline. Built without
  `CARGO_TARGET_DIR`.
- Clippy, baseline in the same worktree at `445eb69fa` before any edit: `cargo clippy -p kernel --features board-rpi4`
  and `board-qemu-virt`: kernel 121 / tos-api 73 before and after. `cargo clippy -p tos-api --all-targets`: 118
  warning lines before and after, per-file counts identical. **Delta 0.**
- Full `cargo xtask test` (worktree with `worktree-setup` links, final code): exit 0, **2554 passed / 0 failed / 7
  ignored** (sum of the `test result:` lines; the baseline 2544 plus the 10 new of `vdi_v66_scrninfo32`), no SKIPPED.
- `cargo xtask doc-check` on the edited docs and on every doc citing `eddi.rs`: no failure introduced (the
  remaining failures are gitignored inputs absent from the worktree, `sdk/gemlib/gemlib-0.44.0/*`,
  `Applications/z-tools/*`, and one old anchor into `tos-api/src/xbios/screen.rs` in a dated snapshot).
  `cargo xtask status-check --new-only`: 0 rejected.

## Tester handoff

- **Host:** `cargo xtask test`; specifically `vdi_v66_scrninfo32` (10), `vdi_eddi_scrninfo` (6, the 16-plane bar,
  untouched), `vdi_v62_planes_setting` (5).
- **Default boot (16 planes):** nothing may change. `vq_scrninfo` answers the Falcon HiColor table; any tosdump run of
  `tosdump/core/scrninfo.c` against `falcontc` keeps its verdict.
- **`vdiplanes=32` boot** (QEMU `-append`, `cmdline.txt`): `vq_scrninfo` answers the 32-plane table of
  `docs/api/vdi/eddi-vq_scrninfo-32.md` §2 instead of `INQ_TAB` (`CONTRL[4] = 272`, `[2] = 32`, `[3]:[4] =
  0x0100:0`, `[14] = 1`). `tosdump/core/scrninfo.c` accepts up to 32 planes and would dump it (expected values: the
  spec, labelled NewTOS contract, never oracle). Program level (ADR-038 point 13 (iii), not this ticket's
  acceptance): DOOM's `eddi_check` and HighWire's `raster_32` choose their 32-bit paths from this answer.
- **Real-Pi4-only:** nothing specific to this change (a table answer, no hardware path).

## Proposed `docs/STATUS.md` line

2026-11-18 — V66: `vq_scrninfo` at 32 planes. At `vdiplanes=32` the EdDI call answers the 32-plane table of ADR-038
point 8 (packed, 32 planes, 2^24 colours, 8/8/8 bits at 16/8/0, the X byte as 8 unused bits 24..31, bit order 1)
instead of falling through to `INQ_TAB`; other plane counts still fall through; the 16-plane answer is unchanged
word for word. The structural self-test (bit tables partition the pixel, colour count, table-built pixel through
`vro_cpyfm` and `v_get_pixel`, and the pen copied back) runs at both depths in `vdi_v66_scrninfo32`. Commit <hash>.

## Out-of-scope observations

- `docs/api/vdi/eddi-vq_scrninfo.md` stays German and without a pointer to the 32-plane file (TASK/V66.md: no ticket yet).
- `tos-api/src/cookie.rs:48@Doom` still says "Doom does not look yet" (stale; spec open point (a)).
- `docs/COMPATIBILITY.md` keeps verbatim German owner quotes (with or without a gloss) in sections not edited here.
- The tree is not rustfmt-clean (default width): `vdi_planes32_common` and the V62 test differ already; new code
  follows the surrounding style.
- `cargo xtask build` fails when `CARGO_TARGET_DIR` is set (known); built without it.
