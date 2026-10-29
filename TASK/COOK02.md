# COOK02 — `_AKP` folgt dem aktiven Tastaturlayout

Status: **OFFEN** · Prioritaet: **P3** · Typ: Developer — Anwenderentscheidung 2026-10-29 (Empfehlung des Leads)

## Sachverhalt

- EmuTOS baut `_AKP` aus Land und Tastatur (`references/emutos/bios/country.c:133-162`,
  `detect_akp`; Tastatur im niedrigen, Sprache im hohen Byte, `:98`, `:113`).
- NewTOS setzt `_AKP` und `_IDT` fest (`tos-api/src/cookie.rs:222@AKP`, „bring-up
  policy“); das Layout waehlt `kernel/src/keymap.rs:367@init_from_disk`
  unabhaengig davon (`KEYBOARD.KBD` oder `#K` in NEWDESK.INF). Programme, die
  `_AKP` lesen, sehen ein anderes Layout als das aktive.

## Entscheidung

- `_AKP`-Tastaturbyte = Kennung des aktiven Layouts (Zuordnung nach
  EmuTOS' Laendertabelle).
- Sprachbyte und `_IDT` bleiben fest, bis es eine Einstellung dafuer gibt
  (Einstellungen menschenlesbar, kein rohes `cmdline.txt`).

## Abnahme

- `jar:_AKP` je Layout gleich dem Orakel mit demselben Land; Hosttest;
  STATUS im selben Commit.

[Gemeinsame Regeln](README.md) gelten.
