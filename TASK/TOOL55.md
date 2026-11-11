# TOOL55 — migrated IMPLEMENTED tickets with a legacy DONE receipt; accept never overwrites a receipt

Status: OPEN (2026-11-11) | Priority: P1

Type: tool. Owner decision 2026-11-11 (LEGACY01 collision 1, option b). Tooling slot; must land before any migration
batch that contains such rows runs. Follows [TOOL54](TOOL54.md) and [LEGACY01](LEGACY01.md).

## Facts

- The LEGACY01 drafts (`docs/analysis/legacy-triage-2026-11-11.md`) move 165 legacy tickets to `IMPLEMENTED` whose
  `<ID>-STATUS-DONE.MD` receipt already exists (open clause per the owner's Q1, partly done, unverified commits).
  ADR-037 point 13 rejects a `-STATUS-DONE.MD` while line 3 is not `DONE` (`done-receipt-not-done`), so `migrate`
  refuses those rows.
- `cargo xtask accept` writes `<ID>-STATUS-DONE.MD` and refuses if it exists (TOOL41); receipts are immutable evidence
  (owner 2026-11-11, TOOL54 rework).

## Work

1. **status-check exception**, only for migrated tickets: a ticket whose `Migrated (owner batch N, …)` line records
   the target state `IMPLEMENTED` (state the exact marker the check reads; `migrate` may need to write it in the
   Migrated line) does not get `done-receipt-not-done` for its legacy DONE receipt. No other ticket gets the exception.
2. **`accept` never overwrites a receipt**: when `<ID>-STATUS-DONE.MD` exists, PASS writes the next free name
   `<ID>-STATUS-DONE-2.MD`, `-3`, … and line 3 `Receipt:` points to that file; the receipt parser, `status-check` and
   the gap rules accept the numbered names. FAIL keeps its current behaviour (appends to `-STATUS-UNFINISHED.MD`)
   unless the receipt says otherwise.
3. Host tests: the exception applies to a migrated IMPLEMENTED ticket and to no other; `accept` PASS on a ticket with a
   legacy DONE receipt → new `-STATUS-DONE-2.MD`, line 3 points to it, the old receipt byte-identical; a third
   acceptance after a reopen → `-3`; `status-check` 0 rejected afterwards.
4. The TOOL52 repository check and the TOOL54 rules stay as they are.

## Acceptance

- Tests of point 3 green; a mutation that lets `accept` overwrite the old receipt fails a test (tester shows it).
- `cargo xtask test` green, warnings delta 0, doc-check of the edited docs 0 failures.
