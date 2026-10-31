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

0. Pi, zuerst (Anwender 2026-10-30, aus MEM12): `P0PAR.NAP` + `P0KID.NAP`
   (`sdk/examples/p0par.c`, `sdk/examples/p0kid.c`) laufen lassen — prueft die
   TLB-Pflege beim Umschalten von Seite 0 auf echtem Silizium; ein `WRONG` bei
   Fall `S` oder `T` waere ein TLB-Fehler (Prueffolge in `TASK/MEM12.md`).
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

## Stand des Pi-Laufs vom 2026-10-29 (fuer eine spaetere Bisektion)

Anwender, 2026-10-30: zurueckgestellt bleibt es; hier der Stand der Karte, auf
der der Menue-Freeze auftrat. Die Karte wurde **nicht** mit `tools/mkcard.sh`
gebaut, sondern dateiweise aktualisiert (19 Dateien ersetzt) — es gibt daher
**kein** Kartenmanifest. Festgehalten ist, was ersetzt wurde:

- Kernel: `kernel8.img`, Release-Bau aus `58e35ac8` (Baum sauber), sha256
  beginnt mit `1da96072c71554b9`; der volle Hash ist nicht aufgezeichnet und
  beim naechsten Einstecken der Karte aus `kernel8.img` auf BOOT nachzutragen.
  Vorheriger Kernel als `KRN1024B.IMG` auf der Karte (sha256 beginnt mit
  `fe967851f17c`, Stand 2026-10-24).
- `C:\Apps\DOOM\DOOM.NAP`: sha256
  `86abca15e09202d8538afb8504f4835fdda2ecc5f33f74f5852570832e1df132`
  (= `measurements/measurement-2026-10-29-doom-freeze/RUNINFO.TXT`, „new“).
- Weitere ersetzte Dateien (sha256-Praefix, Quelle unveraendert seit dem
  Kopieren 2026-10-29 17:12): `C:\ACC\SYSACC.NAC` 812ddd574dda3f85,
  `C:\USBSNAP.NCP` 522c8e9423ffae24, `C:\VT52PI.NAP` d7861cfa087eb349,
  `C:\Apps\COREMARK\CORE.NCP` 1a93c4679784517e, `COREVAL.NCP` 18728315e8d5317c,
  `C:\Apps\HYPVIEW\HYP_VIEW.NAP` 12b27a3a85656210, `C:\Apps\PRESENT\PRESENT.NAP`
  1433062a1ad0d434, `C:\Apps\QED\QED.NAP` a8777bbf7e83fa27,
  `C:\Apps\TERADESK\DESKTOP.NAP` 48dbf1c5dcbb98d0, `C:\Apps\ZVIEW\ZVIEW.NAP`
  efb1aebbd303ba57 und die zehn `CODECS/*.NLD`. Die `.rsc`/`.RSC` waren
  identisch und wurden nicht kopiert.
- Bisektionsgrenzen: Kern `795066d9` (2026-10-24, kein Freeze gesehen) bis
  `58e35ac8`; DOOM vom 2026-10-24 (`bec47b14...`) bis `86abca15...`.

## Querverweis (2026-10-31)

[TICK01](TICK01.md) (gemessen): `$4BA` wird nur in die Seite 0 des laufenden
Prozesses geschrieben; ein Prozess, der nicht lief, liest bis zu seinem
naechsten Tick einen veralteten Wert. Geparkt — wieder aufnehmen, wenn die
Tester Fehlerbild K6 ("DOOMs `_hz_200` steht") sehen.

