# DESK05 — Options → Desktop configuration (`.INF`-Einstellungen)

Status: PARKED (2026-11-11) | Priority: P3 | Reason: legacy, evidence unverified; receipt DESK05-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 18, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by 09c940b2f (feat(desk): Desktop configuration, and the menu sh), named by body

Line 3 before the migration:
Status: **ERLEDIGT (2026-09-23)** · Prioritaet: **P3** · Typ: Funktionsluecke desk/

Abhaengigkeiten: [DESK09](DESK09.md)

## Sachverhalt

`Item::DeskConfig` ohne Handler. EmuTOS: der Punkt, der die
Desktop-Einstellungen (Hintergrundmuster/-farbe, Anzeigeart, Sortierung)
in einem Dialog zusammenfasst bzw. `.INF`-Inhalte bearbeitet — genaue
Fundstelle und Umfang aus der Quelle bestimmen (`desk/deskmain.c`
`do_optnmenu`, `desk_rsc.c`-Baeume).

## Arbeit

Erst Quelle lesen und Umfang festlegen (was gehoert hierher, was nach
DESK02), dann bauen.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

---

## Abschluss

**Erledigt 2026-09-23.** Quittung: [DESK05-STATUS-DONE.MD](DESK05-STATUS-DONE.MD).
Messung: `measurements/measurement-2026-09-23-desk05/`.

Umfang aus der Quelle: `inf_conf` (`deskinf.c:1096-1246`) hinter
`CONF_WITH_DESKTOP_CONFIG` (Vorgabe **1**, `include/config.h:1774-1776`),
`CONFITEM` in keiner `ILL_*`-Tabelle, also immer schwarz. Gebaut sind der
Kuerzel-Editor samt der fehlenden `deskmain.c`-Maschinerie
(`menu_shortcuts[]`, `shortcut_mapping[]`, `default_shortcuts[]`,
`copy_menu_items`, `install_shortcuts`/`recover_shortcuts`, feldgetriebenes
`lookup_ascii_shortcut`, `#K` lesen und schreiben), der Funktionstasten-
Anzeiger und die Speicherzeile. Weggelassen mit Fundstelle: *Default dir*
und *Parameter*, deren Verbraucher in `do_aopen` bzw. `fun_win2win` liegen
und die NewTOS nicht hat. Abgrenzung zu DESK02: keine — `ADSETPRE` und
`ADDESKCF` teilen kein Feld.
