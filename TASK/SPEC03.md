# SPEC03 — Altlast der Doku-Fehler abbauen (doc-check ohne Dateiliste)

Status: **GEPARKT** 2026-10-31 (Anwender, Vor-Start-Pruefung: reine Aufraeumarbeit; der Hook blockiert die Altlast nicht) · Prioritaet: **P3** · Typ: Researcher (Doku), stapelweise — Anwenderentscheidung 2026-10-30

## Sachverhalt

- `cargo xtask doc-check` ohne Dateiliste prueft den Standardsatz:
  `CLAUDE.md` plus jedes `*.md` unter `docs/`, ohne `docs/archive/` und ohne
  datierte Snapshots (Modul-Doku "Default set" in `tools/xtask/src/doc_check.rs`,
  `ARCHIVE_DIR`, `is_dated_snapshot`). Stand 2026-10-30: **769 Fehler**.
- Die Arbeitslaeufe zeigten 0, weil CLAUDE.md ("Commit Discipline") verlangt,
  nur die bearbeiteten Dateien zu pruefen (`doc-check <files you edited>`);
  ein Filter verbirgt nichts. CLAUDE.md nennt die Altlast (752 am 2026-10-27).
- Verteilung (Top): `docs/analysis/desktop-userspace-plan-v2.md` 74,
  `docs/analysis/coop-kernel/` 73, `docs/analysis/desktop-userspace-plan.md` 72,
  `docs/analysis/aes-audit/` 72, `docs/INVENTORY.md` 66,
  `docs/analysis/aes-abi-trace/` 39, `docs/analysis/aes-object-form-render-audit.md` 38,
  `docs/analysis/emutos-full-function-inventory.md` 34, `docs/HARDWARE-PROFILE.md` 27.

## Arbeit

1. Stapelweise abbauen (je Stapel ein Verzeichnis oder eine Datei, eigener
   Commit): Zitat berichtigen, mit Anker versehen, oder — wenn das Dokument
   Geschichte ist — datieren bzw. nach `docs/archive/` verschieben (Regel in
   CLAUDE.md "Placement rules").
2. Nach jedem Stapel die Gesamtzahl im Ticket festhalten.

## Abgrenzung

Der pre-commit-Hook ([TOOL29](TOOL29.md)) lehnt nur Fehler ab, die ein Commit
neu einfuehrt; die Altlast blockiert keinen Commit.

## Abnahme

- `cargo xtask doc-check` ohne Dateiliste meldet 0 Fehler.

[Gemeinsame Regeln](README.md) gelten.
