# TOOL65 — `mapping.tsv` / `deps --coverage` scope: `desk/`, `include/`, `ext` (441 nodes)

Status: OPEN (2026-11-11) | Priority: P3

Type: tooling decision. From the inventory snapshot docs/analysis/inventory-2026-11-11.md, gap G16 (T14).

## Facts

- `desk/` 325, `include/` 58 and `ext` 58 nodes have no `mapping.tsv` rows by construction, so `cargo xtask deps --coverage` shows `desk` and `include` 100% unmapped.
- The desktop is an EL0 program (ADR-026, `docs/MEMORY.md:2433`); 31 `desk/` rows are named by live tickets.

## Work

1. Owner decides the class for `desk/` (for example `design:ADR-026`), the inline trap bindings of `include/` and `ext`.
2. Extend `tools/deps/gen-deps.py` or the mapping rule accordingly.

## Acceptance

- `deps --coverage` reports no unclassified node outside a stated scope.
