# TOOL41 — ticket tools: status-check, accept, set-status, report, commit-msg hook

Status: IMPLEMENTED (2026-11-11) | Priority: P1 | Commit: self | Missing: DONE through cargo xtask accept; tester PASS in measurements/measurement-2026-11-11-tool41-retest/RUNINFO.TXT

Type: tool (enforcement). Owner decisions 2026-11-10/11: ADR-037 with its addendum of 2026-11-11 (points 11–14,
decisions e–g) in `docs/MEMORY.md`. Order: starts only after [TOOL40](TOOL40.md) is done — both change
`tools/git-hooks/` and the xtask hook plumbing. Hooks are switched on only after the owner's OK. Developed in its own
git worktree; the main tree's hooks change only when TOOL41 is merged after the tester's acceptance (owner
2026-11-11, CLAUDE.md *Commit Discipline*). It lands with `git merge --squash` and one normal `git commit -F` in the main
tree, never as a fast-forward (owner 2026-11-11).

## Facts

- `docs/analysis/ticket-inventory-2026-11-10.md`: 657 tickets, 24 status words, classes C1–C6 (51 done but not
  closed, 23 closed without evidence, 23 STATUS.md mismatches, 81 parked without reason), measured with keyword
  heuristics because no line 3 is machine-readable yet.
- Tickets in the ADR-037 form so far: [TOOL01](TOOL01.md) (`PARKED`), [TOOL39](TOOL39.md) (`DONE`, closed by hand
  before these tools existed), [TOOL40](TOOL40.md) and this ticket (`OPEN`).
- The pre-commit hook (`tools/git-hooks/pre-commit`, TOOL29, extended by TOOL40) runs xtask checks on the index; the
  `commit-msg` stage does not exist yet.

## Work

1. **Line-3 parser** (one module, used by every command below): the form of ADR-037 point 3, the six states, the
   required fields per state (point 1, `Commit: self` per point 11), `Priority` `P1`–`P4` always second, ASCII only,
   no `|` in values. Errors name ticket, field and what was expected. The non-ticket files of point 9 are an
   explicit list in the code.
2. **`cargo xtask status-check`** — the gap check of point 13 and the classes C1–C6 of the inventory, over all tickets
   (report) or `--new-only` against a committed baseline file listing today's legacy violations (only the tickets the
   commit touches or names are checked; a ticket that leaves the baseline cannot come back). `--staged` reads the
   index. Resolves `Commit: self` with `git log -L3,3`. Rejecting gaps exit != 0; reporting ones (blocked ticket now
   `DONE`, self-acceptance per point 12) print without failing. Checks `docs/STATUS.md` lists against line 3
   (point 8).
3. **`cargo xtask accept <ID> --commit <hash> --result PASS|FAIL [--note ...] --agent <name>`** — refuses unless
   line 3 is `IMPLEMENTED`, `<hash>` exists on the current history and its message names the id (word boundary), and
   `<name>` differs from the developer recorded for the ticket. `PASS`: writes `<ID>-STATUS-DONE.MD` (commit, result,
   role tester, agent, note) and sets line 3 to `DONE`; `FAIL`: writes `<ID>-STATUS-UNFINISHED.MD`, line 3 stays.
   Commits by itself with a generated message (decision e); refuses with other staged changes present.
4. **`cargo xtask set-status <ID> PARKED|OBSOLETE|ACCEPTED|OPEN ...`** — lead only (point 11, decision f): mandatory
   `--reason`/`--reopen`, `--reason [--target]`, `--ref`, `--reason` per state. Never `IMPLEMENTED` or `DONE`.
   Writes line 3; the lead commits.
5. **`cargo xtask report`** — counts per state and per violation class, from the repo only; one short summary form
   for a SessionStart hook (not installed by this ticket).
6. **Hooks.** A `commit-msg` hook for the checks that need the message (point 14: `fix`/`feat` subject naming an id
   whose staged line 3 is `OPEN` or `DONE`; every named id, decision g), and the `--staged --new-only` gap check added
   to `pre-commit`. `cargo xtask install-hooks` installs both, but the new checks stay **off** until the owner
   switches them on (a documented switch, e.g. a git config key the hook reads); with the switch off the hook
   behaves exactly as after TOOL40.
7. **Host tests** for the parser (every state, every missing field, non-ASCII, `self`), each gap of point 13, the
   baseline (legacy passes, new fails, a fixed ticket cannot regress), `accept` (PASS, FAIL, self-acceptance refused,
   wrong state refused, unknown commit refused), `set-status` (missing fields refused), and the hooks in a temp repo
   (switch off = unchanged; switch on = each rejection).
