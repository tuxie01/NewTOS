# TOOL58 — status-check verifies every Migrated line against its approved batch file

Status: OPEN (2026-11-11) | Priority: P1

Type: tool (enforcement). Owner decision 2026-11-11; ADR-037 addendum (c), point 18. Tooling slot, before TOOL48.

## Facts

- The TOOL55 exception (a migrated ticket keeps its legacy DONE receipt) and the legacy-developer path of `accept`
  trust the text of a `Migrated (owner batch N, <date>[, to <STATE>]), evidence: …` line in the ticket. The TOOL55
  tester showed that a hand-written line gets the same treatment
  (`measurements/measurement-2026-11-11-tool55-test/RUNINFO.TXT`, check 5).
- Every real Migrated line comes from a committed, owner-approved `TASK/migration/batch-<N>.tsv` (batches 1–22); the
  migration is closed (`TASK/migration/closed.txt`).

## Work

1. `status-check` (and with it the pre-commit hook) verifies every Migrated line of every ticket against the committed
   batch file it names: the file exists in `HEAD` (or the index under `--staged`), carries the owner's approval line,
   and has a row for this id whose target state and evidence match the line (the `to <STATE>` part where present; the
   lines of batches 1–13 have none — then the row's state must equal the state `migrate` wrote, read from the commit
   that added the line, or the check uses the row alone; choose and justify). A hand-written or mismatching line is
   rejected, naming ticket, line and batch.
2. One shared check used by `status-check`, the TOOL55 exception and `accept`'s legacy-developer path, so none of
   them trusts a line the check rejects.
3. The real repository passes: all Migrated lines of batches 1–22 match (show the count).
4. Host tests: a hand-written line (no batch file, wrong batch number, id not in the batch, state or evidence
   differing, batch file without approval) → rejected; a real line → passes.

## Acceptance

- Tests of point 4 green; a mutation that skips the evidence comparison fails a test (tester shows it).
- `cargo xtask status-check` on `main` 0 rejected with the check active.
- `cargo xtask test` green, warnings delta 0, doc-check of the edited docs 0 failures.
