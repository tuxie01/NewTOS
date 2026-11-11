# TOOL53 — worktree agents' scratch inside the worktree; `land` removes a merged worktree

Status: OPEN (2026-11-11) | Priority: P2

Type: tool. Owner decision 2026-11-11. Changes `tools/hooks/**` (the SubagentStart hook): developed in its own git
worktree and landed with `cargo xtask land` after the tester's acceptance. Respects the [TOOL52](TOOL52.md) repository
check.

## Facts

- [TOOL46](TOOL46.md) names `target-agents/agent-<id>/` in the main repo as every worker's scratch directory. The
  TOOL50 developer, running in its own worktree, could not write there: the harness keeps a worktree agent's writes
  inside its worktree. Its scratch went to the worktree's own `target/` instead (TOOL50 receipt), outside
  `target-agents`, so neither `accept` nor `land` removes it; it goes only when the worktree is removed.
- The lead removes each merged worktree and its branch by hand today (`git worktree unlock` + `git worktree remove` +
  `git branch -D`; the harness leaves a lock on the worktree after the agent ends).

## Work

1. **(a)** For a worktree agent, the SubagentStart hook names `<worktree>/target/agent-scratch/` instead of
   `target-agents/agent-<id>/`; other agents keep `target-agents/agent-<id>/`. State in the receipt how the hook tells
   the two apart (hook input fields such as `cwd`, documented with source and date).
2. **(b)** `cargo xtask land <branch> -F <msg>` removes the merged worktree and its branch itself — only **after** the
   squash commit has passed main's hooks, and only if the worktree is clean. Otherwise it refuses the removal and lists
   the uncommitted and untracked files; it never discards work. A lock the harness left is released only for a
   worktree that is clean and whose branch is fully contained in the squash commit. `git worktree remove` without
   `--force`.
3. **Tests** (scratch repo): dirty worktree → kept and its files listed; failed commit (hook rejects) → nothing
   removed; clean success → worktree, branch and its `<worktree>/target/agent-scratch/` gone; a worktree with ignored-only files
   (e.g. `target/`) counts as clean. The TOOL52 check applies to `land` as to every writing command.
4. **Git environment** (TOOL52 tester and developer, 2026-11-11): `tools/xtask/src/tickets.rs` and
   `tools/xtask/src/scratch.rs` run git without clearing `GIT_DIR`, `GIT_WORK_TREE` and related variables, so a
   caller's `GIT_DIR` redirects their git calls (the TOOL52 check itself clears them). Clear them in one helper for
   every git call of xtask; a test with `GIT_DIR` set to another repository.
5. Agent briefs and the TOOL46 lines in CLAUDE.md: proposed in the receipt, the lead applies them.

## Acceptance

- Tests of point 3 green; a mutation that removes a dirty worktree fails a test (tester shows it).
- One real `land` of a worktree branch removes its worktree and branch.
- `cargo xtask test` green, warnings delta 0, doc-check of the edited docs 0 failures.
