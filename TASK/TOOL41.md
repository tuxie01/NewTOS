# TOOL41 — ticket tools: status-check, accept, set-status, report, commit-msg hook

Status: OPEN (2026-11-11) | Priority: P1

Type: tool (enforcement). Owner decisions 2026-11-10/11: ADR-037 with its addendum of 2026-11-11 (points 11–14,
decisions e–g) in `docs/MEMORY.md`. Order: starts only after [TOOL40](TOOL40.md) is done — both change
`tools/git-hooks/` and the xtask hook plumbing. Hooks are switched on only after the owner's OK. Developed in its own
git worktree; the main tree's hooks change only when TOOL41 is merged after the tester's acceptance (owner
2026-11-11, CLAUDE.md *Commit Discipline*).

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
