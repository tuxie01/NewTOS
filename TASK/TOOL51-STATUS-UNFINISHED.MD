# TOOL51 — receipt (unfinished: tester acceptance pending)

Role: developer
Agent: agent-a3897cb9c6a9b2bb9

Worktree `.claude/worktrees/agent-a3897cb9c6a9b2bb9`, branch `worktree-agent-a3897cb9c6a9b2bb9` (the TOOL46 branch),
base `3d129eac8`. The fix was first written under TOOL46 (branch commit "accept compares agents without the agent-
prefix", fixcheck PASS by the TOOL46 tester, run measurement-2026-11-11-tool46-fixcheck in the main tree); by owner
decision 2026-11-11 it is its own ticket, landing in the squash commit that names TOOL46 and TOOL51.

## Done

- `agent_key` in `tools/xtask/src/tickets.rs`, used by `accept`, the `status-check` self-acceptance flag and the
  TOOL46 scratch clean-up (`tools/xtask/src/scratch.rs`). Details and cites: *Implementation* of
  [TOOL51.md](TOOL51.md).
- Host tests `accept_refuses_the_developer_in_either_spelling` and `self_accept_flag_normalises_the_agent_prefix`
  in `tools/xtask/src/tickets/tests.rs`.

## Checks run

- Mutation (file copied aside, edited, copied back, `cmp` equal): with the old `contains(&x.to_lowercase())`
  comparisons in `accept` and `status-check`, both new tests FAIL; restored, both pass.
- Full `cargo xtask test` on the rebased branch and clippy against main: totals in the TOOL46 receipt
  ([TOOL46-STATUS-UNFINISHED.MD](TOOL46-STATUS-UNFINISHED.MD)) and the developer's report; warnings delta 0.

## Missing (acceptance)

- Tester: the point 2 tests green, the reverted comparison failing them; `accept` in a scratch repo refused for each
  spelling of the developer's name; `cargo xtask test` green, warnings delta 0.
