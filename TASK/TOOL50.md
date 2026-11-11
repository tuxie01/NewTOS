# TOOL50 — the agent guard keeps the EmuTOS checkouts on the pin; `land` is the only merge path

Status: DONE (2026-11-11) | Priority: P1 | Commit: 64abacbbe | Receipt: TOOL50-STATUS-DONE.MD

Type: tool (enforcement). Owner decision 2026-11-11. Changes `tools/hooks/**`, the live hooks of every agent session:
developed in its own git worktree and landed by squash merge after the tester's acceptance (CLAUDE.md *Commit
Discipline*). The rule itself is in CLAUDE.md (*The EmuTOS checkouts stay on the pin*) and in every agent brief.

Rationale (owner 2026-11-11, corrected): upstream EmuTOS has moved past the pin `1a7484ab`. Nobody may fetch, pull,
re-clone from upstream or otherwise move `references/emutos*` off the pin; reads stay allowed. A pin change happens only
by owner decision, per `docs/analysis/ORACLE.md` §7 (rebuild, calibration, control runs). The checkouts are unmodified
locally (see *Facts*).

## Facts (lead, 2026-11-11, read only; `GIT_OPTIONAL_LOCKS=0`)

- `references/emutos`: branch `master` at `1a7484ab` (= `origin/master`, 0 local commits), 0 stashes, 0 modified
  tracked files; untracked `.DS_Store`; 225 ignored files (build outputs such as `aes/gem_rsc.c`, `bios/ctables.h`,
  and others). `references/emutos-oracle/1a7484ab` (own clone, the oracle build tree): at `1a7484ab`, no modified
  tracked file, 236 ignored files, `etos512de.img` dated 2026-10-26; `references/emutos-oracle/v1.4`: one untracked
  `.DS_Store`. The checkouts are unmodified and on the pin; the risk is moving them off it.
- Only the `.git` directory entries changed recently (`references/emutos/.git` 2026-11-11 08:24,
  `references/emutos-oracle/1a7484ab/.git` 2026-11-11 00:10) — no ref, no index, no `HEAD` changed (index dated
  2026-07-10 and 2026-10-26): a git command that took and dropped a lock, e.g. `git status` refreshing the index. Origin
  unknown.
- Who touches them today:
  - `tools/oracle/build-emutos.sh`: clones *from* `references/emutos` into the tree dir it is given, then `checkout`,
    `make clean`, `make 512 COUNTRY=de` **in that tree dir**; `references/emutos` is only read. The 2026-10-26 run used
    `references/emutos-oracle/1a7484ab` as tree dir (`make clean` ran there). The 2026-11-10 ROM rebuild (TOOL01) used a
    fresh clone in the session scratchpad (`measurements/measurement-2026-11-10-tool01/RUNINFO.TXT`), not
    `references/emutos`.
  - `tools/deps/gen-deps.py`: in `references/emutos-oracle/<pin>` `git rev-parse` and `git status --porcelain` (status
    can rewrite the index), compiler runs with that tree as cwd writing to a temporary directory (`-E` to stdout, gimple
    dumps into `-dumpdir` under the temp dir), `make -n` only in a throw-away clone.
  - `tools/emutos-config.sh`: `git -C references/emutos rev-parse HEAD` and `git status --porcelain` (read; status may
    refresh the index).
  - xtask (`doc_check`, `emutos_consts`, `rsc_*` tests): read files, `git ls-files`, `ls-tree`, `show`, `cat-file`.
  - `cargo xtask worktree-setup` (TOOL44): symlinks `references/*` into linked worktrees, so a command run in a worktree
    on `references/emutos` reaches the owner's checkout.
- The guards today (`tools/hooks/scratch-guard.sh`, `tools/hooks/no-verify-guard.sh`) say nothing about `references/`.

## Work

