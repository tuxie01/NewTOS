# TOOL55 — receipt (developer, acceptance missing)

Role: developer
Agent: agent-ac416cda8b0fce8e5

## Done

- The `done-receipt-not-done` exception for tickets whose `Migrated` line records the target state `IMPLEMENTED`
  (marker: `Migrated (owner batch <N>, <YYYY-MM-DD>, to IMPLEMENTED), evidence: ...`, written by `migrate` since this
  change); only their legacy `<ID>-STATUS-DONE.MD` is exempt.
- `accept` PASS writes the next free `<ID>-STATUS-DONE[-<n>].MD` and never overwrites a receipt; line 3 `Receipt:`
  points to it; parser, `status-check` and gap rules accept numbered names.
- Details and cites: [TOOL55.md](TOOL55.md), *Implementation*.

## Rework (owner decision 2026-11-11)

- a) The exception now covers every migrated target state but `DONE` (`to <STATE>` in the `Migrated` parenthesis),
  and only a legacy receipt older than the migration: its current text (working tree or index) equals its text at
  the parent of the commit that added the `Migrated` line (`git log --format=%H -S<line> -- TASK/<ID>.md`, oldest
  entry `M`; `git show M^:TASK/<receipt>`; `HEAD` while the line is not committed yet).
- b) `accept` on a migrated ticket without a `Role: developer` receipt accepts and writes
  `Developer: legacy, developer unknown` plus `Self-acceptance: not applicable (ADR-037 point 12); ...` in the
  receipt header; `status-check` does not flag self-acceptance for such a receipt. A non-migrated ticket without a
  developer still refuses.
- Details and cites: [TOOL55.md](TOOL55.md), *Implementation of the rework*.

## Missing

- Tester acceptance through `cargo xtask accept TOOL55`, including the mutation that lets `accept` overwrite the old
  receipt (it must fail `accept_writes_numbered_receipts_and_never_overwrites_one`).

## Scratch (TOOL46)

No clone; the test repositories lie under the worktree's own `target/` (`xtask-test-*`), no gitignored output is
cited.
