# TOOL40 — pre-commit hook runs the mapping check

Status: IMPLEMENTED (2026-11-11) | Priority: P1 | Commit: self | Missing: DONE through cargo xtask accept (TOOL41); tester PASS in measurements/measurement-2026-11-11-tool40-test/RUNINFO.TXT

Type: tool (enforcement). Owner decision 2026-11-10. Order: after the [TOOL39](TOOL39.md) acceptance, before the
ADR-037 ticket tools.

## Facts

- Commit `7bba77f6` added three lines to `docs/MEMORY.md` ahead of ADR-024, ADR-029, ADR-030 and ADR-031. 31 design
  rows of `docs/analysis/deps/mapping.tsv` then cited a line in the wrong ADR, and the host test
  `deps::tests::mapping_rows_exist_on_both_sides` failed. The pre-commit hook passed it: `doc-check --staged` does not
  read `mapping.tsv`'s `ADR-NNN@<file>:<line>` cites. Fixed in `f43f5a79`.
- The hook today: `tools/git-hooks/pre-commit` (TOOL29) runs `cargo xtask doc-check --staged` on the index, after a
  cheap `git diff --cached --quiet` pre-filter; only errors the commit introduces block.
- `cargo xtask deps --check-mapping` checks every row against `docs/analysis/deps/edges.tsv`, the NewTOS file and
  function of the row, and the ADR line (`tools/xtask/src/deps.rs`). It reads the working tree.
- `mapping.tsv` names 104 distinct NewTOS files (column `newtos_file`; first written as 105, counting the header).

## Work

1. **Trigger.** The hook runs the mapping check when the commit stages (additions, changes, deletions, renames)
   `docs/MEMORY.md`, `docs/analysis/deps/mapping.tsv`, `docs/analysis/deps/edges.tsv`, or a file named in
   `mapping.tsv`'s `newtos_file` column — the last only if it is cheap to detect (read the staged `mapping.tsv` once;
   no cargo run for a commit that touches none of them). Otherwise it skips the check without starting cargo.
2. **Staged content, not the working tree.** The check reads the index versions of every file it uses (the same
   principle as `doc-check --staged`); an unstaged working-tree fix must not let a broken commit through.
3. **New errors only.** A problem that the check also reports at `HEAD` does not block; a problem the commit
   introduces rejects it, with the check's own message lines. A check that cannot run rejects the commit, as
   doc-check does.
4. **One place for the logic.** Extend `cargo xtask deps --check-mapping` (e.g. a `--staged` mode) or call it from
   the hook; the hook stays a thin shell script. The comment block of `tools/git-hooks/pre-commit` documents the new
   part.
5. **Host tests** for the new mode (introduced vs legacy problem, staged vs working tree), run by `cargo xtask test`.
6. **CLAUDE.md:** the "Commit Discipline" paragraph on the hook names the mapping check. No "remember to run X after
   editing Y" rule is added for agents: the hook enforces it (owner 2026-11-10).

## Acceptance (in a throwaway clone, never in the working repo)

1. Insert lines into `docs/MEMORY.md` that shift ADR anchors cited by `mapping.tsv`, stage, commit → rejected,
   naming the rows.
2. A clean `docs/MEMORY.md` edit (no anchor cited by `mapping.tsv` moves) → passes.
3. A commit that touches none of the trigger files → check skipped (no cargo run for it).
4. Runtime of the hook for each of the three cases reported (wall clock).
- `cargo xtask test` green, warnings delta 0, doc-check of the edited docs 0 failures.

## Owner decision (2026-11-11)

Keep as built: a commit that edits a `mapping.tsv` row owns that row. Any error on it blocks, even one that already
exists at `HEAD` and even when only the `note` column changed. A row is never matched across a commit by its EmuTOS
side alone (that would let a changed NewTOS side hide an old error).

## Implementation

Developer, 2026-11-11, uncommitted in the working tree, base `8f6f66afe` (receipt:
[TOOL40-STATUS-UNFINISHED.MD](TOOL40-STATUS-UNFINISHED.MD)).

- **Check on a tree:** `check_mapping_in` (`tools/xtask/src/deps.rs:808@check_mapping_in`) is the former
  `check_mapping`, reading every file through a callback and returning `Problem`s (row line, row columns, message;
  `tools/xtask/src/deps.rs:773@Problem`). `check_mapping` keeps its signature and output for the working tree and the
  host test `mapping_rows_exist_on_both_sides`.
- **Staged mode:** `cargo xtask deps --check-mapping --staged` (`tools/xtask/src/deps.rs:650@cmd`) runs
  `staged_problems` (`tools/xtask/src/deps.rs:920@staged_problems`): the check once on the index and once on `HEAD`,
  both through doc-check's `Resolver` views (`Tree::Index`, `Tree::Rev`; new accessor `Resolver::tree_file`,
  `tools/xtask/src/doc_check.rs:1035@tree_file`): a file exists if the tree records it and is read from the tree
  where the working copy differs. A problem is legacy when `HEAD` reports the same message for a row with the same
  columns (a moved row keeps its problems; a row the commit edits is checked as new); all others reject. `edges.tsv`
  or `mapping.tsv` missing from the index or not parsing, or git failing, makes the check fail ("cannot run").
- **Hook:** `tools/git-hooks/pre-commit` keeps the doc-check pre-filter and adds a trigger in shell: `git diff
  --cached --name-only --no-renames HEAD` against `docs/MEMORY.md`, `mapping.tsv`, `edges.tsv` and the files the
  staged `mapping.tsv` names (`newtos_file`, the file of an `ADR-NNN@<file>:<line>` class, a `source:` path; one
  `git show` + `awk`). Without a hit the mapping check is not started; both checks run when triggered, either failure
  rejects. Comment block updated.
- **Host test:** `deps::tests::staged_reports_only_what_the_commit_introduces`
  (`tools/xtask/src/deps.rs:1135@staged_reports_only_what_the_commit_introduces`), in a scratch repo under `target/`
  (doc-check's `TempRepo`, now `pub(crate)`): legacy row not blocking; a MEMORY.md line insert reported; unstaged
  fix ignored and staged fix honoured; staged fn rename reported; moved legacy row stays legacy; staged deletion of
  the mapped file reported; mapping missing from the index is an error.
- **Docs:** `tools/deps/README.md` ("Checks"), `cargo xtask` usage text (`tools/xtask/src/main.rs`).

## Acceptance (tester, 2026-11-11)

PASS on all checks, including three mutants killed by the host test and the owner's row-ownership case (5b):
`measurements/measurement-2026-11-11-tool40-test/RUNINFO.TXT`. Not closed: under the ADR-037 addendum only the tester
moves a ticket to `DONE`, through `cargo xtask accept`, which TOOL41 builds.
