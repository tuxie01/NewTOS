# TOOL42 — no-verify-guard blocks hook-bypassing git plumbing in the main repo

Status: OPEN (2026-11-11) | Priority: P1

Type: tool (enforcement). Owner decision 2026-11-11. Small; lands in its own commit before [TOOL41](TOOL41.md) is
merged. Developed in its own git worktree like the git-hook tickets: `tools/hooks/no-verify-guard.sh` is a live
PreToolUse hook of every agent session (`.claude/settings.json`), so the main tree's copy changes only when this
ticket is merged after the tester's acceptance.

## Facts

- `tools/hooks/no-verify-guard.sh` (TOOL29) blocks `--no-verify`, `git commit -n…` clusters, an inline
  `-c core.hooksPath=…` and any `git config core.hooksPath` other than `tools/git-hooks`. Its pipe tests are
  `tools/hooks/no-verify-guard-test.sh` (run from the repo root; not part of `cargo xtask test`).
- It does not see commits made without `git commit`: during the TOOL40 acceptance (2026-11-11) the tester built a
  commit with `git commit-tree` and moved the branch with `git update-ref`, in a throwaway clone — legitimate there,
  but the same plumbing in the main repo would put a commit on `main` that no hook ever checked.
- `git hash-object -w` writing a blob, followed by `git update-index --cacheinfo`, is used by the lead to stage part of
  a file (2026-11-10); it creates no commit and passes through the hook at `git commit`. It stays allowed.

## Work

1. **Block in the main repo**, with the existing exit-2 message style: `git commit-tree`; `git update-ref` on
   `refs/heads/*` (also `HEAD`, a bare branch name, `--stdin` input, and `-d`); `git hash-object -w` with `-t commit`
   (or `--stdin` of a commit object, as far as a text match can tell); and their equivalents that create or move a
   branch commit without `git commit`/`git merge` — at least `git mktree`+`commit-tree` chains, `git fast-import`,
   `git replace`, `git symbolic-ref` on `HEAD`, and `git filter-branch`/`git filter-repo`. Name in the comment block
   what a text match cannot see (a command assembled at run time), as the script already does.
2. **"Main repo"** = the git directory of `CLAUDE_PROJECT_DIR` (the main repository; read `git rev-parse
   --git-common-dir`, so its worktrees under the repo path count too). The guard decides the target repo from the
   command as far as a text match allows: `git -C <path>`, `--git-dir=`, `GIT_DIR=`, a leading `cd <path> &&`, else
   the tool call's working directory. A target whose git common dir differs from the main repo's (a throwaway clone
   or worktree of another repo, under the scratchpad or job tmp) is allowed.
3. **Tests** in `tools/hooks/no-verify-guard-test.sh`: each form of point 1 blocked in the main repo (with and
   without `-C`, `cd … &&`); the same allowed in a scratch clone (created by the test under the scratch rules and
   removed after); blob `hash-object -w` and `update-index --cacheinfo` allowed in the main repo; every existing case
   unchanged. Wire the script into `cargo xtask test` if cheap (one test that runs it and fails on a `FAIL` line).
4. **Docs:** the comment block of the guard; CLAUDE.md is not edited by the developer — the receipt proposes the
   one-line change to the hook paragraph ("Agents must not bypass it: …"), the lead applies it.

## Acceptance

- All pipe tests pass; the new cases fail against the current guard (shown once, before the change).
- A tester in a scratch clone can still use `git commit-tree`/`update-ref` (the TOOL40 setup works).
- `cargo xtask test` green, warnings delta 0, doc-check of the edited docs 0 failures.
