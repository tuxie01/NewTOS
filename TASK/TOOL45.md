# TOOL45 — cross-check: graph, mapping, tickets and COMPATIBILITY agree; coverage computed

Status: OPEN (2026-11-11) | Priority: P2

Type: tool (consistency). Owner decisions 2026-11-10/11. Starts after [TOOL41](TOOL41.md) (needs the line-3 parser
and `status-check`).

## Work

1. **Graph ↔ mapping ↔ tickets.** `cargo xtask status-check` cross-checks `docs/analysis/deps/edges.tsv`,
   `docs/analysis/deps/mapping.tsv` and the ticket status lines: every EmuTOS function of `edges.tsv` has a mapping
   row (already checked by `deps --check-mapping`), and every ticket a mapping row or note names exists and is in a
   state that agrees with the row (e.g. a row noting an open deviation names a ticket that is not `DONE`).
2. **Mapping ↔ COMPATIBILITY.** Every `mapping.tsv` row that is mapped with a known deviation (a ticket in state
   `ACCEPTED`, or a reference to `docs/COMPATIBILITY.md`) has a matching entry in `docs/COMPATIBILITY.md`
   ("Sanctioned deviations, one by one"), and every COMPATIBILITY entry points to an existing mapping row or ADR
   (owner 2026-11-11). Define the machine-readable link both ways (an anchor or id per COMPATIBILITY entry) before
   checking it; legacy entries go into the baseline of TOOL41.
3. **Coverage per layer, computed.** A report command derived from `edges.tsv`, `mapping.tsv`, the tosdump case tables
   and the ticket states replaces the hand-kept `docs/INVENTORY.md` (as of 2026-09-04) and
   `docs/MEASUREMENT-COVERAGE.md` (as of 2026-09-06); both become dated history or are removed, owner decides.
   Owner 2026-11-11: the command is `cargo xtask report --coverage`, the living view that replaces the frozen
   snapshot of [INV01](INV01.md) (docs/analysis/inventory-2026-11-11.md, once written); that snapshot's section
   "Queries for `report --coverage`" lists what it must compute (functions, mapped, spec, measured, deviation,
   ticket per layer), and on the snapshot's date its numbers must match the snapshot's.
4. Host tests for each cross-check (agreeing, each kind of disagreement, legacy baseline).

## Acceptance

- On today's tree with the baseline: exit 0; each kind of disagreement injected in a temp repository is reported.
- The coverage report reproduces the per-layer numbers of `cargo xtask deps --coverage` and adds the measured and
  ticket columns.
- `cargo xtask test` green, warnings delta 0, doc-check 0 failures.
