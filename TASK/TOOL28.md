# TOOL28 — `host_clang` und `optional_missing` automatisch ins RUNINFO

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner pre-start check 2026-10-31: the manual rule in CLAUDE.md covers it | Reopen: the manual rule fails or the owner wants it automated

Migrated (owner batch 12, 2026-11-11), evidence: TASK/TOOL28.md status line: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: Handregel in CLAUDE.md deckt es ab)

Line 3 before the migration:
Status: **GEPARKT** 2026-10-31 (Anwender, Vor-Start-Pruefung: Handregel in CLAUDE.md deckt es ab) · Prioritaet: **P3** · Typ: Developer (Werkzeug) — Anwenderentscheidung 2026-10-30, Folge von [SDK08](SDK08.md)

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
