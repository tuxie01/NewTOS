# SPEC05 — Decide the four `DEVIATIONS.md` rows with no live ticket

Status: OPEN (2026-11-11) | Priority: P3

Type: decision (no code). From the inventory snapshot docs/analysis/inventory-2026-11-11.md, gap G14 (T12).

## Facts

- Rows whose status says "no ticket": `docs/DEVIATIONS.md:25` (AES drawing bypasses the VDI trap; V35 DONE), `:33` (`vex_butv/motv/timv` before a `v_opnwk`; V53 DONE), `:34` (pointer frozen during one long system call; V55 DONE), `:57` (`Fcreate` with a name longer than 8.3 refused where EmuTOS truncates; PORT12).
- Row 53 (`Fclose` write-back failure) names "user decision open" under HW08 (DONE).

## Work

1. Owner decides per row: sanction in `docs/COMPATIBILITY.md` (named, dated) or open a fix ticket.

## Acceptance

- Each row has a sanction entry or a live ticket in its Status column.
