# TOOL46 — receipt (unfinished: tester acceptance, CLAUDE.md lines, squash merge pending)

Role: developer
Agent: agent-a3897cb9c6a9b2bb9

Worktree `.claude/worktrees/agent-a3897cb9c6a9b2bb9`, branch `worktree-agent-a3897cb9c6a9b2bb9`, rebased onto
`b76a31c6b` (first base `dd6e2690e`). Commits on the branch: `6162096a6` (first version, tester PARTIAL; its
run directory measurement-2026-11-11-tool46-test is in the main tree, untracked), `e91e10348` (the rework per the
owner decision of 2026-11-11, TOOL46.md *Owner decision*; retest PARTIAL, run measurement-2026-11-11-tool46-retest,
main tree, untracked) and the retest fixes below. The main tree is untouched, so the hooks of every session
stay the old ones until the squash merge (Claude Code's file watcher picks up a changed `.claude/settings.json`,
hooks docs: "Direct edits to hooks in settings files are normally picked up automatically").

## Done

- `tools/hooks/agent-scratch.sh`: hook modes `subagent-start` (names the directory, removes an old `.stopped`),
  `subagent-stop` (writes `.stopped`, deletes nothing), `session-start` (72 h expiry, 20 GiB warning) and the command
  mode `clean <id>...`. It holds the only deletion code (`remove_agent_dir`) and the only safety checks. Wired in
  `.claude/settings.json` (SubagentStart 10 s, SubagentStop 10 s, SessionStart 120 s; no matcher).
- `tools/xtask/src/scratch.rs` (new): `cargo xtask scratch-clean <id>... | --ticket <ID> [--except <agent>]` and
  `cargo xtask land <branch> -F <file>`; both call the script's `clean` mode. `accept` (`tools/xtask/src/tickets.rs`)
  calls the same clean-up after its commit, skipping the acceptor.
- `tools/hooks/scratch-guard.sh`: blocks a `git clone` or a cargo target dir that names the session scratch root;
  the comment block names the known false positives and gaps. The old checks are unchanged.
- Tests: `tools/hooks/scratch-guard-test.sh`, `tools/hooks/agent-scratch-test.sh`, two host tests in
  `tools/xtask/src/git_hooks.rs`, two in `scratch.rs`, one in `tools/xtask/src/tickets/tests.rs`.
- `.claude/agents/tester.md`, `.claude/agents/developer.md`: where clones and target dirs go, when they are deleted,
  RUNINFO records path, size, sha256 first. `.gitignore`: `/target-agents/`.
- Details and `path:line@symbol` cites: section *Implementation* of [TOOL46.md](TOOL46.md).

## Retest fixes (third commit, after the PARTIAL of the retest of `e91e10348`)

- Test gap: with the accept call site mutated to `clean_ticket(root, id, None)` every test passed, because
  `accept_cleans_the_recorded_agents_scratch` uses `--result FAIL` (the FAIL tester stands below a heading, so it
  is not a recorded agent). New PASS-path test `accept_pass_cleans_the_developer_but_not_the_acceptor`: the DONE
  receipt records `agent-tester-b`, its directory must stay, the developer's must go. Mutation rerun (file copied
  aside, edited, copied back, `cmp` equal): with `None` the new test FAILs.
- Self-acceptance hole (pre-existing since TOOL41, found by the TOOL46 tester): own ticket
  [TOOL51](TOOL51.md) by owner decision 2026-11-11, fixed in the same commit, accepted separately; receipt
  [TOOL51-STATUS-UNFINISHED.MD](TOOL51-STATUS-UNFINISHED.MD). The scratch clean-up uses its `agent_key`.
- Full `cargo xtask test` after these fixes: 2408 passed, 0 failed, 6 ignored (three more than `e91e10348`).
  Clippy (`-p xtask -p tos-api -p tos-types --all-targets`) against the `b76a31c6b` baseline: the same 116
  summary lines, xtask bin 1 and bin test 9 unchanged, no warning in `scratch.rs`, `tickets.rs` or
  `tools/xtask/src/tickets/tests.rs`; only which tos-api summary line carries a "(N duplicates)" label varies between runs. Delta 0.

