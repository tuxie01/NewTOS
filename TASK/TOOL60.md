# TOOL60 — `mapping.tsv`: classify 49 unmapped `aes/`, `bdos/`, `vdi/`, Line-A rows

Status: PARKED (2026-11-11) | Priority: P3 | Reason: table maintenance, same as TOOL22; needs an owner decision per cluster (ADR versus gap) | Reopen: TOOL22 is taken up

Type: doc (table maintenance). From the inventory snapshot docs/analysis/inventory-2026-11-11.md, gap G05 (T03).

## Facts

- 49 rows are `unmapped`: 42 `aes/` (EVB list, key queue and process 14; `gsx_*` form, clip and workstation helpers 14; window lists 7; misc 7: `aestrap_intercepted`, `drawrat`, `eralert`, `load_mouse_cursors`, `setup_mouse_cursors`, `sh_path`, `xlate_obj_array`), 4 `bdos/` (`bdos_trap2`, `get_default_handle`, `log2ul`, `namlen`), `ok_for_direct_blit`, `set_screen_shift`, `inline_strcpy` (`docs/analysis/deps/mapping.tsv:1156-2000` for `aes/`).
- Notes say "no counterpart" with reasons, several "no ADR". ADR-029 (`docs/MEMORY.md:2749`) is the cooperative-kernel decision; no ADR names the EVB list or the key queue.
- Already covered elsewhere: `bdos_trap2` by PROC07, `rect_change` by A136, loadable cursors by GUI03, translation by I18N01; `sh_path` is A149.

## Work

1. Per cluster, owner decides: new ADR or amended ADR-029 evidence, or a gap ticket.
2. Record `design:` rows in `mapping.tsv`.

## Acceptance

- No unmapped row of the 49 without a decision; `cargo xtask test` green.
