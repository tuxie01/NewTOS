# TOOL52 — receipt (unfinished: tester acceptance pending)

Role: developer
Agent: agent-aaa8980ff98a0788b

Worktree `.claude/worktrees/agent-aaa8980ff98a0788b`, branch `worktree-agent-aaa8980ff98a0788b`, base `86f89935b`.

## Done

- `tools/xtask/src/repo_guard.rs` (new): `COMMANDS` marking, `access`, `check_cwd`, `check`; unit tests.
- `tools/xtask/src/main.rs`: `main` applies `repo_guard::check_cwd` before the dispatch (same line count in `main`,
  so the anchored cites into `main.rs` stay valid); `mod repo_guard;` at the end of the file.
- `tools/xtask/tests/repo_guard.rs` (new): the built binary end to end (the 2026-11-11 case).

## Rule chosen: refuse

A writing command refuses when the working directory's work tree top level is not the binary's root, and never acts
on the working directory's repository instead:

1. `cargo xtask` builds the xtask of the workspace the working directory is in, so a normal call always passes; the
   refusal hits only a binary started by path from another repository, which is the mistake of 2026-11-11.
2. The binary carries its own tree's rules: the ADR-037 checks, the baseline format, `tools/hooks/agent-scratch.sh`
   by build path, the hooks directory. Redirecting would apply one branch's rules to another's files; the TOOL51
   tester wanted the clone's own `accept` and would silently have tested main's.
3. A refusal is visible and changes nothing in either repository.

Tests keep passing an explicit root to the command functions (`tickets::accept`, `scratch::land`, ...); only the
dispatcher applies the check. Read-only: `doc-check`, `deps`, `help` (no mode writes); every other command, and any
unmarked name, is guarded (`status-check --prune-baseline`, `report --readme`, `emutos-consts --update`, `prgflags
<file> <value>`, the build commands write).

## Checks

- `cargo test -p xtask repo_guard`: 8 unit tests passed; `cargo test -p xtask --test repo_guard`: 3 passed.
- Mutation (`if cmd == "accept" { return Ok(()); }` at the top of `check_cwd`, restored from a copy): the end-to-end
  test fails with `xtask: accept: TASK/ZZTOOL52.md: No such file or directory`, nothing committed.
- `cargo clippy -p xtask --all-targets`: 10 warnings, identical to `86f89935b` (a `git archive` export built in
  `target-agents/agent-aaa8980ff98a0788b/base`); delta 0, none in the new files.
- Full `cargo xtask test`: 2427 passed / 0 failed / 7 ignored (main 2416 / 0 / 7, plus the 11 new tests).
- `cargo xtask doc-check TASK/TOOL52.md TASK/TOOL52-STATUS-UNFINISHED.MD`: 0 failures.

Scratch: `/Volumes/Daten/Development/PiRusTOS/target-agents/agent-aaa8980ff98a0788b/base` (export of `86f89935b` for
the clippy baseline, no outputs cited); the test repositories live under the worktree's `target/` and remove
themselves.

## For the tester

- Mutation for the acceptance: skip the helper for `accept` (as above, or mark `accept` `ReadOnly` in `COMMANDS`,
  which fails `the_writing_commands_are_guarded` and the end-to-end test).
- 2026-11-11 case: build this branch's xtask, `cd` into a scratch clone, run
  `<worktree>/target/debug/xtask accept TOOL51 --commit HEAD --result PASS --note x --agent dev-a`: refused, naming
  both paths; HEAD of both repositories unchanged.

## Proposed lines (lead)

- STATUS: `2026-11-11 TOOL52: writing xtask commands refuse when the working directory is not in the binary's own
  repository (repo_guard::check_cwd in the dispatcher; read-only: doc-check, deps, help). Commit <hash>.`
- CLAUDE.md (Commit Discipline, after the ticket-state paragraph): `A writing xtask command runs only from inside the
  repository its binary was built for; started by path from another one it refuses and names both (TASK/TOOL52.md).`

## Out-of-scope observations

- `tickets.rs` and `scratch.rs` run git without clearing `GIT_DIR`/`GIT_WORK_TREE`; a caller's `GIT_DIR` would
  redirect their git calls (the guard itself ignores them).
