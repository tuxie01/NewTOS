# NET04 — Socket-API als GEMDOS-Geraet (`mintsock`)

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner decision 2026-11-11 at the legacy migration: not on the work queue | Reopen: a card program or port is affected, or the owner schedules it

Migrated (owner batch 11, 2026-11-11), evidence: TASK/NET04.md status line: OFFEN, no DONE receipt, no fix/feat commit names the id

Line 3 before the migration:
Status: **OFFEN** · Prioritaet: **P3** · Typ: API-Schicht

Abhaengigkeiten: [NET03](NET03.md)

## Sachverhalt

FreeMiNT reicht Sockets nicht ueber eigene Traps heraus, sondern als
**Geraetedatei**: `Fopen("u:\dev\socket")` plus `Fcntl`-Kommandos
(`references/freemint/sys/sockets/mintsock.h`, `inetdev.c`). Das passt zu
unserem GEMDOS, braucht aber einen Geraeteknoten-Mechanismus, den wir
nicht haben (kein `u:\`-Laufwerk, keine `.xdd`-Geraete).

## Arbeit

1. Minimalen Geraeteknoten-Weg entscheiden (fester Pfad? `Fcntl` auf einem
   reservierten Handle? eigenes Laufwerk?) — mit Blick auf MOD01.
2. `mintsock`-Kommandos abbilden (socket, bind, connect, send, recv, select).
3. SDK: `sdk/libc`-Bindungen (mintlibs `sockets`-Teil pruefen, was baut).
4. Abnahme: ein `.NCP` holt eine HTTP-Seite und gibt sie auf der Seriellen aus.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.
