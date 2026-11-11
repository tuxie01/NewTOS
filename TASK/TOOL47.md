# TOOL47 — tests that need gitignored inputs skip only under an explicit CI switch

Status: OPEN (2026-11-11) | Priority: P2

Type: tool (tests). Owner decision 2026-11-11, from the [TOOL43](TOOL43.md) report
(`docs/analysis/ci-backstop-2026-11-11.md`). Needed before the TOOL43 workflow can run `cargo xtask test` green.

## Facts

- In a clean clone without `references/` and the `sdk/apps/<app>/` checkouts, `cargo xtask test` ends 2388 passed,
  8 failed, 5 ignored; all 8 in the `xtask` target (report, table of failing tests):
  four need `references/emutos/aes/gem.rsc` (`rsc_obfix_tests` ×3, `rsc_cicon_tests::an_old_format_file_declares_no_colour_icons`),
  one needs FreeMiNT's `nvdi.pal` and EmuTOS `vdi_col.c` (`rsc_recolour_tests::recolour_palettes_equal_their_sources`),
  one `sdk/apps/teradesk/cicons.rsc`, one `sdk/apps/hypview/hyp_view.rsc`, and
  `doc_check::tests::resolves_against_the_workspace`, whose failure without `references/` the report did not explain.
- None of them can tell "input absent" from a failure: they panic or the python helper raises `FileNotFoundError`.
  Only `doc_check::tests::emutos_resolves_at_the_pin` skips today, silently (an `eprintln!`), whenever its input is
  absent.

## Work

1. **One switch.** An environment variable `NEWTOS_CI=1` (exact value; anything else counts as unset). Without it a
   missing input stays a **failure**, locally as today. With it, a test whose input is absent does not fail: it prints
   one line `SKIPPED <test name>: <input path> absent (NEWTOS_CI=1)` and returns. One helper in xtask that every such
   test calls with the path it needs; no per-test ad-hoc checks.
2. **The 8 tests** use it. For `resolves_against_the_workspace`, first find why it fails without `references/` (it
   claims to hold "even without references/"); if that is a test defect, fix it so it passes without the switch, and
   say so in the receipt.
3. `emutos_resolves_at_the_pin` gets the same rule (skips only under the switch, otherwise fails when absent), unless
   the receipt shows why it must keep skipping; that is an owner decision then.
4. **Report.** `cargo xtask test` prints at the end how many tests were SKIPPED under the switch and which; with the
   switch unset and nothing skipped it prints nothing new.
5. Host tests for the helper (absent + switch → skip line; absent without switch → failure; present → runs).
6. CLAUDE.md *Testing*: one line on `NEWTOS_CI` (proposed in the receipt, the lead applies it).

## Acceptance

- Clean clone without the inputs: `NEWTOS_CI=1 cargo xtask test` exits 0 and lists the 8 skips; without the switch it
  fails on the same tests as today (exit != 0).
- Main tree with the inputs: same totals as before, no SKIPPED line.
- `cargo xtask test` green, warnings delta 0, doc-check of the edited docs 0 failures.
