# Testszenario echte Pi 4 — Entwurf (2026-10-31)

Entwurf fuer den Anwender und seinen Berater. **Noch keine Karte gebaut.**
Zweck: die Pruefungen, die nur auf echtem Silizium gehen, in einem Lauf der
Tester sammeln, statt sie je Ticket einzeln auf die Pi zu tragen (Anwender
2026-10-30).

## Stand und Karte

- **Kern:** Release-Bau eines festen Commits (`cargo xtask build --board rpi4
  --release`), der Commit-Hash und der sha256 von `kernel8.img` stehen im
  Lauf. Fuer FS26 T1/T2 ein **zweiter** Kern mit `--features fs-io-trace`
  (Debug-Feature, ausgeschaltet byte-gleich, `measurements/measurement-2026-10-31-fs26/RUNINFO.TXT`
  RESULT 6).
- **Karte:** Programmdateien wie die Alltagskarte, alle Programme aus einem
  Neubau; Manifest mit `optional_missing` und `host_clang`
  (`tools/sd-manifest.sh`).
  - **Offener Punkt:** `tools/mkcard.sh` baut ein **QEMU**-Abbild (MBR + FAT16
    `C:`) ohne die Firmware-Startpartition der Pi. Fuer die echte Karte fehlt
    ein Bauweg mit Firmware (`start4.elf`, `config.txt`, `cmdline.txt`) und
    Manifest. Vorschlag: `mkcard.sh` um eine Startpartition erweitern oder
    einen eigenen Weg fuer die Testkarte schreiben — vor dem Bau zu klaeren.
- **Zwei Phasen** (Reihenfolge wichtig):
  - **Phase A** — Release-Kern ohne Feature, **kein** UART-Adapter
    (Schritt 5 braucht eine offene RX-Leitung).
  - **Phase B** — Kern mit `fs-io-trace`, UART-Adapter angesteckt,
    `cmdline.txt` mit `serial=on`, serielles Protokoll mitschreiben.
- **Allgemein festhalten:** Datum/Uhrzeit, Pi-Modell und RAM, Kartenmodell,
  `cmdline.txt` (insbesondere `asid=on` ja/nein), Fotos mit sichtbarem
  Schirm. Nach dem Lauf: Karte in den Mac, `C:` komplett kopieren, sha256
  des Abbilds nach dem Lauf.

## Schritte

Bewertung: **PASS** / **FAIL** / **NICHT AUSGEFUEHRT** (mit Grund). Bei FAIL
nicht wiederholen, sondern Foto und weiter zum naechsten Schritt.

### Phase A (Release-Kern, ohne Adapter)

**A0 — Boot.**
Tun: Karte einlegen, einschalten. Erwartet: Desktop erscheint. Festhalten:
Foto des Desktops, Zeit bis Desktop (Stoppuhr). PASS: Desktop ohne
Absturzanzeige.

