# TOOL48 — xtask range mode: new-errors-only checks over a pushed range

Status: OPEN (2026-11-11) | Priority: P2

Type: tool. Owner decision 2026-11-11, from the [TOOL43](TOOL43.md) report (`docs/analysis/ci-backstop-2026-11-11.md`).
The TOOL43 workflow uses it; the `git reset --soft` emulation of the report is not to be used.

## Facts

- The new-errors-only checks run on the index against `HEAD`: `cargo xtask doc-check --staged` (TOOL29),
  `cargo xtask deps --check-mapping --staged` (TOOL40), `cargo xtask status-check --staged --new-only` (TOOL41).
  `doc-check --base <rev>` checks the anchor rule for lines new since `<rev>` but does not restrict the cite check to a
  range (report).
- A push to `main` carries a range `<before>..<after>` (GitHub's `github.event.before` / `github.sha`); the report
  emulated it by `git reset --soft <before>` and the staged modes, unproven for `doc-check --staged <files>`.

## Work

1. `--range <from>..<to>` for the three checks: compare the tree of `<to>` with the tree of `<from>` the way the
   staged modes compare the index with `HEAD`; only problems `<to>` introduces against `<from>` fail, with the same
   messages and the same legacy rules (doc-check: lines unchanged since `<from>`; mapping: row ownership as TOOL40;
   status-check: the tickets the range touches or names, the `fix`/`feat` subject rule applied to **every commit** in
   the range, ids from the subject only, ADR-037 point 13). Reads only git objects, never the working tree.
2. Edge cases, each defined and tested: `<from>` all zeros (new branch: GitHub sends `0000…`) — check against the
   parent of the first new commit, or say why another base; `<from>` not an ancestor of `<to>` (force push): fail with
   a clear message, never pass silently; empty range: pass.
3. Shared machinery with the staged modes (doc-check's `Resolver` views, `Tree::Rev`), not a second implementation.
4. Host tests in a scratch repo under `target/`: introduced vs legacy per check, a range of several commits, the three
   edge cases, and that the working tree is never read.
5. `cargo xtask` usage text and `tools/deps/README.md`; CLAUDE.md line proposed in the receipt.

## Acceptance

- On a range of this repository with a known introduced error (made in a throwaway clone), each check fails naming it;
  on the same range without it, all pass; a 40-commit range of `main` passes.
- `cargo xtask test` green, warnings delta 0, doc-check of the edited docs 0 failures.
