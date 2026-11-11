# INV01 — inventory 2026-11-11: frozen snapshot of what is implemented, measured and missing

Status: OPEN (2026-11-11) | Priority: P1

Type: research (read-only snapshot), development slot. Owner decisions 2026-11-11: after the legacy clean-up (the
baseline is empty, `TASK/migration/closed.txt`), the inventory runs; every gap it finds becomes a ticket through the
tools, with a pre-start check; the living view comes from `cargo xtask report --coverage`. The lead reports to the
owner before more than about 20 new tickets are filed. The `G-*` tickets were made OBSOLETE with the reason
"superseded by inventory-2026-11-11 (gaps regenerated from the graph)".

## Work

1. **Frozen snapshot**, a dated analysis docs/analysis/inventory-2026-11-11.md (dated, so historical once written): per layer (BIOS,
   XBIOS, GEMDOS/BDOS, AES, VDI, line-A, cookies/sysvars) and per function, from the repository only:
   - the EmuTOS function and its graph node (`docs/analysis/deps/edges.tsv`, `docs/analysis/deps/layers.md`),
   - the NewTOS counterpart (`docs/analysis/deps/mapping.tsv`; `unmapped` named as such),
   - spec present (`docs/api/`), measured against the oracle (tosdump cases and references, `tosdump/cases/`,
     `tosdump/reference/`), known deviation (`docs/DEVIATIONS.md`, `docs/COMPATIBILITY.md`),
   - the ticket that covers it (any state), or none.
   Counts per layer; the hand-kept `docs/INVENTORY.md` and `docs/MEASUREMENT-COVERAGE.md` are inputs to compare, not the
   source.
2. **Gap list** in the same document: one row per gap (no counterpart, no spec, never measured, deviation without
   ticket), with the proposed ticket (title, layer, priority, pre-start check proposal: OPEN or PARKED with
   Reason/Reopen). No ticket is filed in this step.
3. Filing: after the lead's report to the owner, gaps become tickets in batches (owner decides when more than about 20).
4. The living view `cargo xtask report --coverage` does not exist yet: the snapshot names the queries a tool must
   compute so it can replace the snapshot (input for the tooling slot).

## Acceptance

- Every function in the graph appears in exactly one row; counts per layer add up; doc-check 0 failures.
- The gap list names its evidence per row (file:line or table/case).
