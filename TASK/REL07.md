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

1. **Version aus einer Quelle** (Anwender 2026-11-04: "mach es richtig"):
   die Nummer steht einmal in der Workspace-`Cargo.toml`
   (`[workspace.package] version = "0.7.0"`, die Crates erben sie);
   `kernel/src/initinfo.rs@VERSION_STRING` wird daraus gebildet
   (`concat!("NewTOS ", ...)`, Anzeige "NewTOS 0.7"); der `ver`-Befehl in
   `kernel/src/shell.rs` nutzt `VERSION_STRING`; der Desktop liest die Version
   zur Laufzeit ueber `Ssystem(2019)` (`S_NEWTOS_KVERSION`) statt seiner Kopie
   `programs/desktop/src/inf.rs@VERSION_STRING` (Weg a: About zeigt den Kern,
   der wirklich laeuft). Specs, die den Text zeigen (`docs/api/desktop/`),
   anpassen. TOS-sichtbare Versionen (`_TOS`, `NTOS`-Cookie, `Sversion`)
   bleiben. Eigener Developer-Auftrag.
2. **Paket** wie `v0.6.1` (`NewTOS-0.6.1.zip`): Ordner NewTOS-0.7 mit
   dem Kartenordner KARTE (Firmware `start4.elf`, `fixup4.dat`, `bcm2711-rpi-4-b.dtb`,
   `config.txt`, `LICENCE.broadcom` daneben; `kernel8.img`; `NEWDESK.INF`,
   `TERADESK.INF`, `KEYBOARD.KBD`, GEMSYS\RSC, ACC\SYSACC.NAC; APPS:
   DOOM, BALL, ZVIEW mit Codecs und Icons, QED, COREMARK, TERADESK, HYPVIEW),
   dazu README/LIESMICH/TESTING/TESTANLEITUNG/FEEDBACK(-EN). Jede Datei aus dem
   Stand des Release-Commits gebaut bzw. benannt; Liste mit sha256 in einem
   Messlauf. Umfang wie 0.6.1 (Anwender hat nichts geaendert, 2026-11-04). Eigener Auftrag nach Schritt 1.
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

## Stand Schritt 1 (Developer, 2026-11-04, nicht committet)

- **Eine Quelle:** `[workspace.package] version = "0.7.0"` in der Wurzel-`Cargo.toml`;
  `kernel`, `tos-api`, `tos-types` erben sie (`version.workspace = true`), `Cargo.lock` und
  `programs/Cargo.lock` (tos-types) zeigen 0.7.0. `tools/xtask` behaelt 0.1.0: ein Host-Werkzeug,
  das nicht ausgeliefert wird.
- **Anzeigeregel** = EmuTOS' eigene (`references/emutos/version.mk:22-39`): Fix-Stand 0 →
  `x.y`, sonst `x.y.z`; `0.7.0` erscheint als „NewTOS 0.7". Ein Pre-Release-Suffix bricht den
  Bau ab (const-Assert), statt still zu verschwinden.
  `kernel/src/initinfo.rs@VERSION_STRING` wird zur Uebersetzungszeit aus
  `CARGO_PKG_VERSION_MAJOR/MINOR/PATCH` gebildet; `ver` (`kernel/src/shell.rs`) nutzt es.
- **Desktop:** Kopie `VERSION_STRING` entfernt; About liest `Ssystem(2019)` ueber die neue
  Bindung `programs/newtos-gem/src/gemdos.rs@dos_kversion` und zeigt den Text vor dem
  Baustempel (die letzten zwei Felder, jetzt in `docs/api/gemdos/154-ssystem.md`
  festgeschrieben). **Fehlschlag** (`EINVFN`/`ERROR`): Wert leer, das Label steht allein —
  EmuTOS' Regel fuer einen Dialogwert, den GEMDOS nicht liefert (`deskinf.c:682-683`,
  `inf_disk`). Vom Developer gewaehlt; Bestaetigung durch Lead/Anwender offen.
- TOS-sichtbare Versionen unveraendert (`_TOS`/OSHEADER 4.04, `NTOS`-Cookie 0x0100,
  `Sversion`): im Diff nicht beruehrt.
- Beleg: `measurements/measurement-2026-11-04-rel07-version/RUNINFO.TXT` (Bootschirm, About,
  `ver`, About mit EINVFN-Kern). `cargo xtask test` 1958/0/1.

## Stand Schritt 2 (Developer, 2026-11-04, nicht committet)

- **Paket-Entwurf** `NewTOS-0.7.zip` aus `797e0078` (nur im Scratch, nicht im Repo; Pfad und
  sha256 `895e294d…` im Messlauf). Gleiche 74 Namen wie `NewTOS-0.6.1.zip` (bis auf den
  Ordnernamen), keine `._`/`.DS_Store`, `kernel8.img` = `--release --features crash-walk` mit
  "NewTOS 0.7" und "(797e0078)". 25 Dateien neu gebaut und anders als 0.6.1 (Kern, alle
  Programme ausser SYSACC, die zView-Codecs, `GEM.RSC`, die Texte), 33 gleich.
