# TOOL47 — receipt (unfinished: tester acceptance, CLAUDE.md line)

Role: developer
Agent: agent-a8b2d21b93aa0c6b8

Worktree `.claude/worktrees/agent-a8b2d21b93aa0c6b8`, branch `worktree-agent-a8b2d21b93aa0c6b8`, base `b76a31c6b`.
Code cites: section *Implementation* of [TOOL47.md](TOOL47.md).

## Done

- Work 1: one switch `NEWTOS_CI` (exactly `1`), one helper module `tools/xtask/src/test_inputs.rs`
  (`require`, `require_paths`). Absent without the switch: the test panics naming the input; absent with it: one line
  `SKIPPED <test>: <input> absent (NEWTOS_CI=1)` on the uncaptured stderr, appended to the `NEWTOS_SKIP_LOG` file.
- Work 2: the seven `rsc_*` tests of the report call `require_paths` with the paths they read
  (`recolour_palettes_equal_their_sources` with both, `nvdi.pal` and `vdi_col.c`).
- Work 2, `doc_check::tests::resolves_against_the_workspace`: **test defect, fixed, passes without the switch.** Why
  it failed: a backtick span that does not resolve is also tried under `SEARCH_ROOTS`, which include
  `references/emutos` and `references/freemint`; when such a tree is absent, `unresolved` returns `Skipped`
  (`under_missing_ref`), as documented in the module ("skipped like any cite when the tree is absent"). So
  `` `kernel/src/no_such_module_x.rs` `` gives `Skipped`, not `Fail("missing")`, without references; the same holds
  for the case-mismatch span `` `tools/XTASK/Cargo.toml` `` (it would have failed next). The test's claim "fails even
  without references/" was never exercised: test and code came in one commit (`c8e67a478`), always run with references
  present. The test now asserts `Skipped` for these two spans when a reference tree is absent and `Fail` when present,
  and adds the Markdown-link forms (one candidate beside the file), which fail in both trees. `doc-check` behaviour is
  unchanged. Measured: `ok` in the clean clone with and without the switch, and in the worktree.
- Work 3, `doc_check::tests::emutos_resolves_at_the_pin`: same rule, no reason found to keep the silent skip. Input
  named `references/emutos at <commit>` (pin, then calibration commit). Consequence: a local tree whose
  `references/emutos` lacks one of the two commits now fails this test (before: silent pass).
- Work 4: `cargo xtask test` (full and `--fast`) sets `NEWTOS_SKIP_LOG` to `xtask-test-skips.log` in the target directory (removed before
  the run) and prints `xtask test: N test(s) SKIPPED under NEWTOS_CI=1 (input absent):` plus the sorted lines;
  nothing when no test skipped.
- Work 5: 5 host tests in `test_inputs::tests` (only `1` is the switch; present runs with and without, prints and logs
  nothing; absent + switch: exact line, printed = logged, summary text; absent without switch panics; empty log →
  no summary).
- Fix after the tester's PARTIAL on `c4e8d1c6b` (RUNINFO.TXT of the run measurement-2026-11-11-tool47-test, in the main tree,
  mutation M3b: `require` forced to Skip for an absent input passed every test, because `require` and its environment
  read were reached only through `decide`/`apply`): 2 tests and 1 `#[ignore]`d child in `test_inputs::tests`. The
  child calls `require_paths` (absent path) and `require` directly; the two tests run it in a fresh process of the
  test binary (`current_exe`, `--exact --ignored --nocapture`) with `NEWTOS_CI` removed or `0` (child must fail with
  the panic message, nothing logged) and `1` (child passes, both SKIPPED lines printed and logged in order). The
  subprocess keeps the environment of the other tests untouched; the child is ignored in normal runs (reason in the
  attribute), hence ignored 6 -> 7. Mutations, measured in the worktree with `cargo test -p xtask test_inputs`, each
  restored by hand from a copy: M3b (`require` gives Skip whenever absent) -> `require_on_an_absent_input_fails_without_the_switch`
  FAILED; the converse (`require` ignores the environment, always Fail when absent) ->
  `require_on_an_absent_input_skips_under_the_switch` FAILED; unmutated 7 passed, 1 ignored.
