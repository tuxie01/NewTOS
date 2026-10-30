# TOOL29 — doc-check als git-pre-commit-Hook

Status: **OFFEN** · Prioritaet: **P2** (klein) · Typ: Developer (Werkzeug) — Anwenderentscheidung 2026-10-30, direkt nach [A126](A126.md)

## Sachverhalt

- `CLAUDE.md` verlangt `cargo xtask doc-check` vor dem Commit. Am 2026-10-30
  ging ein Commit trotz doc-check-Fehler durch, weil die Befehlskette den
  Fehlercode nicht weitergab (`abc6276a`, berichtigt in `ebda3d72`).

## Arbeit

1. Eingechecktes Hook-Skript (pre-commit), das `cargo xtask doc-check` fuer die
   gestagten Doku-Dateien laeuft (mit der Pruefung "Anker auf neuen Zeilen"
   gegen `HEAD`) und den Commit bei einem Fehler ablehnt — unabhaengig davon,
   wie der Commit-Befehl verkettet ist.
2. Installationsschritt in der Setup-Doku bzw. als xtask-Befehl.
3. Laufzeit klein halten (nur gestagte Dateien).

## Abnahme

- Ein Commit mit einem kaputten Zeilenverweis wird abgelehnt, ein sauberer
  Commit geht durch (beides vorgefuehrt, im Ticket festgehalten).
- Hosttests gruen.

[Gemeinsame Regeln](README.md) gelten.
