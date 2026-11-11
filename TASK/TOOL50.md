# TOOL50 — the agent guard keeps the EmuTOS checkouts on the pin; `land` is the only merge path

Status: OPEN (2026-11-11) | Priority: P1

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
