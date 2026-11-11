# TOOL44 — `cargo xtask worktree-setup`: inputs a fresh worktree needs

Status: OPEN (2026-11-11) | Priority: P2

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
