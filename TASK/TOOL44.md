# TOOL44 — `cargo xtask worktree-setup`: inputs a fresh worktree needs

Status: IMPLEMENTED (2026-11-11) | Priority: P2 | Commit: self | Missing: tester acceptance through cargo xtask accept

Type: tool. Owner decision 2026-11-11. Small.

## Facts

- A fresh git worktree lacks the gitignored inputs of the main tree: `references/*` (`.gitignore:26`, only
  `references/README.md` is tracked) and the untracked checkouts `sdk/apps/<app>/` (`sdk/.gitignore:77`, e.g.
  `sdk/apps/teradesk`, `sdk/apps/hypview`). Without them 8 xtask tests fail and cargo stops the remaining test targets
  ([TOOL42](TOOL42.md) receipt, 2026-11-11). The TOOL42 developer linked them by hand.

## Work

1. `cargo xtask worktree-setup` run inside a worktree of the main repository: for every gitignored input the main tree
   has and the worktree lacks (`references/*` entries, `sdk/apps/<app>/` checkouts; derive the list from the ignore
   rules and the main tree, do not hard-code app names), create a symlink to the main tree's copy. Idempotent: an
   existing correct link is left alone; a wrong link or a real directory is reported, never replaced or deleted.
   Refuses outside a worktree of this repository and in the main tree itself.
2. The links stay ignored by git (`git status` clean after the command); state how in the code comment.
3. `--check` mode: lists what is missing, exit != 0 if anything is.
4. Host tests in a temp repository with a worktree (links created, second run unchanged, wrong link reported, refusal
   in the main tree).
5. CLAUDE.md worktree paragraph: one line that a worktree runs `cargo xtask worktree-setup` first (proposed in the
   receipt; the lead applies it).

## Acceptance

- In a fresh worktree: after the command `cargo xtask test` runs the same set as the main tree (same totals).
- `git status` clean; second run prints nothing changed.
- `cargo xtask test` green, warnings delta 0, doc-check of edited docs 0 failures.

## Implementation (2026-11-11, developer)

Worktree branch `worktree-agent-ac57b09f973d6b6ff`, based on `628d5dac8`. Receipt:
[TOOL44-STATUS-UNFINISHED.MD](TOOL44-STATUS-UNFINISHED.MD).

- **Command** (Work 1, 3): `tools/xtask/src/worktree_setup.rs:305@cmd`, dispatched from
  `tools/xtask/src/main.rs:162@worktree_setup`, usage text
  `tools/xtask/src/worktree_setup.rs:44@USAGE`; `--check` only looks and fails on anything missing.
- **Refusal** (Work 1): `tools/xtask/src/worktree_setup.rs:143@context` needs the root of a linked worktree
  (`--git-dir` differs from `--git-common-dir`); the main tree, a subdirectory, a directory that is no git
  checkout and a bare main repository are refused. The main tree is the first record of `git worktree list
  --porcelain`.
- **Inputs, derived** (Work 1): `tools/xtask/src/worktree_setup.rs:184@inputs`. Every child directory of
  `references/` and `sdk/apps/` (`tools/xtask/src/worktree_setup.rs:56@INPUT_DIRS`) in the main tree that the main
  tree's rules ignore as a directory (`git check-ignore <path>/`) and that holds no tracked file. No app name is
  hard-coded; `sdk/apps/doom/` (tracked content) and the negated `sdk/apps/apitest/`, `sdk/apps/chaintest/`, ...
  are not inputs.
- **Never destructive** (Work 1): `tools/xtask/src/worktree_setup.rs:214@state` classifies the worktree entry; a
  correct link is kept, a real directory is reported and kept, a wrong or dangling link or a file is reported, kept,
  and fails the run (`tools/xtask/src/worktree_setup.rs:264@run`).
- **`git status` clean** (Work 2): the module comment, `tools/xtask/src/worktree_setup.rs:23@status`. git sees a
  symlink as a file: `/references/*` covers the links, `sdk/.gitignore`'s dir-only `/apps/*/` does not. Every link
  the rules miss gets an anchored entry in `info/exclude` of the common git dir, which all worktrees read, under
  `tools/xtask/src/worktree_setup.rs:59@EXCLUDE_HEADER` (`tools/xtask/src/worktree_setup.rs:239@exclude`).
- **Host tests** (Work 4), in a scratch repository under `target/` with a worktree at `.claude/worktrees/w`:
  `tools/xtask/src/worktree_setup.rs:372@links_inputs_keeps_status_clean_and_is_idempotent`,
  `tools/xtask/src/worktree_setup.rs:399@wrong_link_and_real_dir_are_reported_not_replaced`,
  `tools/xtask/src/worktree_setup.rs:419@refuses_in_the_main_tree_and_outside_a_worktree_root`,
  `tools/xtask/src/worktree_setup.rs:430@refuses_outside_a_git_checkout`.
- **CLAUDE.md line** (Work 5): proposed in the receipt; the lead applies it.
