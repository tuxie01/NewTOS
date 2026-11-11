# TOOL57 — receipt (developer, acceptance missing)

Role: developer
Agent: agent-ac7b152d2e13d3249

## Done

- `the_a01_case_migrates_to_done_through_the_commit_map` and `the_real_commit_map_translates_the_a01_receipt_hash`
  read A01's ticket and receipt from committed fixtures (`tools/xtask/src/tickets/migrate/fixtures/`, the blobs of
  `add86da91^`), not from the live `TASK/`; the live `docs/commit-map.txt` stays the proof of the old-hash
  translation. A new test pins the fixtures to the `add86da91^` blob ids.
- No other xtask test reads live `TASK/` state (grep over `tools/xtask/src/**` and `tools/xtask/tests/`).
- `TASK/TOOL54.md` and `TASK/TOOL57.md`: the shifted anchors into `tools/xtask/src/tickets/migrate/tests.rs` fixed; TOOL54's test description
  names the fixtures.

## Results

- Full `cargo xtask test` (worktree, base `f47088192`): exit 0, 2443 passed / 0 failed / 7 ignored (one more than the
  expected 2442: the new provenance test); no warning of the run points into `tools/xtask/src/tickets/migrate/`.
- `cargo clippy -p xtask --all-targets`: 12 warnings before (the original `tests.rs` swapped in by hand) and after,
  delta 0.
- `cargo xtask doc-check` of `TASK/TOOL54.md`, `TASK/TOOL57.md` and this receipt: 0 failures;
  `cargo xtask status-check --new-only`: 0 rejected.
- No scratch clone; the scratch repositories of the tests live under the worktree's `target/` and are removed by
  the tests.

## Why the hooks did not catch it

The pre-commit hook runs `doc-check --staged`, `deps --check-mapping --staged` and (switched on) `status-check`, never
`cargo xtask test`; the batch-20 commit made by `migrate` staged only `TASK/*.md` and the baseline, so nothing in the
hook looked at a test that reads those files. The test itself was correct only as long as A01 stayed a legacy ticket.

## Proposal

`migrate` refuses (or warns) when a ticket of the batch is read by a test: a scan of `tools/xtask/src/**` for the
literal `TASK/<ID>` of each migrated id before writing. Independent of that, the TOOL43 server check runs the full
`cargo xtask test` on every pushed `main` commit, which catches every such case, not only this pattern.
