# MEM09 — `memprot::on_alloc` ignoriert den Rueckgabewert von `record`

Status: PARKED (2026-11-11) | Priority: P3 | Reason: legacy, evidence unverified; receipt MEM09-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 20, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: 526ad8ac0 (docs(task): close six open tickets per the open-list review) added the receipt TASK/MEM09-STATUS-DONE.MD itself, the report of this research or measurement ticket (16 files), bookkeeping or bulk commit, not accepted as evidence

Line 3 before the migration:
Status: **ERLEDIGT** 2026-10-17 ([Quittung](MEM09-STATUS-DONE.MD)) · Prioritaet: **P3** · Typ: Robustheit — Nebenfund MEM05-Entwurf (2026-10-13)

Schlaegt `record` fehl (Tabelle voll), faellt der Block still auf `PROT_P`
zurueck. MEM05 reserviert die Kapazitaet kuenftig vor `ffit`; dieser Fall
soll trotzdem nicht still bleiben. Pruefen, ob nach MEM05 noch erreichbar;
sonst als Invariante absichern.

[Gemeinsame Regeln](README.md) gelten.
