# TOOL56 — generate the STATUS.md queue and parked lists from ticket line 3

Status: OPEN (2026-11-11) | Priority: P2

Type: tool. Owner decision 2026-11-11 (d). Tooling slot, after the queue in docs/STATUS.md *Way of working*.

## Facts

- `cargo xtask status-check` reports `status-md` when `docs/STATUS.md` names a ticket as open whose line 3 says
  otherwise (`tools/xtask/src/tickets.rs`, `status_md_claims`: a keyword window after each mention). After the
  legacy migration of 2026-11-11 it reported 13 such mentions; the lead fixed them by hand (adding "(parked)").
- The lists in `docs/STATUS.md` (*Phase status* bullets P1/P2/P3–P4, the queue, the parked and deferred lists) are
  hand-written prose with ticket links; nothing keeps them in step with line 3.

## Work

1. A generated block in `docs/STATUS.md` between two marker comments: the open queue (`OPEN` and `IMPLEMENTED` by
   priority, with each ticket's title), the parked list (`PARKED` with Reason, shortened), and the accepted list,
   all read from line 3 of `TASK/<ID>.md`. `cargo xtask report --status-md` writes it; `--check` fails when the block
   is stale. The hand-written prose outside the markers stays.
2. The pre-commit hook runs the check when a ticket's line 3 or `docs/STATUS.md` is staged (cheap: no cargo start
   otherwise), so the block cannot go stale; or `migrate`, `accept` and `set-status` refresh it in their own commit.
   Choose one, justify it.
3. The lead moves the hand-written lists that duplicate the block into it (proposed in the receipt; the lead applies
   the STATUS edit — `docs/STATUS.md` is the lead's).
4. Host tests: block generation from fixture tickets; stale block → check fails; prose outside the markers untouched.

## Acceptance

- Tests of point 4 green; a mutation that skips a state in the block fails a test.
- `status-check` reports no `status-md` mismatch for the generated block.
- `cargo xtask test` green, warnings delta 0, doc-check of the edited docs 0 failures.
