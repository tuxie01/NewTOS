# DESK61 — `act_chg` setzt `f_selected` bei unveraendertem Zustand nicht

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner decision 2026-11-11 at the legacy migration: not on the work queue | Reopen: a card program or port is affected, or the owner schedules it

Migrated (owner batch 9, 2026-11-11), evidence: TASK/DESK61.md status line: OFFEN, no DONE receipt, no fix/feat commit names the id

Line 3 before the migration:
Status: **OFFEN** · Prioritaet: **P3** · Typ: Konformitaet — Nebenfund DESK59 (2026-10-16)

EmuTOS `act_chg` setzt `fn->f_selected` immer aus dem neuen State
(`desk/deskact.c:447-449`), bevor es prueft, ob sich am Bildschirm etwas
aendert. NewTOS (`programs/desktop/src/act.rs`, `DeskState::act_chg`) kehrt
bei unveraendertem Zustand vorher zurueck und laesst `f_selected` stehen.
Pruefen, ob `f_selected` und `ob_state` dadurch auseinanderlaufen koennen
(Fall finden, Selbsttest rot), dann angleichen.

[Gemeinsame Regeln](README.md) gelten.
