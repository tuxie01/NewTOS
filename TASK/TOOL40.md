# TOOL40 — pre-commit hook runs the mapping check

Status: OPEN (2026-11-10) | Priority: P1

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
- `mapping.tsv` names 105 distinct NewTOS files (column `newtos_file`).

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
