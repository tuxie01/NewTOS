# TOOL61 — GEMDOS console, aux and printer calls: oracle case table

Status: OPEN (2026-11-11) | Priority: P3

Type: measurement. From the inventory snapshot docs/analysis/inventory-2026-11-11.md, gap G09 (T06).

## Facts

- 15 GEMDOS entries have no case in any table: `Cconin`, `Cconout`, `Cauxin`, `Cauxout`, `Cprnout`, `Crawio`, `Crawcin`, `Cnecin`, `Cconws`, `Cconrs`, `Cconis`, `Cconos`, `Cprnos`, `Cauxis`, `Cauxos` (rows in `docs/analysis/inventory-2026-11-11.md` section 2.3).
- `Cconin`, `Cnecin`, `Crawcin` block on input. The existing GEMDOS cases are only `gemdos-mc-*`, `super-r1` (15 entries).

## Work

1. Step 1: decide which of the 15 run unattended; scripted keystrokes in Hatari, or exclude with the reason as `tosdump/cases/COVERAGE-BIOS.md` does.
2. Write the table (`1:0xNN` lines), run on the oracle, compare on NewTOS.

## Acceptance

- Table, oracle dump under `tosdump/reference/`, tosdiff result recorded; excluded calls listed with reason.