## Hook-input facts (source)

<https://code.claude.com/docs/en/hooks> (markdown form `hooks.md`), fetched 2026-11-11, Claude Code 2.1.322 on the
host. Sections *Common input fields*, *SubagentStart input*, *SubagentStop input*, *SessionStart input* and
*decision control*, *Exit code 0*, *JSON output*, *Reference scripts by path*:

- SubagentStop receives `agent_id`, `agent_type`, `agent_transcript_path`, `last_assistant_message`,
  `stop_hook_active`, plus the common `session_id`, `cwd`, `transcript_path`. SubagentStart receives `agent_id` and
  `agent_type` and can return `hookSpecificOutput.additionalContext` into the subagent; it also runs when a
  subagent is resumed. The same id in both.
- SubagentStop "runs when a Claude Code subagent has finished responding"; it also fires for Claude Code's internal
  agents (prompt suggestions, `/btw`), with an empty `agent_type`. The hook does nothing when the id names no
  directory.
- Exit 2 or `decision: "block"` on SubagentStop keeps the subagent running: the hook always exits 0.
- SessionStart: plain stdout or `additionalContext` reaches Claude; `systemMessage` is shown to the user.
- `${CLAUDE_PROJECT_DIR}` stays at the root where the session started when Claude works in a worktree; the script
  resolves the main repo through `git rev-parse --git-common-dir`, so it is right from a worktree too.
- The docs' examples show `agent_id` once as `def456` (transcript `agent-def456.jsonl`) and once as
  `agent-abc123`. Measured here: this run's own transcript (in the session's subagents folder, named after the
  worktree) carries `"agentId":"a3897cb9c6a9b2bb9"`, and the worktree is `agent-a3897cb9c6a9b2bb9`. The script
  strips a leading `agent-` and names the directory `agent-<id>`, the worktree's name, whichever form arrives.
