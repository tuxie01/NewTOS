# TOOL39 — receipt DONE (2026-11-11)

Commit `b7bd932a` (implementation, host tests, docs, CLAUDE.md "Required host tools", ORACLE.md section 1 cflow row,
tester run). Independent tester PASS on all seven checks (`measurements/measurement-2026-11-11-tool39-test/RUNINFO.TXT`):
`cargo xtask test` 2374/0/6, a mutant `check_cflow` fails all five new tests, real cflow 1.8 -> `--check` byte-identical,
pin 1.7 and a missing path -> one line and exit 1 in both modes, `host_cflow` in the manifest, warnings delta 0, doc-check 0.
Closed by hand by the lead: `cargo xtask close` (ADR-037 point 5) does not exist yet.
Out of scope, no ticket: `tools/sync-card.sh`, `tools/release/package-release.sh` and `tosdump/probes/run-xdprobe.sh`
write their own `host_clang` lines without `host_cflow`.

The developer's receipt (UNFINISHED, 2026-11-11) follows, unchanged except its title is now a second-level heading.

## TOOL39 — receipt (unfinished: tester and commit pending)

Developer, 2026-11-11. The change is in the working tree, not committed. Base: `f43f5a79b`.

## Done

- cflow pinned in `tools/oracle/oracle.env` (`ORACLE_CFLOW`, `ORACLE_CFLOW_VERSION=1.8`).
- `tools/deps/gen-deps.py` checks the pinned cflow before any other work, in both modes, and stops with one line
  (tool, expected version and path, found, `fix: brew install cflow`), exit 1, no traceback, no fallback, no
  override (`tools/deps/gen-deps.py:2489@check_cflow`).
- `tools/sd-manifest.sh` writes a `host_cflow` line (or `host_cflow not found`).
- Host test `tools/xtask/src/gen_deps_tests.rs` (missing, wrong version, not GNU cflow, not executable, right
  version) with fake executables, no real cflow needed.
- Docs: `tools/deps/README.md`, `docs/sdk/06-image-and-testing.md`.

Files: `tools/oracle/oracle.env`, `tools/deps/gen-deps.py`, `tools/sd-manifest.sh`, `tools/xtask/src/main.rs`,
`tools/xtask/src/gen_deps_tests.rs` (new), `tools/deps/README.md`, `docs/sdk/06-image-and-testing.md`,
`TASK/TOOL39.md` (section "Implementation").

## Checks run

- `cargo xtask test`: 2374 passed / 0 failed / 6 ignored (base 2369/0/6; +5 new tests).
- Warnings, base and change, both boards: `cargo xtask build` rustc tos-api 64 / kernel 93; `cargo clippy -p kernel
  --no-default-features --features board-<b>` tos-api 73 / kernel 121; `cargo clippy -p xtask --tests` 9. Delta 0.
- Acceptance, cflow 1.8 installed:
  1. `python3 -I tools/deps/gen-deps.py --check` reaches its comparison: `edges.tsv, layers.md byte-identical`,
     exit 0 (after `f5567131a` moved the cross-compiler line; before it, the same run reported the cc-line diff).
  2. `ORACLE_CFLOW_VERSION=1.7`: generate and `--check` both print
     `gen-deps: cflow: expected GNU cflow 1.7 at /opt/homebrew/bin/cflow, found version 1.8; fix: brew install cflow`,
     exit 1, no traceback, nothing generated (no temporary directory created, outputs unchanged).
  3. `ORACLE_CFLOW=/opt/homebrew/bin/cflow-missing`: both modes print
     `gen-deps: cflow: expected GNU cflow 1.8 at /opt/homebrew/bin/cflow-missing, found no file there; fix: brew install cflow`,
     exit 1. `oracle.env` restored after both.
- `tools/sd-manifest.sh files` + `write` on a scratch image: `host_cflow /opt/homebrew/bin/cflow (GNU cflow) 1.8`;
  with a copy of the script beside an `oracle.env` naming a missing path: `host_cflow not found`.
- `cargo xtask doc-check` of the edited docs and of `TASK/SPEC04.md` (the other doc with anchored cites into
  `gen-deps.py`): 0 failures. `check_cflow` stands just before `main` and the test module at the end of
  `tools/xtask/src/main.rs`, so existing line cites into both files keep their anchors.

## Remaining

- Tester: independent run of the acceptance above and of `cargo xtask test`.
- Lead: CLAUDE.md "Required host tools", commit, status line.
