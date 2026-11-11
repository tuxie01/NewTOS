# TOOL42 — no-verify-guard blocks hook-bypassing git plumbing in the main repo

Status: DONE (2026-11-11) | Priority: P1 | Commit: 9bc27d82b | Receipt: TOOL42-STATUS-DONE.MD

Type: tool (enforcement). Owner decision 2026-11-11. Small; lands in its own commit before [TOOL41](TOOL41.md) is
merged. Developed in its own git worktree like the git-hook tickets: `tools/hooks/no-verify-guard.sh` is a live
PreToolUse hook of every agent session (`.claude/settings.json`), so the main tree's copy changes only when this
ticket is merged after the tester's acceptance. It lands with `git merge --squash` and one normal `git commit -F` in
the main tree, never as a fast-forward (owner 2026-11-11, CLAUDE.md *Commit Discipline*).

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
4. **Optional (owner 2026-11-11): no fast-forward into `main`.** In the main repo, block a `git merge` that would
   fast-forward a local branch into `main` — `--ff-only`, and any `git merge <branch>` without `--squash` while
   `main` is checked out (the rule is squash only; a `--no-ff` merge commit is not the way either). `git pull` from `origin` stays allowed. If a text match cannot tell the
   cases apart reliably, say so in the receipt and leave it out rather than block too much.
5. **Docs:** the comment block of the guard; CLAUDE.md is not edited by the developer — the receipt proposes the
   one-line change to the hook paragraph ("Agents must not bypass it: …"), the lead applies it.

## Acceptance

- All pipe tests pass; the new cases fail against the current guard (shown once, before the change).
- A tester in a scratch clone can still use `git commit-tree`/`update-ref` (the TOOL40 setup works).
- `cargo xtask test` green, warnings delta 0, doc-check of the edited docs 0 failures.

## Implementation

Developer, 2026-11-11, worktree branch `worktree-agent-a15921e5b605761cb` on base `d033a78a2` (receipt:
[TOOL42-STATUS-UNFINISHED.MD](TOOL42-STATUS-UNFINISHED.MD)).

- **Parser:** the guard's TOOL42 section splits the command into simple commands, quote-aware, at `;` `&` `|`
  newline, `( )`, `$( )` and backticks (inner commands are checked as their own simple commands; a sub-shell or
  `$( )` restores the directory), skipping `#` comments and here-document bodies. A word the shell expands at run
  time is marked; `$HOME`, `$TMPDIR` and `$CLAUDE_PROJECT_DIR` are expanded from the hook's environment by
  `expand` (`tools/hooks/no-verify-guard.sh:174@expand`), which walks the word by prefix/suffix removal: bash 3.2
  ends a `${w//pat/rep}` pattern at the first `/` even inside quotes. A quoted command word (`"git"`) counts.
  `process_seg` (`tools/hooks/no-verify-guard.sh:300@process_seg`) tracks `cd`/`pushd`/`popd` and `GIT_DIR`
  exports, finds the git command word (also after `env`, `command`, `exec`, `nohup`, `nice`, `sudo`, `time`,
  `xargs`, and the dashed `git-<verb>` form), reads `-C`, `--git-dir` and a `GIT_DIR=` prefix.
- **Rules:** `classify` (`tools/hooks/no-verify-guard.sh:217@classify`): `commit-tree`, `fast-import`,
  `filter-branch`, `filter-repo`; `replace` except listing; `update-ref` on `HEAD`, `refs/heads/*`, a bare name
  (with or without `-d`), any `--stdin`; `symbolic-ref` that sets or deletes; `hash-object -w` with type `commit`
  (`-t commit`, `-tcommit`, clusters). `hash-object -w --stdin` without `-t` writes a blob, so point 1's "`--stdin`
  of a commit object" needs no text heuristic: the type flag decides. Point 4: `merge --ff-only`, `merge <branch>`
  without `--squash` (`--no-ff` included), `pull` from `.` or a local path, only when the target is the main repo
  and its `HEAD` is `refs/heads/main`; a merge of remote-tracking refs only (`refs/remotes/*`, `<remote>/<branch>`
  for a remote the target has; `remote_refs_only`, `tools/hooks/no-verify-guard.sh:408@remote_refs_only`) is
  allowed, as `git pull` is.
- **Target:** `common_dir` (`tools/hooks/no-verify-guard.sh:146@common_dir`) compares `git rev-parse
  --git-common-dir` of the target with that of `$CLAUDE_PROJECT_DIR`, both canonical (`cd -P`, `pwd -P`, then
  `canon`, `tools/hooks/no-verify-guard.sh:137@canon`: repeated slashes squeezed, trailing slash dropped); the base directory is the hook input's `cwd`,
  else the hook's working directory. A plumbing target the text cannot decide (other variable, `cd -`, a path that
  does not exist yet, not a repo) is blocked; for point 4 it is allowed (no over-blocking).
- **Tests:** `tools/hooks/no-verify-guard-test.sh`, 192 cases (17 old, unchanged); scratch clone by `git clone
  --shared --no-checkout` under `mktemp -d` in `$NVG_TEST_TMP` or `$TMPDIR`, removed by an EXIT trap. Wired into
  `cargo xtask test` as `git_hooks::tests::no_verify_guard_pipe_tests_pass`
  (`tools/xtask/src/git_hooks.rs:159@no_verify_guard_pipe_tests_pass`), scratch under `target/`, about 10 s.

## Owner decision (2026-11-11)

Scope stays as built: the guard is not extended for `fetch` + `reset`, `branch -f`, `rebase` or `cherry-pick`, nor for
quoting tricks inside the verb (`git commit-"tree"`, `commit-tr\ee`, `g=git; $g ...`). A pattern guard can never be
complete; the backstop is a server-side check on every push ([TOOL43](TOOL43.md)). The guard's comment block lists what
it does not see.

## Acceptance (tester, 2026-11-11)

First run PARTIAL (`measurements/measurement-2026-11-11-tool42-test/RUNINFO.TXT`): F1, `$CLAUDE_PROJECT_DIR` expanded
to `//path/` under bash 3.2 and the main repo was taken for a foreign one; F2, a quoted `git` was not seen; a false
positive on `git merge origin/main`. Fixed in the second commit of the branch. Re-run by another tester PASS
(`measurements/measurement-2026-11-11-tool42-retest/RUNINFO.TXT`): 192/0 pipe tests, 356 adversarial cases with no
false negative beyond the three documented forms, 98/98 plumbing forms allowed in a scratch clone, the `cwd` hook input
field confirmed in the Claude Code hooks documentation. Line 3 stays `IMPLEMENTED` until `cargo xtask accept`
([TOOL41](TOOL41.md)).
