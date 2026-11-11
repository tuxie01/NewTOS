# TOOL44 — receipt (unfinished: tester acceptance, CLAUDE.md line, squash merge pending)

Role: developer
Agent: agent-ac57b09f973d6b6ff
Worktree `.claude/worktrees/agent-ac57b09f973d6b6ff`, branch `worktree-agent-ac57b09f973d6b6ff`, base `628d5dac8`.
The commit is on the worktree branch; the main tree is untouched except its common git dir's `info/exclude` (see
below).

## Done

- `tools/xtask/src/worktree_setup.rs`: new module, `cargo xtask worktree-setup [--check]`; details in section
  "Implementation" of [TOOL44.md](TOOL44.md).
- `tools/xtask/src/main.rs`: dispatch, `xtask help` appends the module's usage, `mod worktree_setup` (after the
  usage text the lines keep their numbers: anchored cites at the edge of the +-5 window, e.g. in
  `docs/sdk/06-image-and-testing.md`, still hold).

## Checks run

- Host tests of the module: 4 passed (links created, second run unchanged, `git status` clean in worktree and main
  tree, wrong and dangling link reported and kept, real directory kept, refusal in the main tree, in a
  subdirectory and outside a git checkout).
- This worktree as the first real case. Before: `cargo xtask worktree-setup --check` listed 21 missing inputs
  (16 `references/*`, `sdk/apps/{coremark,highwire,hypview,qed,teradesk}`), exit 1. `cargo xtask worktree-setup`
  linked all 21 and added the 5 `sdk/apps/*` entries under the TOOL44 header to the main repository's
  `.git/info/exclude` (the 16 `references/*` links are covered by `/references/*`). Second run and `--check`:
  `worktree-setup: nothing changed (21 inputs linked)`, exit 0. `git status --porcelain` shows only this ticket's
  own files.
- `cargo xtask test` (full) in this worktree after the setup: 2400 passed, 0 failed, 6 ignored, exit 0. The main
  tree has 2396 / 0 / 6; the difference is the 4 new tests, so the worktree runs the main tree's set. (Run on the code
  before the last edit, which only folded `);` into the preceding line of `print_usage`; after it `cargo test -p xtask`: 276 passed, 0 failed.)
- `cargo clippy -p xtask --all-targets`: 12 warning lines before and after (9 in the test
  build, 1 in the bin), none in `worktree_setup.rs`: delta 0.
- `cargo xtask doc-check TASK/TOOL44.md TASK/TOOL44-STATUS-UNFINISHED.MD`: 0 failures.
- `cargo xtask status-check --new-only`: 1 checked, 0 rejected.

## Not done / for the lead

- The main tree's `.git/info/exclude` now carries the 5 entries (local, untracked; they name directories the
  tracked rules already ignore there, so the main tree's status is unchanged). In every other worktree a link at
  one of these paths is ignored by them, too.
- `sdk/apps/doom/` has ignored content in the main tree beside tracked files (its `build` directory, absent
  here); it is not an input (a link would replace tracked files) and no test needs it.
- Proposed CLAUDE.md line for the worktree paragraph (*Commit Discipline*, after "A ticket that changes
  `tools/git-hooks/**` is developed in its own git worktree ..."):

  > A fresh git worktree first runs `cargo xtask worktree-setup`: it links the main tree's gitignored inputs
  > (`references/*`, `sdk/apps/<app>/`) into it, so `cargo xtask test` runs the main tree's set (`--check` lists
  > what is missing; TOOL44).
