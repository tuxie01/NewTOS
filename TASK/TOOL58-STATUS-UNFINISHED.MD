# TOOL58 — receipt (developer, acceptance missing)

Role: developer
Agent: agent-a7b4c73759c67c267

## Done

- `status-check` (working tree: batch files from `HEAD`; `--staged`: from the index) verifies every
  `Migrated (owner batch N, <date>[, to <STATE>]), evidence: ...` line against the committed, owner-approved
  `TASK/migration/batch-<N>.tsv`: row for the id, `to <STATE>` equal to the row's state where present, evidence equal
  to the row's evidence column. A failing line is the rejecting, never-baselined `migrated-unverified` violation,
  naming ticket, line and batch.
- One check (`verify_migrated`) used by `status-check`, the TOOL55 receipt exception, the `done-no-receipt`
  evidence rule and `accept`'s legacy-developer path; none of them trusts a rejected line.
- Lines without `to <STATE>` (batches 1-13): checked against the row alone (reasons in the ticket's Implementation
  section).
- The real tree: `656 of 656 Migrated lines match their owner-approved batch row`, `status-check` 0 rejected (working
  tree and `--staged`).

## Rework (tester PARTIAL on ca1552318)

- A staged change to a batch file makes the staged check (pre-commit) verify every ticket whose `Migrated` line
  names that batch: an edited row or a deleted approval line rejects in the commit itself (findings a, b).
- Once `TASK/migration/closed.txt` was added (closing commit `d1d6e5de8`), any staged addition, change or deletion
  under `TASK/migration/` is the rejecting `migration-frozen` violation naming the file; a full `status-check`
  compares `TASK/migration/` in `HEAD` with the closing commit (finding c going forward, and a change committed past
  the hook).
- Real tree: unchanged since the closing commit, `status-check` 0 rejected, 656 of 656 Migrated lines match.

## Missing

- Tester acceptance through `cargo xtask accept TOOL58` (Work 4 tests, the evidence-comparison mutation, the real
  `status-check`).