1. **Block for agents** (exit 2, the guards' message style) any Bash command that would modify `references/emutos` or
   `references/emutos-oracle/*`, reached by path (also via `git -C`, `--git-dir`, `--work-tree`, `cd <path> &&`, a
   worktree's `references/` symlink path, `$CLAUDE_PROJECT_DIR/references/...`): git `clone` into it, `fetch`, `pull`,
   `checkout`, `switch`, `reset`, `clean`, `stash`, `restore`, `rebase`, `merge`, `commit`, `am`, `apply`, `rm`, `mv`,
   `gc`, `prune`, `worktree add|remove` on it, `update-ref`, `config` writes; `rm`, `mv`, `cp`/`rsync` into it,
   `make` with `-C <it>` or cwd in it; output redirection into it; `git fetch`, `git pull`, `git remote update` there
   (anything that moves a ref off the pin). Write/Edit tool calls whose `file_path` lies in it.
   Read-only git (`status` with `GIT_OPTIONAL_LOCKS=0` or not, `log`, `show`, `ls-tree`, `ls-files`, `cat-file`,
   `rev-parse`, `diff`) and `git clone <it> <elsewhere>` stay allowed.
2. Name in the comment block what a text match cannot see (paths assembled at run time, scripts that do it inside),
   as the other guards do. The tools listed under *Facts* that write into a tree dir keep working only on a copy:
   `build-emutos.sh` must refuse a tree dir under `references/` (the oracle tree exists; rebuilding it is an owner act),
   `gen-deps.py`'s `git status` runs with `GIT_OPTIONAL_LOCKS=0`; `emutos-config.sh` likewise.
3. **Tests** in the guard's pipe-test script: every form of point 1 blocked in the main tree and through a worktree's
   symlink path; the read-only forms allowed; the same commands allowed on a scratch copy (a `git worktree` or `git
   archive` export under the `target-agents` directory, or the TOOL46 scratch path); wired into `cargo xtask test`.
4. **`cargo xtask land` is the only merge path for agents** (owner 2026-11-11): in the main repo the guard blocks a
   direct `git merge --squash` (the only merge form TOOL42 still allows) and its message points to
   `cargo xtask land <branch> -F <msg>` (TOOL46). Pipe tests: blocked in the main repo, allowed in a scratch clone,
   `cargo xtask land` itself allowed.
5. CLAUDE.md is not edited by the developer (the rule is already there); the receipt proposes changes if the guard's
   scope differs from it.

## Acceptance

- Pipe tests green; each new block case fails against the current guards (shown once).
- `build-emutos.sh` refuses a tree dir under `references/`; `gen-deps.py --check` still passes on the oracle tree.
- A tester confirms `references/emutos` and the oracle trees unchanged before/after its run (refs, index mtime,
  `git status --short` with `GIT_OPTIONAL_LOCKS=0`).
- `cargo xtask test` green, warnings delta 0, doc-check of edited docs 0 failures.

## Owner decision (2026-11-11, rework)

The tester's run on the first version (`8a170770c`, rebased `e0c1fc7fc`; run directory
measurement-2026-11-11-tool50-test, untracked in the main tree; PARTIAL leaning FAIL) found about 30 false
negatives: paths in another letter case (APFS is case-insensitive), wrappers the parser does not know (`timeout`, `gtimeout`,
`caffeinate`, `xcrun`, `/usr/bin/env`, `env -C`, `command cd`, `cd --`), `sed -Ei` bundles, `find` from an
ancestor, `>|` `>!` `&>|`, `git push` into a protected tree, `$'...'` quoting, other writers (`curl`/`wget -o`,
`sort -o`, `awk -i`, `vim`, `scp`), a symlink made on the same command line. Owner decisions:

1. Stop the pattern race. The pin is protected at OS and git level: the owner makes the trees read-only
   (`chmod -R a-w`, `chflags -R uchg`) and gives them an invalid remote URL; the lead prepares those commands.
2. `pin-guard.sh` only keeps an agent from undoing that lock: `chmod`, `chflags`, `chown` and `git remote`
   (`set-url`/`add`/`remove`/`rename`, also `git config remote.*`) on a `references/emutos*` path or repo (also via
   the worktree symlink, `-C`, `--git-dir`, `cd`). The large write/git allow-list logic goes.
3. Keep the `build-emutos.sh` refusal and `GIT_OPTIONAL_LOCKS=0` in `gen-deps.py` and `emutos-config.sh`.
4. Case-insensitive physical path comparison in one shared helper under `tools/hooks/lib/`, used by all guards and
   by the `build-emutos.sh` refusal.
5. The other gaps, and cherry-pick/rebase/read-tree/apply/`git fetch . x:main` in no-verify-guard, are documented
   as known limits in the guards' comment blocks; the backstops are TOOL43's server check and the OS-level lock.
6. The squash rule (`git merge --squash` into main blocked, use `cargo xtask land`) stays as it is.

## Implementation (developer, 2026-11-11; rework after the owner decision)

Receipt: [TOOL50-STATUS-UNFINISHED.MD](TOOL50-STATUS-UNFINISHED.MD).

- **Shared parser.** The TOOL42 tokenizer and simple-command walker live in `tools/hooks/lib/shell-parse.sh`,
  sourced by `no-verify-guard.sh` and `pin-guard.sh`: `process_seg` (`tools/hooks/lib/shell-parse.sh:125@process_seg`)
  calls the guard's `on_seg` and `on_command`, also tracks `--work-tree`/`GIT_WORK_TREE`; `is_cmd_word` decides where
  a wrapper ends; `parse_command` (`tools/hooks/lib/shell-parse.sh:326@parse_command`) marks every unquoted `>`.
  `no-verify-guard.sh` keeps `classify` (`tools/hooks/no-verify-guard.sh:150@classify`) and its rules in
  `on_command` (`tools/hooks/no-verify-guard.sh:235@on_command`). The TOOL42 cites moved with the code.
- **Shared path comparison** `tools/hooks/lib/paths.sh` (POSIX sh, so `build-emutos.sh` sources it too):
  `path_physical` (`tools/hooks/lib/paths.sh:51@path_physical`: nearest existing directory by `cd -P`, the rest
  lexically normalized) and the ASCII case-folding `path_same`/`path_in`/`path_contains`
  (`tools/hooks/lib/paths.sh:63@path_same`, `:67@path_in`, `:74@path_contains`). Used by `pin-guard.sh`, by
  `no-verify-guard.sh` for the main-repo comparison of its target (two `path_same` calls in `on_command`), by
  `scratch-guard.sh` (the command and the Write/Edit path are folded with `path_lower` before the `/tmp` matches;
  `CARGO_TARGET_DIR` matched with `grep -i`) and by the `build-emutos.sh` refusal.
- **Pin guard** `tools/hooks/pin-guard.sh` (PreToolUse, `Bash|Write|Edit|NotebookEdit`): protected are
  `references/emutos` and `references/emutos-oracle` of the main repo and of `$CLAUDE_PROJECT_DIR`.
  `on_command` (`tools/hooks/pin-guard.sh:222@on_command`): `chmod`, `chflags`, `chown` on a path in a protected
  tree or on a directory containing one (`hit_root`, `tools/hooks/pin-guard.sh:112@hit_root`; glob and brace words by
  their literal prefix; an undecidable path by its text), also as `find <start> -exec ...` (`pin_find`).
  `pin_git` (`tools/hooks/pin-guard.sh:289@pin_git`): `git remote add|set-url|remove|rm|rename` and a remote.*
  write of `git config` (`config_remote_write`, `tools/hooks/pin-guard.sh:263@config_remote_write`: set, `--add`,
  `--replace-all`, `--unset*`, `set`/`unset`, `--rename-section`/`--remove-section`, `-e`) in a protected repo (`-C`,
  `--git-dir`, `--work-tree`, `GIT_DIR=`/`GIT_WORK_TREE=`, `cd`, cwd) or with `--file` in one; reads (`remote -v`,
  `get-url`, `config --get`, a key alone) pass. Write/Edit/NotebookEdit by file path: kept, because the path comes as
  one exact string and the check is cheap. Everything else the first version blocked is left to the OS lock; the
  comment block lists the known limits.
- **Point 2.** `build-emutos.sh` refuses a tree dir under `references/` of the tree or the main repo, physically and
  case folded (`refuse`, `tools/oracle/build-emutos.sh:52@refuse`), before the cross-compiler check and any git or
  make. docs/analysis/ORACLE.md §3 names a copy as the tree dir. `gen-deps.py` runs its `git status` with
  `GIT_OPTIONAL_LOCKS=0` (`tools/deps/gen-deps.py:143@verify_tree`), `emutos-config.sh` likewise
  (`tools/emutos-config.sh:131@DIRTY`).
- **Point 4.** `classify` names `git merge --squash <branch>`; in the main repo with `main` checked out it is blocked
  like the other merges, message `cargo xtask land <branch> -F <msg>`. A squash in a worktree on another branch (a
  worker's own) is not a merge into main and stays allowed; so do a squash of remote-tracking refs only and a bare
  `git merge --squash`. `cargo xtask land` runs git as a subprocess, unseen.
- **Known limits** (comment blocks): `pin-guard.sh` names the tester's gaps and that every other write is left to the
  OS lock; `no-verify-guard.sh` adds apply, read-tree and `git fetch . x:main` to the commands it does not block,
  backstops TOOL43 and the OS lock.
- **Tests.** `tools/hooks/pin-guard-test.sh` (fixture repo with a throw-away `references/emutos`, an oracle clone,
  a worktree with TOOL44 symlinks, a scratch clone and a `git archive` export in a fixture target-agents dir;
  nothing is run, the guard only reads its input): 298 guard cases (lock commands and git remote/config in every
  path form and letter case, reads, five "known limit" cases that pin what is left to the lock) plus 10 for
  `build-emutos.sh` (a fixture copy with `paths.sh` and a cross compiler that does not exist, so a refusal that
  failed stops before git or make). Wired into `cargo xtask test` as `git_hooks::tests::pin_guard_pipe_tests_pass`
  (`tools/xtask/src/git_hooks.rs:234@pin_guard_pipe_tests_pass`), about 35 s. `no-verify-guard-test.sh`: 8 squash
  cases (one old case reversed) and 4 letter-case cases; `scratch-guard-test.sh`: 7 letter-case cases. Against the
  guards and `build-emutos.sh` of main `cb78c9d48` all 228 block cases of the pin test fail; against the first
  version all 42 letter-case and ancestor block cases fail; the new letter-case block cases of the other two tests
  fail against their earlier versions (2 of 4 and 5 of 7; the other cases are allow cases or were blocked before).
- **Retest fixes (2026-11-11; tester retest of `53e53f7f1`, run directory measurement-2026-11-11-tool50-retest,
  untracked in the main tree: PARTIAL leaning PASS).** `git config -f<path>` (attached) and a `GIT_CONFIG` file
  (prefix, `env`, export; also `GIT_CONFIG_GLOBAL`/`GIT_CONFIG_SYSTEM`) in a protected tree now count as the config
  file of the write: the shared parser tracks `GIT_CONFIG*` like `GIT_DIR` (`gc`/`gct`,
  `tools/hooks/lib/shell-parse.sh:128@process_seg`). `config_remote_write` also takes `url.*` (insteadOf,
  pushInsteadOf) and `branch.<b>.remote`/`pushRemote` as fetch-redirecting keys (cheap, so blocked rather than
  documented). The comment block now names as known limits chgrp, setfacl, xattr, a wrapper inside `find -exec`,
  mode-changing scripts (`python3 -c os.chmod`, perl), `env -C` precisely (the wrapper is known, its directory change
  is not tracked), `$'...'` precisely (only the literal text of the word is matched), other config keys, and
  (owner 2026-11-11) url.*.insteadOf in the global or system git config, not guarded, backstop the OS lock.
  Tests: 64 new pin-test cases (now 362 guard cases plus 10 build cases, 372 ok); against the previous `pin-guard.sh`
  all 48 new block cases fail, the 16 new allow and known-limit cases pass.
