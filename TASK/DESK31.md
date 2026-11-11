# DESK31 — Reste des Dateiwerks: `d_dofileren`, `STILLDIR`, `desk_verify`, `fun_rebld_marked`

Status: DONE (2026-11-11) | Priority: P3 | Commit: aa76e41f3 | Receipt: DESK31-STATUS-DONE.MD

Migrated (owner batch 19, 2026-11-11, to DONE), evidence: receipt TASK/DESK31-STATUS-DONE.MD exists, commit aa76e41f3 (feat(desk): the last four of the desktop series), VERIFIED: aa76e41f3 (feat(desk): the last four of the desktop series) names the id by body, its message names the ticket-title identifier desk_verify, d_dofileren, touches code (20 files), legacy line: ERLEDIGT

Line 3 before the migration:
Status: **ERLEDIGT** · Prioritaet: **P3** · Typ: Funktionsluecken — Nebenbefunde DESK24/DESK25/DESK26

Abhaengigkeiten: [DESK24](DESK24.md), [DESK25](DESK25.md), [DESK26](DESK26.md) (erledigt)

## Die Stellen

1. **`d_dofileren` fragt nicht.** EmuTOS hat dort eine eigene
   `item_exists`/`get_new_name`-Schleife samt „Ziel loeschen, damit `Frename`
   gelingt" (`references/emutos/desk/deskdir.c:785-825`); unsere ist ein
   blankes `dos_rename`. Betrifft das Verschieben einer **Datei** auf
   demselben Laufwerk ueber eine gleichnamige.
2. **`STILLDIR` fehlt**: `dir_op`s `source_is_parent`-Zweig setzt nur
   `ok = false`, EmuTOS fragt `Skip | Abort` (`deskdir.c:999-1003`).
3. **`desk_verify` fehlt in `rebuild_window`** (`deskfun.c:140`) — ungeprueft,
   ob beobachtbar.
4. **`fun_rebld_marked` / `WN_REBUILD` gibt es nicht** (`deskmain.c:678`,
   `deskfun.c:150-167`).
5. **`illegal_op_msg` uneinheitlich:** DESK24 hat EmuTOS' Wortlaut
   („Illegal operation!", `desk_rsc.c:2497`) an **einer** Stelle eingefuehrt;
   die zwei aelteren (`deskfun.c:794`, `:1075`) sagen weiter
   „Operation not allowed.".
6. **`fun_desk2desk`/Papierkorb** behaelt den NewTOS-`fun_confirm`-Vorbau
   statt EmuTOS' `wants_to_delete_files()` (`deskfun.c:1049-1051`). Die
   **Zahl** der Rueckfragen stimmt seit DESK25, Wortlaut und *Remove*-Zweig
   nicht.

## Arbeit

Der Reihe nach, jede Stelle mit ihrer Fundstelle; 5 ist eine Zweizeilen-
Angleichung, 1 und 6 brauchen je einen Messlauf am Kartenabbild.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss

Alle sechs Stellen gebaut, 2026-09-25, Basis `desk-rebuild` HEAD `12fddc1`;
die Aenderung liegt **uncommitted** im gemeinsamen Baum.

1. **`d_dofileren`** ist EmuTOS' Schleife (`deskdir.c:785-825`):
   `item_exists` → `get_new_name` → `dos_delete` auf `ret == 1` mit
   `oldname != newname`. `d_dofoldren` ist dasselbe mit `is_folder = true` —
   seine einzige eigene Aufgabe bei EmuTOS ist das Abschneiden der `\*.*`, die
   unser Laeufer nicht mitfuehrt. `false` heisst wie bei EmuTOS „Lauf anhalten"
   (`ctx.stop`), nicht „Fehler" (`ok` bleibt stehen).
2. **`STILLDIR`** in `dir_op`s `source_is_parent`-Zweig, `Skip | Abort`, gebaut
   wie `delete_disk_alert` (zwei Knoepfe, also nicht ueber `fun_alert`).
3. **`desk_verify(handle, work, true)`** in `rebuild_window` statt des blossen
   `win_bldview` — es setzt `g_cwin`/`g_croot`/`g_wlastsel` mit.
4. **`fun_mark_for_rebld` / `fun_rebld_marked`** neu; `WN_REBUILD` gab es
   schon als Konstante ohne Leser. Der Aufschub ist ihr Zweck, und ihr
   Aufrufer ist DESK38s `do_info`-Schleife.
5. **`illegal_op_msg`** an allen drei Stellen, die noch „Operation not
   allowed." sagten (`deskfun.c:794`, `:1075-1076`, `:1286` — die dritte stand
   nicht im Ticket, ist aber dieselbe EmuTOS-Funktion).
6. **`wants_to_delete_files`** im Papierkorb-Zweig von `fun_desk2desk` statt
   `fun_confirm`; Wortlaut und Knopfreihe jetzt `STRMVDEL`
   (`Icons | Files | Cancel`), und der `MN_SELECTED`-Umweg der Antwort *Icons*
   liegt — wie bei EmuTOS — in der Funktion selbst. `fun_confirm` hatte danach
   keinen Aufrufer und keine EmuTOS-Entsprechung mehr und ist entfallen.

Messung: [`measurements/measurement-2026-09-25-desk-rest/`](../measurements/measurement-2026-09-25-desk-rest/)
§4, mit den Dateiwirkungen aus dem Kartenabbild zurueckgelesen. Spezifikation:
[`docs/api/desktop/deskdir.md`](../docs/api/desktop/deskdir.md) und
[`deskfun.md`](../docs/api/desktop/deskfun.md). Hardwarepunkt **71**.

Beim Messen gefunden, kein Fehler: einen Ordner auf **sich selbst** zu ziehen
erreicht `STILLDIR` nicht — `act_bdown` bricht einen Wurf auf das gezogene
Objekt eine Ebene hoeher ab (`deskact.c:666-670`). Quelle und Ziel muessen
verschiedene Objekte sein.
