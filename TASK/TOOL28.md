# TOOL28 — `host_clang` und `optional_missing` automatisch ins RUNINFO

Status: **OFFEN** · Prioritaet: **P3** · Typ: Developer (Werkzeug) — Anwenderentscheidung 2026-10-30, Folge von [SDK08](SDK08.md)

## Sachverhalt

- Seit SDK08 traegt jedes Abbild-Manifest eine Zeile `host_clang`
  (`tools/sd-manifest.sh`). `cargo xtask tosdump` uebernimmt aus dem Manifest
  nur `image_sha256` und `optional_missing` (`tools/xtask/src/tosdump.rs`,
  `manifest_lines`); die Sonden-Laufskripte (`tosdump/probes/run-*.sh`)
  kopieren das Manifest nur.
- Bis dieses Ticket erledigt ist, verlangt `CLAUDE.md` die Zeile von Hand.

## Arbeit

1. `cargo xtask tosdump` und die Sonden-Laufskripte schreiben `host_clang` und
   die `optional_missing`-Zeilen jedes gebooteten Abbilds automatisch in ihre
   Laufnotiz bzw. ins `RUNINFO.TXT`-Geruest.
2. Danach die Handregel in `CLAUDE.md` auf "wird automatisch geschrieben"
   umstellen.

## Abnahme

- Ein `cargo xtask tosdump`-Lauf und ein Sondenlauf zeigen beide Zeilen ohne
  Handarbeit; Hosttests gruen.

[Gemeinsame Regeln](README.md) gelten.
