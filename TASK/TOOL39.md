# TOOL39 — cflow is a required, pinned host tool

Status: DONE (2026-11-11) | Priority: P1 | Commit: b7bd932a | Receipt: TOOL39-STATUS-DONE.MD

Type: tool (reproducibility). Owner decision 2026-11-10: "cflow mandatory", before the ticket tools of ADR-037.
Split out of [TOOL01](TOOL01.md), which stays parked for everything else.

## Facts

- `tools/deps/gen-deps.py` is the only program that runs cflow (before this ticket a module
  constant, `CFLOW = "/opt/homebrew/bin/cflow"`, line 44 at `9ff4e188c`). `cargo xtask deps` only reads `docs/analysis/deps/edges.tsv`.
- Without cflow the generator died with a Python traceback (`FileNotFoundError: … '/opt/homebrew/bin/cflow'`,
  2026-11-10). Its version is printed into the headers of `edges.tsv` and `layers.md`, but never compared.
- Installed 2026-11-10 with `brew install cflow`: `/opt/homebrew/bin/cflow (GNU cflow) 1.8`, the version the
  committed `edges.tsv` header names.
- With it, `tools/deps/gen-deps.py --check` against pin `1a7484ab` reports `differs: edges.tsv, layers.md`; the
  only difference is the cross-compiler line (`MiNT 20230908` committed, `MiNT 20240501` installed). That is
  TOOL01's subject (cross toolchain pin), not this ticket's; `edges.tsv` is not regenerated here.

## Work

1. **Pin** cflow in `tools/oracle/oracle.env` (the one file for pins, `docs/analysis/ORACLE.md` §1): path and
   version (`1.8`, parsed from the first line of `cflow --version`).
2. **`tools/deps/gen-deps.py` aborts before any other work** if the pinned cflow is missing or its version differs:
   one clear line naming the tool, expected and found, and `brew install cflow` as the fix; non-zero exit. No
   silent skip, no fallback path, no environment override. Both modes (generate and `--check`).
3. **Record the version like `host_clang`:** a `host_cflow` line in the manifest written by `tools/sd-manifest.sh`
   (format as the `host_clang` line, `not found` when absent).
4. **Host test** of the version check (missing tool, wrong version, right version) without requiring the real
   cflow, run by `cargo xtask test`.
5. **Docs:** `tools/deps/README.md` ("Inputs and reproducibility") states the pin and the abort. The lead adds the
   tool to CLAUDE.md ("Required host tools").

## Acceptance

- With cflow 1.8 installed: `tools/deps/gen-deps.py --check` runs to its comparison (today: the cross-compiler
  line only).
- With the pin changed to another version, or the path to a missing file: exit != 0, the one-line message, no
  traceback, nothing generated.
- A manifest from `tools/sd-manifest.sh` carries `host_cflow`.
- `cargo xtask test` green, warnings delta 0, doc-check of the edited docs 0 failures.

## Implementation

Developer, 2026-11-10/11, uncommitted in the working tree (receipt: [TOOL39-STATUS-DONE.MD](TOOL39-STATUS-DONE.MD)).

- **Pin:** `tools/oracle/oracle.env` gains `ORACLE_CFLOW=/opt/homebrew/bin/cflow` and `ORACLE_CFLOW_VERSION=1.8`;
  its header names the two new readers (`tools/sd-manifest.sh`, `tools/deps/gen-deps.py`).
- **Check:** `check_cflow` (`tools/deps/gen-deps.py:2489@check_cflow`) runs in `main`
  (`tools/deps/gen-deps.py:2520@main`) right after reading `oracle.env`, before the build-tree check and before any
  temporary directory, in both modes. It stops with exit 1 and one line
  `gen-deps: cflow: expected GNU cflow <pin> at <path>, found <what>; fix: brew install cflow`, where `<what>` is
  `no file there`, `not runnable (<errno text>)`, `` `--version` exit <n> ``, `no GNU cflow version line (<line>)`
  or `version <v>`. The module constant `CFLOW` is gone: `cflow_edges` gets the checked path, the `cflow:` header
  line takes the path from `ORACLE_CFLOW` and the version line from the check (same bytes as before).
- **Manifest:** `tools/sd-manifest.sh` writes `host_cflow <first line of --version>` of `ORACLE_CFLOW` (read from
  `oracle.env` in a subshell) after `host_clang`, or `host_cflow not found`. Existing readers filter by key
  (`tools/xtask/src/tosdump.rs:888@manifest_lines`: `image_sha256`, `optional_missing`) and are unaffected.
- **Host test:** `tools/xtask/src/gen_deps_tests.rs` (5 tests, run by `cargo xtask test`) loads `gen-deps.py` with
  `python3 -I` as a module and calls `check_cflow` against fake `cflow` scripts in
  `target/xtask-gen-deps/`: missing, version 1.7, not GNU cflow, not executable (each: exit 1, exactly the one
  line), version 1.8 (passes).
- **Docs:** `tools/deps/README.md` ("Inputs and reproducibility"), `docs/sdk/06-image-and-testing.md` (manifest
  format: `host_cflow`).

## Acceptance (tester, 2026-11-11)

PASS on all seven checks, `measurements/measurement-2026-11-11-tool39-test/RUNINFO.TXT`. Closed by hand (no `cargo xtask close` yet, ADR-037).
