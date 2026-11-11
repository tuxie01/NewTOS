# TOOL59 — `mapping.tsv`: give the 59 unmapped `bios/` helper rows a `design:` evidence or a gap decision

Status: PARKED (2026-11-11) | Priority: P3 | Reason: table maintenance, same as TOOL22 (owner pre-start check 2026-10-31); proposal for the owner | Reopen: TOOL22 is taken up (do both in one pass)

Type: doc (table maintenance). From the inventory snapshot docs/analysis/inventory-2026-11-11.md, gap G03 (T01).

## Facts

- 59 `bios/` rows are `unmapped` in `docs/analysis/deps/mapping.tsv` (rows `bios/` + `unmapped`, lines 295-1106), outside the XHDI/SCSI rows (ADR-032) and the partition layer (BIOS18): `bios.c` 6, `initinfo.c` 10, `kprint.c` 7, `screen.c` 5, `serport.c` 5, `conout.c` 4, `country.c` 4, `ikbd.c` 3, `processor.S` 3, `delay.c` 2, `biosmem.c` 2, `startup.S` 2, `vectors.S` 2, `delayasm.S`, `panicasm.S`, `font.c`, `vt52.c` 1 each.
- Every row note already says "no NewTOS counterpart" and why; none carries a `design:` evidence, so `cargo xtask deps --coverage` counts `bios/` 16% unmapped.
- TOOL22 covers groups B, C, F only.

## Work

1. Per file cluster: choose `design:<evidence>` (ADR, `hardware:`, `source:`) or record a real gap and open a ticket.
2. Owner decision where no ADR exists.

## Acceptance

- `cargo xtask deps --coverage` shows no unmapped row of these 59 without a decision; `cargo xtask test` green (mapping test).
