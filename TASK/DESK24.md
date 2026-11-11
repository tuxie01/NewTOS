# DESK24 — Ordner-Namenskonflikt fragt nicht (`output_path`/`get_new_name`)

Status: DONE (2026-11-11) | Priority: P3 | Commit: b7356cf8b | Receipt: DESK24-STATUS-DONE.MD

Migrated (owner batch 19, 2026-11-11, to DONE), evidence: receipt TASK/DESK24-STATUS-DONE.MD exists, commit b7356cf8b (feat(desk): the folder name conflict, STDELDIS, and the info line afte), VERIFIED: b7356cf8b (feat(desk): the folder name conflict, STDELDIS, and the info) names the id by body, its message names the ticket-title identifier get_new_name, output_path, touches code (12 files), legacy line: ERLEDIGT (2026-09-23)

Line 3 before the migration:
Status: **ERLEDIGT** (2026-09-23) · Prioritaet: **P3** · Typ: Funktionsluecke — Nebenbefund DESK13

Abhaengigkeiten: [DESK13](DESK13.md) (erledigt)

## Sachverhalt

DESK13 hat den **Datei**-Namenskonflikt gebaut (`ADCPALER` in
`output_fname`). Der **Ordner**-Konflikt ist der andere Zweig: EmuTOS fragt
in `output_path`/`get_new_name` (`references/emutos/desk/deskdir.c:700-779`)
nach einem neuen *Ordner*namen, wenn der Zielordner schon existiert.

NewTOS mischt stattdessen weiter hinein — der Inhalt landet im bestehenden
Ordner, ohne Rueckfrage.

## Arbeit

`output_path`/`get_new_name` nachbauen, Dialog wie beim Dateikonflikt ueber
die DESK08-Kette; Abnahme: ein Ordner mit demselben Namen im Ziel, beide
Antworten am Kartenabbild nachgewiesen.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss

Bestaetigt und gebaut. `kernel/src/desk/dir.rs`:

* **`get_new_name(dstpth, ctx)`** (`deskdir.c:700-723`) — fragt ueber denselben
  `ADCPALER`-Kasten wie der Dateikonflikt nach einem neuen Ordnernamen und
  gibt EmuTOS' `2`/`1`/`0`/`-1` als `NewName::{Changed,Unchanged,Stop,Skip}`
  zurueck. Beide Felder tragen den **Ziel**namen (`:705-709`) — anders als
  beim Dateikonflikt, wo `CACURRNA` der Quellname ist (`:304-307`).
* **`output_path(op, dstpth, ctx)`** (`deskdir.c:734-779`) — die Schleife mit
  ihren zwei Zweigen: ueber Laufwerksgrenzen legt `dos_mkdir` den Ordner an,
  und ein `OK` auf unveraendertem Namen heisst *hineinmischen*; bei
  `Op::Rename` darf das Ziel gar nicht existieren, ein unveraenderter Name
  bricht die Schleife also **nicht** ab.
* **`item_exists(path, is_folder)`** (`deskdir.c:186-208`) — EmuTOS' zweite
  Form: das angehaengte `\*.*` macht die Probe ordner-exklusiv.
* **`illegal_op_msg()`** (`deskdir.c:220-224`, `STILLOP`) — fuer ein
  `dos_mkdir`, das fehlschlaegt, ohne dass etwas im Weg steht.
* Verdrahtet in `dir_op`s Ordnerzweig an EmuTOS' Stelle (`:1002`), also
  **vor** dem Selbstvergleich; das `let _ = dos_mkdir(&dst_sub)` ist weg.
  `d_doop`s rekursiver Zweig mischt weiter wortlos — das tut EmuTOS auch
  (`:626-632`), gefragt wird nur der oberste Ordner.

Messung: [`measurements/measurement-2026-09-23-desk24-desk25-desk26/`](../measurements/measurement-2026-09-23-desk24-desk25-desk26/)
§2. Beide Antworten am Kartenabbild nachgewiesen (`serial/cards.txt`):
`OK` unveraendert → `C:\FOLD\{EXIST.TXT, INNER.TXT}`; `FOLD2` getippt →
neuer Ordner `C:\FOLD2\INNER.TXT`, `C:\FOLD` unberuehrt. Vorher (HEAD)
kommt gar keine Frage.
