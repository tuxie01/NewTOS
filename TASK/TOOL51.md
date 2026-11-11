# TOOL51 — accept refuses the developer under either spelling of the agent name

Status: OPEN (2026-11-11) | Priority: P1

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