- **Herkunft** jeder Datei (Build-Schritt oder "aus 0.6.1 uebernommen") in
  `measurements/measurement-2026-11-04-rel07-package/RUNINFO.TXT`. Ohne Quelle im Repo und
  daher aus 0.6.1 uebernommen: Firmware (die einzige andere lokale Kopie ist aelter),
  `LICENCE.broadcom`, `NEWDESK.INF`, `ZVIEW.INF`, `QED\CONFIG.TXT`, `DOOM1.WAD`, `QED.HYP`,
  `TERADESK.HYP`. Ball wird aus einer Kopie von `/Volumes/Daten/Development/ball-demo`
  gebaut (ausserhalb des Repos, ohne Versionskontrolle).
- **Werkzeug:** `measurements/measurement-2026-11-04-rel07-package/method/package-release.sh`
  (`<version> <work-dir> <vorheriges Release>`), baut alles, prueft Namen, Stempel, 8.3,
  AppleDouble. Es lehnt Texte mit `TODO-REL`-Zeilen ab; der jetzige Entwurf lief mit
  `ALLOW_TODO=1`.
- **Texte** `docs/release/v0.7/`: die Texte von 0.6.1 mit Versionsnummer und Tag `v0.7`;
  14 Zeilen `TODO-REL07:` markieren, was Schritt 3 inhaltlich schreiben muss. Nach Schritt 3
  das ZIP mit demselben Werkzeug neu bauen (ohne `ALLOW_TODO`).
- **Rauchtest** QEMU raspi4b: Bootschirm "NewTOS 0.7", Desktop mit Laufwerk C:, About
  "NewTOS 0.7", zView und QED per `#Z`, zViews Dateiauswahl per Injektor. Die echte Pi bleibt
  Schritt 4.

## Entscheidungen nach Schritt 2 (Anwender 2026-11-04)

- About-Box bei fehlgeschlagenem `Ssystem(2019)`: Wert **leer** (wie umgesetzt,
  EmuTOS `inf_disk`-Muster).
- **Ball faellt aus dem Paket** (Quelle ausserhalb des Repos, unversioniert):
  `APPS\BALL\BALL.NAP`, `BALL.CFG`, `BALL.NTX` entfallen; Texte und
  Paketskript entsprechend.
- Rueckmeldekanal in allen Texten einheitlich: **GitHub Issues** (Bug-Report-
  Vorlage), keine Mailingliste.
- Release-Texte: Lead entwirft (Researcher sammelt die testerrelevanten
  Aenderungen seit `v0.6.1`), Anwender gibt frei.
- Firmware, `NEWDESK.INF` (Auslieferungsstand, nicht die persoenliche der
  Anwenderkarte), `ZVIEW.INF`, `QED\CONFIG.TXT`, `DOOM1.WAD`, HYP-Dateien,
  `GEM.RSC` wie in 0.6.1 (keine Einwaende).

## Stand Schritt 2b (finales Paket) (Developer, 2026-11-04, nicht committet)

- **Werkzeug** `tools/release/package-release.sh` (neu, aus dem Skript des Entwurfslaufs):
  Ball entfernt; Texte aus `docs/release/v<version>/`, jedes `TODO-REL` wird vor dem ersten
  Bau abgelehnt, ohne Override (`ALLOW_TODO` gestrichen); `docs/release/` muss committet
  sein. Neue Parameter: das vorige Release als ZIP (oder Ordner) und
  `NAMES_REMOVED`/`NAMES_ADDED`, damit die Namenslisten-Pruefung gegen das vorige Release
  streng bleibt. Neu geprueft: das ZIP entpackt sich bytegleich zum Ordner. Kopf =
  Aufruf, Zusicherungen, Datei hinzufuegen/entfernen.
- **Paket** `NewTOS-0.7.zip` aus `d7711c42` (nur im Scratch, daneben der entpackte Ordner):
  sha256 `a5191394…`, 6602464 Byte, 70 Eintraege (55 Dateien). Gegenueber dem Entwurf: die
  vier Ball-Eintraege fehlen (Ordner `APPS\BALL`, `BALL.NAP`, `BALL.CFG`, `BALL.NTX`),
  `kernel8.img` traegt "(d7711c42)", die sechs Texte sind die freigegebenen (bytegleich
  `HEAD`); alles andere bytegleich. Alle Pruefungen PASS.
- **Rauchtest** QEMU raspi4b aus genau KARTE: Bootschirm "NewTOS 0.7" / "(d7711c42)",
  Desktop mit Laufwerk C:, zView per `#Z` vollflaechig mit Menue. Die echte Pi bleibt
  Schritt 4.
- Beleg: `measurements/measurement-2026-11-04-rel07-final/RUNINFO.TXT` (Dateiliste mit
  sha256 und Herkunft, Unterschiede zum Entwurf und zu 0.6.1).

## Stand Schritt 4 (Abnahme, Anwender 2026-11-04)

Release-Kern `8ec56ee7…` (aus `NewTOS-0.7.zip`, Stand `d7711c42`) auf der
Anwenderkarte: *„soweit passt alles, die Version wird auch richtig
reported“* (`docs/HW-TEST-LOG.md`). Naechster Schritt: Veroeffentlichen erst
nach ausdruecklichem Go (Push, Tag `v0.7`, Pre-Release mit ZIP), dann Texte an
die Tester zur Freigabe.

[Gemeinsame Regeln](README.md) gelten.
