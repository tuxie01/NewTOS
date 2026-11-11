# DESK25 — `STDELDIS`: die Zusatzfrage beim Loeschen eines Laufwerks fehlt

Status: DONE (2026-11-11) | Priority: P4 | Commit: b7356cf8b | Receipt: DESK25-STATUS-DONE.MD

Migrated (owner batch 19, 2026-11-11, to DONE), evidence: receipt TASK/DESK25-STATUS-DONE.MD exists, commit b7356cf8b (feat(desk): the folder name conflict, STDELDIS, and the info line afte), VERIFIED: b7356cf8b (feat(desk): the folder name conflict, STDELDIS, and the info) names the id by body, its message names the ticket-title identifier stdeldis, touches code (12 files), legacy line: ERLEDIGT (2026-09-23)

Line 3 before the migration:
Status: **ERLEDIGT** (2026-09-23) · Prioritaet: **P4** · Typ: Funktionsluecke — Nebenbefund DESK13

Abhaengigkeiten: [DESK13](DESK13.md) (erledigt)

## Sachverhalt

EmuTOS stellt beim Loeschen eines ganzen Laufwerks eine **zweite** Frage
(`STDELDIS`, `references/emutos/desk/deskdir.c:971-975`); sie haengt an
`icontype == AT_ISDISK`.

NewTOS' `dir_op` bekommt den Symboltyp nicht uebergeben, also kann die Frage
dort nicht entstehen. Der Bestaetigungskasten aus DESK13 ist davon
unberuehrt — es geht um die eine zusaetzliche Rueckfrage.

## Arbeit

Den Symboltyp bis `dir_op` durchreichen (EmuTOS' Parameter), dann die Frage
an EmuTOS' Stelle stellen.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss

Bestaetigt und gebaut. `dir_op` und `fun_op` in `kernel/src/desk/dir.rs`
haben EmuTOS' zweiten Parameter bekommen (`icontype: Option<AType>`; `None`
ist sein `-1`, `Some(AType::Disk)` sein `AT_ISDISK`), ebenso
`fun_file2desk` (`icontype_src`). Die Werte folgen EmuTOS Aufrufstelle fuer
Aufrufstelle — die Tabelle steht in `docs/api/desktop/deskdir.md`.

Die Frage selbst steht an EmuTOS' Stelle (`deskdir.c:967-976`): hinter dem
`ADCPYDEL`-Kasten, vor der Schleife, **nicht** an `g_cdelepref` gebunden.
`delete_disk_alert(letter)` baut `STDELDIS` mit dem Laufwerksbuchstaben
(`desk_rsc.c:2499-2500`, `fun_alert_merge(2, …)`), Voreinstellung `No`,
`== 1` heisst `Yes`.

Messung: [`measurements/measurement-2026-09-23-desk24-desk25-desk26/`](../measurements/measurement-2026-09-23-desk24-desk25-desk26/)
§3. `Nein` laesst die Karte unveraendert (57 Dateien wie das Ausgangsabbild),
`Ja` leert sie bis auf die Datentraegerkennung. Vorher (HEAD) wird ohne
zweite Frage geloescht.

**Nicht angefasst, eigener Befund:** der `AT_ISTRSH`-Zweig in
`fun_file2desk` stellt bei EmuTOS `wants_to_delete_files()` voran, wenn
`icontype_src >= 0` (`deskfun.c:1049-1051`); NewTOS haelt dort weiter den
eigenen `fun_confirm`-Vorbau in `fun_desk2desk`. Die Zahl der Rueckfragen
stimmt, der Wortlaut und der *Remove*-Zweig nicht.
