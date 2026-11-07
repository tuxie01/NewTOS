# G-VDI-101 — v_clsvwk (VDI 101): kleinsten Messfall entwerfen

Status: **BACKLOG** · Priorität: **P3** · Typ: Funktionsweise ungemessen / Messdesign

Abhängigkeiten: Keine

## Begrenzter Auftrag

Nur v_clsvwk, Opcode 101. Die aktuelle Runde enthält keinen gemessenen Record für diese Funktion. Zuerst Eingang/Ausgang, Seiteneffekt, Referenz-Build-Verfügbarkeit und Reset aus der Spezifikation erheben; genau einen Normalfall entwerfen. Keine pauschale Funktionsimplementierung und keine Änderung der Sperrliste ohne Quellenbeleg.

Nachtrag 2026-11-07: Der Fbench-Bericht (`docs/analysis/fbench-functions-2026-11-07.md`) bestätigt: kein Fall, nur als Lebenszyklus genutzt; Fbench ruft die Funktion (`VDI.VWK`).

## Abnahme

Eine funktionsspezifische Fallskizze mit exakten Parametern, erwarteten Feldern/Pixeln und Rückbau des Zustands liegt vor. Wenn blockierend, zeigerabhängig oder auf TT nicht verfügbar: benötigte Voraussetzung konkret benennen; keinen unsicheren Lauf ausführen. Runner-Erweiterung, TT-/Pi-Aufnahme und etwaige Reparatur als getrennte Nachfolgetasks anlegen.



## Einstiegspunkte

- [tosdump/cases/COVERAGE.md](../tosdump/cases/COVERAGE.md)
- [docs/api/vdi/_dispatcher.md](../docs/api/vdi/_dispatcher.md)

## Arbeitsregel

[Gemeinsame Regeln und Messgrenzen](README.md) gelten. Bestehende Tests und Entscheidungen zuerst prüfen; nur fehlende Prüfungen ergänzen. Ein Messunterschied ist ohne Einordnung kein Fehlerauftrag. Abschluss hier mit Commit bzw. Artefakt, ausgeführten Prüfungen und verbleibender Hardwareabnahme festhalten.
