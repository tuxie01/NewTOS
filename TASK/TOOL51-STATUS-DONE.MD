# TOOL51 — receipt DONE (2026-11-11)

Role: tester
Agent: dev-a
Commit: 68bbdca69
Result: PASS

Written by `cargo xtask accept` (ADR-037 point 11). Developer: agent-a3897cb9c6a9b2bb9. Line 3 before: `Status: IMPLEMENTED (2026-11-11) | Priority: P1 | Commit: self | Missing: tester acceptance through cargo xtask accept`.

## Note

x

## Correction (lead, owner decision 2026-11-11)

The entry above is kept unchanged as history; its `Agent:` line and note are wrong.

- **Real acceptor:** `agent-a3ed1ae22a1ca895c` (tester; not a developer of TOOL51, whose developer is
  `agent-a3897cb9c6a9b2bb9`).
- **Real checks** on `68bbdca69`: `cargo test -p xtask agent` (the spelling tests) 4 passed / 0 failed; in a scratch
  clone with its own xtask build and a receipt `Agent: agent-dev-a`, `accept --agent dev-a` and
  `accept --agent agent-dev-a` were both refused as the recorded developer; full `cargo xtask test` 2415 / 0 / 7.
- **Cause:** a test invocation of the main tree's xtask binary from inside that clone. xtask takes its repository root
  from its build, not from the working directory, so `accept TOOL51 --commit 68bbdca69 --result PASS --note x
  --agent dev-a` ran against the main repository and committed `03499102b` there. Follow-up: [TOOL52](TOOL52.md).
