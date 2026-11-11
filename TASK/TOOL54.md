# TOOL54 — `cargo xtask migrate --batch <N>`: one-time migration of legacy tickets

Status: DONE (2026-11-11) | Priority: P1 | Commit: af80ca3da | Receipt: TOOL54-STATUS-DONE.MD

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

## Implementation (developer, 2026-11-11)

Receipt: [TOOL54-STATUS-UNFINISHED.MD](TOOL54-STATUS-UNFINISHED.MD).

- **Command.** `cargo xtask migrate --batch <N>` (`tools/xtask/src/tickets/migrate.rs:277@migrate`), dispatched by
  `tickets::cmd` (`tools/xtask/src/tickets.rs:153@cmd_migrate`) and marked writing for the TOOL52 check
  (`tools/xtask/src/repo_guard.rs:73@migrate`). The module comment defines the formats; usage text
  `tools/xtask/src/tickets/migrate.rs:89@USAGE`, shown by `cargo xtask help`.
- **Batch file** `TASK/migration/batch-<N>.tsv`, read from `HEAD`
  (`tools/xtask/src/tickets/migrate.rs:140@parse_batch`): line 1 is the owner's approval line
  `# approved: owner YYYY-MM-DD [note]` (`tools/xtask/src/tickets/migrate.rs:132@is_approval`); `#` lines are
  comments; a header row `id<TAB>state<TAB>fields<TAB>evidence` is skipped; each row has four tab-separated columns:
  id, target state, the fields in line-3 syntax in the order of ADR-037 point 1 (`Priority: Pn | Key: value | ...`,
  `Priority` always), the evidence. The fields go through `parse_line3`, so a row is valid exactly when its line 3
  is; `Commit: self` is refused.
- **Writing.** Line 3 from `Line3::render`, the state date the day of the run; below it an empty line, the line
  `Migrated (owner batch <N>, <date>), evidence: <evidence>`, an empty line, `Line 3 before the migration:` and the
  old line 3 onward verbatim (`tools/xtask/src/tickets/migrate.rs:229@migrated_text`); the ticket parses (tested).
  The baseline loses every entry of the migrated ids and no other; their `TASK/README.md` rows are regenerated; one
  commit `docs(task): migrate legacy tickets, owner batch <N>` through the hooks, the ids in the body (the
  `commit-msg` checks read ids from the subject only).
