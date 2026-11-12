# XBIOS21 — receipt (developer): the ST(e) palette shadow starts with the EmuTOS boot palette

Role: developer
Agent: developer-xbios21 (Opus, agent ac17b8da97df23700)

Date 2026-11-12. Branch `xbios21` (worktree `target-agents/agent-ac17b8da97df23700/wt`), base `993e57b99`.
Spec: `docs/api/xbios/07-setcolor.md` (section "Boot palette", Pi binding),
`docs/api/xbios/06-setpalette.md` (Pi binding). Owner decision 2026-11-12 in `TASK/XBIOS21.md`.

## What changed

- `tos-api/src/xbios/screen.rs`: `PAL` starts as `DFLT_PALETTE` (`dflt_palette & 0x0FFF`,
  register order); `setcolor` returns the previous word `& 0x0FFF`, writes `& 0x0FFF` only for
  `color >= 0`; `setpalette` stores the 16 words `& 0x0FFF` in the same shadow; `apply_to_clut`
  removed (owner decision 2: no XBIOS palette write reaches the VDI colour table).
- `tos-api/tests/xbios21_boot_palette.rs`: 5 host tests (boot table for every n and a query does not
  write; set returns the old word and query the new; `Setpalette` feeds `Setcolor`; no
  `set_color`/`set_active_palette` on the backend; index wrap `& 0x0F` and any negative colour queries).
- Specs: Pi bindings of `07-setcolor.md` and `06-setpalette.md` rewritten for the decision.
- `docs/analysis/deps/mapping.tsv`: `atari_setcolor` -> `setcolor`, `initialise_ste_palette` -> `palette`.

## Verified (developer)

- `cargo xtask build --board rpi4` and `--board qemu-virt`: clean; tos-api 64 / kernel 93 rustc
  warnings, equal to the base.
- `cargo clippy -p tos-api --lib`: 73 warnings before and after (none in `xbios/screen.rs`); the new
  test file clippy-clean.
- `cargo xtask test` (full): 2459 passed, 0 failed, 7 ignored.
- `cargo xtask deps --check-mapping`: all rows resolve. doc-check of the edited docs: 0 failures.

## Missing

- Oracle rerun of `tosdump/cases/bios-r1.tsv` (cases 93/94 EQUAL on tt and falcontc, no case equal
  in `measurements/measurement-2026-11-12-xbios-full/RUNINFO.TXT` differing anew). Not done: `cargo
  xtask tosdump` builds the card with `tools/mkcard.sh` (`hdiutil attach`, `diskutil
  partitionDisk/mount`), which workers may not run (lead 2026-11-12). The attempt was stopped; the
  partial output is not committed.
- Tester acceptance.
