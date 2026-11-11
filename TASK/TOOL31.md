# TOOL31 — `sbprobe`-Faelle nach tosdiff

Status: PARKED (2026-11-11) | Priority: P3 | Reason: associated fix A128 accepted via probe, tool ticket behind the product fixes (2026-11-04) | Reopen: the owner wants the probe moved to tosdiff

Migrated (owner batch 12, 2026-11-11), evidence: TASK/TOOL31.md status line: GEPARKT 2026-11-04 (zugehoeriger Fix A128 per Sonde abgenommen, Werkzeug-Ticket hinter den Produkt-Fixes)

Line 3 before the migration:
Status: **GEPARKT** 2026-11-04 (zugehoeriger Fix A128 per Sonde abgenommen; Werkzeug-Ticket hinter den Produkt-Fixes) · Prioritaet: **P3** · Typ: Developer (Werkzeug) — Pflicht-Folgeticket der Sondenregel (CLAUDE.md "Testing") aus [A128](A128.md)

## Sachverhalt

Der `graf_slidebox`-Teil von A128 ist mit der dauerhaften Sonde
`tosdump/probes/sbprobe.c` abgenommen (13 Faelle ohne Ziehen, Taste oben,
gleich dem Orakel; `measurements/measurement-2026-11-04-a128/RUNINFO.TXT`).
Faelle mit Ziehen brauchen Maus-Injektion im tosdump-Runner.

## Abnahme

- Neue Tabelle (eingefrorene bleiben unveraendert), beide Seiten, Ergebnis
  gleich der `sbprobe`-Messung.

[Gemeinsame Regeln](README.md) gelten.
