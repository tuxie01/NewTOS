# TOOL19 — `DISKUNIT.TXT`: content in RUNINFO, refusal on a missing or wrong unit

Status: DONE (2026-11-11) | Priority: P2 | Commit: 6f38e9b9f | Receipt: TOOL19-STATUS-DONE.MD

Migrated (owner batch 5, 2026-11-11), evidence: receipt TASK/TOOL19-STATUS-DONE.MD exists, commit 6f38e9b9f, TASK/TOOL19-STATUS-DONE.MD, commit 6f38e9b9f (feat(tosdump): profile values from one definition, recorded, no defaul), fix/feat commits naming the id: 6f38e9b9f (n=1), the receipt names no hash for this commit, the Migrated line names it

Line 3 before the migration:
Status: **ERLEDIGT** 2026-10-29 ([Quittung](TOOL19-STATUS-DONE.MD)) · Prioritaet: **P2** · Typ: Developer (Werkzeug) — Anwenderentscheidung 2026-10-28, aus [TOOL07](TOOL07.md)

## Facts

- TOOL07 resolves the unit token `u:0` through `D:\DISKUNIT.TXT`, which the
  profile definition writes (since TOOL19 from `tosdump/profiles.tsv`:
  `tools/hatari-oracle.sh:237@DISKUNIT`: 18;
  `tools/xtask/src/tosdump.rs:1276@DISKUNIT`: 26). DUMPBIOS does not run a table
  with `u:0` when the file is missing (`tosdump/core/dumpbios.c:51@unit_path`).
- User 2026-10-28: acceptable **if** (a) the file is always generated from the
  profile definition, never by hand; (b) its content is recorded in RUNINFO;
  (c) the runner refuses when it is missing or does not match the profile — no
  default value.

## Work

1. Check and show that (a) holds (no handwritten copy in
   `measurements/` methods or tables).
2. (b): the unit is recorded in the RUNINFO or manifest of both sides (oracle
   script and `cargo xtask tosdump`) and in the dumper's LOG.
3. (c): `cargo xtask tosdump` and `tools/hatari-oracle.sh` refuse a table
   with `u:0` when the file is missing or its value does not match the
   profile's unit; tosdiff compares only runs whose recorded
   unit matches the respective profile. No default value.
4. Tests for the refusals.

## Acceptance

- Refusals measured (missing, wrong value); host tests green
  (`cargo xtask test`, `make -C tosdump host-test`); doc-check 0 failures.
