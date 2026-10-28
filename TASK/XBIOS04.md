# XBIOS04 — `Ikbdws` ist ein No-op: IKBD-Kommandos erzeugen keine Antwortpakete

Status: **OFFEN** · Prioritaet: **P2** · Typ: Researcher (Umfang entscheiden), danach Developer — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-xbios-chardev.md`

## Sachverhalt

**XBIOS-CHARDEV-004 — S3 — MEASURED — NOT LOCALIZED.** Beleg: Bericht Abschnitt 2, Lauf `measurements/measurement-2026-10-28-xbios-chardev-audit/RUNINFO.TXT`.

- Sonde W01–W04: `Ikbdws(0,{0x1C})` (Uhr abfragen), `{0x88}` (Mausmodus erfragen), `{0x16}` (Joystick abfragen) — EmuTOS ruft je den Haken einmal, NewTOS nie (`measurements/measurement-2026-10-28-xbios-chardev-audit/runs/PROBE-EMUTOS-tt`, `measurements/measurement-2026-10-28-xbios-chardev-audit/runs/PROBE-NEWTOS-raspi4b`). Rueckgabe `void` (EmuTOS `d0` = 2, NewTOS 0; nicht verglichen).
- Vermutete Wurzel (nicht lokalisiert): `tos-api/src/xbios/system.rs:152@ikbdws` (gibt 0 zurueck, ignoriert beide Argumente). Die `--down`-Abhaengigkeit `bcostat4` ist DIFF (BIOS-CORE-001, [BIOS04](BIOS04.md)): `bios-r1` 67 muss erst gleich sein.
- EmuTOS-Stelle: `references/emutos/bios/ikbd.c:948-954`.
- Spec: `docs/api/xbios/19-ikbdws.md` (No-op, Interpreter „does not exist yet“). IKBD-Uhr und Joystick fallen unter die Hardware-Nicht-Ziele; Mausmodus-Kommandos (0x08/0x09/0x0A/0x12) betreffen beobachtbares Verhalten, nicht gemessen (Bericht Abschnitt 6).
- Fehlerbild: Uhr per `0x1C` lesen und auf das Paket warten haengt; Maus per `0x12` abschalten wirkt nicht.

## Arbeit

1. Researcher: Umfang festlegen — COMPATIBILITY-Eintrag (No-op) oder Interpreter fuer die beobachtbaren Kommandos; Mausmodus-Kommandos mit einem Fall messen (Bericht Abschnitt 6, offen).
2. Danach (Developer) nur nach freigegebener Spec; kein Stub.

## Abnahme

- Bestaetigender Fall: Sonde W01–W04 gleich dem Orakel (je ein Haken-Aufruf), sofern die Entscheidung einen Interpreter verlangt; bei Entscheidung „No-op“ steht der Eintrag in `docs/COMPATIBILITY.md` und der Fall wird ACCEPTED.
- Kein vorher gleicher Fall in `bios-r1.tsv`, `bios-core-r1.tsv`, `xbios-chardev-r1.tsv`, `xbios-system-r1.tsv` weicht neu ab.
- `cargo xtask test` gruen.
- `docs/STATUS.md` und Spec im selben Commit; `cargo xtask doc-check` der geaenderten Doku 0 Fehler.
