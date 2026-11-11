# TOOL42 — receipt (unfinished: tester acceptance, CLAUDE.md line, squash merge pending)

Role: developer
Agent: agent-a15921e5b605761cb
(Role and Agent split into two header lines by lead, 2026-11-11; the developer wrote: "Role: developer. Agent:
developer subagent `agent-a15921e5b605761cb` (Claude Opus 5.5), 2026-11-11.")
Worktree `.claude/worktrees/agent-a15921e5b605761cb`, branch `worktree-agent-a15921e5b605761cb`, base `d033a78a2`
(rebased from `ced5e68f6` on the lead's note). The commit is on the worktree branch; the main tree is untouched,
so the live guard of every session is still the old one until the squash merge.

## Done

- `tools/hooks/no-verify-guard.sh`: new TOOL42 section (points 1, 2 and the optional point 4) and the comment block
  that names what it blocks, how the target repo is decided, and what a text match does not see. The old checks are
  unchanged; two small edits to them: `block` takes an optional second line (the old text stays the default), and
  the first "does the command name git" filter also lets `git-<verb>` through (for `git-filter-repo`). Details:
  section "Implementation" of [TOOL42.md](TOOL42.md).
- `tools/hooks/no-verify-guard-test.sh`: 146 new cases next to the 17 old ones (unchanged). Main-repo cases run with
  `CLAUDE_PROJECT_DIR=$PWD` (the worktree; its common dir is the main repo's `.git`), and with `-C <main tree>` and
  `--git-dir=<main tree>/.git` from a clone cwd. The point 4 cases use the scratch clone as the main repo with `HEAD`
  set to `main`, then to another branch, so they do not depend on what the main tree has checked out.
- `tools/xtask/src/git_hooks.rs`: host test `no_verify_guard_pipe_tests_pass`; it fails on a non-zero exit or any
  `FAIL` line, and prints the FAIL lines.

## Checks run

- New cases against the old guard (the `ced5e68f6` copy, put back with `cp` for the run, then the new one copied
  back and checked with `cmp`): 163 cases, 93 ok, 70 FAIL. All 70 FAILs are new block cases (`want 2`, got 0). The
  17 old cases and the new allow cases pass, as they should. Against the new guard: 163 ok, 0 FAIL. The Rust test
  fails against the old guard and passes against the new one.
- By hand, the worktree copy with `CLAUDE_PROJECT_DIR=/Volumes/Daten/Development/PiRusTOS`: `git commit-tree` in the
  main tree and in this worktree blocked; `git merge feature` in the main tree (main checked out) blocked, in this
  worktree (other branch) allowed; `git merge --squash` allowed; `git hash-object -w x && git update-index
  --cacheinfo …` allowed. A 100 kB command and a 3000-line here-document: under 1 s each.
- `cargo xtask test`: 2376 passed / 0 failed / 6 ignored (base 2375/0/6, +1 new test). Note: a fresh worktree has
  no `references/` and no `sdk/apps/teradesk` or `sdk/apps/hypview` (not tracked). Without them 8 xtask tests fail
  and cargo stops before the remaining test targets. For the run I linked them from the main tree (symlinks inside
  the ignored `references/`, and two untracked symlinks in `sdk/apps/`, removed after the run, not committed).
- Warnings: `cargo clippy -p xtask --tests` 9 before and after; `cargo build -p xtask` 0 before and after. Delta 0.
  No kernel, tos-api or tos-types code touched, so no board build is affected.
- `cargo xtask doc-check TASK/TOOL42.md TASK/TOOL42-STATUS-UNFINISHED.MD`: 0 failures.

## What I relied on and what is uncertain

- **Hook input `cwd`.** The guard reads `.cwd` from the PreToolUse JSON. Claude Code's hook documentation lists
  `cwd` among the input fields every hook event gets (with `session_id`, `transcript_path`, `hook_event_name`).
  Nothing in this repo read it before, and I did not see it in a live hook input. If the field is missing, the
  guard falls back to its own working directory, which is probably the project dir. Then a bare `git commit-tree`
  run in a shell already `cd`'d into a scratch clone would be blocked: fail-closed, and `git -C <clone>` or `cd
  <clone> && …` works. The tester should check this once in a live session (cwd in a clone, no `-C`).
- **Not blocked by design (named in the comment block):** `eval`, `bash -c '…'`, scripts, git aliases, `xargs` with
  the verb in its input. Also `reset`, `branch -f`, `checkout -B`, `cherry-pick`, `rebase`, `revert`, `am`, and
  fetch/push into the main repo. These move a branch to an existing commit, or replay commits without the
  pre-commit hook. So a commit made with plumbing in a scratch clone can still be fetched into the main repo. If
  the owner wants those closed too, that is a follow-up decision; blocking `cherry-pick`/`rebase` would hit normal
  work.
- **Point 4 limits:** `git merge` with no branch (merges the upstream) is allowed, like `git pull` from a remote.
  `git pull` is blocked only for `.`, `/…`, `./…`, `../…`, `~…`, `file://…`. A remote *name* for a local path
  (`git remote add wt ../wt`, then `git pull wt x`) is not caught. Neither is `git fetch` + `git reset --hard`.
  An undecidable target is allowed for point 4.
- **Side effect for the lead:** `git update-ref -d refs/heads/<branch>` in the main repo is now blocked. Use `git
  branch -D <branch>` to delete a merged worktree branch.

## Proposed CLAUDE.md line (hook paragraph, Commit Discipline)

Replace "Agents must not bypass it: `tools/hooks/no-verify-guard.sh` blocks the bypass flags and any
`core.hooksPath` other than `tools/git-hooks`." with:

> Agents must not bypass it: `tools/hooks/no-verify-guard.sh` blocks the bypass flags, any `core.hooksPath` other
> than `tools/git-hooks`, and in the main repo the plumbing that makes or moves a branch commit without `git commit`
> (`commit-tree`, `update-ref` on branches, `hash-object -t commit`, `fast-import`, `replace`, `symbolic-ref`,
> `filter-branch`/`filter-repo`) and any non-squash `git merge` into `main` (TASK/TOOL42.md); in a scratch clone
> plumbing stays allowed.

## Fixes after tester PARTIAL

Verdict PARTIAL on `fa053cef5` (tester run measurement-2026-11-11-tool42-test, RUNINFO.TXT; in the main tree, not on this branch). The fixes are in a
new commit on the same branch (no amend), by the same developer agent.

- **F1, real bypass.** `$CLAUDE_PROJECT_DIR` in a path word was let through. There were two causes:
  - `expand` used `${w//"$DYN\$NAME/"/$v/}`. Bash 3.2.57 (`/bin/bash`, which `#!/usr/bin/env bash` finds here) ends
    the pattern at the first `/`, even inside quotes. So the word came out as `//<path>/`. The `${NAME}` form was
    not affected.
  - `common_dir` compared plain strings. `pwd -P` keeps a leading `//`. In a repo that is not a worktree, `git
    rev-parse --git-common-dir` prints the relative `.git`, so the result stayed `//…/.git`. That is why the bug
    showed with the main tree as the project, and not in the first round of pipe tests, which used the worktree as
    the project: a worktree prints an absolute common dir.

  Fix:
  - `expand` now walks the word with prefix/suffix removal only (`tools/hooks/no-verify-guard.sh:174@expand`).
  - `common_dir` compares canonical paths on both sides (`tools/hooks/no-verify-guard.sh:146@common_dir`): `cd -P`,
    `pwd -P`, then `canon` (`tools/hooks/no-verify-guard.sh:137@canon`), which squeezes repeated slashes and drops a
    trailing slash.

  The new cases run twice, once with the worktree and once with the scratch clone as the project. The forms are:
  `-C $CLAUDE_PROJECT_DIR`, `-C "$CLAUDE_PROJECT_DIR"`, `${…}`, `…/tools` (`…/.git` for the clone), `cd
  $CLAUDE_PROJECT_DIR &&`, a project dir and a target written with `//` and a trailing slash, and
  `$CLAUDE_PROJECT_DIRX` (another variable, so undecidable). There is also `-C $CLAUDE_PROJECT_DIR merge`.
- **F2.** `"git" commit-tree` and `'git' commit-tree` were let through. The tokenizer already removed the quotes,
  but the first git filter of the script required whitespace after `git`. It now also accepts a quote character
  there. Three new cases.
- **F3, documented, not chased** (owner 2026-11-11). The comment block now lists, under "not seen", `git
  commit-"tree"`, `git commit-tr\ee` and `g=git; $g commit-tree`. It says the pattern guard can never be complete
  and its scope stays. It also names fetch + reset, `branch -f`, `rebase` and `cherry-pick` as not blocked by owner
  decision, with the server-side backstop of ticket TOOL43 as the answer.
- **False positive `git merge origin/main`.** A merge whose names are all remote-tracking refs is now allowed while
  main is checked out. That means `refs/remotes/*`, or `<remote>/<branch>` for a remote that `git remote` of the
  target lists (`tools/hooks/no-verify-guard.sh:408@remote_refs_only`). This applies with or without `--ff-only`,
  like `git pull`. Five new cases: allowed for `origin/main`, `--ff-only origin/main` and `refs/remotes/origin/main`;
  blocked for `origin/main feature` and `nosuchremote/x`. Limit: a local branch literally named `origin/x` counts
  as remote-tracking. `--squash --ff-only` stays blocked, as the lead accepted.

Checks:
- Pipe tests: 192 cases. Against the new guard 192 ok / 0 FAIL. Against the `fa053cef5` guard, swapped in with
  `cp` and restored and checked with `cmp`: 179 ok / 13 FAIL. The 13 are 7 F1 (clone as project), 2 F2, 3
  remote-tracking merges, and 1 F1 merge.
- The tester's adv.sh (method dir of that run) against the new guard: 86 ok, 5 MISMATCH. The 5 are the three F3 forms (out of
  scope by owner decision) and the two cases RUNINFO calls "not defects": `git mktree` alone and `update-ref
  refs/remotes/*`.
- `cargo xtask test` (with the symlinks as before; the two `sdk/apps` links removed afterwards): 2376 passed / 0
  failed / 6 ignored, unchanged.
- `cargo clippy -p xtask --tests`: 9 warnings, unchanged. No Rust touched in this round.
- `cargo xtask doc-check TASK/TOOL42.md TASK/TOOL42-STATUS-UNFINISHED.MD`: 0 failures.

## Missing

- Tester acceptance (the pipe tests, and the TOOL40 setup in a scratch clone with the merged guard).
- The CLAUDE.md line above (lead).
- The squash merge into main (lead).
