# DESK49 — TeraDesks `filetypes`-Liste kennt die vier nativen Endungen nicht

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner decision 2026-11-11 at the legacy migration: not on the work queue | Reopen: a card program or port is affected, or the owner schedules it

Migrated (owner batch 9, 2026-11-11), evidence: TASK/DESK49.md status line: OFFEN, no DONE receipt, no fix/feat commit names the id

Line 3 before the migration:
Status: **OFFEN** · Prioritaet: **P3** · Typ: Werkzeugkette — Nebenfund aus [DESK41](DESK41.md)

Abhaengigkeiten: [DESK41](DESK41.md) (dort wird `TERADESK.INF` erzeugt)

## Sachverhalt

DESK41 hat den `apptypes`-Abschnitt der erzeugten `TERADESK.INF` um `*.NTP`
und `*.NAC` ergaenzt — damit weiss TeraDesk, **wie** eine Datei zu starten ist.

Der **`filetypes`**-Abschnitt derselben Datei ist eine andere Liste: die
Maskenauswahl des Fenster-Popups („zeige nur …"). Er kennt `*.NAP`, `*.NCP`,
`*.NTP`, `*.NAC` weiterhin nicht.

Das ist kein Fehler von DESK41 — dessen Auftrag nannte `apptypes`/`ptype`.
Es ist der zweite Ort, an dem dieselbe Familie steht.

## Arbeit

1. In `sdk/apps/teradesk` klaeren, **was** `filetypes` steuert und ob die
   Liste eine Vorauswahl oder eine Filterung ist — bevor entschieden wird, ob
   die vier Endungen ueberhaupt hineingehoeren.
2. Wenn ja: in `tools/make-sd-image.sh` im selben Heredoc ergaenzen, damit
   beide Listen aus einer Quelle kommen und nicht auseinanderlaufen koennen.

Abnahme: das Fenster-Popup bietet die vier Endungen an, oder das Ticket
schliesst mit der Begruendung, warum es dort nicht hingehoert.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.
