# TOOL22 — `mapping.tsv`: TOOL17-Gruppen B, C, F einordnen

Status: **OFFEN** · Prioritaet: **P3** · Typ: Developer (Doku/Werkzeugdaten) — Anwenderentscheidung 2026-10-29 (Empfehlungen des Leads uebernommen), aus [TOOL17](TOOL17.md)

## Sachverhalt

Erhebung `docs/analysis/tool17-groups-bcf-survey-2026-10-29.md`. Entschieden:

- **Gruppe B** (XHDI 22 Zeilen, SCSIDRV 16 Zeilen): `design:ADR-032@docs/MEMORY.md:<zeile>`.
- **Gruppe C** (OS-Pool, DND-Baum): `design:ADR-033@docs/MEMORY.md:<zeile>`;
  `dopath` dort `mapped`, wo die Erhebung ein Gegenstueck nennt.
- **Gruppe F** (Start- und Laufzeitfunktionen): Zeilen mit Gegenstueck laut
  Erhebung Abschnitt „Group F“ werden `mapped`; `bdos_trap2` bleibt
  `unmapped` bis [PROC07](PROC07.md).

## Arbeit

1. Zeilen umstellen, Notiz je Zeile mit Gegenstueck bzw. ADR-Fundstelle.
2. `cargo xtask deps --coverage` vorher/nachher in die Quittung.

## Abnahme

- Hosttest der Klassenpruefung gruen (`cargo xtask test`); doc-check 0 Fehler;
  STATUS im selben Commit.

[Gemeinsame Regeln](README.md) gelten.
