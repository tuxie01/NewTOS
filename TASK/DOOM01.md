# DOOM01 — DOOM friert auf der Pi im Hauptmenue ein (Pfeiltasten)

Status: **ZURUECKGESTELLT** (Anwender 2026-10-29) · Prioritaet: **P2** · Typ: Diagnose zuerst (Pi), danach Developer — Anwenderbefund Pi-Runde 2026-10-29

Abhaengigkeiten: —

## Befund (Anwender, echte Pi 4, Release-Kernel 58e35ac8, neu gebautes DOOM.NAP)

- DOOM starten, Esc ins Hauptmenue, mit den Pfeiltasten hoch/runter.
- 1. Versuch: kurzer Haenger, dann lief DOOM weiter. 2. Versuch: Bild und Menue
  stehen dauerhaft.
- Strg+Alt+Entf startet neu: der Kern lebt, die Tastenpumpe laeuft.
- Mit der vorigen Karte (Kernel vom 2026-10-24, damaliges DOOM.NAP) nicht
  gesehen. Kein serielles Protokoll.
- Einschaetzung des Anwenders: eher DOOM bzw. SDK als Kern — offen.

## Bisher (QEMU raspi4b, nicht reproduziert)

`measurements/measurement-2026-10-29-doom-freeze/RUNINFO.TXT`: 11 Laeufe
(Hauptmenue hoch/runter, gehalten, schnell, mit Accessories, alter/neuer Kern
x alter/neuer DOOM), 0 Haenger, keine Latenzspitze am Hook, DOOMs Uhr nie
zurueck. Statischer Rundweg einer Taste (USB-Poll, `route_key`, Upcall,
Hook, `Ssystem(2002)`, `Psigreturn`) im Abschnitt FOLLOW-UP.

Strg+Alt+Entf schliesst einen Haenger im Hook-Upcall und eine stehende
KBDVEC-Maske aus (dort wird nicht gepumpt). Uebrig, beide unbelegt:

- **K5** DOOM stuerzt ab, der Kern wartet in der Absturzanzeige auf eine
  Taste.
- **K6** DOOMs Seite-0-`_hz_200` steht, DOOM bleibt in seiner Warteschleife
  (`TryRunTics`), Tasten laufen weiter.

## Richtung des Anwenders (2026-10-29)

*„Der Unterschied zwischen QEMU und Pi ist der Cache.“* QEMU bildet weder
I-/D-Cache noch TLB des Cortex-A72 nach. Nicht als Diagnose, sondern als Auftrag,
wo zu schauen ist:

- Code, den der Kern zur Laufzeit in den Prozess schreibt (Upcall-Trampolin,
  `KBDVEC_DEFAULT_CODE` in `tos-api/src/xbios/kbdvecs.rs`): folgt jeder
  Schreibung die I-Cache-Pflege (`dc cvau`/`ic ivau`/`dsb`/`isb`)?
- Seite 0 nur lesbar/beschreibbar je Upcall (MEM12, `set_page0_writable`):
  ist die TLB-Pflege vollstaendig und richtig geordnet (`dsb`/`isb`)?
- `_hz_200` in Seite 0 (`tick.rs` `publish`): erreicht die Schreibung des
  Kerns DOOMs Lesung ueber dieselbe physische Adresse und Attributlage?

## Arbeit

1. Pi: beim Freeze zuerst Leertaste, 3 s warten, dann Strg+Alt+Entf.
   Desktop kommt = K5, nichts = K6. Dazu `cmdline.txt` auf `asid=on` pruefen.
2. Pi: `DOOM-old.NAP` (DOOM vom 2026-10-24, altes SDK) auf dem neuen Kern und
   `DOOM-nokbdvec.NAP` (AES-Tastenweg) gegenpruefen — trennt Kern gegen
   DOOM/SDK-Bau. Sha256 im Laufverzeichnis.
3. Bei K6: Debug-Variante mit Schnappschuss bei Strg+Alt+Entf (DOOMs
   `_hz_200` gegen den Kernzaehler, ELR, `upcall_depth`, Maske, Ringe) —
   Produktaenderung, nur nach Freigabe.
4. Mit UART: `serial=on` liefert die `[crash]`-Zeile fuer K5.

## Abnahme

- Ursache mit Messung auf der Pi belegt; Fix nur nach Vorlage beim Anwender.

[Gemeinsame Regeln](README.md) gelten.
