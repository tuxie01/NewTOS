# G-AES-043 — objc_find (AES 43): kleinsten Messfall entwerfen

Status: **BACKLOG** · Priorität: **P3** · Typ: Funktionsweise ungemessen / Messdesign

Abhängigkeiten: Keine

## Begrenzter Auftrag

Nur objc_find, Opcode 43. Die aktuelle Runde enthält keinen gemessenen Record für diese Funktion. Zuerst Eingang/Ausgang, Seiteneffekt, Referenz-Build-Verfügbarkeit und Reset aus der Spezifikation erheben; genau einen Normalfall entwerfen. Keine pauschale Funktionsimplementierung und keine Änderung der Sperrliste ohne Quellenbeleg.

Nachtrag 2026-11-07: Der Fbench-Bericht (`docs/analysis/fbench-functions-2026-11-07.md`) bestätigt: kein Fall, kein Test; Fbench ruft die Funktion (`AES.OBJFIND`). Der Fall soll auch [TOOL33](TOOL33.md) Kandidat 11 treffen: `tos-api/src/aes/object.rs:4147@find_hit` rechnet mit dem nominalen statt dem 3D-vergrößerten Elternursprung (`references/emutos/aes/gemoblib.c:1066-1096`), gelesen, nicht gemessen.

## Abnahme

Eine funktionsspezifische Fallskizze mit exakten Parametern, erwarteten Feldern/Pixeln und Rückbau des Zustands liegt vor. Wenn blockierend, zeigerabhängig oder auf TT nicht verfügbar: benötigte Voraussetzung konkret benennen; keinen unsicheren Lauf ausführen. Runner-Erweiterung, TT-/Pi-Aufnahme und etwaige Reparatur als getrennte Nachfolgetasks anlegen.



## Einstiegspunkte

- [tosdump/cases/COVERAGE-AES.md](../tosdump/cases/COVERAGE-AES.md)
- [docs/api/aes/_dispatcher.md](../docs/api/aes/_dispatcher.md)

- [docs/api/aes/object-form.md](../docs/api/aes/object-form.md)

## Arbeitsregel

[Gemeinsame Regeln und Messgrenzen](README.md) gelten. Bestehende Tests und Entscheidungen zuerst prüfen; nur fehlende Prüfungen ergänzen. Ein Messunterschied ist ohne Einordnung kein Fehlerauftrag. Abschluss hier mit Commit bzw. Artefakt, ausgeführten Prüfungen und verbleibender Hardwareabnahme festhalten.
