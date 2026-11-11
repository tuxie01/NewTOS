# TOOL24 — `gemdos-trace`: DTA-Erfassung nach FS23 an Regel C1 anpassen

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner pre-start check 2026-10-31: debug feature, blocks nothing | Reopen: a fix needs the debug feature

Migrated (owner batch 11, 2026-11-11), evidence: TASK/TOOL24.md status line: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: Debug-Feature, blockiert nichts)

Line 3 before the migration:
Status: **GEPARKT** 2026-10-31 (Anwender, Vor-Start-Pruefung: Debug-Feature, blockiert nichts) · Prioritaet: **P3** · Typ: Developer (Werkzeug) — Fund aus [FS23](FS23.md) (2026-10-29)

## Sachverhalt

`tos-api/src/gemdos/trace.rs:34@put_state` (Modulkopf, Regel C1) und `tos-api/src/gemdos/trace.rs:618@put_state` (Fall
`0x4E`) begruenden die 44-Byte-DTA-Erfassung damit, dass `put_state` +
`makbuf` alle 44 Bytes schreiben. Seit FS23 schreibt `makbuf` nur Name und
NUL; die Erfassung liest damit Bytes, die der Aufruf nicht beruehrt hat —
gegen C1 („nur, was die Implementierung geschrieben hat“).

## Arbeit

Erfassungslaenge auf das tatsaechlich Geschriebene begrenzen (Kopf bis
`d_fname` + Name + NUL) oder die Regel begruendet aendern; Kommentare
mitziehen. Feature ist standardmaessig aus; Bau ohne Feature bytegleich.

## Abnahme

- Hosttest der Erfassung; Bau ohne Feature bytegleich (TOOL13-Pruefung).

[Gemeinsame Regeln](README.md) gelten.