**A1 — P0PAR (MEM12, TLB beim Umschalten von Seite 0).**
Tun: `C:\P0PAR.NAP` doppelklicken; bei jedem der fuenf Absturzberichte eine
Taste. Erwartet: fuenf Berichte ("Memory violation: page-0 …", "execute-only
stub memory …"), dann auf dem Schirm jede Zeile `OK`, "own state after L: user
OK", **"P0PAR: PASS"** (`TASK/MEM12.md` Prueffolge Punkt 3). Festhalten: Foto
der Ergebniszeilen. PASS: "P0PAR: PASS". FAIL: jedes `WRONG` — besonders bei
Fall `S` oder `T` (Hinweis auf fehlende TLB-Pflege). Zweimal ausfuehren
(einmal direkt nach dem Boot, einmal nach Schritt A3).

**A2 — `_hz_200` zaehlt im Anwendermodus weiter (DOOM01 Kandidat K6).**
Tun: Sonde `HZPROBE` starten (siehe "Vorarbeiten"), 30 s laufen lassen.
Erwartet: der Zaehler in `$4BA` steigt im Anwendermodus um ~200 je Sekunde,
auch waehrend die Sonde laufend in den Super-Zustand und zurueck wechselt.
Festhalten: `C:\HZPROBE.LOG`, Foto der Schlusszeile. PASS: kein Stillstand
laenger als ein Tick gegenueber der Kern-Uhr, Summe 30 s ± 1 %.

**A3 — DOOM01 (Menue-Freeze mit Pfeiltasten).**
Tun: zuerst A1 wiederholen (Schritt 0 aus `TASK/DOOM01.md`). Dann DOOM
starten, Esc ins Hauptmenue, 2 Minuten lang mit den Pfeiltasten hoch/runter,
auch gedrueckt halten; dann ein Spiel starten, laufen, F10 → `y`.
Erwartet: kein Haenger, zurueck zum Desktop, Tastatur arbeitet.
Bei Freeze: **Leertaste, 3 s warten, dann Strg+Alt+Entf** — Desktop kommt =
Absturzanzeige wartete (K5), nichts passiert = DOOM steht in seiner Schleife
(K6). Festhalten: Foto beim Freeze, welche Reaktion, Minute des Auftretens,
`asid=on` ja/nein. PASS: 2 Minuten Menue ohne Freeze und sauberer Ausstieg.

**A4 — Alltagsprogramme.**
Je Programm: starten, eine typische Handlung, beenden; danach Desktop-Tastatur
und Maus pruefen.
- Desktop: Laufwerk C: oeffnen, Ordner wechseln, Datei kopieren/loeschen.
- TeraDesk: starten, C: oeffnen, Shift+Esc, beenden.
- QED: Datei oeffnen, tippen (Umlaute, Shift, Alt), speichern, beenden.
- HypView: Hilfe oeffnen, mehrmals Pfeil-runter, Link folgen.
- zView: Bild aus `SAMPLES` oeffnen, zoomen.
Festhalten: je Programm Foto eines Arbeitsschritts, Auffaelligkeiten. PASS:
kein Absturz, keine haengende Taste, Rueckkehr zum Desktop.

**A5 — BIOS09: AUX wartet ohne Kabel (kein Adapter angesteckt).**
Tun: `AWPROBE.NAP` starten (`tosdump/probes/awprobe.c`), auf "AWPROBE:WAIT"
warten, einige Tasten druecken, dann Strg+Alt+Entf. Erwartet: `Bconin(1)`
kehrt nicht zurueck; der `kbdvec`-Haken der Sonde zeigt je Taste Bytes; Strg+Alt+Entf
startet neu. Festhalten: Foto waehrend des Wartens, `C:\AWPROBE.LOG` falls
geschrieben. PASS: Warten + Haken-Bytes + Neustart. Hinweis: kehrt die erste
Runde einmal mit `0` zurueck und wartet die zweite, ist das die offene
RXD0-Leitung ([UART01](../../TASK/UART01.md), geparkt) — als Befund notieren,
kein FAIL von BIOS09.

**A6 — FS26 T3 (sauberer Wert, Release-Kern).**
Tun: `C:\AUTO\DCPROBE.NCP` liegt auf der Karte (laeuft beim Boot vor dem
Desktop, legt `C:\FS26D1..5` an und wieder weg); danach im Desktop dreimal
"Neuer Ordner" mit Stoppuhr. Festhalten: `C:\DCPROBE.LOG` (Ticks D01–D05,
× 5 ms), die drei Stoppuhrzeiten. PASS: die Werte liegen vor (es ist eine
Messung, kein Bestehen/Durchfallen).

### Phase B (Kern mit `fs-io-trace`, Adapter, `serial=on`)

**B1 — FS26 T1.** Boot mit `DCPROBE.NCP` in `C:\AUTO`. Festhalten: das
serielle Protokoll komplett (je `gemdos 0x39` die Zeilen `dur=` und
`dev w=19/19/<us>`, die 19 `W lba= us=`-Zeilen, die acht `bios 0x04`-Zeilen).

**B2 — FS26 T2.** Mit demselben Kern im Desktop dreimal "Neuer Ordner"
(C: oeffnen, Datei > Neuer Ordner, Name, Return). Festhalten: serielles
Protokoll; welcher der drei Durchgaenge langsam ist.
Auswertung (Lead): `measurements/measurement-2026-10-31-fs26/method/timeline.py`.
Entscheidung: Summe der `W us` nahe den beobachteten Sekunden → Kartenlatenz
(Fix im Schreibweg, Mehrsektor-Schreibung ist Anwenderentscheidung); `Dcreate`
klein, Neuzeichnen gross → GUI06.

## Vorarbeiten vor dem Kartenbau

1. **HZPROBE** (fuer A2) gibt es noch nicht: kleine Sonde (Muster der
   bestehenden unter `tosdump/probes/`), liest `$4BA` im Anwendermodus,
   wechselt dabei laufend per `Super` hin und her, vergleicht mit `Tgettime`,
   schreibt `C:\HZPROBE.LOG`. Vorher in QEMU pruefen.
2. **Kartenbauweg** mit Firmware-Startpartition und Manifest (siehe oben).
3. Alle Programme aus einem Neubau; `P0PAR.NAP`/`P0KID.NAP`
   (`sdk/examples/p0par.c`, `p0kid.c`), `AWPROBE.NAP`, `DCPROBE.NCP`,
   `HZPROBE.NAP` auf die Karte; `NEWDESK.INF` ohne `#Z`-Autostart (die Tester
   starten von Hand).
4. Ein Ablaufblatt fuer die Tester (eine Seite) aus diesem Entwurf.

## Offene Frage an den Anwender: automatische Wiederholung

A1 prueft die TLB-Pflege nur wenige Male (fuenf Fehlerfaelle, je eine Taste).
Ein zufaellig bestehender Lauf beweist wenig. Optionen:

| Option | Was | Kosten | Wirkung |
|---|---|---|---|
| (a) keine | A1 zweimal von Hand | 0 | schwach gegen seltene TLB-Fehler |
| (b) Bootargument `crash=nowait` (Standard aus) | Absturzbericht nur protokollieren, Programm ohne Tastenwarten beenden; P0PAR-Variante wiederholt jeden Fall z. B. 1000× und schreibt `C:\P0PAR.LOG` | Produktaenderung im Kern (klein, abschaltbar), neue Sondenvariante, Hosttest; ca. ein Developer-Ticket | stark: stille TLB-Fehler (Schreiben gelingt nach Verlassen des Super-Zustands) werden ueber viele Wiederholungen sichtbar; laeuft ohne Tester-Eingriff |
| (c) Debug-Feature statt Bootargument | wie (b), aber nur in einem eigenen Kern | wie (b); laesst sich in den Phase-B-Kern mit einbauen (kein dritter Kern) | wie (b); kein Einfluss auf den Release-Kern |

Empfehlung des Leads: (c) — die Testkarte hat ohnehin einen zweiten Kern
(Phase B); das Feature kommt dort mit hinein, der Release-Kern bleibt
unberuehrt. A1 in Phase B dann als Dauerlauf (z. B. 1000 Wiederholungen je Fall).
