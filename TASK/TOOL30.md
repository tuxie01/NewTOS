# TOOL30 — `akprobe`-Faelle nach tosdiff

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner pre-start check 2026-10-31: associated fix A126 accepted via probe | Reopen: the owner wants the probe moved to tosdiff

Migrated (owner batch 12, 2026-11-11), evidence: TASK/TOOL30.md status line: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: zugehoeriger Fix (A126) per Sonde abgenommen)

Line 3 before the migration:
Status: **GEPARKT** 2026-10-31 (Anwender, Vor-Start-Pruefung: zugehoeriger Fix (A126) per Sonde abgenommen) · Prioritaet: **P3** · Typ: Developer (Werkzeug) — Pflicht-Folgeticket der Sondenregel (CLAUDE.md "Testing") aus [A126](A126.md)

## Sachverhalt

A126 ist mit der dauerhaften Sonde `tosdump/probes/akprobe.c` abgenommen
(MEASURED). Zwei Teile:

- **Teil P** (das Programm setzt `Kbshift`, liest dann `graf_mkstate`,
  `evnt_multi` mit Timer, `evnt_button`): geht als Tabellenfall, sobald der
  AES-Dumper vor einem Fall `Kbshift(flag)` setzen darf — die Aufrufregel von
  DUMPBIOS laesst heute nur Abfragen zu.
- **Teil K** (gehaltene Tasten CapsLock + Alt): braucht Tasten- und
  Maus-Injektion im tosdump-Runner.

## Abnahme

- Neue Tabelle (eingefrorene bleiben unveraendert), beide Seiten, Ergebnis
  gleich der `akprobe`-Messung.

[Gemeinsame Regeln](README.md) gelten.
