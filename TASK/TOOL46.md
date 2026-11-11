# TOOL46 — worker scratch off the system disk

Status: DONE (2026-11-11) | Priority: P1 | Commit: 68bbdca69 | Receipt: TOOL46-STATUS-DONE.MD

Type: tool (enforcement). Owner decision 2026-11-11. Small. Changes `tools/hooks/**` and `.claude/settings.json`, the
live hooks of every agent session: developed in its own git worktree and landed by `git merge --squash` after the
tester's acceptance, like the git-hook tickets (CLAUDE.md *Commit Discipline*).

## Facts

- 2026-11-11 the system disk ran full (ENOSPC on the volume of `/tmp/claude-501`, so no Bash command could start).
  The lead's session scratchpad held 14 GB, 13.6 GB of it three test clones of the TOOL41 acceptance runs (10 GB,
  1.9 GB, 1.7 GB, each with its own cargo `target/`), left behind by the testers. Older scratchpads of another project
  held 11 GB more. The owner freed space by hand; the lead then emptied the scratchpads.
- The system disk is small (460 GB, shared APFS container); the data volume `/Volumes/Daten`, which holds the repo, has
  1.2 TB free.
- `tools/hooks/scratch-guard.sh` (PreToolUse, `.claude/settings.json`) allows scratch only under
  `/private/tmp/claude-<uid>/` and blocks other `/tmp` paths; it says nothing about where a clone or a cargo target
  directory should go, and nothing removes a worker's scratch when the worker ends.

## Work

1. **Place.** Worker test clones and worker `CARGO_TARGET_DIR`s go under the `target-agents` directory in the repo root on the
   data volume (repo-local, gitignored: one `.gitignore` line; it is a build-output directory like `/target`, so the
   "nothing new in the repository root" rule is met by the ignore entry, not by a tracked file). One subdirectory per
   worker run, named so a hook can find it (e.g. `target-agents/<agent-id>/`). `scratch-guard.sh` allows it.
2. **Clean-up.** When a worker ends, its subdirectory is removed: a `SubagentStop` hook, or `cargo xtask scratch-clean`
   called by it (the developer checks in the Claude Code hooks documentation what a SubagentStop hook receives, e.g.
   the agent id, and says so in the receipt). Evidence is kept first: the worker's `RUNINFO.TXT` records the clone's
   path, its size, and the sha256 of every output it cites (images, binaries, logs that stay gitignored).
3. **Warning.** A SessionStart hook prints a warning when the `target-agents` directory exceeds 20 GB (size and the largest
   subdirectories).
4. **Briefs.** `.claude/agents/tester.md` and `.claude/agents/developer.md` say where clones and target directories
   go and that the run's RUNINFO records path, size and sha256 before the clean-up. CLAUDE.md is not edited by the
   developer: the receipt proposes the lines for *Scratch* and the lead applies them.
5. **Tests.** Pipe tests for the scratch-guard change (allowed under the `target-agents` directory, the old cases unchanged) and for
   the clean-up script (removes only the finished worker's own subdirectory, never another one, never outside
   the `target-agents` directory), wired into `cargo xtask test` if cheap.

## Acceptance

- One real worker run (a tester in a clone under the `target-agents` directory) leaves no clone behind after it ends; its RUNINFO
  names path, size and sha256s.
- The SessionStart warning shown once with a directory over the threshold (a test fixture, not 20 GB of data).
- Guard and clean-up tests green; `cargo xtask test` green, warnings delta 0, doc-check of the edited docs 0 failures.

## Owner decision (2026-11-11)

After the tester's PARTIAL on the first implementation (deletion at SubagentStop: a worker resumed by SendMessage
finds its directory gone, and it is unknown whether SubagentStop fires while a worker waits on its own background
work):

1. SubagentStop no longer deletes. It only records the stop time in a marker file inside
   `target-agents/agent-<id>/` (`.stopped`, overwritten on every stop); SubagentStart of a resumed run removes it.
