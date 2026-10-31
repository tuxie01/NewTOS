# PITEST01 — Testkarte und Ablauf fuer die Tester (echte Pi 4)

Status: **OFFEN** · Prioritaet: **P2** · Typ: Developer (Sonden, Debug-Feature, Werkzeug) — Anwenderentscheidung 2026-10-31

Grundlage: `docs/analysis/tester-scenario-draft-2026-10-31.md` (Entwurf, Schritte
A0–A6, B1–B2).

## Anwenderentscheidungen 2026-10-31

1. **Automatische Wiederholung von A1 (P0PAR): Option (c).** Ein Debug-Feature
   nur im Phase-B-Kern: ein Absturzbericht wird protokolliert und das Programm
   ohne Tastenwarten beendet (`crash=nowait`-Verhalten), dazu eine
   P0PAR-Variante, die jeden Fall vielfach wiederholt und ein Protokoll auf
   `C:` schreibt. Der Release-Kern bleibt unberuehrt (Feature aus =
   byte-gleich bis auf `panic::Location`, CLAUDE.md "Testing").
2. **Kartenweg:** neues Skript, das die Build-Ausgabe auf eine **bestehende**
   Pi-Karte (mit Firmware-Startpartition) abgleicht, jede Datei per sha256
   gegen die Build-Ausgabe prueft und ein Kartenmanifest mit `host_clang` und
   Kern-Commit schreibt. Kein voller Abbildbau.
3. **HZPROBE** fuer A2 (`_hz_200` im Anwendermodus) und ein **einseitiges
   Ablaufblatt** fuer die Tester.
4. Melden, wenn alles bereit ist; **die Karte erst nach Bestaetigung des
   Anwenders bespielen.**

## Arbeit

- Teil 1 (Developer): Debug-Feature + P0PAR-Wiederholungsvariante + HZPROBE,
  in QEMU raspi4b geprueft.
- Teil 2 (Developer): Karten-Abgleichskript (gegen ein Abbild im Scratch
  getestet, nie gegen eine echte Karte) + Ablaufblatt.

## Abnahme

- QEMU: P0PAR-Dauerlauf und HZPROBE laufen ohne Tester-Eingriff und schreiben
  ihr Protokoll; Release-Kern byte-gleich (Feature-Pruefung).
- Abgleichskript: Probe gegen ein Scratch-Abbild mit Firmware-Partition;
  Manifest mit `host_clang` und Kern-Commit; absichtlich verfaelschte Datei
  wird erkannt.
- Hosttests gruen.

[Gemeinsame Regeln](README.md) gelten.
