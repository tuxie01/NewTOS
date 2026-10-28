# XBIOS10 — `NVMaccess` ist nicht verdrahtet: jede Operation liefert `0x2E`

Status: **OFFEN** · Prioritaet: **P2** · Typ: Researcher (Entscheidung „kein NVRAM wie EmuTOS ohne Chip“ + COMPATIBILITY, oder emuliertes 50-Byte-NVRAM → Developer) — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-xbios-system.md`

## Sachverhalt

**XBIOS-SYS-003 — S3 — CONFIRMED** (S2, falls das Falcon-Profil NVRAM zeigen muss). Beleg: Bericht Abschnitt 2, Lauf `measurements/measurement-2026-10-28-xbios-system-audit/RUNINFO.TXT`.

- Faelle: `bios-r1` 189–194 und `xbios-system-r1` 68–76, alle EXACT-DIFF (EmuTOS `0x0000` bzw. `0xFFFB`, NewTOS `0x002E`; beim Lesen bleibt der Puffer vergiftet) (`measurements/measurement-2026-10-28-xbios-system-audit/runs/NEWTOS-bios-r1/tosdiff-newtos-vs-emutos.txt`, `measurements/measurement-2026-10-28-xbios-system-audit/runs/NEWTOS-xbios-system-r1/tosdiff-newtos-vs-emutos.txt`; falcontc dieselben Fall-IDs).
- Eingabepruefung: EmuTOS — 9 Aufrufe ohne Argumente im Trace (`measurements/measurement-2026-10-28-xbios-system-audit/runs/TRACE-EMUTOS-xs/trace-program.txt`), Ergebnisse passen zu den Armen der Tabelle (Lesen 0, Bereichs-/Op-Fehler -5); NewTOS — die Antwort haengt von keinem Argument ab.
- Lokalisiert: `tos-api/src/xbios/mod.rs:138@other` (Auffangarm).
- EmuTOS-Stelle: `references/emutos/bios/nvram.c:155-214`. Auffaellig, nicht gemessen: `0x2E` ist EmuTOS' Antwort auf einer Maschine ohne NVRAM (`references/emutos/bios/nvram.c:161-162`, `!has_nvram`); ein gepinntes Profil ohne NVRAM gibt es nicht.
- Spec widerspruechlich: `docs/api/xbios/_triage.md:80` („`CONF_WITH_NVRAM` off“) gegen `docs/api/xbios/_full-function-inventory.md` (`make 512`: `CONF_WITH_NVRAM` = 1); `docs/COMPATIBILITY.md` nur im Tastaturabschnitt „NewTOS has no NVRAM“.
- Fehlerbild: Falcon-/TT-Kontrollfelder sehen „kein NVRAM“ und fallen zurueck; Daten gehen nicht verloren.

## Arbeit

1. Entscheidung vorlegen: „kein NVRAM“ mit COMPATIBILITY-Eintrag oder emuliertes NVRAM; Spec widerspruchsfrei machen ([XBIOS13](XBIOS13.md)).
2. Developer-Ticket nur bei Entscheidung fuer NVRAM; Fix gegen `references/emutos/bios/nvram.c:155-214`.

## Abnahme

- Bei Entscheidung NVRAM: `bios-r1` 189–194 und `xbios-system-r1` 68–76 EQUAL gegen das Orakel (tt und falcontc); sonst Eintrag in `docs/COMPATIBILITY.md` und die Faelle ACCEPTED.
- Kein vorher gleicher Fall in den vier Tabellen weicht neu ab; `cargo xtask test` gruen; `cargo xtask doc-check` der geaenderten Doku 0 Fehler.
