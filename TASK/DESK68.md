# DESK68 — „Neuer Ordner“-Dialog ist nicht EmuTOS' ADMKDBOX

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner decision 2026-11-11 at the legacy migration: not on the work queue, unmeasured candidate from reading, not yet shown to affect a program | Reopen: a measurement or a program shows the divergence

Migrated (owner batch 10, 2026-11-11), evidence: TASK/DESK68.md status line: OFFEN, no DONE receipt, no fix/feat commit names the id

Line 3 before the migration:
Status: **OFFEN** · Prioritaet: **P3** · Typ: Abweichung von EmuTOS (ungeprueft) — Nebenbefund aus [A111](A111.md) (2026-10-25)

## Befund (Developer A111, ungeprueft)

Der Neuer-Ordner-Dialog in `programs/desktop/src/dlg.rs` (`edit_name`) ist
nicht EmuTOS' Dialog ADMKDBOX (`desk/deskfun.c` bzw. Desktop-RSC).

## Auftrag

Gegen EmuTOS (Funktion, die ADMKDBOX zeigt, und RSC-Baum) und rscview/
Hatari-Falcon vergleichen; bei Abweichung 1:1 nach ADR-022 nachbauen.

[Gemeinsame Regeln](README.md) gelten.
