# DESK33 — `process_funkey`: F1…F20 starten die gebundene Anwendung nicht

Status: DONE (2026-11-11) | Priority: P3 | Commit: aa76e41f3 | Receipt: DESK33-STATUS-DONE.MD

Migrated (owner batch 19, 2026-11-11, to DONE), evidence: receipt TASK/DESK33-STATUS-DONE.MD exists, commit aa76e41f3 (feat(desk): the last four of the desktop series), VERIFIED: aa76e41f3 (feat(desk): the last four of the desktop series) names the id by body, its message names the ticket-title identifier process_funkey, touches code (20 files), legacy line: ERLEDIGT

Line 3 before the migration:
Status: **ERLEDIGT** · Prioritaet: **P3** · Typ: Funktionsluecke — Nebenbefund DESK20

Abhaengigkeiten: [DESK20](DESK20.md) (erledigt)

## Sachverhalt

Seit DESK20 kann eine installierte Anwendung eine Funktionstaste tragen
(`APFUNKEY`, angezeigt in *Desktop configuration*). Ein Druck auf die Taste
startet sie aber nicht: `process_funkey`
(`references/emutos/desk/deskmain.c:1087-1106`) hat kein Gegenstueck in
`kernel/src/desk/main.rs`.

Vor DESK20 war das nicht beobachtbar, weil keine Anwendung eine Taste tragen
konnte.

## Arbeit

`process_funkey` nachbauen (Tastencode → `a_funkey` → `do_aopen`), an
EmuTOS' Stelle im Tastaturweg. Abnahme: eine Anwendung auf `F7` legen, `F7`
druecken, sie startet — mit dem Arbeitsverzeichnis aus DESK18.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss

`process_funkey` gebaut, 2026-09-25, Basis `desk-rebuild` HEAD `12fddc1`; die
Aenderung liegt **uncommitted** im gemeinsamen Baum.

`inf::app_by_funkey` sucht den `ANODE` mit diesem `a_funkey` — in unserer
Zuordnungsliste, denn nur die kann einen tragen (ein Schreibtischknoten wird
mit `a_funkey = 0` angelegt und niemand setzt ihn, `deskins.c:243,825,1037,
1385`). Danach `pathname`/`pfname` wie EmuTOS und
`do_aopen(pa, TRUE, -1, pathname, pfname, NULL)`, was fuer eine installierte
Anwendung genau `main::do_aopen_drop` mit leerem Kommandoschwanz ist —
`pro_run` schreibt fuer `tail == NULL` denselben leeren Schwanz
(`deskpro.c:56-57`). `check_function_key` gibt jetzt `Some(done)` zurueck, und
`hndl_kbd` reicht das ohne `men_update` durch, wie EmuTOS
(`deskmain.c:1224-1226`).

Benannte Luecke, keine neue: `opn_appl`s Parameterabfrage fuer `.TTP`/`.GTP`
(`desksupp.c:978-985`) gibt es fuer `.NTP` auch beim Doppelklick nicht
(Stage A); eine `.NTP` an einer Taste startet also mit demselben leeren
Schwanz.

Abnahme: [`measurements/measurement-2026-09-25-desk-rest/`](../measurements/measurement-2026-09-25-desk-rest/)
§5 — `#G 06 FF C:\ARGVTEST.NAP@ *.XYZ@ 107 @` in `NEWDESK.INF`, F7 gedrueckt
mit geparktem Zeiger und ohne einen einzigen Klick:
`[d33] process_funkey(7) -> C:\ARGVTEST.NAP appdir=true`, dann die Ausgabe des
Programms; F6 (unbelegt) tut nichts. Spezifikation:
[`docs/api/desktop/deskmain.md`](../docs/api/desktop/deskmain.md).
Hardwarepunkt **70** (echte Tastatur, und derselbe Test nach einem Kaltstart).
