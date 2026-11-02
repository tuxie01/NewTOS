# DOOM01 — DOOM friert auf der Pi im Hauptmenue ein (Pfeiltasten)

Status: **ZURUECKGESTELLT** 2026-11-02 (Anwender: erst [PROC08](PROC08.md); der naechste Schritt unten bleibt der Wiedereinstieg; vorher in Diagnose mit Pi-Laeufen 2026-10-31, zurueckgestellt 2026-10-29) · Prioritaet: **P2** · Typ: Diagnose zuerst (Pi), danach Developer — Anwenderbefund Pi-Runde 2026-10-29

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
  `1da96072c71554b9d61d8a1400baad2b415ed051660a5e29baab6be2be018884` (nachgetragen
  2026-10-31 aus der Kartensicherung,
  `measurements/measurement-2026-10-31-pitest01-card/RUNINFO.TXT` F2).
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

## Echte Pi, PITEST01 Phase A (2026-10-31)

Reproduziert mit dem Release-Kern aus `6796787c` (kernela.img
`e0fe680b25402385...`, `measurements/measurement-2026-10-31-pitest01-card/RUNINFO.TXT`)
und dem neu gebauten DOOM: Demo laeuft; Hauptmenue aufrufen und scrollen ->
Freeze; Strg+Alt+Entf startet neu (Anwender). P0PAR lief vorher ohne Befund
durch, HZPROBE (`_hz_200` im Anwender- und Super-Zustand) bestand
(`measurements/measurement-2026-10-31-pitest01-phase-a/RUNINFO.TXT`).
Vermutung des Anwenders: der `kbdvec`-Haken in DOOM selbst — offen, nicht
belegt. Naechste Trennschritte: Freeze in Phase B mit seriellem Protokoll
(sieht der Kern noch Aufrufe von DOOM?) und Gegenprobe `DOOM-nokbdvec.NAP`.

Gegenprobe vorbereitet (2026-10-31): `C:\Apps\DOOM\DOOMNOKB.NAP` = DOOM mit
`DOOM_ARGS "-mb","6","-nokbdvec"` (Tasten ueber die AES statt ueber den
`kbdvec`-Haken; `sdk/apps/build-doom.sh`), gebaut aus `32352665`, sha256
`af10a51706cb7c9f7b389ba76f6faabd94c9a96bc666fee79848f258db10a95a` — bytegleich
mit der nokbdvec-Variante vom 2026-10-29
(`measurements/measurement-2026-10-29-doom-freeze/RUNINFO.TXT`). Auf der Karte
nach Remount per sha256 geprueft; daneben `DOOM.NAP` `86abca15...` (die
Freeze-Variante), Kern Phase B `kernelb.img` `bc543d48...`, `serial=on`.


## Echte Pi, PITEST01 Phase B (2026-10-31)

Mit `kernelb.img` (`crash-nowait`, `fs-io-trace`) und `serial=on` friert
weder `DOOM.NAP` noch `DOOMNOKB.NAP` ein: Tastatur geht, Menue scrollt, Spiel
startet, beide enden mit rc=0, kein `[crash]`
(`measurements/measurement-2026-10-31-pitest01-phase-b/RUNINFO.TXT`). Jede
protokollierte AES/GEMDOS-Zeile kostet dort im Mittel 16,6 ms UART-Ausgabe —
der Lauf hat also ein ganz anderes Zeitverhalten. Die Hook-Vermutung ist damit
weder belegt noch widerlegt. Vorgeschlagene Trennung (Entscheid Owner): C1
`kernela.img` + `serial=on`, C2 `kernelb.img` ohne `serial=on`, je nur
`DOOM.NAP`.

## Trennlaeufe C1 und Kontrolle (2026-10-31)

- C1: Release-Kern `e0fe680b...` + `serial=on` — kein Freeze, `DOOM.NAP` und
  `DOOMNOKB.NAP` enden mit rc=0
  (`measurements/measurement-2026-10-31-pitest01-c1/RUNINFO.TXT`).
- Kontrolle (Anwender, ohne Mitschrift): derselbe Kern, `cmdline.txt` leer —
  Freeze beim zweiten Druck auf Pfeil-runter im Menue.
- Damit ist `serial=on` der Schalter. Laut Code schaltet er nur das Senden
  ein (`kernel/src/console.rs:91@configure_serial_tx`, `SerialConsole`
  "the switch is TX-only"): Unterschied sind also die UART-Schreibzugriffe
  selbst (Zeit, Device-Speicherzugriffe), nicht ein anderer Codepfad im
  Programm. Ursache offen; Ursachenforschung vor jedem Fix.

## M1: ohne X-Shot (2026-10-31, Anwender, ohne Mitschrift)

