# SPEC06 — `DEVIATIONS.md`: index the sanctioned deviations of `COMPATIBILITY.md`

Status: OPEN (2026-11-11) | Priority: P3

Type: doc (mechanical). From the inventory snapshot docs/analysis/inventory-2026-11-11.md, gap G15 (T13).

## Facts

- `docs/DEVIATIONS.md:15` says the list is incomplete; it has 28 rows against 69 title paragraphs in `docs/COMPATIBILITY.md:66-1852` (90 bold-led, 69 after dropping continuation labels).
- Two deviations of `docs/MEASUREMENT-COVERAGE.md` are in neither file: `v_get_pixel` after `v_clrwk` (0xFFDF versus 0xFFFF, parked 2026-09-06) and `D-VDI-25` (`vro_cpyfm` oversized destination; `docs/api/vdi/_discrepancies.md:44`).

## Work

1. Add a row per sanctioned deviation (trigger, both sides, basis, status link).
2. Add the two missing ones.

## Acceptance

- Every paragraph title of the section has a row or a stated reason; `cargo xtask doc-check docs/DEVIATIONS.md` clean.
