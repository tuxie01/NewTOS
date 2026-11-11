# DESK26 — Infozeile bleibt nach `fun_rebld` auf der alten Auswahl stehen

Status: DONE (2026-11-11) | Priority: P4 | Commit: b7356cf8b | Receipt: DESK26-STATUS-DONE.MD

Migrated (owner batch 19, 2026-11-11, to DONE), evidence: receipt TASK/DESK26-STATUS-DONE.MD exists, commit b7356cf8b (feat(desk): the folder name conflict, STDELDIS, and the info line afte), VERIFIED: b7356cf8b (feat(desk): the folder name conflict, STDELDIS, and the info) names the id by body, its message names the ticket-title identifier fun_rebld, touches code (12 files), legacy line: ERLEDIGT (2026-09-23)

Line 3 before the migration:
Status: **ERLEDIGT** (2026-09-23) · Prioritaet: **P4** · Typ: Defekt — Nebenbefund DESK13

Abhaengigkeiten: [A110](A110.md) (erledigt)

## Sachverhalt

Nach einem Loeschvorgang baut `fun_rebld` das Fenster neu; die Auswahl ist
damit weg, die Infozeile zeigt aber weiter `… 1 selected items.`
(sichtbar in `measurements/measurement-2026-09-23-desk13/screendumps/f-deleted.png`).

A110 hat `win_sinfo(handle, check_selected)` an EmuTOS' fuenf Aufrufstellen
gesetzt; `fun_rebld` ist offenbar keine davon oder ruft es mit dem falschen
Argument. Zu klaeren an der Quelle (`desk/deskfun.c` `fun_rebld`,
`desk/deskwin.c:867-906`), welcher der beiden Faelle es ist.

## Arbeit

Quelle lesen, die fehlende Aktualisierung nachziehen, Screendump vor/nach.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss

Bestaetigt und gebaut. Die Quelle sagt: **`fun_rebld` ist eine sechste
Aufrufstelle**, keine der fuenf mit falschem Argument. EmuTOS ruft
`win_sinfo(pwin, FALSE)` in `rebuild_window` (`deskfun.c:135-142`), und
`rebuild_window` ist genau das, was `fun_rebld` (`:173`) und
`fun_rebld_marked` (`:150`) fahren.

NewTOS' `rebuild_window` (`kernel/src/desk/dir.rs`) rief es gar nicht. Die
A110-Notiz in `docs/api/desktop/deskwin.md` behauptete, beide `FALSE`-Stellen
saessen in `set_window_strings` („`do_diropen`/`rebuild_window`") — das
stimmte nur fuer `do_diropen` (`desksupp.c:450`); `set_window_strings` hat in
`rebuild_window` keinen Aufrufer. Die Notiz ist berichtigt.

Gebaut: `win_sinfo(handle, false)` in `rebuild_window`, nach dem
Neueinlesen und vor dem Neuzeichnen, an EmuTOS' Stelle.

Messung: [`measurements/measurement-2026-09-23-desk24-desk25-desk26/`](../measurements/measurement-2026-09-23-desk24-desk25-desk26/)
§4:

    vorher  " 5 bytes used in 1 selected items."   (d26-before-e-deleted.png)
    nachher " 5 bytes used in 2 items."            (d26-after-e-deleted.png)
