# LEGACY01 — classify the legacy tickets for migration (read-only)

Status: OPEN (2026-11-11) | Priority: P1

Type: research (read-only), development slot. Owner decision 2026-11-11: the legacy tickets are cleaned up before the
inventory creates new tickets; starts on the development slot after the [TOOL52](TOOL52.md) cut. An inventory snapshot
(measurement and coverage) may run in parallel, but it files no new tickets until the clean-up batches are decided.
The migration tool is [TOOL54](TOOL54.md).

## Work

1. Classify every ticket with an entry in `tools/xtask/ticket-baseline.tsv` into exactly one class:
   - **M1** done with evidence (a receipt and a fix commit) → `DONE`;
   - **M2** duplicate or superseded, with an explicit marker in the ticket → `OBSOLETE` with `Target`;
   - **M3** overtaken by measurement (the claimed divergence is no longer in today's tosdiff tables) → verified by the
     researcher → `DONE` or `OBSOLETE`;
   - **M4** `G-*` "never measured" backlog (September 2026) → `OBSOLETE`, Reason: superseded by inventory-2026-11-11
     (gaps regenerated from the graph);
   - **M5** genuinely open → `OPEN` in the new form, then a pre-start check (most → `PARKED` with `Reason`/`Reopen`);
   - **M?** unclear → listed for the owner.
2. Output: a dated analysis docs/analysis/legacy-triage-2026-11-11.md (counts per class, one row per ticket with the evidence it rests
   on) and draft batch files `TASK/migration/batch-<N>.tsv`, one per class, a class over 50 tickets split into
   sub-batches.
3. Per batch: a hand check of a random sample of 5 against code, receipts and measurements, reported with the batch.
4. Order: M4 and M1 first, then M2, M3, M5, M?.

## Acceptance

- Every baseline ticket in exactly one class; counts and sample results reported to the owner before any `migrate`
  run.
- No ticket file changed by this ticket (read-only); doc-check of the triage document 0 failures.