2. Deletion is automatic, not a lead rule: `cargo xtask accept <ID>` and the squash-merge step call
   `cargo xtask scratch-clean` for the agents recorded in that ticket's receipts (`Agent:` header lines of
   `TASK/<ID>-STATUS-*.MD`, `agent-` prefix normalised). The acceptor's own directory is not deleted from inside its
   own accept run; it expires by point 3. `scratch-clean <id>` is also callable by hand. One implementation of the
   deletion safety checks.
3. SessionStart deletes agent directories whose last stop (marker mtime) is older than 72 h; directories without a
   marker are never deleted automatically, only reported; the 20 GB warning stays.
4. The scratch-guard extension stays; its comment block documents the known false positives and gaps.

## Implementation

Developer, 2026-11-11, worktree branch `worktree-agent-a3897cb9c6a9b2bb9`, rebased onto `b76a31c6b` (receipt:
[TOOL46-STATUS-UNFINISHED.MD](TOOL46-STATUS-UNFINISHED.MD)). First commit `6162096a6` (tester PARTIAL), rework per
the owner decision in the next commit.

- **Hook input (the facts the design rests on).** <https://code.claude.com/docs/en/hooks>, read 2026-11-11 (Claude
  Code 2.1.322): SubagentStart and SubagentStop both receive `agent_id` (plus `agent_type`, and for SubagentStop
  `agent_transcript_path`, `last_assistant_message`, `cwd`); SessionStart receives no agent id; SubagentStart and
  SessionStart may return `hookSpecificOutput.additionalContext`; `systemMessage` is shown to the user;
  `${CLAUDE_PROJECT_DIR}` is the root where the session started, also when a subagent works in a worktree; exit 2
  on SubagentStop would keep the subagent running. No environment variable gives a worker its own id, so the
  worker learns its directory from the SubagentStart context. The id in this session's subagent transcripts is the
  worktree name without `agent-`; the docs show both forms, so a leading `agent-` is stripped and the directory is
  always `agent-<id>`.
- **One deletion implementation:** `tools/hooks/agent-scratch.sh`. `remove_agent_dir`
  (`tools/hooks/agent-scratch.sh:114@remove_agent_dir`) is the only code that deletes; it takes the id through
  `norm_id` (`tools/hooks/agent-scratch.sh:91@norm_id`: `[A-Za-z0-9][A-Za-z0-9_-]*`, `agent-` stripped) and the
  directory through `agent_dir` (`tools/hooks/agent-scratch.sh:99@agent_dir`: no symlink in place of the directory,
  physical path exactly `<top>/target-agents/agent-<id>`); a symlink in place of `target-agents` is refused before
  any mode runs. `<top>` is the parent of the git common dir of `$CLAUDE_PROJECT_DIR`
  (`tools/hooks/agent-scratch.sh:86@top`).
- **Hooks** (`.claude/settings.json`): `subagent-start` names the directory in the worker's context
  (`tools/hooks/agent-scratch.sh:193@ctx`) and removes a `.stopped` left by an earlier run of the same id;
  `subagent-stop` deletes nothing and writes the UTC stop time into `.stopped`
  (`tools/hooks/agent-scratch.sh:201@stopped`); both append a line to `.stop-log`, which makes the timing of
  SubagentStop observable. A marker symlink is removed as a link, never written through. `session-start`
  (`tools/hooks/agent-scratch.sh:136@mode`) deletes the directories whose `.stopped` is older than
  `expire_min=4320` minutes (`tools/hooks/agent-scratch.sh:51@expire_min`), lists those without a marker, and warns
  over `NEWTOS_AGENT_SCRATCH_WARN_KB` (default 20 GiB, `tools/hooks/agent-scratch.sh:155@limit`).
- **Commands** (`tools/xtask/src/scratch.rs`): the `clean` mode of the script is called by `clean`
  (`tools/xtask/src/scratch.rs:55@clean`); `clean_ticket` (`tools/xtask/src/scratch.rs:88@clean_ticket`) takes the
  `Agent:` header values of every receipt of the ticket (`recorded_agents`) and skips the running acceptor.
  `cargo xtask scratch-clean <id>... | --ticket <ID> [--except <agent>]` (`tools/xtask/src/scratch.rs:111@cmd_scratch_clean`).
  `accept` calls `clean_ticket` after its commit with `--agent` as the exception
  (`tools/xtask/src/tickets.rs:1907@clean_ticket`).
