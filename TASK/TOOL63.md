# TOOL63 — AES case tables for trees, menus, forms, resources, shell, scrap and events (40 entries)

Status: OPEN (2026-11-11) | Priority: P2

Type: measurement (dumper change first). From the inventory snapshot docs/analysis/inventory-2026-11-11.md, gap G10 (T08).

## Facts

- 40 of 78 AES API nodes have no case: `appl_exit`, `appl_read/write`, `evnt_*`, `menu_*`, `objc_draw/find/offset/change`, `form_*`, `graf_dragbox/rubberbox/slidebox/watchbox`, `fsel_*`, `rsrc_load/obfix`, `scrp_*`, `shel_*`. 151 of 290 mapped `aes/` internals are unreachable from any measured entry (inventory section 1.5).
- `measurements/measurement-2026-09-12-aes-r4/NOT-RUN.TXT` names the blockers: ALERT (modal alert hangs), GUARD, PTR (`addr_in` needs a fixture).

## Work

1. Step 1: decide the fixture mechanism (a case column naming a tree, as `vdi-raster.tsv` names a buffer); dumper change.
2. Then the tables, oracle run, NewTOS comparison.

## Acceptance

- The 40 entries have oracle cases or a recorded exclusion reason.
