# TOOL55 — migrated IMPLEMENTED tickets with a legacy DONE receipt; accept never overwrites a receipt

Status: DONE (2026-11-11) | Priority: P1 | Commit: c0912b390 | Receipt: TOOL55-STATUS-DONE.MD

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

## Implementation (developer, 2026-11-11)

Receipt: [TOOL55-STATUS-UNFINISHED.MD](TOOL55-STATUS-UNFINISHED.MD).

- **The marker.** `migrate` now writes the target state into the parenthesis of the `Migrated` line:
  `Migrated (owner batch <N>, <YYYY-MM-DD>, to <STATE>), evidence: <evidence>`
  (`tools/xtask/src/tickets/migrate.rs:223@migrated_line`, called with the row's state at
  `tools/xtask/src/tickets/migrate.rs:338@migrated_line`). `migrated_target`
  (`tools/xtask/src/tickets.rs:2243@migrated_target`) reads exactly that parenthesis: three parts, batch number,
  date, `to <STATE>`; the words in the evidence do not count. The lines of batches 1-3 (no target state; their
  tickets are OBSOLETE/DONE) record none and get no exception. (First version: only `to IMPLEMENTED` counted;
  widened by the rework below.)
- **The exception.** `stray_done_receipts` (`tools/xtask/src/tickets.rs:2267@stray_done_receipts`) lists the DONE
  receipts that are a `done-receipt-not-done` gap: while line 3 is not `DONE`, every DONE receipt of the ticket,
  plain or numbered, except the legacy `<ID>-STATUS-DONE.MD` of a migrated ticket as the rework below defines; used
  by `ticket_violations` (`tools/xtask/src/tickets.rs:962@stray_done_receipts`), so `status-check` and `migrate`'s
  after-migration check apply the same rule. A numbered receipt of such a ticket (after a later reopen) gets no
  exception.
- **Numbered receipts.** `done_number`, `is_receipt_state`, `done_receipt_number`, `done_receipt_file`
  (`tools/xtask/src/tickets.rs:2189@done_number`, `tools/xtask/src/tickets.rs:2206@done_receipt_number`): a receipt
  `<ID>-STATUS-DONE-<n>.MD`, `n >= 2` without a leading zero, is a receipt (`Analysis::load`,
  `tools/xtask/src/tickets.rs:698@is_receipt_state`), line 3 `Receipt:` may name it
  (`tools/xtask/src/tickets.rs:399@done_receipt_number`), and `Analysis::done_receipt` takes the receipt line 3
  names (`tools/xtask/src/tickets.rs:731@done_receipt`, `tools/xtask/src/tickets.rs:2230@line3_done_receipt`) for
  the `done-no-receipt` and self-acceptance checks.
- **`accept` never overwrites.** PASS writes `next_done_receipt`
  (`tools/xtask/src/tickets.rs:1788@next_done_receipt`, `tools/xtask/src/tickets.rs:2286@next_done_receipt`):
  `<ID>-STATUS-DONE.MD` while the ticket has no DONE receipt, else the number after the highest (`-2`, `-3`, ...);
  the file is created with `create_new` (`tools/xtask/src/tickets.rs:2300@write_new`); line 3 `Receipt:` and the
  commit message name it. FAIL is unchanged. The TOOL52 repository check and the TOOL54 rules are untouched.
- **Tests:** `tools/xtask/src/tickets/tool55_tests.rs:56@the_marker_is_the_target_state_in_the_migrated_parenthesis`,
  `tools/xtask/src/tickets/tool55_tests.rs:80@numbered_done_receipts_parse_and_name`,
  `tools/xtask/src/tickets/tool55_tests.rs:201@accept_writes_numbered_receipts_and_never_overwrites_one` (`-2`, the
  legacy receipt byte-identical, a reopen and `-3`, `status-check` 0 rejected after each, FAIL unchanged); the tests
  of the rework below.

## Owner decision (2026-11-11, rework)

