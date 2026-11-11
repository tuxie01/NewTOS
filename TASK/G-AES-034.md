# G-AES-034 — menu_text (AES 34): kleinsten Messfall entwerfen

Status: OBSOLETE (2026-11-11) | Priority: P3 | Reason: superseded by inventory-2026-11-11 (gaps regenerated from the graph)

Migrated (owner batch 1, 2026-11-11), evidence: TASK/G-AES-034.md BACKLOG measurement-design stub (README G-* list), no receipt, no commit names the id

Line 3 before the migration:
Status: **BACKLOG** · Priorität: **P3** · Typ: Funktionsweise ungemessen / Messdesign

Abhängigkeiten: Keine

## Begrenzter Auftrag

Nur menu_text, Opcode 34. Die aktuelle Runde enthält keinen gemessenen Record für diese Funktion. Zuerst Eingang/Ausgang, Seiteneffekt, Referenz-Build-Verfügbarkeit und Reset aus der Spezifikation erheben; genau einen Normalfall entwerfen. Keine pauschale Funktionsimplementierung und keine Änderung der Sperrliste ohne Quellenbeleg.

## Abnahme

Eine funktionsspezifische Fallskizze mit exakten Parametern, erwarteten Feldern/Pixeln und Rückbau des Zustands liegt vor. Wenn blockierend, zeigerabhängig oder auf TT nicht verfügbar: benötigte Voraussetzung konkret benennen; keinen unsicheren Lauf ausführen. Runner-Erweiterung, TT-/Pi-Aufnahme und etwaige Reparatur als getrennte Nachfolgetasks anlegen.



## Einstiegspunkte

- [tosdump/cases/COVERAGE-AES.md](../tosdump/cases/COVERAGE-AES.md)
- [docs/api/aes/_dispatcher.md](../docs/api/aes/_dispatcher.md)

- [docs/api/aes/menu.md](../docs/api/aes/menu.md)

## Arbeitsregel

[Gemeinsame Regeln und Messgrenzen](README.md) gelten. Bestehende Tests und Entscheidungen zuerst prüfen; nur fehlende Prüfungen ergänzen. Ein Messunterschied ist ohne Einordnung kein Fehlerauftrag. Abschluss hier mit Commit bzw. Artefakt, ausgeführten Prüfungen und verbleibender Hardwareabnahme festhalten.
