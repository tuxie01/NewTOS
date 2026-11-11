# NET05 — HighWire: portieren, erst lokal, dann online

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner decision 2026-11-11 at the legacy migration: not on the work queue | Reopen: a card program or port is affected, or the owner schedules it

Migrated (owner batch 11, 2026-11-11), evidence: TASK/NET05.md status line: OFFEN, no DONE receipt, no fix/feat commit names the id

Line 3 before the migration:
Status: **OFFEN** · Prioritaet: **P3** · Typ: Anwendungsport

Abhaengigkeiten: [NET04](NET04.md) fuer den Onlineteil; der lokale Teil geht frueher

## Sachverhalt

HighWire (`sdk/apps/highwire/`, geklont 2026-09-23 von
`neilrackett/atarist-highwire`, v0.4.5): 73 `.c`, nur 3 Assemblerdateien
(`src/rastr_pc.s` + zwei in den Netzmodulen). Braucht laut `Makefile:105`
`-lgem -lcflib -liio -lgif -ljpeg -lpng -lz -lm`. Vorhanden: gemlib, cflib,
libm. **Fehlend: zlib, libpng, libjpeg, giflib, iio.** Netz haengt an
STinG/STiK2 als `.OVL`-Modulen (m68k) — bei uns spaeter ueber NET04.
Rendert ueber normales VDI, wenn kein SpeedoGDOS/NVDI da ist.

## Arbeit

1. Bibliotheken nachziehen: zlib, libpng, libjpeg(-turbo?), giflib —
   je Tarball+Overlay wie `sdk/libm/`; `iio` klaeren (Teil von cflib-Umfeld?).
2. `sdk/apps/build-highwire.sh` + `sdk/overlays/highwire/` nach dem
   QED/TeraDesk-Muster; Regel „Anwendungsquelle bleibt unveraendert".
3. Erst **lokal**: `file://`-Seiten aus `sdk/apps/highwire/html/` rendern,
   Screendump gegen die Referenz (Hatari-Falcon mit demselben HighWire, wenn
   es dort laeuft — sonst gegen den Browser auf dem Host als grobe Sicht).
4. Dann **online** ueber NET04.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten. Ports: retargeten und uebersetzen,
nie nachbauen.
