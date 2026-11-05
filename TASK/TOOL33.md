# TOOL33 — `mapping.tsv` fuer die AES-Ebene (EmuTOS `aes/` → NewTOS)

Status: **OFFEN** · Prioritaet: **P1** (Anwender 2026-11-05: nach [TOOL32](TOOL32.md) VDI, vor [A119](A119.md)) · Typ: Researcher (Zuordnung lesen), danach Lead (Pruefregel) — Werkzeug, das die Lokalisierung der folgenden Fixes braucht (CLAUDE.md "Testing")

## Sachverhalt

`cargo xtask deps --coverage` am 2026-11-05: von den 418 EmuTOS-Funktionen
der Ebene `aes` haben 29 eine Zeile in `docs/analysis/deps/mapping.tsv`
(nachgezogen von A127, A128, A132, A133 u. a.), 389 keine. Gleiche Ursache wie
bei der VDI: die Zuordnung deckt von Hand nur `bios/` und `bdos/` ab
(`MAPPED_DIRS`, `tools/xtask/src/deps.rs:63@MAPPED_DIRS`).

Funktionen je Datei: `gemgsxif.c` 43, `gemwmlib.c` 39, `gemgraf.c` 30,
`gemrslib.c` 28, `geminput.c` 26, `gemshlib.c` 23, `gemoblib.c` 23,
`geminit.c` 21, `gemgrlib.c` 19, `gemmnext.c` 17, `gemfslib.c` 17,
`gemmnlib.c` 15, `gemdosif.S` 13, `gemobed.c` 12, `gemfmlib.c` 10,
`gemctrl.c` 9, `gemevlib.c` 8, `gemasync.c` 8, `gempd.c` 7, `gemdisp.c` 7,
`gemwrect.c` 6, `gemaplib.c` 6, `gemfmalt.c` 4, `gemflag.c` 4, `gemasm.S` 4,
`gemsuper.c` 3, `gemstart.S` 3, `gemsclib.c` 3, `gemobjop.c` 3,
`gemqueue.c` 2, `gem_rsc.c` 2, `gsx2.c`/`gemevlib.h`/`aesvars.h` je 1.

## Auftrag

Wie [TOOL32](TOOL32.md): jede `aes/`-Funktion bekommt eine Zeile (Format und
Klassen `tools/deps/README.md`), vorhandene Zeilen werden geprueft, beim
Lesen gefundene Abweichungen nur gemeldet. Zuschnitt in Auftraege zu je
etwa 100 Funktionen, je ein frischer Researcher; wird nach TOOL32 festgelegt.
Danach (Lead): `"aes/"` in `MAPPED_DIRS`, `cargo xtask test` gruen.

## Abnahme

- `cargo xtask deps --check-mapping` ohne Befund; `--coverage` zeigt fuer
  `aes` keine Funktion ohne Zeile.
- `MAPPED_DIRS` enthaelt `aes/`, Hosttests gruen.
- Liste der beim Lesen gefundenen Abweichungen im Ticket.

[Gemeinsame Regeln](README.md) gelten.