- `docs/analysis/ci-backstop-2026-11-11.md` cites `doc_check.rs` line 2519 with the anchor `emutos_resolves_at_the_pin`; since
  `c4e8d1c6b` the function stands at 2528, so `doc-check` on that file reports the shifted anchor. The pre-commit
  hook did **not** reject `c4e8d1c6b` for it (its output: "1955 docs the commit leaves alone, 660 of their cites name
  one of the 10 changed paths, 0 broken by this commit"; dated snapshots are not checked there), so the dated
  snapshot is left untouched, per the lead's rule (re-point only if the hook rejects).
- `docs/STATUS.md` and its archive are not changed by this branch (owner rule 2026-11-11: the lead edits them); the
  proposed lines are under *Missing*.

## Measured (2026-11-11, macOS host)

- Worktree with the inputs (linked by `cargo xtask worktree-setup`): `cargo xtask test` exit 0, after the fix commit **2407 passed,
  0 failed, 7 ignored** (+2 tests, +1 ignored child), no SKIPPED line; on `c4e8d1c6b` **2405 passed,
  0 failed, 6 ignored**, no SKIPPED line. Base 2400/0/6 (main after TOOL44, its acceptance run) + 5 new tests.
- Clean clone of `b76a31c6b` under the worktree's `target/` (no `references/`, no `sdk/apps/<app>/`), baseline
  before the change: `cargo test -p xtask` 268 passed, 8 failed (the 8 of the report, among them
  `resolves_against_the_workspace`, its assertion on line 2084 before the change).
- Same clone with the changed files copied in:
  - `NEWTOS_CI=1 cargo xtask test`: exit 0, 2405 passed, 0 failed, 6 ignored, 8 `SKIPPED` lines in the stream and the
    summary:
    `emutos_resolves_at_the_pin` (references/emutos at 1a7484ab…), `an_old_format_file_declares_no_colour_icons`,
    `mkrsc_convert_ilp32_…`, `mkrsc_convert_lp64_…`, `the_committed_gem_fsel_blob_…` (gem.rsc),
    `teradesk_cicons_rsc_converts_and_fits_rsc_cap` (cicons.rsc), `hypview_rsc_8plane_…` (hyp_view.rsc),
    `recolour_palettes_equal_their_sources` (nvdi.pal).
  - `cargo xtask test` without the switch: exit 1, xtask target 273 passed, 8 failed, 0 SKIPPED lines.
    **The failing set differs from the report's by one swap:** `resolves_against_the_workspace` passes now (test
    defect fixed), `emutos_resolves_at_the_pin` fails now (silent skip removed, Work 3). The seven `rsc_*` are the same.
  - The clone was removed afterwards.
- clippy `-p xtask --all-targets`: identical warning list before (clean clone of main) and after: 1 (bin) + 9 (test),
  none in a touched line. **Delta 0.** Kernel/boards untouched (xtask only), so no board build is affected.

## Missing

- Tester acceptance through `cargo xtask accept TOOL47`.
- CLAUDE.md *Testing*, line proposed for the lead (Work 6), after the `cargo xtask test --fast` sentence:

  > - Tests that read gitignored inputs (`references/…`, `sdk/apps/<app>/`) fail when they are absent; only with
  >   `NEWTOS_CI=1` (exactly `1`, for a machine without them, e.g. the TOOL43 runner) they print
  >   `SKIPPED <test>: <input> absent (NEWTOS_CI=1)` and pass, and `cargo xtask test` lists them at the end. New such
  >   tests call `test_inputs::require_paths` (TOOL47).
- `docs/STATUS.md`, proposed for the lead (owner rule 2026-11-11):
  - *Current state*, cell of the row "Host tests (`cargo xtask test`)": **2407 / 0 / 7** (measured 2026-11-11 with
    TOOL47, Developer, +7 `test_inputs::tests`, +1 ignored subprocess child with reason; before that 2400 on
    2026-11-11 with TOOL44, Tester, +4 worktree-setup tests; before that 2396 on 2026-11-11 with TOOL41, ... as
    today).
  - *Recent Decisions*, new top row (the oldest row, REL072, then moves to the archive):

```
| 2026-11-11 | **[TOOL47](../TASK/TOOL47.md): tests that need gitignored inputs skip only under `NEWTOS_CI=1` (<commit>, Developer; Tester PASS, <run>; IMPLEMENTED until `cargo xtask accept`).** Why: owner decision 2026-11-11 from the [TOOL43](../TASK/TOOL43.md) report; in a clone without `references/` and `sdk/apps/<app>/` 8 xtask tests failed and none could tell an absent input from a regression. One helper (`tools/xtask/src/test_inputs.rs`): without the switch an absent input still fails; with it the test prints `SKIPPED <test>: <input> absent (NEWTOS_CI=1)` and returns; `cargo xtask test` lists the skips at the end. Seven `rsc_*` tests and `emutos_resolves_at_the_pin` (which skipped silently before) use it; `resolves_against_the_workspace` was a test defect (a missed span may live in an absent reference tree and is skipped by design) and now passes without references. Clean clone: with the switch exit 0, 8 SKIPPED; without it exit 1, 8 failed. Host tests 2400 -> 2407/0/7 (one `#[ignore]`d subprocess child, reason in the attribute); warnings delta 0. |
```
