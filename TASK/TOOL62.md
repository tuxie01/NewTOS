# TOOL62 — GEMDOS file, directory, time, memory, process and version calls: oracle case tables (23)

Status: OPEN (2026-11-11) | Priority: P2

Type: measurement. From the inventory snapshot docs/analysis/inventory-2026-11-11.md, gap G09 (T07).

## Facts

- 23 GEMDOS entries have no case: file (`Fopen`, `Fread`, `Fseek`, `Fattrib`, `Fdup`, `Fforce`, `Fdatime`, `Frename`), directory (`Dcreate`, `Ddelete`), time (4), memory (`Malloc`, `Mfree`, `Mxalloc`, `Mshrink`, `Maddalt`), process (`Pterm0`, `Pterm`, `Ptermres`), `Sversion`.
- DUMPBIOS can issue `1:0xNN` calls, as `tosdump/cases/gemdos-mc-fs20.tsv` does, so no new dumper is needed. TOOL03 (DUMPGDOS) is PARKED.

## Work

1. Step 1: a Hatari run with the oracle ROM (listed first).
2. Write tables, run both sides, record differences as tickets.

## Acceptance

- Each of the 23 appears in a table with an oracle dump; differences ticketed.
