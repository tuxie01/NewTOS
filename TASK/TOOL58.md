# TOOL58 — status-check verifies every Migrated line against its approved batch file

Status: IMPLEMENTED (2026-11-11) | Priority: P1 | Commit: self | Missing: tester acceptance through cargo xtask accept

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

## Implementation (developer, 2026-11-11)

Receipt: [TOOL58-STATUS-UNFINISHED.MD](TOOL58-STATUS-UNFINISHED.MD).

- **The committed batch files.** `committed_batches` (`tools/xtask/src/tickets.rs:2455@committed_batches`) reads every
  `TASK/migration/batch-<N>.tsv` from `HEAD` for the working-tree view and from the index for `--staged` (the
  pre-commit hook), and parses each with `migrate::parse_batch`, the parser `migrate` used (approval line, rows,
  fields); a file that does not parse is kept as its errors. Loaded once per view into `Aids::batches`
  (`tools/xtask/src/tickets.rs:2191@committed_batches`). A batch file only in the working tree does not count.
- **The one check.** `verify_migrated` (`tools/xtask/src/tickets.rs:2511@verify_migrated`) on a line parsed by
  `parse_migrated` (`tools/xtask/src/tickets.rs:2434@parse_migrated`; the parenthesis ends at the first
  `), evidence: `, as in `migrated_target`): the named batch file is committed, approved and parses; it has a row for
  the ticket's id; the `to <STATE>` part, where present, equals the row's state; the evidence equals the row's
  evidence column byte for byte (`migrate` writes `migrated_line(n, date, row.state, &row.evidence)` and
  `parse_batch` refuses an evidence with surrounding blanks, so the comparison is exact). The date is not compared:
  the batch file has no date column.
- **Batches 1-13 (no `to <STATE>`): the row alone.** Such a line records no state, so there is nothing to compare
  with the row's state; no consumer reads a state from it (the receipt exception requires `to <STATE>`, ADR-037
  point 18; `accept` and the evidence rule only need a matching line); and line 3 of its migration commit was
  rendered by `migrate` from that same row, so reading it back from that commit (`git log -S` per ticket, 383
  lines, in every pre-commit run) would only restate the row. A forged line still needs an exact id and evidence
  match in a committed, approved batch file.
- **Its users.** `status-check` (and with it the hooks): `ticket_violations` adds one rejecting
  `migrated-unverified` violation per failing line, naming ticket file, line and batch
  (`tools/xtask/src/tickets.rs:966@migrated_violations`, `tools/xtask/src/tickets.rs:2542@migrated_violations`); the
  code is never baselined (`tools/xtask/src/tickets.rs:798@MigratedUnverified`). Only verified lines
  (`tools/xtask/src/tickets.rs:2536@verified_migrated`) count for the TOOL55 receipt exception
  (`tools/xtask/src/tickets.rs:2286@verify_migrated`), for the `done-no-receipt` evidence rule of TOOL54 rework A
  (`tools/xtask/src/tickets.rs:981@verified_migrated`) and for `accept`'s legacy-developer path
  (`tools/xtask/src/tickets.rs:1852@verified_migrated`; the refusal names the mismatch, `unverified_note`).
  `migrate`'s own after-migration check sees its batch, committed before the run, in `HEAD`.
- **The real tree.** `status-check` prints `656 of 656 Migrated lines match their owner-approved batch row`
  (`tools/xtask/src/tickets.rs:1414@migrated_counts`, `tools/xtask/src/tickets.rs:2565@migrated_counts`), in the
  working-tree and the `--staged` view: every migrated ticket of batches 1-22 (rows 40+40+43+29+28+16+13+12+39+39+
  39+36+9+46+46+25+24+43+43+43+2+1 = 656).
- **Tests** (`tools/xtask/src/tickets/tool58_tests.rs`):
  `tools/xtask/src/tickets/tool58_tests.rs:57@parse_migrated_reads_what_migrate_writes`;
  `tools/xtask/src/tickets/tool58_tests.rs:80@hand_written_and_mismatching_lines_are_rejected_real_ones_pass` (no batch
  file, wrong batch number, id in no batch, state differs, evidence differs by one character, batch file without
  approval line, batch file only in the working tree, malformed line: rejected with ticket, line and batch named;
  a line with and one without `to <STATE>` pass; the staged batch file verifies under the index view, not under the
  working-tree view); `tools/xtask/src/tickets/tool58_tests.rs:152@no_consumer_trusts_an_unverified_line` (a
  hand-written `to IMPLEMENTED` line gets no receipt exception, a hand-written DONE evidence line names no commit,
  `accept` refuses the legacy-developer path for it and takes it for the real one);
  `tools/xtask/src/tickets/tool58_tests.rs:208@the_real_migrated_lines_all_match_their_batch_rows`. Mutation: with
  the evidence comparison skipped, the second test fails (run by hand). The TOOL55 and TOOL54 tests that wrote
  `Migrated` lines by hand now commit their batch rows
  (`tools/xtask/src/tickets/tool55_tests.rs:330@write_batch`, `tools/xtask/src/tickets/migrate/tests.rs:393@batch`).

