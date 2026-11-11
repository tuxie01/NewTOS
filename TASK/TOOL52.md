# TOOL52 — writing xtask commands act on the repository of the working directory, or refuse

Status: IMPLEMENTED (2026-11-11) | Priority: P1 | Commit: self | Missing: tester acceptance through cargo xtask accept

Type: tool (enforcement). Owner decision 2026-11-11. Queue: right after [TOOL50](TOOL50.md).

## Facts

- xtask takes its repository root from its build (the workspace it was compiled in), not from the working directory.
  On 2026-11-11 a tester ran the main tree's xtask binary from inside a scratch clone to test a refusal; `cargo xtask
  accept TOOL51 ... --note x --agent dev-a` ran against the main repository and committed `03499102b` there with a
  wrong acceptor and a placeholder note ([TOOL51](TOOL51.md), receipt *Correction*).
- The TOOL44 and TOOL46 testers met the same property (a branch binary acts on the worktree it was built in).

## Work

1. Every writing or committing xtask command — `accept`, `land`, `set-status`, `scratch-clean`, `worktree-setup`,
   `install-hooks`, and any future one — determines the repository it acts on from the current working directory
   (`git rev-parse --show-toplevel` there), or refuses when the working directory is not inside the repository the
   binary was built for. Choose one rule for all of them and justify it in the receipt; the error names both paths
   (the binary's root and the working directory's repository).
2. One shared helper used by every such command; a new writing command cannot skip it (e.g. the dispatcher calls it
   for every command not marked read-only, with a test that lists the commands).
3. Host tests in a scratch repo under the worker's scratch directory: invoked from a clone → refuses (or acts on the
   clone, if that is the chosen rule); invoked in the main tree → works; no commit lands in the wrong repository
   (HEAD of both repositories unchanged after a refusal).

## Acceptance

- Tests of point 3 green; a mutation that skips the helper for `accept` fails a test (tester shows it).
- The 2026-11-11 case reproduced in a scratch setup: the main-tree binary run from a clone no longer commits in the
  main repository.
- `cargo xtask test` green, warnings delta 0, doc-check of the edited docs 0 failures.

## Implementation (developer, 2026-11-11)

Receipt: [TOOL52-STATUS-UNFINISHED.MD](TOOL52-STATUS-UNFINISHED.MD).

- **Rule: refuse, never redirect.** A writing command runs only when `git rev-parse --show-toplevel` of the working
  directory is the binary's root; otherwise it refuses, naming both paths, before any work
  (`tools/xtask/src/repo_guard.rs:91@check`, the reasons in the module comment). `cargo xtask` builds the xtask of the
  workspace the working directory is in, so the normal call always passes; only a binary started by path from
  another repository is refused. A redirect would apply one tree's rules (ADR-037 checks, baseline, the build-path
  `agent-scratch.sh`, hooks directory) to another tree's files. A repository nested in the binary's tree
  (`target-agents/…/clone`) counts as another one; outside any work tree is refused too.
- **One helper, applied by the dispatcher.** `main` calls `repo_guard::check_cwd` before its `match`
  (`tools/xtask/src/main.rs:152@check_cwd`), so `tickets::cmd` (`status-check`, `accept`, `set-status`, `report`) is
  covered by the same call. `COMMANDS` (`tools/xtask/src/repo_guard.rs:50@COMMANDS`) marks every command; only
  `doc-check`, `deps` and `help` are `ReadOnly` (no mode of them writes); an unmarked name counts as writing
  (`tools/xtask/src/repo_guard.rs:76@access`).
- **Tests.** Unit (`tools/xtask/src/repo_guard.rs`): `every_dispatched_command_is_marked`
  (`tools/xtask/src/repo_guard.rs:228@every_dispatched_command_is_marked`) parses the arms of `main()` and
  `tickets::cmd` and fails on an unmarked one; `main_applies_the_guard_before_the_dispatch`; refusal from another
  scratch repository naming both paths with both HEADs unchanged
  (`tools/xtask/src/repo_guard.rs:263@refuses_from_another_repository_and_names_both_paths`); pass inside its own
  repository and a subdirectory; nested repository; outside any work tree. End to end
  (`tools/xtask/tests/repo_guard.rs:71@the_main_tree_binary_run_from_a_clone_commits_nowhere`): the built binary run
  from a scratch repository with the 2026-11-11 `accept` command (and `set-status`, `land`) is refused by the guard,
  HEAD of the binary's repository and of the scratch repository unchanged; from its own tree it passes the guard.
  Mutation `if cmd == "accept" { return Ok(()); }` in `check_cwd` fails that test (the ids are fictitious, so the
  mutated binary fails on the missing ticket and commits nothing).
