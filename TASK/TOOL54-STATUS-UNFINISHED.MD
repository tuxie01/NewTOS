# TOOL54 — receipt (unfinished: tester acceptance pending)

Role: developer
Agent: agent-a6eee2de23e7c5095

Worktree `.claude/worktrees/agent-a6eee2de23e7c5095`, branch `worktree-agent-a6eee2de23e7c5095`, base `fc05daf11`.

## Done

- `tools/xtask/src/tickets/migrate.rs` (new): `migrate --batch <N>`, the batch parser, the migrated text.
- `tools/xtask/src/tickets/migrate/tests.rs` (new): the host tests of Work 4.
- `tools/xtask/src/tickets.rs`: the `migrate` arm in `cmd`, `migration` in `NON_TICKETS`, `pub mod migrate;` at the
  end (two lines added before the cited functions; every anchored cite stays within its window, doc-check clean).
- `tools/xtask/src/repo_guard.rs`: `migrate` marked `Writes`; the guarded-commands test names it.
- `tools/xtask/src/main.rs`: the help text appends `tickets::migrate::USAGE` (same line).

## Formats (defined here, TOOL54 Work 2)

Batch file `TASK/migration/batch-<N>.tsv`, `<N>` decimal digits, committed and unchanged in the working tree; read
from `HEAD`.

- **Owner approval line:** line 1, exactly `# approved: owner YYYY-MM-DD`, optionally followed by one blank and a note
  (`# approved: owner 2026-11-12 the closed C1 tickets`). A record, not a proof (like the roles, ADR-037 point 12).
- Further `#` lines are comments, empty lines are skipped, a header row `id<TAB>state<TAB>fields<TAB>evidence` is
  skipped.
- **Rows:** four tab-separated columns.
  1. `id`: the ticket `TASK/<id>.md`.
  2. target state: `OPEN`, `IMPLEMENTED`, `DONE`, `PARKED`, `ACCEPTED`, `OBSOLETE`.
  3. fields: line-3 syntax, ` | `-separated, in the order of ADR-037 point 1, `Priority` first in every state:
     `Priority: P2` / `Priority: P1 | Commit: <hash> | Missing: ...` / `Priority: P1 | Commit: <hash> | Receipt:
     <ID>-STATUS-DONE.MD` / `Priority: P3 | Reason: ... | Reopen: ...` / `Priority: P2 | Ref: ADR-NNN` /
     `Priority: P3 | Reason: ...[ | Target: <ID>]`. `Commit: self` is refused. (Not `key=value;...`: values are
     prose and contain `;`, while `|` is already forbidden on line 3.)
  4. evidence: free text on one line; for `DONE` it must name the receipt file and the fix commit.
- Line 3's date is the day of the run.

**The switch that ends the migration:** the file `closed.txt` beside the batch files (content: the owner's
decision, date, note). Once it exists in the working tree or in `HEAD`, every run refuses. A committed file, not a
git config key, so the decision is in the history and holds in every clone and worktree.

**Second run of the same batch:** refuses (its ids left the baseline with the first run: "not in
tools/xtask/ticket-baseline.tsv"); nothing is written.

## Verification

- `cargo xtask test` (full): 2432 passed, 0 failed, 7 ignored (main `fc05daf11`: 2427/0/7; +5 migrate tests).
- `cargo clippy -p xtask --all-targets`: 1 (bin) + 9 (test) warnings, equal to `HEAD` `fc05daf11` built from a
  `git archive` copy; delta 0, none in the touched files.
- Mutation: the "not in the baseline" refusal disabled → `refusals_change_nothing` and
  `a_batch_of_every_target_state_migrates_in_one_commit` fail; restored.
- No real batch file is created by this ticket; the batch directory stays the researcher's (LEGACY01).

## For the tester

- `cargo test -p xtask -- tickets::migrate` (5 tests) and `repo_guard` (enumeration test).
- Show the mutation of the acceptance: e.g. `if false && !legacy.contains(id)` in `migrate` fails the tests.
- An end-to-end run in a scratch clone with a hand-written batch and the main tree's hooks on is the remaining
  check; the unit tests use `core.hooksPath=/dev/null` and check the staged content with the hooks' own
  `status-check --staged --new-only [--message-file]` logic.

## Rework (2026-11-11, owner decisions A-C)

Tester verdict on `f359f92b7`: PARTIAL; the owner decided A-C (TOOL54.md, *Owner decision (2026-11-11, rework)*).
Branch rebased onto main `8a67f0134`; the first commit is now `ce467ce9a`; the rework is one `fix(xtask)` commit on top.

- **A.** `docs/commit-map.txt` is read once per view (`Aids`); `names_commit_mapped` translates old hash tokens like
  `tools/oldhash.sh`; `evidence_names_commit` is the one rule (a receipt or a `Migrated` line of the ticket names the
  commit), used by `status-check` (`done-no-receipt`) and by `migrate` (the DONE row's evidence as the `Migrated` line
  it writes). A row with an old hash as `Commit` is refused with its translation. Receipts are not touched. The real
  A01 case: its receipt names `d7ab92d`, the real map translates that to `87b03c250`; the test also migrates the real
  A01 ticket and receipt in a scratch repository with a fixture map line to the scratch fix commit.
- **B.** `migrate` snapshots the files it writes; any failure after the first write (a rejecting `pre-commit` or
  `commit-msg` hook included) unstages exactly those paths and restores their bytes. Tested: work tree bytes,
  `git status`, staged blobs and `HEAD` identical to before; the rerun commits.
- **C.** `historical-receipts.tsv` beside the batch files: line 1 `# approved: owner YYYY-MM-DD [note]`, `#` comments,
  optional header `receipt<TAB>reason`, rows `<receipt file><TAB><reason>`. A listed orphan receipt is no
  `orphan-receipt` violation; its baseline entry is then stale and is removed by `status-check --prune-baseline` in the
  commit that adds the list. A list without the approval line or with a malformed row makes `status-check` fail. No
  receipt is moved; the real list is not created.
- Two fields (`Ticket::migrated`, `Analysis::aids`) moved the anchored cites into `tickets.rs` in TOOL41, TOOL46,
  TOOL51 and TOOL54 by up to 9 lines; updated to the lines doc-check names.

Verification of the rework:

- `cargo xtask test` (full): 2436 passed, 0 failed, 7 ignored (+4 rework tests). A clippy-only change
  (`std::slice::from_ref`) followed; `cargo test -p xtask -- tickets::migrate` after it: 9 passed.
- `cargo clippy -p xtask --all-targets`: 1 (bin) + 9 (test) warnings, as on main; delta 0.
- Mutations: `revert` a no-op → the hook test fails; the commit-map translation off → both A01 tests fail; restored.
- doc-check of TOOL41, TOOL46, TOOL51, TOOL54 and this receipt: 0 failures.

## Proposed docs/STATUS.md line (lead)

- 2026-11-11 — TOOL54 implemented: `cargo xtask migrate --batch <N>` rewrites line 3 of legacy tickets from an
  owner-approved, committed `TASK/migration/batch-<N>.tsv` (ADR-037 points 15-17), prunes their baseline entries,
  commits once; ended by the file `closed.txt` beside the batch files. Rework: old hashes translated through
  `docs/commit-map.txt` in `migrate` and `status-check`, a rejected commit reverted, an owner-approved list of
  historical receipts accepted as non-orphans. Acceptance pending.