- No environment variable carries the agent id (checked in this subagent's environment), hence the
  SubagentStart context.

## Squash-merge trigger: why `cargo xtask land`

Probe (a throw-away repo under the worktree's `target/`, hooks logging the state; git 2.54.0, Apple Git-157):
after `git merge --squash feat` the git dir holds `SQUASH_MSG` and no `MERGE_HEAD`; during `git commit -F m`,
`pre-commit`, `prepare-commit-msg` and `commit-msg` see `SQUASH_MSG`, `post-commit` does not (git removed it), and
no file names the branch. A git-hook trigger would have to hand a flag from `commit-msg` to `post-commit` through a
file and would also fire for a commit made over a stale `SQUASH_MSG` (a squash abandoned without `git reset`).
`cargo xtask land <branch> -F <file>` runs `git merge --squash`, `git commit -F` (the normal hooks check the commit)
and the clean-up of every ticket the subject names, in one command, and changes no git hook of the main tree. A
lead who squashes by hand without `land` loses nothing: the directories expire 72 h after their last stop, or
`cargo xtask scratch-clean --ticket <ID>` cleans them.

The acceptor's own directory: `accept` runs inside the tester, whose clone may still be its working directory, so
`accept` skips the agent it is given as `--agent` and says so; that directory gets its `.stopped` when the tester
stops and expires after 72 h (or goes at `land`, whose receipts then record the tester in the DONE receipt).

## Checks run

- `bash tools/hooks/scratch-guard-test.sh`: 40 ok, 0 FAIL. The same file against the guard of `b76a31c6b`
  (`git show`, run from a copy under `target/`): the 16 pre-TOOL46 cases, the target-agents allow cases and the
  pinned gaps pass; exactly the 9 new block cases and the pinned false positive fail (want 2, got 0).
- `AS_TEST_TMP=$PWD/target/tool46-test bash tools/hooks/agent-scratch-test.sh`: 65 ok, 0 FAIL. Mutations of the
  script (copies under `target/`, removed after) are caught: deleting at stop instead of marking (8 FAIL), expiry
  at 4300 min (1 FAIL), auto-deleting a marker-less dir (7 FAIL), start not removing the marker (3 FAIL), removing
  `$root` instead of the worker dir (8 FAIL).
- `cargo test -p xtask --bin xtask scratch` / `accept`: the new Rust tests pass.
- Warnings against main: `cargo clippy -p xtask -p tos-api -p tos-types --all-targets` on a `git archive` of
  `b76a31c6b` (own target dir under `target/`, removed after) and on this branch: the same 116 summary lines, the
  same counts per crate (`xtask` bin 1, bin test 9; `tos-api` lib 73); only the order of the "(N duplicates)"
  labels differs. The one xtask warning in a touched file, `tools/xtask/src/main.rs` `this assertion has a
  constant value`, is the pre-existing one, two lines further down. Delta 0.
- Full `cargo xtask test` after `cargo xtask worktree-setup` (TOOL44): 2405 passed, 0 failed, 6 ignored; five of
  them are TOOL46's (two pipe tests, two in `scratch.rs`, one `accept` test). Compiler warnings: `tos-api` 64 (lib),
  `xtask` bin test 2 (`kernel/src/drivers/bcm2711/clock_tags.rs`), as on main.

## Missing (acceptance)

- One real worker run under the merged hooks: a tester clones into `target-agents/agent-<id>/`, its RUNINFO names
  path, size and sha256s; after its stop the directory and contents are still there with a `.stopped`; after
  `cargo xtask accept` / `land` of the ticket the directories of its recorded agents are gone.
- The SessionStart warning shown once in a real session with a fixture over a lowered threshold
  (`NEWTOS_AGENT_SCRATCH_WARN_KB` in the environment of the Claude Code process), and one expiry (a fixture
  directory whose `.stopped` mtime is set back by more than 72 h, e.g. with
  `perl -e '$t=time-73*3600; utime $t,$t,shift' <dir>/.stopped`).
- The live measurement below; CLAUDE.md *Scratch* lines (below), applied by the lead.

## How to measure whether SubagentStop fires while a worker waits on its own background work

After the merge, in a session whose `.claude/settings.json` has the hooks:

1. Spawn a worker (any agent type) with the brief: `mkdir -p` the directory its context names; run
   `sleep 120; date -u > <dir>/bg-done` as a background Bash command (`run_in_background`); without waiting, answer
   "waiting"; when the notification arrives, write `date -u > <dir>/fg-done` and end with the report.
2. Afterwards read `<dir>/.stop-log` (one `<UTC> start` / `<UTC> stop` line per hook run) next to `bg-done` and
   `fg-done`. A `stop` line earlier than `bg-done` means SubagentStop fired while the worker waited on its
   background work; only one `stop`, later than `fg-done`, means it fires at the end only. Either way the
   directory must still exist (the stop only marks).
3. Record the three timestamps and the verdict in the run's RUNINFO, then `cargo xtask scratch-clean <agent-id>`.

## Proposed CLAUDE.md lines (Placement rules, replacing the *Scratch* bullet)

```
- **Scratch** (temporary images, scratch kernels, probes) goes to
  `$CLAUDE_JOB_DIR/tmp` or the session scratchpad, never the repo or shared `/tmp`.
  Both lie on the small system disk: **test clones and cargo target dirs** of a
  worker go to `target-agents/agent-<id>/` in the main repo (gitignored, data
  volume), the directory the SubagentStart hook names in the worker's context.
  It survives the worker's stops; `cargo xtask accept <ID>` and
  `cargo xtask land <branch> -F <msg>` (squash merge plus commit) delete the
  directories of the agents the ticket's receipts record, the SessionStart hook
  those whose last stop is 72 h old, `cargo xtask scratch-clean` any by hand.
  So the run's `RUNINFO.TXT` records the clone's path, size and the sha256 of
  every cited gitignored output first. `tools/hooks/scratch-guard.sh` blocks a
  clone or a target dir in the scratchpad; SessionStart warns when the
  `target-agents` directory exceeds 20 GiB (`tools/hooks/agent-scratch.sh`; TOOL46).
```
