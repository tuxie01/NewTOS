# DESK68 — „Neuer Ordner“-Dialog ist nicht EmuTOS' ADMKDBOX

Status: **OFFEN** · Prioritaet: **P3** · Typ: Abweichung von EmuTOS (ungeprueft) — Nebenbefund aus [A111](A111.md) (2026-10-25)

## Befund (Developer A111, ungeprueft)

Der Neuer-Ordner-Dialog in `programs/desktop/src/dlg.rs` (`edit_name`) ist
nicht EmuTOS' Dialog ADMKDBOX (`desk/deskfun.c` bzw. Desktop-RSC).

## Auftrag

Gegen EmuTOS (Funktion, die ADMKDBOX zeigt, und RSC-Baum) und rscview/
Hatari-Falcon vergleichen; bei Abweichung 1:1 nach ADR-022 nachbauen.

[Gemeinsame Regeln](README.md) gelten.