- **a)** The exception applies to every migrated target state except `DONE`: a ticket whose `Migrated` parenthesis
  carries `to <STATE>` with `STATE` not `DONE` keeps its legacy `<ID>-STATUS-DONE.MD` without
  `done-receipt-not-done`, and only for a receipt older than the migration commit (its last change precedes the
  commit that wrote the `Migrated` line); a receipt added or changed after the migration gets no exception. A ticket
  without the `to <STATE>` tag gets none; a `DONE` target needs none and gets none.
- **b)** `accept` on a migrated ticket (a `Migrated` line present) without any `Role: developer` receipt accepts
  instead of refusing; the receipt records `Developer: legacy, developer unknown` and a line that the self-acceptance
  check (ADR-037 point 12) does not apply. A non-migrated ticket without a developer still refuses; `status-check`'s
  self-acceptance flag does not fire for such receipts.
- Numbered receipts and the never-overwrite rule stay as built.

## Implementation of the rework (developer, 2026-11-11)

- **a) Target states.** `migrated_not_done` (`tools/xtask/src/tickets.rs:2257@migrated_not_done`): the ticket's
  `Migrated` lines whose `to <STATE>` is not `DONE` (read by `migrated_target`).
- **a) Receipt age.** `receipt_predates_migration` (`tools/xtask/src/tickets.rs:2350@receipt_predates_migration`):
  the receipt's text as the check sees it (working tree, or the index for `--staged`) equals its text just before the
  commit that wrote one of those `Migrated` lines. That text comes from `History::text_before_line`
  (`tools/xtask/src/tickets.rs:2312@History`, `tools/xtask/src/tickets.rs:2321@GitRepo`), two git queries:
  `git log --format=%H -S<Migrated line> -- TASK/<ID>.md`, whose oldest entry is the commit `M` that added the line;
  then `git show M^:TASK/<receipt>`. While no commit holds the line yet (the run of `migrate` and the hooks of its
  commit) the revision is `HEAD`. A receipt absent at `M^` (added in or after `M`) or different from it (changed in
  or after `M`, committed or not) gets no exception. `History` is a supertrait of `Repo`
  (`tools/xtask/src/tickets.rs:911@Repo`), implemented for `GitRepo` only.
- **b)** `accept` refuses for a missing developer only when the ticket has no `Migrated` line
  (`tools/xtask/src/tickets.rs:1831@devs`); the DONE receipt then carries the header lines of
  `legacy_developer_header` (`tools/xtask/src/tickets.rs:2363@legacy_developer_header`):
  `Developer: legacy, developer unknown` and `Self-acceptance: not applicable (ADR-037 point 12); ...`, and the body
  names the developer through `developer_text` (`tools/xtask/src/tickets.rs:1848@developer_text`). The
  self-acceptance flag skips a receipt with that header (`tools/xtask/src/tickets.rs:1011@legacy_developer_receipt`,
  `tools/xtask/src/tickets.rs:2385@legacy_developer_receipt`).
- **Tests:**
  `tools/xtask/src/tickets/tool55_tests.rs:121@the_exception_applies_to_every_non_done_target_with_an_older_receipt_only`
  (IMPLEMENTED, PARKED, OBSOLETE, OPEN, ACCEPTED with an older receipt: no gap; a receipt changed after the
  migration, added after it, added in the migration commit, changed uncommitted, and the same change staged in the
  index view: gap; a DONE target now PARKED: gap; DONE target and DONE: none needed; no `to` tag, not migrated, the
  tag only in the evidence, a numbered receipt: gap),
  `tools/xtask/src/tickets/tool55_tests.rs:251@accept_of_a_migrated_ticket_without_a_developer_records_the_legacy_developer`
  (a non-migrated ticket without a developer refused; the migrated one accepted with the legacy developer header,
  its legacy receipt byte-identical; no self-acceptance flag even with a later developer receipt naming the acceptor,
  the flag fires once the header is removed),
  `tools/xtask/src/tickets/migrate/tests.rs:500@an_implemented_row_with_a_legacy_done_receipt_migrates` (refused
  while the receipt has an uncommitted change, migrates once it equals the committed one). The TOOL54 refusal test's
  `done-receipt-not-done` case now edits L3's receipt in the working tree, since an OBSOLETE row with an older
  receipt is exempt (`tools/xtask/src/tickets/migrate/tests.rs:266@l3_receipt`).