## Owner decision context (rework, 2026-11-11)

The tester's verdict on `ca1552318` was PARTIAL (the tester run `measurement-2026-11-11-tool58-test`, not committed when this was written):
every hand-written or mismatching ticket line is rejected, but the batch files were not protected: (a) a commit
editing a batch row and (b) one deleting a batch's approval line passed the hook (only staged tickets were checked;
the next full run caught them); (c) a consistent forgery, batch row and ticket line changed together, was green. The
migration is closed (`TASK/migration/closed.txt`, owner 2026-11-11: baseline empty, migration closed), so its
evidence is frozen from then on. The lead's rework: 1. a staged batch file re-checks every ticket naming it; 2. once
`closed.txt` is committed, any staged addition, change, deletion or rename under `TASK/migration/` rejects, and a
full `status-check` compares `TASK/migration/` in `HEAD` with the commit that added `closed.txt`; 3. tests.

## Implementation of the rework (developer, 2026-11-11)

- **Staged batch files re-check their tickets.** Under `--staged` (the pre-commit hook), `migration_evidence`
  (`tools/xtask/src/tickets.rs:2630@migration_evidence`, called from `check` at
  `tools/xtask/src/tickets.rs:1404@migration_evidence`) reads the staged changes under `TASK/migration/`
  (`tools/xtask/src/tickets.rs:2589@staged_migration_changes`: `git diff --cached --name-status --no-renames HEAD`)
  and adds every ticket whose `Migrated` line names a changed batch to the checked keys
  (`tools/xtask/src/tickets.rs:2597@tickets_naming_batches`). The staged view reads the batch files from the index,
  so an edited row or a dropped approval line rejects that ticket (`migrated-unverified`) in the commit itself.
- **The closed migration is frozen.** The closing commit is the oldest commit of `HEAD`'s history that added
  `closed.txt` (`tools/xtask/src/tickets.rs:2581@migration_closing_commit`); taken from the history, so deleting
  the file does not lift the freeze. Once it exists, every staged addition, change or deletion under
  `TASK/migration/` (batch files, `historical-receipts.tsv`, `closed.txt` itself; a rename is a deletion plus an
  addition, both named) is the rejecting, never-baselined `migration-frozen` violation keyed by the file
  (`tools/xtask/src/tickets.rs:801@MigrationFrozen`). A full `status-check` (neither `--staged` nor `--new-only`)
  compares the blobs of `TASK/migration/` in `HEAD` with those of the closing commit
  (`tools/xtask/src/tickets.rs:2606@migration_blobs`) and rejects a changed, missing or added file, so a change
  committed past the hook is caught too. The commit-msg check (`--message-file`) is unchanged.
- **The real tree.** `TASK/migration/` in `HEAD` equals the closing commit `d1d6e5de8`; `status-check` 0 rejected,
  656 of 656 Migrated lines match.
- **Tests:** `tools/xtask/src/tickets/tool58_tests.rs:277@a_staged_batch_file_rechecks_every_ticket_naming_it`
  (migration open: an edited row and a deleted approval line reject the ticket naming the batch in the staged check,
  the other batch's ticket stays unchecked, the restored file passes);
  `tools/xtask/src/tickets/tool58_tests.rs:304@the_closed_migration_is_frozen` (closed: the consistent forgery of
  finding c, a deleted approval line, a new batch file, a deleted `closed.txt` and a changed historical list each
  reject in the staged check, naming the file; the untouched tree passes; committed past the hook, the full check
  rejects the forged batch file, then also the deleted `closed.txt` and an added batch);
  `tools/xtask/src/tickets/tool58_tests.rs:228@the_real_migration_dir_equals_the_closing_commit`. Mutation: without
  the re-check of point 1 both scratch tests fail (run by hand).
