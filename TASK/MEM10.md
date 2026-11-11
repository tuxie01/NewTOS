# MEM10 — Speicherschutz-Synchronisierung ohne volle Momentaufnahme

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner decision 2026-11-11 at the legacy migration: not on the work queue | Reopen: a card program or port is affected, or the owner schedules it

Migrated (owner batch 11, 2026-11-11), evidence: TASK/MEM10.md status line: OFFEN, no DONE receipt, no fix/feat commit names the id

Line 3 before the migration:
Status: **OFFEN** · Prioritaet: **P3** · Typ: Laufzeit — Anwenderentscheidung 2026-10-13 (MEM05 Teil A)

`memprot.rs` baut bei jeder Synchronisierung (jedes `Malloc`/`Mfree` mit
Speicherschutz) die sortierte Momentaufnahme `SNAP` aller belegten Bloecke
neu (`sync_now` `:732`, `structure_delta` `:1263`). Kosten O(N·64 + N log N);
bei 8192 Bloecken eines Prozesses rund 0,5 Mio Schritte je `Malloc`.
Ziel: inkrementelle Aktualisierung, Kosten O(geaenderte Bloecke). Erst
messen, dann Entwurf zur Freigabe.

Abhaengigkeiten: [MEM05](MEM05.md)

[Gemeinsame Regeln](README.md) gelten.
