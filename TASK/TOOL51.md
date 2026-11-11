# TOOL51 — accept refuses the developer under either spelling of the agent name

Status: IMPLEMENTED (2026-11-11) | Priority: P1 | Commit: self | Missing: tester acceptance through cargo xtask accept

Type: tool (enforcement), follow-up of [TOOL41](TOOL41.md) (DONE). Owner decision 2026-11-11: own ticket; the fix may
land in the [TOOL46](TOOL46.md) squash commit that names both ids, and TOOL51 is accepted separately.

## Facts

- The TOOL46 retest (`measurements/measurement-2026-11-11-tool46-retest/RUNINFO.TXT`, out-of-scope observation) found
  that `cargo xtask accept <ID> --agent dev-a` is not refused when the ticket's receipt records `Agent: agent-dev-a`:
  the developer-vs-acceptor comparison in `accept` lowercases but does not drop the `agent-` prefix, so a developer can
  accept their own ticket by spelling the name differently (ADR-037 point 12). `status-check`'s self-acceptance flag
  compares the same way.
- The acceptances so far (TOOL40, TOOL41, TOOL42, TOOL44) used acceptors that were not developers of those tickets.

## Work

1. One normalisation (trim, lowercase, drop a leading `agent-`) used for the developer-vs-acceptor check in `accept`, the
   self-acceptance flag of `status-check`, and the scratch clean-up's `--except` (TOOL46); stored names stay as written.
2. Host tests: refused for `dev-a`/`agent-dev-a` in both directions and in mixed case; flagged by `status-check` the
   same way; a different agent passes.
3. The developer's receipt `TOOL51-STATUS-UNFINISHED.MD` records `Role: developer` and `Agent:`.

## Acceptance

- The tests of point 2 green; reverting to the old comparison fails them (tester shows it).
- In a scratch repo, `accept` with each spelling of the developer's name is refused.
- `cargo xtask test` green, warnings delta 0.

## Implementation

Developer, 2026-11-11, on the TOOL46 worktree branch `worktree-agent-a3897cb9c6a9b2bb9` (base `3d129eac8`), in the
commit that names TOOL46 and TOOL51 (receipt: [TOOL51-STATUS-UNFINISHED.MD](TOOL51-STATUS-UNFINISHED.MD)).

- **One normalisation:** `agent_key` (`tools/xtask/src/tickets.rs:2052@agent_key`) trims, lowercases and drops a
  leading `agent-`. It stands at the end of the file so no line cite into the rest of `tickets.rs` shifts. Stored
  names stay as written: `developers()` returns them raw, so the "Developer:" line of a DONE receipt is unchanged.
- **Three call sites:** the developer-vs-acceptor check of `accept`
  (`tools/xtask/src/tickets.rs:1829@agent_key`), the self-acceptance flag of `status-check`
  (`tools/xtask/src/tickets.rs:1008@agent_key`), and the `--except` of the scratch clean-up of TOOL46
  (`tools/xtask/src/scratch.rs:93@agent_key`), whose own `norm` was removed.
- **Tests:** `accept_refuses_the_developer_in_either_spelling`
  (`tools/xtask/src/tickets/tests.rs:533@accept_refuses_the_developer_in_either_spelling`): recorded/given
  `dev-a`/`agent-dev-a`, `agent-dev-a`/`dev-a`, `Agent-Dev-A`/`dev-a`, `dev-a`/`DEV-A` are refused, plus asserts on
  `agent_key`; `self_accept_flag_normalises_the_agent_prefix`
  (`tools/xtask/src/tickets/tests.rs:346@self_accept_flag_normalises_the_agent_prefix`): `dev-1`/`agent-dev-1` and
  `agent-dev-1`/`Dev-1` are flagged, `agent-dev-1`/`dev-2` is not. With the old `contains(&x.to_lowercase())`
  comparisons both tests fail (developer's mutation run, 2026-11-11).
