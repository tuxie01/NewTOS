# TOOL54 — `cargo xtask migrate --batch <N>`: one-time migration of legacy tickets

Status: OPEN (2026-11-11) | Priority: P1

Type: tool. Owner decision 2026-11-11 (legacy ticket clean-up before the inventory creates new tickets). Implements the
ADR-037 addendum on the one-time migration (once accepted by the owner; text shown to the owner before commit). Runs on
the tooling slot. Goes with [LEGACY01](LEGACY01.md), the classification.

## Facts

- `tools/xtask/ticket-baseline.tsv` lists the legacy violations recorded by TOOL41: 680 keys; by code 656 `form`,
  147 `parked-no-reason`, 53 `done-no-receipt`, 22 `done-receipt-not-done`, 21 `orphan-receipt` (2026-11-11). It only
  shrinks (`status-check --prune-baseline`).
- `cargo xtask set-status` (TOOL41) changes one ticket after an owner decision; there is no batch path, and none that
  records the evidence of a migrated state.

## Work

1. `cargo xtask migrate --batch <N>` reads `TASK/migration/batch-<N>.tsv` (columns `id`, target state, the fields
   ADR-037 point 1 requires for it, evidence) and for each row writes line 3 in ADR-037 form plus the line
   `Migrated (owner batch <N>, <date>), evidence: <evidence>`; prunes the migrated entries from the baseline; commits
   once (subject names the batch), through main's hooks.
2. Refuses: an id not in the baseline; a missing required field; a batch file not committed or without the owner's
   approval line (format defined in the receipt); any run once the baseline holds no ticket entry or after an owner
   decision that ends the migration (a switch the receipt names). Never writes `-STATUS-DONE.MD`.
3. The [TOOL52](TOOL52.md) repository check applies.
4. Host tests in a scratch repo: each refusal; a batch of each target state (DONE, OBSOLETE with Target, PARKED with
   Reason/Reopen, OPEN); baseline shrinks by exactly the migrated keys; second run of the same batch refuses or is a
   no-op (state which).

## Acceptance

- Tests of point 4 green; a mutation that migrates a ticket not in the baseline fails a test (tester shows it).
- `cargo xtask test` green, warnings delta 0, doc-check of the edited docs 0 failures.
