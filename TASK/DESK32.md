# DESK32 — Drei Klappmenues sind eine Zeichenzelle zu breit

Status: PARKED (2026-11-11) | Priority: P4 | Reason: legacy, evidence unverified; receipt DESK32-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 19, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by aa76e41f3 (feat(desk): the last four of the desktop series), named by body

Line 3 before the migration:
Status: **ERLEDIGT** · Prioritaet: **P4** · Typ: Kosmetik — Nebenbefund DESK21

Abhaengigkeiten: [DESK21](DESK21.md) (erledigt)

## Sachverhalt

Desk misst 22 Zellen statt 21, File 23 statt 22. Options (30) und View (23,
seit DESK21) stimmen.

Ursache: die Beschriftungen in `kernel/src/desk/menu.rs`s `MENUS` tragen ein
Fuellzeichen mehr als der laengste Text, und `adjust_menu`s `maxlen` zaehlt
es mit. EmuTOS misst die von `copy_menu_items` gepolsterte Zeichenkette
(`references/emutos/desk/deskmain.c:1781`, `:1608-1620`).

## Arbeit

Die Polsterung aus den `MENUS`-Zeichenketten nehmen und sie wie EmuTOS beim
Bauen entstehen lassen; Screendump aller vier Klappmenues vor/nach mit
gemessener Zellenbreite.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss

Nachgemessen und behoben, 2026-09-25, Basis `desk-rebuild` HEAD `12fddc1`; die
Aenderung liegt **uncommitted** im gemeinsamen Baum.

**Die Messung bestaetigt das Ticket, in beide Richtungen.** Zwei Kernel, die
sich in `kernel/src/desk/menu.rs` allein unterscheiden, dieselbe Aktionsliste,
`[probe] box N abs (x,y) WxH` (8 px je Zeichenzelle):

| Kasten | vorher | nachher | EmuTOS |
|---|---|---|---|
| Desk | 176 px = 22 | 168 px = **21** | 21 |
| File | 184 px = 23 | 176 px = **22** | 22 |
| View | 184 px = 23 | 184 px = 23 | 23 |
| Options | 240 px = 30 | 240 px = 30 | 30 |

EmuTOS' Zahlen kommen aus der Quelle, nicht aus einem Bild: laengste
Nicht-Trenner-Zeichenkette + `1 + SHORTCUT_SIZE` (`copy_menu_items`,
`deskmain.c:1776-1781`), minus NUL, plus `adjust_menu`s `m++` (`:1620`) —
17+3+1, 18+3+1, 19+3+1, 26+3+1. Der Startwert `MIN_DESKMENU_WIDTH - 1` (19)
des Desk-Kastens bindet nicht.

Die Polsterung ist aus den `MENUS`-Zeichenketten heraus (`build_menu` polstert
die RAM-Kopie, wie `copy_menu_items`, und hat das immer getan); die Trenner
stehen wieder auf der Ressourcen-Form `"-"`, die `build_menu` ohnehin
ueberschreibt; und `adjust_menu` misst jetzt **alles ausser Trennern** statt
nur der Aktionseintraege — `copy_menu_items`' eigene Regel, die die sechs
Accessory-Platzhalter `"1"`…`"6"` einschliesst. Die Platzhalter stehen darum
auch auf ihrem Ressourcentext; sichtbar sind sie nur, wenn kein Accessory
registriert ist, und dann verknuepft `menu_fixup` sie ohnehin nicht.

Messung: [`measurements/measurement-2026-09-25-desk-rest/`](../measurements/measurement-2026-09-25-desk-rest/)
§2, mit allen vier Klappmenues vorher und nachher. Spezifikation:
[`docs/api/desktop/deskmain.md`](../docs/api/desktop/deskmain.md).
Hardwarepunkt **72** (1920×1080).
