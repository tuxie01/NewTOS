# MEM08 — `Mfree`/`Mshrink`/`detach` suchen linear: `Pterm` ist O(L×N)

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner decision 2026-11-11 at the legacy migration: not on the work queue | Reopen: a card program or port is affected, or the owner schedules it

Migrated (owner batch 11, 2026-11-11), evidence: TASK/MEM08.md status line: OFFEN, no DONE receipt, no fix/feat commit names the id

Line 3 before the migration:
Status: **OFFEN** · Prioritaet: **P3** · Typ: Laufzeit — Nebenfund MEM05-Entwurf (2026-10-13)

`Mfree`, `Mshrink`, `set_owner`, `block()` und `region::detach_all` laufen
linear ueber `mp_mal` aller Prozesse (`region.rs:285-330,375-425`,
`traps.rs:742-790`); `Pterm` eines Prozesses mit vielen Bloecken wird damit
quadratisch. Deshalb ist die MEM05-Grenze je Prozess auf 8192 gesetzt. Umbau
auf MD-direkte Suche wuerde die Grenze anhebbar machen. Erst messen.

[Gemeinsame Regeln](README.md) gelten.
