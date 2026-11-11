# TOOL50 — receipt (unfinished: tester acceptance, squash merge by `cargo xtask land` pending)

Role: developer
Agent: agent-aeebbf8f8af122b9d

Worktree `.claude/worktrees/agent-aeebbf8f8af122b9d`, branch `worktree-agent-aeebbf8f8af122b9d`, rebased onto
`c4fd814ee` (first base `f0eb31889`). Commits on the branch: `f4f4e18b3` (first version, earlier `8a170770c` and
`e0c1fc7fc`; tester PARTIAL leaning FAIL, run directory measurement-2026-11-11-tool50-test, untracked in the main
tree), `1fad7a17e` (the rework after the owner decision of 2026-11-11, earlier `53e53f7f1`; retest PARTIAL leaning
PASS, measurement-2026-11-11-tool50-retest) and the retest fixes (TOOL50.md *Implementation*, last item). The main
tree is untouched, so every session's hooks stay the old ones until the squash merge (TOOL50 changes
`tools/hooks/**` and `.claude/settings.json`).

## Done

- `tools/hooks/lib/shell-parse.sh` (new): the TOOL42 parser, shared by `no-verify-guard.sh` and `pin-guard.sh`.
- `tools/hooks/lib/paths.sh` (new, rework): physical, ASCII case-folded path comparison, used by `pin-guard.sh`,
  `no-verify-guard.sh` (main-repo comparison), `scratch-guard.sh` (`/tmp` matches) and `build-emutos.sh`.
- `tools/hooks/pin-guard.sh` (new; reduced by the rework), registered in `.claude/settings.json` (PreToolUse,
  `Bash|Write|Edit|NotebookEdit`, 10 s): blocks `chmod`/`chflags`/`chown` (also via `find -exec`) and `git remote`
  add/set-url/remove/rm/rename and `git config` writes of remote.*, url.* and branch.*.remote/pushRemote on the
  protected trees (also by `-f<path>` or a `GIT_CONFIG*` file in one), and Write/Edit/NotebookEdit
  into them (kept: one exact path, cheap). The comment block lists the known limits; backstops are the owner's OS
  lock and TOOL43.
- `tools/hooks/no-verify-guard.sh`: `git merge --squash <branch>` blocked while main is checked out in the main
  repo, message `cargo xtask land <branch> -F <msg>` (unchanged by the rework); repo comparison case folded;
  known limits (apply, read-tree, `git fetch . x:main`, backstops) in the comment block.
- `tools/oracle/build-emutos.sh` refuses a tree dir under `references/` (physical, case folded, via `paths.sh`);
  `tools/deps/gen-deps.py` and `tools/emutos-config.sh` run `git status` with `GIT_OPTIONAL_LOCKS=0`;
  docs/analysis/ORACLE.md §3 names a copy.
- Tests: `tools/hooks/pin-guard-test.sh` (rewritten for the reduced scope, wired as
  `git_hooks::tests::pin_guard_pipe_tests_pass`), `tools/hooks/no-verify-guard-test.sh` (+8 squash cases, one old
  case reversed, +4 letter-case cases), `tools/hooks/scratch-guard-test.sh` (+7 letter-case cases).
- TOOL42.md and its receipt: the six `path:line@symbol` cites moved with the code (cite text only, first commit).
- Details and cites: section *Implementation* of [TOOL50.md](TOOL50.md).

## Evidence (developer, not the acceptance)

- Retest fixes: `pin-guard-test.sh` 372 ok, 0 FAIL (362 guard cases, 10 build cases); against the `pin-guard.sh`
  of `1fad7a17e` all 48 new block cases FAIL and the 16 new allow/known-limit cases pass.
- Rework: `pin-guard-test.sh`: 308 ok, 0 FAIL (298 guard cases, 10 `build-emutos.sh` cases). Against the guards
  (no-verify-guard.sh, scratch-guard.sh) and `build-emutos.sh` of main `cb78c9d48`: all 228 block cases FAIL. Against
  the first version of `pin-guard.sh`: 42 block cases FAIL (38 letter-case, 4 ancestor/`find` from an ancestor).
- `no-verify-guard-test.sh`: 204 ok, 0 FAIL. The letter-case block cases `merge --squash` and `merge --ff-only`
  FAIL against the first version (the `commit-tree` one was blocked before too).
- `scratch-guard-test.sh`: 47 ok, 0 FAIL; the 5 new letter-case block cases FAIL against the guard of `cb78c9d48`.
- First version only (unchanged code paths): `gen-deps.py --check` on the real oracle tree byte-identical, rc 0,
  with read-only snapshots of the real trees before and after (`GIT_OPTIONAL_LOCKS=0`: refs, mtime of
  `.git/index`/`.git/HEAD`/`.git`, sha256 of `status --short`, stash count) identical. The rework ran nothing in or
  on `references/`.
- Full `cargo xtask test` after the retest fixes: 2416 passed, 0 failed, 7 ignored; clippy of xtask: the same
  8 warning lines as a worktree of main (delta 0; no Rust changed since the first version).

## Missing

(The OS/git lock was applied by the owner on 2026-11-11, measurement-2026-11-11-tool50-pinlock.)

- Tester acceptance through `cargo xtask accept`.
- Landing: `cargo xtask land worktree-agent-aeebbf8f8af122b9d -F <msg>` in the main tree after acceptance.

## Proposals for the lead

- STATUS line: "TOOL50 (2026-11-11): the EmuTOS checkouts are locked at OS/git level (owner); the PreToolUse guard
  `tools/hooks/pin-guard.sh` keeps agents from undoing the lock (chmod/chflags/chown, git remote and remote.*
  config, Write/Edit there); `build-emutos.sh` refuses a tree dir under `references/`; `git merge --squash` into
  main blocked for agents, `cargo xtask land` is the only merge path; shared `tools/hooks/lib/` (parser, case-folded
  physical path comparison)."
- CLAUDE.md, *Commit Discipline*: "it lands with `git merge --squash <branch>` and a normal `git commit -F`" now
  differs from the guard for agents; proposed: "it lands with `cargo xtask land <branch> -F <msg>` (a squash merge
  and a normal commit in the main tree, so main's stable hooks check it; agents cannot run `git merge --squash`
  into main, TOOL50)". *The EmuTOS checkouts stay on the pin*: could add "locked read-only at OS level (owner);
  agents may not unlock them (`tools/hooks/pin-guard.sh`, TOOL50)".

## Scratch

The worktree's own `target/t50` (gitignored; the worktree isolation refused writes under the TOOL46 agent dir,
which stays empty): pipe-test outputs, copies of the earlier guards and build script, the snapshots of the first
round. The detached worktree of the clippy baseline and its target dir are removed after use. 1.4M (`du -sh`, 2026-11-11).
No gitignored output is cited as evidence.
