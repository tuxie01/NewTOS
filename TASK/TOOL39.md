# TOOL39 — cflow is a required, pinned host tool

Status: OPEN (2026-11-10) | Priority: P1

Type: tool (reproducibility). Owner decision 2026-11-10: "cflow mandatory", before the ticket tools of ADR-037.
Split out of [TOOL01](TOOL01.md), which stays parked for everything else.

## Facts

- `tools/deps/gen-deps.py` is the only program that runs cflow (`CFLOW = "/opt/homebrew/bin/cflow"`,
  `tools/deps/gen-deps.py:44@CFLOW`). `cargo xtask deps` only reads `docs/analysis/deps/edges.tsv`.
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