8. **Docs.** `TASK/README.md` points to ADR-037 for the states; its status column is generated from line 3 (a
   `cargo xtask report --readme` or similar; owner 2026-11-10). The developer proposes the CLAUDE.md text (Commit
   Discipline: roles, `accept`, the switch) in the receipt; the lead applies it. The agent briefs in `.claude/agents/`
   (developer, tester) name their command — proposed text in the receipt too.
9. **No legacy migration.** Existing tickets are not rewritten; the baseline file records them. Cleanup batches come
   after this ticket, one owner decision each (ADR-037 point 7).

## Acceptance

- In a throwaway clone with the switch on: a `fix(...) (X)` commit with `X` `OPEN` → rejected by `commit-msg`; the
  same commit with line 3 `IMPLEMENTED | Commit: self | Missing: ...` → passes; a DONE receipt without line 3 `DONE`
  → rejected; `accept X --result PASS` by another agent → one commit, receipt + line 3 `DONE`; by the developer's
  agent → refused; `set-status Y PARKED` without `--reopen` → refused.
- With the switch off: the hook output and exit codes equal TOOL40's on the same commits.
- `cargo xtask status-check` over today's tree with the committed baseline → exit 0; `cargo xtask report` prints the
  counts.
- `cargo xtask test` green, warnings delta 0, doc-check of the edited docs 0 failures; hook runtime per case reported.

## Implementation (2026-11-11, developer)

Worktree branch `worktree-agent-a823a57e109be35e1`, based on `d033a78a2`. Receipt:
[TOOL41-STATUS-UNFINISHED.MD](TOOL41-STATUS-UNFINISHED.MD).

- **Parser** (Work 1): `tools/xtask/src/tickets.rs:297@parse_line3`, the one parser of all commands. Six states,
  required keys in the order of ADR-037 point 1, `Target` optional for `OBSOLETE`, no other key, `Priority: P1`–`P4`
  second, `(YYYY-MM-DD)`, ASCII only, ` | ` as separator and no `|` in a value. `Commit` is `self` or 7–40 hex digits
  for `IMPLEMENTED`, a hash for `DONE`; `Receipt` must be `<ID>-STATUS-DONE.MD`; `Ref` is `ADR-NNN` (a heading in
  `docs/MEMORY.md`) or a `docs/COMPATIBILITY.md` entry. Each error names ticket, line, field and expectation. A line 3
  whose state word is not one of the six is *legacy*; the report maps it by point 2
  (`tools/xtask/src/tickets.rs:409@legacy_state`). The non-tickets of point 9 are `NON_TICKETS`
  (`tools/xtask/src/tickets.rs:92@NON_TICKETS`).
- **`cargo xtask status-check`** (Work 2, `tools/xtask/src/tickets.rs:1362@check`): rejecting gaps of point 13
  (`IMPLEMENTED` without `Commit`/`Missing`, `PARKED` without `Reason`/`Reopen`, a DONE receipt while line 3 is not
  `DONE`, `DONE` without a DONE receipt that names its commit), a malformed line 3, a `Commit` hash that is no commit,
  a `Target`/`Ref` naming nothing, an orphan receipt, an unknown file in `TASK/`. Reporting: blocked by a `DONE`
  ticket (point 10), self-acceptance (point 12), an `OPEN`/`PARKED` ticket a `fix`/`feat` commit of the history names
  (close to the inventory's C1), the `docs/STATUS.md` lists against line 3 (point 8; keyword candidates like the inventory's C3, so reporting
  only), a stale `TASK/README.md` column. `--new-only`: only the tickets the change touches (a ticket file or any of
  its receipts); `--staged` reads the index; `--message-file`: the ids the subject names plus the `fix`/`feat` rule
  (`tools/xtask/src/tickets.rs:1047@message_violations`). `Commit: self` resolves with `git log -L3,3`
  (`tools/xtask/src/tickets.rs:1312@resolve_self`), which follows a rename of the ticket file.
- **Baseline** `tools/xtask/ticket-baseline.tsv` (899 entries: 656 `form`, 147 `parked-no-reason`, 53
  `done-no-receipt`, 22 `done-receipt-not-done`, 21 `orphan-receipt`): `<key>\t<code>`. A listed violation passes, any
  other rejects; an entry whose violation is gone rejects until `--prune-baseline` removes it; the checked baseline
  may hold no entry `HEAD`'s lacks (`tools/xtask/src/tickets.rs:1113@against_baseline`). So a ticket that leaves the
  baseline cannot come back. `set-status` and `accept` prune the entries of the ticket they write.