Release-Kern `e0fe680b...`, `cmdline.txt` leer (wie Phase A und Kontrolle):
- beide Accessories umbenannt (`XSHOT.NA_`, `SYSACC.NA_`): kein Freeze;
- nur `C:\ACC\XSHOT.NAC` umbenannt, `SYSACC.NAC` geladen: kein Freeze.

Der Freeze braucht also X-Shot im Accessory-Slot (Kandidaten:
`docs/analysis/doom01-root-cause-2026-10-31.md`). X-Shot wartet in
`evnt_multi(MU_MESAG|MU_BUTTON|MU_KEYBD)` ohne Timer (TASK/A117.md). Die
`.NAC` auf der Karte stammt nicht aus diesem Repo: Datei vom 2026-10-24,
119744 Bytes, aus dem X-Shot-Repo (dort dist/newtos/xshot.nac, GitHub #14), sha256 `766627b79f6e38b9...`; von
uns nie gebaut. Offen: ob ein Accessory, das auf Tasten wartet, bei EmuTOS
waehrend eines laufenden Programms ueberhaupt geweckt wird, und ob ein
Neubau von X-Shot etwas aendert.

## X-Shot neu gebaut (2026-10-31, Anwender, ohne Mitschrift)

X-Shot-Quelle `https://github.com/ragnar76/X-Shot` Commit `5ed45575a54f3b1d56c6cc5fd4a1645d6bc26151`
(dessen dist/newtos/xshot.nac = die alte Kartendatei `766627b7...`), mit dem
heutigen SDK gebaut (`sdk/bin/newtos-cc -Wall -nac -DXSHOT_VERSION='"1.0.1"'`),
sha256 `f7607bb141675eb656b3836b817e14d8be8dc2cd042c4c109ae7cad5dd294af9`, als
`C:\ACC\XSHOT.NAC` auf der Karte; Release-Kern, `cmdline.txt` leer:
**Freeze bleibt.** Das alte Kompilat ist nicht die Ursache; gesucht wird im
Kern (Zusammenspiel DOOM + auf Tasten wartendes Accessory ohne Timer).

## Stand 2026-11-02 und naechster Schritt

Belegt (echte Pi, Release-Kern `e0fe680b...`):

| Lauf | `serial=on` | Accessories | Freeze |
|---|---|---|---|
| Phase A, Kontrolle | nein | SYSACC + X-Shot (alt) | ja |
| C1 | ja | SYSACC + X-Shot (alt) | nein |
| M1 | nein | keine | nein |
| M1 | nein | nur SYSACC | nein |
| Neubau | nein | SYSACC + X-Shot (neu gebaut) | ja |

Phase B (Debug-Kern, `serial=on`): kein Freeze.

Unterschied der Accessories: SYSACC wartet im Ruhezustand mit `evnt_mesag`
(`sdk/examples/sysacc.c`, Kopfkommentar), X-Shot mit
`evnt_multi(EV_MESAG|EV_BUTTON|EV_KEYBD)` ohne Timer (X-Shot-Quelle xshot.c,
Zeile 1476 auf Commit `5ed45575`). In C1 wechselt der Kern waehrend DOOM 309-mal
je Richtung zwischen X-Shot (Slot 1) und DOOM (Slot 2); die einzige
Kernausgabe dort sind die `[sched]`-Zeilen dieser Wechsel.

Offene Frage (nicht belegt): Die Tasten gehen seit [KEYOWNER](KEYOWNER.md)
(erledigt 2026-10-08, nur QEMU) an den Eigentuemer wie EmuTOS `gl_mowner`.
Laut Researcher-Bericht (`docs/analysis/doom01-root-cause-2026-10-31.md`)
weckt NewTOS ein auf `MU_KEYBD` wartendes Accessory trotzdem bei jeder
Eingabe (Folgerung aus dem Code, nicht gezaehlt). Ob EmuTOS ein Accessory
weckt, dem die Tastatur nicht gehoert, ist nicht geprueft.

Naechster Schritt (vorgeschlagen, Start auf Wort des Anwenders): Researcher
unter diesem Ticket —
1. EmuTOS-Quelle: wer wird geweckt, wenn ein Programm laeuft und ein
   Accessory ohne Fenster auf `MU_KEYBD` wartet; Vergleich mit NewTOS.
2. QEMU raspi4b ohne serielle Ausgabe: DOOM + X-Shot oder ein Test-Accessory,
   das nur auf `MU_KEYBD` wartet, Tasten per Injektor (die QEMU-Laeufe vom
   2026-10-29 hatten zwei SYSACC, nie X-Shot); hoechstens drei Versuche.
3. Bericht vor jedem Fix. Bestaetigt sich das Wecken als EmuTOS-Abweichung:
   eigenes Ticket (Nachfolger KEYOWNER).
Danach [PROC08](PROC08.md) (ein Agent zur Zeit).
