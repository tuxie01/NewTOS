# TOOL57 — the A01 migrate test reads the live ticket, which the migration changed

Status: OPEN (2026-11-11) | Priority: P1

Type: tool (test defect). Follow-up of [TOOL54](TOOL54.md) and [TOOL55](TOOL55.md) (both DONE). Found by the tester
accepting A39/A46 on 2026-11-11; `cargo xtask test` on `main` is red since batch 20 (`add86da91`).

## Facts

- `tickets::migrate::tests::the_a01_case_migrates_to_done_through_the_commit_map`
  (`tools/xtask/src/tickets/migrate/tests.rs:355@the_a01_case_migrates_to_done_through_the_commit_map`) copies the
  real `TASK/A01.md` and its receipt from the working tree into a scratch repository and asserts that A01 is in the
  baseline (`tools/xtask/src/tickets/migrate/tests.rs:372@baseline`). Owner batch 20 migrated A01 to `DONE`, so the live ticket
  is no longer a legacy ticket and the assertion fails.
- Full `cargo xtask test` at `c04ba2741`: xtask 314 passed, 1 failed, 1 ignored (only this test); the run stops there.
- The migration emptied the baseline, so any test that reads a live legacy ticket from the working tree breaks the
  same way.

## Work

1. The test reads A01's ticket and receipt as they were before the migration (e.g. `git show add86da91^:TASK/A01.md`,
   or a committed fixture copy under the test's fixture directory), not from the working tree; it keeps proving the
   real commit-map case.
2. Check every other xtask test for reads of live `TASK/` legacy state and fix them the same way.
3. A note in the receipt on why the hooks did not catch it (the pre-commit hook does not run `cargo xtask test`;
   `migrate` changed only ticket files) and a proposal, e.g. `migrate` refusing when a test fixture cites a ticket it
   migrates, or the TOOL43 server check catching it.

## Acceptance

- `cargo xtask test` green on `main` again (2442/0/7 expected).
- `cargo xtask test` green, warnings delta 0, doc-check of the edited docs 0 failures.