- **Refusals,** all before anything is written: the file `closed.txt` beside the batch files exists in the working
  tree or `HEAD` (`tools/xtask/src/tickets/migrate.rs:82@CLOSED_FILE`, the owner's end of the migration); no ticket
  entry left in the baseline; batch file not committed or changed since; no or malformed approval line; a missing
  required field or a line 3 that does not parse; an id twice, without a ticket, or not in the baseline (a second run
  of the same batch refuses here); a `Commit` not on the current history; `DONE` whose receipt `<ID>-STATUS-DONE.MD`
  does not exist or whose evidence does not name receipt and commit; a migrated ticket that would keep a rejecting
  `status-check` gap; staged changes, or uncommitted changes in a file it writes. Never writes a `-STATUS-DONE.MD`.
- `TASK/migration` is listed as a non-ticket entry of `TASK/` (`tools/xtask/src/tickets.rs:92@NON_TICKETS`); without
  it `status-check` rejects the committed batch file as an unknown file.
- **Tests** in scratch repositories under `target/`
  (`tools/xtask/src/tickets/migrate/tests.rs:155@a_batch_of_every_target_state_migrates_in_one_commit`,
  `tools/xtask/src/tickets/migrate/tests.rs:227@refusals_change_nothing`,
  `tools/xtask/src/tickets/migrate/tests.rs:297@refuses_once_the_baseline_holds_no_ticket`, plus the parser tests).

## Owner decision (2026-11-11, rework)

The tester's verdict on `f359f92b7` was PARTIAL (all checks green, two findings); the owner decided:

- **A.** `migrate` and `status-check` translate pre-2026-10-19 hashes through `docs/commit-map.txt` (the map of
  `tools/oldhash.sh`): a receipt naming an old hash that maps to the fix commit names it; the ticket's
  `Migrated (owner batch N, ...), evidence: ...` line counts as evidence naming the commit too. Receipts are never
  rewritten. One shared function for both. Test with the real case A01 (its receipt names `d7ab92d`).
- **B.** If a hook rejects the migration commit, `migrate` reverts exactly its own writes (ticket files, staging,
  baseline, README rows) to the state before the run; a rerun is possible.
- **C.** (LEGACY01 Q7) An owner-approved list `historical-receipts.tsv` beside the batch files (line 1 the approval
  line of the batch files, then one receipt file name per row with a reason column); `status-check` accepts the
  listed orphan receipts permanently, so their baseline entries can be pruned. No receipt file is moved.

## Implementation of the rework (developer, 2026-11-11)

- **A.** `CommitMap` reads `docs/commit-map.txt` (`old new`, `#` comments) and translates an old prefix of 7+ digits
  as `tools/oldhash.sh` does (`tools/xtask/src/tickets.rs:2085@CommitMap`); `names_commit_mapped`
  (`tools/xtask/src/tickets.rs:2111@names_commit_mapped`) is `names_commit` plus the translated tokens. The one shared
  rule is `evidence_names_commit` (`tools/xtask/src/tickets.rs:2121@evidence_names_commit`): the receipt or one of the
  ticket's `Migrated` lines (`Ticket::migrated`, `tools/xtask/src/tickets.rs:2127@migrated_lines`) names the commit.
  `status-check` uses it for the `done-no-receipt` gap (`tools/xtask/src/tickets.rs:975@evidence_names_commit`),
  `migrate` for the DONE row's evidence, as the `Migrated` line it writes
  (`tools/xtask/src/tickets/migrate.rs:311@Aids`). A row giving an old hash as `Commit` is refused with its
  translation. The map and the list of C are read once per view (`tools/xtask/src/tickets.rs:2163@Aids`, the
  `aids` field of `Analysis`).
- **B.** The run snapshots the files it writes (all clean, checked before), then writes, stages and commits; on any
  failure, a hook rejection included, `revert` (`tools/xtask/src/tickets/migrate.rs:467@revert`) unstages exactly
  those paths and puts their bytes back. The refusal says so.
- **C.** `parse_historical` (`tools/xtask/src/tickets.rs:2132@parse_historical`): line 1 the approval line
  (`is_approval`), `#` comments, a header row `receipt<TAB>reason` skipped, rows `<receipt file><TAB><reason>` with a
  receipt name `<ID>-STATUS-<DONE|UNFINISHED|FAILURE|BLOCKED>.MD` and a non-empty reason, no file twice; a malformed
  list makes `status-check` fail. A listed orphan receipt yields no `orphan-receipt` violation
  (`tools/xtask/src/tickets.rs:1030@historical`); its baseline entry is then stale and goes with
  `status-check --prune-baseline`, in the commit that adds the list. The real list is not created here.
- **Tests:** `tools/xtask/src/tickets/migrate/tests.rs:333@the_real_commit_map_translates_the_a01_receipt_hash`
  (the real map and receipt),
  `tools/xtask/src/tickets/migrate/tests.rs:355@the_a01_case_migrates_to_done_through_the_commit_map` (the real
  A01 ticket and receipt in a scratch repository, a fixture map line to its fix commit; refused without the line; the
  receipt bytes unchanged; the `Migrated` line alone satisfies `status-check`),
  `tools/xtask/src/tickets/migrate/tests.rs:424@a_rejecting_hook_reverts_every_write_and_a_rerun_works` (a
  `pre-commit` and a `commit-msg` hook that exit 1: work tree bytes, `git status`, staged blobs and `HEAD` identical
  to before; the rerun commits),
  `tools/xtask/src/tickets/migrate/tests.rs:449@historical_receipts_are_no_orphans_once_listed`.
- Two lines added to `Ticket` and to `Analysis` shifted the anchored cites into `tickets.rs` in TOOL41, TOOL46 and
  TOOL51; they are updated to the new lines.

