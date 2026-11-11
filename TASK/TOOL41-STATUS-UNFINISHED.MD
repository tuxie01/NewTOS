# TOOL41 — receipt (unfinished: tester acceptance, owner's switch, CLAUDE.md texts pending)

Role: developer
Agent: agent-a823a57e109be35e1

Developer, 2026-11-11. Worktree branch `worktree-agent-a823a57e109be35e1`, rebased on `dc4c12093`; not merged.
The ticket's *Implementation* section lists what was built and where ([TOOL41](TOOL41.md)).

## Done

- `tools/xtask/src/tickets.rs` (parser, `status-check`, `accept`, `set-status`, `report`), 18 host tests in
  `tools/xtask/src/tickets/tests.rs`, the baseline `tools/xtask/ticket-baseline.tsv` (899 legacy violations of
  today's tree), `tools/git-hooks/commit-msg` (new), step 3 of `tools/git-hooks/pre-commit`, `install-hooks` installs
  and reports both, `TASK/README.md`: an English section pointing to ADR-037 and the Status column regenerated from
  line 3 (it was stale in 48 rows).
- `main.rs` and `git_hooks.rs` keep the line positions of every symbol cited by a doc (`doc-check --staged`: 0
  broken): the ticket commands are dispatched through the fallback arm (`tickets::cmd`) and their usage text is
  `tickets::USAGE`, appended to `cargo xtask help`.

## Design decisions (for the lead and the tester)

1. **Switch:** `git config newtos.ticketChecks true` (repository config, shared by all worktrees). Unset, `false`,
   `no`: `commit-msg` exits 0 at once (no output, no cargo) and `pre-commit` gives the same exit code, output and
   cargo calls as at `ced5e68f6` (host test `hooks_with_the_switch_off_behave_as_after_tool40`, which runs TOOL40's
   hook from the history against the new one with a fake cargo; also checked with real cargo in a clone).
2. **Names:** an id is named on a word boundary (letters, digits and `_` are word characters, `-` is not, so
   `DESK43-U5a` names `DESK43`). The hooks take the ids from the **subject only** (bodies mention other tickets
   freely); `accept` checks the **whole message** of the accepted commit.
3. **fix/feat rule:** every id a `fix`/`feat` subject names must be `IMPLEMENTED` in the index, so a legacy ticket
   named by a fix commit must get its new line 3 in that commit (ADR-037 point 7). Point 13 names `OPEN` and
   `DONE`; point 11 requires `IMPLEMENTED`, which also rejects `PARKED`/`ACCEPTED`/`OBSOLETE` and legacy lines.
4. **Baseline:** `<key>\t<code>`, key = ticket id, or the file name of an orphan receipt / unknown file. A fixed
   violation rejects until `cargo xtask status-check --prune-baseline` removes its entry; the checked baseline may not
   contain an entry `HEAD`'s lacks. `--write-baseline` refuses when the file exists. `set-status` and `accept` prune
   the entries of their ticket.
5. **Rejecting vs reporting:** rejecting = point 13 plus malformed line 3, unknown commit hash, `Target`/`Ref` naming
   nothing, orphan receipt, unknown file in `TASK/`. Reporting: blocked-by-DONE, self-acceptance, `OPEN`/`PARKED`
   tickets named by `fix`/`feat` commits of the history, the `docs/STATUS.md` lists (keyword candidates, as the
   inventory's C3 — not reliable enough to reject), a stale README column.
6. **Developer record:** the header of a receipt (lines before its first `## `) with `Role: developer` and
   `Agent: <name>`; several are allowed, all count. `accept` refuses when none is recorded. **TOOL40's receipt has no
   such header, so `accept TOOL40` is refused today** ("no developer recorded"); the lead adds `Role: developer` and
   the TOOL40 developer's `Agent:` to `TASK/TOOL40-STATUS-UNFINISHED.MD` first.
7. **accept FAIL** appends `## Acceptance <date> — FAIL` to `<ID>-STATUS-UNFINISHED.MD` instead of overwriting it,
   so the developer's header survives. PASS writes the commit as 9 hex digits and updates only the ticket's own
   README row. The generated message is `docs(task): accept <ID>, PASS|FAIL on <hash> (<ID>)`, without a
   `Co-Authored-By` trailer (the tool, not an agent, writes it) — the lead may want one.
8. **set-status** writes a `## Status history` entry at the end of the ticket (the old line 3; for `OPEN` the
   reason, which line 3 has no field for); `OPEN` only from `PARKED`; refused on `DONE`.
9. **README:** `report --readme` rewrites the Status cell of every table whose header has a `Status` column and
   whose rows start with a ticket link; legacy tickets show their raw status text (links and `**` removed, 60
   characters). The backlog table without a Status column is untouched.
10. `Commit: self` resolves with `git log -L3,3` and follows a rename of the ticket file (host test). After the
    squash merge it resolves to the squash commit.

## Verification

- `cargo xtask test`: **2393 / 0 / 6** after the rebase; the base `d033a78a2` in the same environment (a clone with
  the same links) 2375 / 0 / 6, so +18 = the new tests; `cargo clippy -p xtask --tests` 9 warnings before and after, `cargo clippy -p xtask` 1 and 1 (all
  pre-existing, `main.rs` and `clock_tags.rs`). No kernel or tos-api code changed.
- `cargo xtask status-check` on this tree: exit 0, 0 rejected, 899 legacy, 12 reported; `cargo xtask report` prints
  the counts.
- Acceptance self-check in a throwaway clone, switch on (times wall-clock, the hooks included): `fix(x): ... (X1)`
  with X1 `OPEN` rejected by commit-msg (1.69 s); the same commit with line 3 `IMPLEMENTED | Commit: self |
  Missing` passes (2.82 s); a DONE receipt without line 3 `DONE` rejected by pre-commit (2.39 s); `accept X1 --result
  PASS` by another agent: one commit, receipt + line 3 `DONE`, tree clean (3.24 s); by the developer's agent refused
  (0.23 s); `set-status TOOL42 PARKED` without `--reopen` refused (0.13 s). Switch off: TOOL40's and the new
  pre-commit give equal output and exit codes on a staged ticket and on an unrelated file, commit-msg is silent,
  a fix commit naming an `OPEN` ticket passes (0.28 s). `status-check` 0.22 s, `report` 0.33 s.
- Gitignored inputs linked for the test run (TOOL44 not there yet): `references/<each entry>` and
  `sdk/apps/{coremark,highwire,hypview,qed,teradesk}` as symlinks to the main tree's copies. The `sdk/apps` links
  show as untracked (the ignore rule matches directories only); they were removed after the run and never staged.

## Fixes after tester PARTIAL

Tester verdict PARTIAL (run measurement-2026-11-11-tool41-test, in the main tree, not on this branch): all 22 acceptance cases pass;
decision g had no host test. Branch rebased on `9bc27d82b` (TOOL42 merged; `git_hooks.rs` merged without conflict).

- **D1:** test `every_named_id_is_checked_not_only_the_first`: a `fix` subject naming `A1` (`IMPLEMENTED`) and `B1`
  rejects with `B1` `OPEN`, `DONE` or `PARKED`, passes with both `IMPLEMENTED`; a `docs` subject naming both rejects
  when `B1` lacks `Missing` (the commit-msg gap check). The tester's mutants script of that run, run unchanged in a clone of
  the branch: all ten mutants killed, d1, d2 and d3 by the new test.
- **D2:** renamed, not recomputed. The inventory's C1–C6 are keyword heuristics on German status text and STATUS.md
  prose; the report's rules are machine rules with other results (every legacy line 3 is `form`, every legacy
  `PARKED` lacks the fields). `report` now prints *gap groups* `done-not-closed`, `closed-without-evidence`,
  `status-md-mismatch`, `parked-without-fields`, `not-adr037-form`, `dangling-reference` and says they are not the
  inventory's classes (test `report_groups_do_not_reuse_the_inventory_class_names`).
- `cargo xtask test` (links as above, made again for the run and the `sdk/apps` ones removed after): **2396 / 0 / 6**
  (2393 + 1 from TOOL42 + 2 new). `cargo clippy -p xtask --tests` 9, `cargo clippy -p xtask` 1: unchanged.
- TOOL40's receipt still lacks `Role: developer` / `Agent:`; untouched here, the lead handles it.

## Open

- Tester acceptance (`Missing` of line 3). The owner's OK for the switch.
- The lead applies the proposed CLAUDE.md and agent-brief texts (hand-off), and adds the developer record to TOOL40's
  receipt before its `accept`.