- **`cargo xtask accept`** (Work 3, `tools/xtask/src/tickets.rs:1753@accept`): refuses unless nothing is staged, the
  files it writes are clean, line 3 is `IMPLEMENTED`, `--commit` is an ancestor of `HEAD` whose message names the id,
  a developer is recorded and the acceptor is not one. `PASS`: `<ID>-STATUS-DONE.MD` (role tester, agent, commit,
  result, note) and line 3 `DONE (date) | Priority | Commit: <9-digit hash> | Receipt: ...`, plus the README cell;
  `FAIL`: a `## Acceptance <date> — FAIL` section appended to `<ID>-STATUS-UNFINISHED.MD` (the developer's header
  stays), line 3 unchanged. One commit with a generated `docs(task): accept <ID>, PASS|FAIL on <hash> (<ID>)`
  message, through the hooks. The message ends, for `PASS` and `FAIL`, with the trailer every commit of this repository
  carries, as its own last paragraph (ADR-037 decision h; `tools/xtask/src/tickets.rs:1745@ACCEPT_TRAILER`).
- **Roles** (point 12): a receipt's header (the lines before its first `## `) carries `Role: developer|tester` and
  `Agent: <name>` (`tools/xtask/src/tickets.rs:615@header_fields`); the developer of a ticket is every `Agent` of a
  header with `Role: developer`, compared trimmed and case-insensitively.
- **`cargo xtask set-status`** (Work 4, `tools/xtask/src/tickets.rs:1631@set_status`): `PARKED --reason --reopen`,
  `OBSOLETE --reason [--target]`, `ACCEPTED --ref`, `OPEN --reason` (only from `PARKED`, decision f); never
  `IMPLEMENTED`/`DONE`, never on a `DONE` ticket; keeps the priority (legacy `Prioritaet`/`Priorität` too) or takes
  `--priority`. Writes line 3 and a `## Status history` entry (the old line 3; for `OPEN` the reason), prunes the
  baseline, regenerates the README cell; does not commit.
- **`cargo xtask report`** (Work 5, `tools/xtask/src/tickets.rs:1981@cmd_report`): states (ADR-037 and mapped legacy),
  gap groups, codes, the ADR-037 tickets with `self` resolved; `--summary` one line (for a SessionStart hook,
  not installed); `--readme [--check]` regenerates the Status column of every `TASK/README.md` table whose rows start
  with a ticket link (`tools/xtask/src/tickets.rs:1231@readme_with_status`): the state, or the raw legacy text.
- **Hooks** (Work 6): new `tools/git-hooks/commit-msg`, and step 3 in `tools/git-hooks/pre-commit`
  (`status-check --staged --new-only`, only for a staged `TASK/` path or baseline). Both run only with
  `git config newtos.ticketChecks true`; off (unset, `false`), `commit-msg` exits 0 at once and `pre-commit` behaves
  as at `ced5e68f6`. `cargo xtask install-hooks` lists both (`tools/xtask/src/git_hooks.rs:30@HOOKS`) and reports the
  switch.
- **Tests** (Work 7): 20 host tests in `tools/xtask/src/tickets/tests.rs`, among them the hooks run against TOOL40's
  `pre-commit` from `ced5e68f6` with a fake `cargo` (switch unset, `false`, `no`: equal exit code, stdout, stderr and
  cargo calls).
- **Fixes after tester PARTIAL** (run measurement-2026-11-11-tool41-test): (D1) decision g had
  no host test, the tester's mutants d1–d3 (only the first named id checked) survived; the test
  `every_named_id_is_checked_not_only_the_first` names two ids (second `OPEN`, `DONE`, `PARKED`, or missing a field)
  and kills all ten mutants of the tester's `mutants.py`. (D2) `report` no longer labels its groups C1–C6: its rules
  are machine rules, not the inventory's keyword classes (C5 656 vs 11, C4 147 vs 81); the groups are named
  `done-not-closed`, `closed-without-evidence`, `status-md-mismatch`, `parked-without-fields`, `not-adr037-form`,
  `dangling-reference` (`tools/xtask/src/tickets.rs:1936@GROUPS`).

## Acceptance (tester, 2026-11-11)

First run PARTIAL (`measurements/measurement-2026-11-11-tool41-test/RUNINFO.TXT`): all 22 acceptance cases and the
switch-off comparison passed, but decision g (every named id) had no host test (mutants d1-d3 survived) and `report`
reused the labels C1-C6 with other meanings. Fixed on the branch. Re-run by another tester PASS
(`measurements/measurement-2026-11-11-tool41-retest/RUNINFO.TXT`): 10 of 10 mutants killed, 2396/0/6, acceptance and
switch-off cases unchanged after the rebase onto TOOL42. The ticket checks stay off until the owner sets
`newtos.ticketChecks`.
