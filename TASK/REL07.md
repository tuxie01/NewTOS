# REL07 — Release NewTOS 0.7 (Alpha) fuer die Tester

Status: **OFFEN** · Prioritaet: **P1** · Typ: Release — Anwenderentscheidung 2026-11-04

## Anlass

Seit `v0.6.1` (2026-10-22) 404 Commits, darunter fuer die externen Tester
sichtbar: Dateiauswahl mit bedienbarem Schieber und Mausrad (A128, GitHub #9),
Funkempfaenger mit Maus auf dem zweiten Interface (USB01 F3, #3), der
PROC08-Fix (Absturz nach `Pexec(4)` in zView), die BIOS/XBIOS-Audit-Fixes,
MEM12. Anwender 2026-11-04: Ragnar informieren und ein neues Image
bereitstellen.

## Entscheidungen (Anwender 2026-11-04)

- Version **0.7**.
- Der Release-Kern enthaelt den Absturzbericht mit Diagnoseblock
  (`--features crash-walk`, mit der Leiter aus PROC08 "Stand Leiter", sobald
  sie fertig ist): ein Foto vom Absturz eines Testers liefert dieselbe
  Diagnose wie beim Anwender.
- Vor der Veroeffentlichung Abnahme auf der Pi des Anwenders aus dem frischen
  ZIP-Inhalt.

## Arbeit

1. **Version:** `0.6.1` -> `0.7` in `kernel/Cargo.toml`, `tos-api/Cargo.toml`,
   `tos-types/Cargo.toml`, `kernel/src/initinfo.rs@VERSION_STRING`,
   `kernel/src/shell.rs` (`ver`), `programs/desktop/src/inf.rs@VERSION_STRING`;
   Lock-Dateien nachziehen; Specs, die den Text zeigen (`docs/api/desktop/`),
   anpassen. TOS-sichtbare Versionen (`_TOS`, `NTOS`-Cookie) bleiben.
2. **Paket** wie `v0.6.1` (`NewTOS-0.6.1.zip`): Ordner NewTOS-0.7 mit
   dem Kartenordner KARTE (Firmware `start4.elf`, `fixup4.dat`, `bcm2711-rpi-4-b.dtb`,
   `config.txt`, `LICENCE.broadcom` daneben; `kernel8.img`; `NEWDESK.INF`,
   `TERADESK.INF`, `KEYBOARD.KBD`, GEMSYS\RSC, ACC\SYSACC.NAC; APPS:
   DOOM, BALL, ZVIEW mit Codecs und Icons, QED, COREMARK, TERADESK, HYPVIEW),
   dazu README/LIESMICH/TESTING/TESTANLEITUNG/FEEDBACK(-EN). Jede Datei aus dem
   Stand des Release-Commits gebaut bzw. benannt; Liste mit sha256 in einem
   Messlauf. Was gegenueber 0.6.1 dazukommt oder wegfaellt, nennt der Anwender.
3. **Texte** im neuen Ordner docs/release/v0.7 (Englisch und Deutsch wie 0.6.1):
   Neuerungen seit 0.6.1, bekannte Grenzen, Bitte um Foto bei Absturz.
4. **Abnahme** auf der Pi des Anwenders aus dem ZIP-Inhalt: Bootschirm und
   About zeigen 0.7, Desktop, zView (Dateiauswahl), QED, HypView, SYSACC.
5. **Veroeffentlichen** nach Go des Anwenders: Tag `v0.7`, GitHub-Pre-Release
   mit ZIP; `docs/HW-TEST-LOG.md`- und STATUS-Eintrag.
6. **Tester informieren** (Text vorher dem Anwender vorlegen): Ragnar in #3
   und #9 (Dateiauswahl, Funkempfaenger, Bitte um erneuten Test der
   Menueleiste fuer [A127](A127.md)), wosch163 in #8 (Rad der Kabelmaeuse
   bleibt offen, [USB03](USB03.md)).

[Gemeinsame Regeln](README.md) gelten.
