# TOOL52 — writing xtask commands act on the repository of the working directory, or refuse

Status: OPEN (2026-11-11) | Priority: P1

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