- **Squash-merge trigger: `cargo xtask land <branch> -F <file>`** (`tools/xtask/src/scratch.rs:158@land`):
  `git merge --squash`, `git commit -F` with the normal hooks, then `clean_ticket` for every ticket the subject
  names. Refuses with changes staged, with `<branch>` checked out, for an unknown branch; a rejected commit leaves
  the squash staged and says how to finish by hand. Not a git hook, because git leaves no usable trace for one:
  measured with git 2.54 (Apple Git-157), `git merge --squash` writes `.git/SQUASH_MSG` and no `MERGE_HEAD`;
  `pre-commit`, `prepare-commit-msg` and `commit-msg` see `SQUASH_MSG`, `post-commit` no longer does, and nothing
  names the branch. A hook trigger would need a hand-over file between two hooks and would fire on a stale
  `SQUASH_MSG`; `land` changes no git hook of the main tree. A raw `git merge --squash` without `land` is caught by
  the 72 h expiry.
- **Guard:** `scratch-guard.sh` already allowed the `target-agents` directory (it lies on the data volume). New: a
  `git clone` or a cargo target dir (`CARGO_TARGET_DIR=`, `--target-dir`) in a command segment that names a path
  under the session scratch root is blocked (`tools/hooks/scratch-guard.sh:58@bulky_in_scratch`); the comment block
  names the known false positives (`git clone` as text in an echo, a commit message or a comment, together with a
  scratch path in the same segment) and gaps (relative clone target after `cd`, `git worktree add`, a target dir
  through a variable, cargo's `build.target-dir`).
- **Briefs:** `.claude/agents/tester.md` and `.claude/agents/developer.md`, *Environment*. `.gitignore`:
  `/target-agents/`.
- **Tests:** `tools/hooks/scratch-guard-test.sh` (40 cases: the 16 pre-TOOL46 behaviours, the target-agents allow
  cases, the new blocks, the documented false positive and gaps); `tools/hooks/agent-scratch-test.sh` (65 cases in
  a `git init` fixture with a worktree: stop marks and keeps, a stop while waiting on background work keeps the
  clone, resume keeps dir and contents, 72 h expiry at 71 h 50 min and 72 h 10 min, a marker-less dir is never
  deleted, every refusal of the old version); both wired through `hook_pipe_test`
  (`tools/xtask/src/git_hooks.rs:188@hook_pipe_test`). Rust: `scratch::tests::scratch_clean_by_id_and_by_ticket`,
  `land_squashes_commits_and_cleans_the_named_tickets_agents`
  (`tools/xtask/src/scratch.rs:249@land_squashes_commits_and_cleans_the_named_tickets_agents`),
  `tickets::tests::accept_cleans_the_recorded_agents_scratch`
  (`tools/xtask/src/tickets/tests.rs:500@accept_cleans_the_recorded_agents_scratch`).
- **Retest fixes (third commit).** (a) The PASS path of the accept clean-up has its own test,
  `accept_pass_cleans_the_developer_but_not_the_acceptor`
  (`tools/xtask/src/tickets/tests.rs:517@accept_pass_cleans_the_developer_but_not_the_acceptor`): the DONE receipt
  `accept` writes records the acceptor, whose directory must stay. The FAIL-path test alone did not catch an accept
  that cleans with no exception, because a FAIL tester is recorded below a heading, not in a header. (b) The
  self-acceptance hole of TOOL41 the TOOL46 tester found (`accept --agent dev-a` not refused when the receipt records
  `agent-dev-a`) is [TOOL51](TOOL51.md) (owner 2026-11-11): its fix, `agent_key`, lands in the same squash commit and
  is the normalisation the scratch clean-up's `--except` uses
  (`tools/xtask/src/scratch.rs:93@agent_key`); details in TOOL51's *Implementation*.
- **Not covered:** a tester that records its verdict without `accept` and a ticket landed without `land` leave
  their directories to the 72 h expiry; a directory never stopped (no marker; a worker killed without SubagentStop)
  is only listed.
