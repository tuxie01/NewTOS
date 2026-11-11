# TOOL49 — `doc-check --fix-anchors`: anchor line numbers follow their symbol

Status: OPEN (2026-11-11) | Priority: P2

Type: tool. Owner decision 2026-11-11, under the [SPEC03](SPEC03.md) umbrella (doc-error backlog). Changes the
pre-commit hook's behaviour (point 3): developed in its own git worktree and landed by `git merge --squash` after the
tester's acceptance (CLAUDE.md *Commit Discipline*).

## Facts

- An anchored cite `<path>:<line>@<symbol>` passes doc-check when the symbol stands within ±5 lines of the cited
  line (`tools/xtask/src/doc_check.rs:477@check_anchor`); on failure doc-check names the nearest line where the
  symbol occurs.
- 2026-11-11 a scan found about 190 anchored cites whose symbol stands 4–5 lines from the cited line, in `TASK/`,
  `docs/api/`, `docs/audits/`, `docs/analysis/`, `docs/sdk/` and `docs/COMPATIBILITY.md`: one line inserted above
  the symbol breaks them. The TOOL44 developer kept `tools/xtask/src/main.rs`'s line numbers for that reason; three
  such cites were re-pointed by hand (`50cc3f83a`). The owner rejected a manual pass over the rest.
- Touching a file today means translating its German text (CLAUDE.md *Language & Communication*); many of the files
  with edge anchors are German.

## Work

1. **`cargo xtask doc-check --fix-anchors [--staged | <paths>]`.** For every anchored cite `file:line@symbol` (also
   `file:a-b@symbol` and comma lists, as doc-check parses them): locate the symbol in the cited file (the index
   version under `--staged`, else the working tree) and rewrite **only the line number** (a range keeps its length)
   to the symbol's current line. Never change the symbol, the path, or any other byte of the doc. A symbol that is
   missing, or that occurs more than once with no unique nearest candidate, is reported and not rewritten. Cites
   without `@symbol` are reported only. EmuTOS/FreeMiNT cites (pinned) are never rewritten. Prints one line per
   rewrite (`<doc>:<line>: <old> -> <new>`) and a summary.
2. **Language-rule exception.** A commit whose only change to a file is line numbers rewritten by `--fix-anchors`
   does not count as touching that file: no translation required. The developer proposes the CLAUDE.md wording; the
   rule itself is recorded by the lead (owner decision).
3. **Hook.** When staged code changes shift `@symbol` anchors in docs the commit leaves alone, the pre-commit
   rejection message suggests `cargo xtask doc-check --fix-anchors <docs>`; a rejection stands only if the symbol no
   longer exists in the cited file (owner 2026-11-11). A shifted anchor whose symbol still exists is rejected too, and
   the message suggests `--fix-anchors` (owner 2026-11-11, option b: no stale line numbers land on `main`).
4. **One repo-wide run, scope first.** Before any rewrite, `--fix-anchors` in a dry-run mode (`--dry-run`) lists every
   file and cite it would change, grouped by tree (`TASK/`, `docs/api/`, `docs/audits/`, dated `docs/analysis/`,
   other docs); `measurements/**` and `docs/archive/` are never touched. The owner decides the scope (e.g. whether
   dated audits and analyses, which are historical, are included); then one commit per decided scope.
5. **Host tests** in a scratch repo under `target/`: a shifted symbol is rewritten to its line; a range keeps its
   length; a missing symbol and an ambiguous symbol are reported, not rewritten; a cite without `@` is reported only;
   `--staged` reads the index, not the working tree; a pinned-reference cite is left alone; the doc's other bytes are
   unchanged (byte compare).

## Acceptance

- Host tests as in point 5; a mutation that rewrites the symbol (or the path) instead of the line number makes a
  test fail (tester shows it).
- The dry run of point 4 reported to the owner before the repo-wide rewrite.
- `cargo xtask test` green, warnings delta 0, doc-check of the edited docs 0 failures.
