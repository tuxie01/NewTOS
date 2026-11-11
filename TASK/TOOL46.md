# TOOL46 — worker scratch off the system disk

Status: OPEN (2026-11-11) | Priority: P1

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
