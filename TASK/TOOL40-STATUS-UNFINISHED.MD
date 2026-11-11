# TOOL40 — receipt (unfinished: tester, CLAUDE.md and commit pending)

Role: developer
Agent: agent-a26d8df52f9c23927
(Role and Agent added by lead from session log, 2026-11-11.)

Developer, 2026-11-11. The change is in the working tree, not committed. Base: `8f6f66afe`.

## Done

- `cargo xtask deps --check-mapping --staged`: the mapping check on the index, only problems `HEAD` lacks reject
  (`tools/xtask/src/deps.rs:920@staged_problems`). It reads `edges.tsv`, `mapping.tsv`, the cited ADR files,
  `source:` paths and NewTOS files through doc-check's `Resolver` index / `HEAD` views
  (`tools/xtask/src/doc_check.rs:1035@tree_file`), the same machinery as `doc-check --staged`. Problems are
  compared by row columns + message, not by line, so rows moved by an insert in `mapping.tsv` keep their legacy
  problems. A check that cannot run (mapping or edges missing from the index or unparsable, git failing) rejects.
- `check_mapping` split into `check_mapping_in` (any tree, structured `Problem`s) and the unchanged working-tree
  wrapper; `--check-mapping` output and the host test are byte-for-byte the same messages.
- `tools/git-hooks/pre-commit`: second check, triggered in shell by a staged `docs/MEMORY.md`, `mapping.tsv`,
  `edges.tsv` or a file the staged `mapping.tsv` names (newtos_file, ADR file, `source:` path), deletions and both
  sides of renames included; otherwise no cargo start for it. Comment block documents both checks.
- Host test `deps::tests::staged_reports_only_what_the_commit_introduces`
  (`tools/xtask/src/deps.rs:1135@staged_reports_only_what_the_commit_introduces`).
- Docs: `tools/deps/README.md` ("Checks", command list), usage text in `tools/xtask/src/main.rs` (same line count,
  so the anchored cites into `main.rs` in `TASK/MEM12.md` and `docs/sdk/06-image-and-testing.md` do not move).

Files: `tools/xtask/src/deps.rs`, `tools/xtask/src/doc_check.rs` (`Resolver::tree_file`, test `TempRepo`
`pub(crate)`), `tools/xtask/src/main.rs`, `tools/git-hooks/pre-commit`, `tools/deps/README.md`, `TASK/TOOL40.md`
(section "Implementation"), this receipt.

## Checks run

- `cargo xtask test`: 2375 passed / 0 failed / 6 ignored (base 2374/0/6; +1 new test). Wall clock 1:50.
- Warnings, base and change identical: `cargo xtask build --board rpi4` and `--board qemu-virt` rustc tos-api 64 /
  kernel 93; `cargo clippy -p kernel --features board-rpi4` and `board-qemu-virt` tos-api 73 / kernel 121;
  `cargo clippy -p xtask --tests` 9 (none in `deps.rs` / `doc_check.rs`). Delta 0.
- `cargo xtask doc-check tools/deps/README.md TASK/TOOL40.md TASK/TOOL40-STATUS-UNFINISHED.MD`: 0 failures.
- `cargo xtask deps --check-mapping --staged` in the working repo (nothing staged): 0 new, 0 legacy.

## Acceptance (throwaway clone)

`git clone` of the repo into the session scratchpad, the five changed code/doc files copied in and committed there
through the new hook (doc-check ran, mapping check skipped: no trigger file), hook installed with `cargo xtask
install-hooks`. xtask prebuilt, so the times are the hook alone. A `cargo` wrapper first in `PATH` logged every
cargo call the hook made. The clone has no `references/`: the mapping check does not need it (it reads only
`edges.tsv`, `mapping.tsv`, `docs/MEMORY.md` and NewTOS sources); doc-check skips its reference cites as usual.

| # | Commit | Result | cargo calls | Wall clock |
|---|---|---|---|---|
| 1 | 3 lines inserted in `docs/MEMORY.md` before `## ADR-024` | rejected: 31 rows, e.g. `docs/analysis/deps/mapping.tsv:1968: docs/MEMORY.md:2749 is in ADR-028, not in ADR-029`; doc-check 0 failures (the original gap) | doc-check, mapping | 2.83 s |
| 1b | same staged, working copy restored (unstaged fix) | rejected, 31 rows | doc-check, mapping | 2.84 s |
| 2 | one line appended at the end of `docs/MEMORY.md` | passes (0 new, 0 legacy) | doc-check, mapping | 2.65 s |
| 3a | `README.md` (root) only | passes | none | 0.17 s |
| 3b | comment in `tos-api/src/aes/trace.rs` (not in `mapping.tsv`) | passes | doc-check only | 1.30 s |
| 4 | `fn dispatch_inner` renamed in staged `tos-api/src/gemdos/mod.rs`, working copy original | rejected: rows 36, 39 `has no fn dispatch_inner` (doc-check also: 3 anchors) | doc-check, mapping | 2.31 s |
| 4b | `fn appl_find` renamed in staged `tos-api/src/aes/appl.rs` (no doc anchor on it) | rejected only by the mapping check: row 1924 | doc-check, mapping | 2.25 s |

The mapping check adds about 1 s to a commit that triggers it (3b vs 2).

## Proposed CLAUDE.md text (for the lead; "Commit Discipline", the hook paragraph)

Replace the bullet that starts "A checked-in pre-commit hook" with:

```
- A checked-in pre-commit hook (`tools/git-hooks/pre-commit`, installed once
  with `cargo xtask install-hooks`, which sets `core.hooksPath`) runs two
  checks on what the commit records (the index, not the working tree):
  `cargo xtask doc-check --staged` on the staged docs, and for a commit that
  stages code, on every doc citing the changed files (TASK/TOOL29.md); and
  `cargo xtask deps --check-mapping --staged` on every row of
  `docs/analysis/deps/mapping.tsv` against the staged `edges.tsv`, NewTOS
  files and ADR lines, when the commit stages `docs/MEMORY.md`,
  `mapping.tsv`, `edges.tsv` or a file `mapping.tsv` names (TASK/TOOL40.md).
  Each rejects the commit only for errors the commit introduces; the legacy
  backlog (TASK/SPEC03.md) never blocks. Agents must
  not bypass it: `tools/hooks/no-verify-guard.sh` blocks the bypass flags and
  any `core.hooksPath` other than `tools/git-hooks`.
```

## Open

- Tester: independent run of the acceptance in a throwaway clone (cases 1-3 + timing; 4/4b optional), host test
  mutant (e.g. `staged_problems` comparing by line or reading the working tree must fail the new test).
- Lead: apply the CLAUDE.md text, commit, `docs/STATUS.md` entry.
- Semantics to confirm: a commit that edits a row which already had a legacy problem (any column, the note
  included) gets that problem reported as new. Chosen deliberately (touching a row = owning it, like a new line in
  doc-check); the alternative key (emutos_fn + emutos_file only) would also let a changed `newtos_fn` keep an old
  problem silent.
