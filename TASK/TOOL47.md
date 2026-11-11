# TOOL47 — tests that need gitignored inputs skip only under an explicit CI switch

Status: IMPLEMENTED (2026-11-11) | Priority: P2 | Commit: self | Missing: tester acceptance through cargo xtask accept

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

## Implementation (2026-11-11, developer)

Worktree branch `worktree-agent-a8b2d21b93aa0c6b8`, based on `b76a31c6b`. Receipt:
[TOOL47-STATUS-UNFINISHED.MD](TOOL47-STATUS-UNFINISHED.MD).

- **Switch and helper** (Work 1): `tools/xtask/src/test_inputs.rs:42@decide` (`NEWTOS_CI` exactly `1`, else
  unset), the one line `tools/xtask/src/test_inputs.rs:52@skip_line`, the core every guarded test calls
  `tools/xtask/src/test_inputs.rs:61@require` and its path form `tools/xtask/src/test_inputs.rs:71@require_paths`
  (the first absent workspace-relative path is the one named). Absent without the switch panics with the path and
  the two remedies (`tools/xtask/src/test_inputs.rs:82@apply`). The skip line goes to the uncaptured
  `std::io::stderr()` and is appended to the file in `NEWTOS_SKIP_LOG`.
- **The guarded tests** (Work 2, 3): `tools/xtask/src/rsc_obfix_tests.rs:176@check_mode` (both mkrsc modes),
  `tools/xtask/src/rsc_obfix_tests.rs:204@the_committed_gem_fsel_blob_carries_char_grid_coordinates`,
  `tools/xtask/src/rsc_cicon_tests.rs:445@an_old_format_file_declares_no_colour_icons`,
  `tools/xtask/src/rsc_cicon_tests.rs:470@teradesk_cicons_rsc_converts_and_fits_rsc_cap`,
  `tools/xtask/src/rsc_recolour_tests.rs:210@recolour_palettes_equal_their_sources` (`nvdi.pal` and `vdi_col.c`),
  `tools/xtask/src/rsc_recolour_tests.rs:231@hypview_rsc_8plane_icons_take_the_nearest_tos_colour`, and
  `tools/xtask/src/doc_check.rs:2528@emutos_resolves_at_the_pin` (input: `references/emutos` at the pin and at the
  calibration commit; it no longer skips silently).
- **`resolves_against_the_workspace` was a test defect** (Work 2): `tools/xtask/src/doc_check.rs:2070@resolves_against_the_workspace`.
  A backtick span that misses is also tried under the `SEARCH_ROOTS`, which include `references/emutos` and
  `references/freemint`; with such a tree absent `unresolved` returns `Skipped` (`under_missing_ref`), as the
  module documentation states ("skipped like any cite when the tree is absent"). The assertion "a top-level path
  that is gone fails even without references/" was never true without them (test and code came in the same commit
  `c8e67a478`, always run with references present); the case-mismatch span assertion had the same flaw. The test
  now expects `Skipped` for those two spans when a reference tree is absent and `Fail` when present
  (`tools/xtask/src/doc_check.rs:2087@refs_absent`), and adds the link forms, which have one candidate and fail in
  both trees. It passes without the switch; doc-check itself is unchanged.
- **Report** (Work 4): `cargo xtask test` points `NEWTOS_SKIP_LOG` at `xtask-test-skips.log` in the workspace target directory
  (`tools/xtask/src/main.rs:1067@skip_log`) and prints the sorted skip lines at the end
  (`tools/xtask/src/main.rs:1117@report_skips`, text from `tools/xtask/src/test_inputs.rs:110@summary`); nothing
  when nothing skipped.
- **Host tests** (Work 5): `tools/xtask/src/test_inputs.rs` `mod tests`: only `1` is the switch; present runs
  (switch or not, prints and logs nothing); absent + switch skips with the exact line, printed = logged, summary
  text; absent without switch panics; empty log prints nothing.
- **`require` itself, through the process environment** (tester PARTIAL on `c4e8d1c6b`, mutation M3b):
  `tools/xtask/src/test_inputs.rs:146@run_child` runs the `#[ignore]`d child
  `tools/xtask/src/test_inputs.rs:164@child_require_on_an_absent_input` (calls `require_paths` with an absent path
  and `require` directly) in a fresh process of the test binary, with `NEWTOS_CI` removed, `0` or `1` and a
  scratch skip log, so no test changes the environment of the tests beside it.
  `tools/xtask/src/test_inputs.rs:171@require_on_an_absent_input_fails_without_the_switch` wants the child to fail
  with the panic message and nothing logged; `require_on_an_absent_input_skips_under_the_switch` wants it to pass
  with both lines printed and logged in order.
