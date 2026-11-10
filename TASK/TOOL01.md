# TOOL01 — `cargo xtask doctor`: check host tools against pinned expectations

Status: PARKED (2026-10-31) | Priority: P2 | Reason: owner, pre-start check: blocks no acceptance | Reopen: a host tool blocks an acceptance or a run is not reproducible because of one

Type: tool (reproducibility) — owner decision 2026-10-27. The cflow part was split out on 2026-11-10 (owner:
"cflow mandatory") into [TOOL39](TOOL39.md); the rest stays parked. Translated to English 2026-11-10 (CLAUDE.md
language rule); the original German text is in git history.

## Facts

Whether a measurement run or an image is reproducible depends on host tools that nobody checks beforehand
today:

- zView `SAMPLES` was missing from the image because `gen-samples.py` ran without Pillow
  (`sdk/zview/build-zview.sh` calls `sdk/zview/samples/gen-samples.py`); the image reported it only as
  `optional_missing` (`tools/make-sd-image.sh`).
- Pillow was put into the Homebrew Python 3.15 with `pip install --user` on 2026-10-27
  (`~/Library/Python/3.15`); `brew install pillow` could only be linked for 3.14. A Homebrew Python upgrade
  silently throws it away.
- `ld.lld`, `llvm-nm` and `gtimeout` are not on `PATH` (as of 2026-10-27); the SDK scripts each find their tools
  in their own way.
- The oracle pin (`tools/oracle/oracle.env`) names Hatari, the cross compiler and the ROM with sha256; it is
  checked only by `tools/hatari-oracle.sh` during a run.
- 2026-11-10: the installed cross compiler reports `m68k-atari-mint-gcc (MiNT 20240501) 7.5.0`, while
  `docs/analysis/ORACLE.md` §1, `tools/oracle/oracle.env` and the committed `docs/analysis/deps/edges.tsv`
  name `MiNT 20230908` (found by `tools/deps/gen-deps.py --check`, [TOOL39](TOOL39.md)). Settled the same day:
  the installed toolchain rebuilds the pinned ROM byte-identical (`measurements/measurement-2026-11-10-tool01/RUNINFO.TXT`);
  `MiNT 20240501` is recorded as verified in `docs/analysis/ORACLE.md` §1 and `tools/oracle/oracle.env`, and the
  header line of `docs/analysis/deps/edges.tsv` and `layers.md` follows it (owner decision 2026-11-10).

## Work

1. **`cargo xtask doctor`** checks presence and version against pinned expectations, paths from
   `tools/oracle/oracle.env` where possible:
   - Hatari 2.6.1 (`ORACLE_HATARI`);
   - `m68k-atari-mint-gcc` 7.5.0 (`ORACLE_CROSS_MINT`);
   - `clang`, `ld.lld`, `llvm-nm` — resolved the way the build scripts resolve them (read the scripts, do not
     guess), version pinned;
   - QEMU (`qemu-system-aarch64`), `gtimeout`, `fsck_msdos` (cflow: [TOOL39](TOOL39.md));
   - pinned ROM present and sha256 == `ORACLE_ROM_SHA256`.
   Where no pinned version exists today: record the measured version as the expectation (one place, like
   `oracle.env`) and say so in the report.
2. **Python tools in a project venv** instead of the user site: e.g. `tools/.venv` (gitignored) plus a
   requirements.txt under `tools/` with Pillow (version pinned). `doctor` checks the venv and the packages
   against that file; the command to create it is in the error message. `tools/make-sd-image.sh` and the Python
   steps it calls (`mk-hello-ncp.py`, `sdk/zview/build-zview.sh` → `gen-samples.py`, the embedded Python at the
   end) run with this venv.
3. **Exit 0 only if everything matches**; each deviation as one line with tool, expected, found.
4. **Callers:** `tools/make-sd-image.sh` calls `doctor` before building and aborts on exit != 0; the Auditor
   calls it as a pre-flight (`.claude/agents/auditor.md`). If a caller needs only a part (the image needs no
   Hatari), a partial check by option — but no silent skipping.
5. Host test for the comparison logic (version parsing, sha256, missing tool), without requiring the real
   tools.
6. Docs: `docs/sdk/06-image-and-testing.md` (build prerequisites) and `docs/analysis/ORACLE.md` (pre-flight)
   point to `doctor`.

## Acceptance

- `cargo xtask doctor` exit 0 on this host; with a deliberately wrong ROM sha256 or a missing venv exit != 0
  with the cause named.
- `tools/make-sd-image.sh` builds with the venv, zView `SAMPLES` is in the image, `optional_missing` in the
  manifest reads `none`.
- `cargo xtask test` green, doc-check of the changed docs 0 failures.
