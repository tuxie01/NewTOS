# Ablaufblatt Tester — echte Pi 4 (PITEST01, 2026-10-31)

Grundlage: [Szenario](tester-scenario-draft-2026-10-31.md), [PITEST01](../../TASK/PITEST01.md). Es gibt **eine**
Karte, die Testkarte. Vom Lead gefuellt (`tools/sync-card.sh`, Manifest `C:\CARDMAN.TXT`); vor dem ersten
Schreiben sichert das Skript `kernel8.img`, `cmdline.txt` und `config.txt` der Karte sowie jede Datei, die es
ersetzt, per sha256 geprueft nach `<Lauf>/card-before/C/` (Liste im Manifest, Zeilen `saved`).
Je Schritt: **PASS / FAIL / NICHT AUSGEFUEHRT (Grund)**.
Bei FAIL: Foto, nicht wiederholen, weiter. Fotos immer mit sichtbarem Schirm.
**Kopf notieren:** Datum, Uhrzeit, Pi-Modell + RAM, Kartenmodell, Inhalt von `cmdline.txt` (`asid=on` ja/nein).

## Phase A — Release-Kern (`kernel8.img` = `kernela.img`), `cmdline.txt` ohne `serial=on`, **kein** UART-Adapter

`cmdline.txt` = `cmdline-a.txt` des Laufs (keine Option). Ohne `serial=on` bleibt die UART-Ausgabe aus; mit ihr
wartet jedes Zeichen auf Platz im Sende-FIFO (`kernel/src/drivers/bcm2711/pl011.rs:165-171@write_byte`), was
Phase A (A0-Bootzeit, A6) verlangsamen wuerde — aus dem Code abgeleitet, nicht gemessen.

| Schritt | Tun | PASS wenn | Festhalten |
|---|---|---|---|
| **A0** Boot | Karte rein, einschalten, Stoppuhr | Desktop, keine Absturzanzeige | Foto Desktop, Sekunden bis Desktop |
| **A1** P0PAR | `C:\P0PAR.NAP` doppelklicken, jeden der 5 Absturzberichte mit einer Taste quittieren | alle Zeilen `OK`, "own state after L: user OK", **"P0PAR: PASS"** | Foto Ergebniszeilen; jedes `WRONG` (v. a. Fall `S`/`T`) |
| **A2** HZPROBE | `C:\HZPROBE.NAP` doppelklicken, 30 s warten, nichts druecken | Schlusszeile **"HZPROBE: PASS ticks=…/… maxstep=…"** | Foto Schlusszeile |
| **A3** DOOM | zuerst A1 nochmal; dann DOOM, Esc, 2 min Pfeil hoch/runter (auch halten), Spiel starten, laufen, F10 → `y` | kein Haenger, zurueck zum Desktop, Tastatur geht | bei Freeze: Foto, Minute; **Leertaste, 3 s warten, dann Strg+Alt+Entf** — Desktop kommt (K5) oder nichts (K6)? |
| **A4** Alltag | je Programm starten, Handlung, beenden; danach Desktop-Tastatur + Maus pruefen: Desktop (C: oeffnen, Ordner wechseln, Datei kopieren/loeschen), TeraDesk (C:, Shift+Esc), QED (Umlaute, Shift, Alt, speichern), HypView (`C:\Apps\HYPVIEW\HYP_VIEW.NAP` oeffnet zViews Hilfe `C:\Apps\ZVIEW\ZVIEW.HYP`; Pfeil-runter, einen Link anklicken), zView (Bild aus `SAMPLES`, zoomen) | kein Absturz, keine haengende Taste, zurueck zum Desktop | je Programm ein Foto, Auffaelligkeiten |
| **A5** AWPROBE | `C:\AWPROBE.NAP` starten, auf "AWPROBE:WAIT" warten, einige Tasten, dann Strg+Alt+Entf | wartet, je Taste Haken-Bytes, Neustart | Foto beim Warten. Kehrt Runde 1 einmal mit `0` zurueck und Runde 2 wartet: als Befund notieren (UART01), kein FAIL |
| **A6** Ordner | `DCPROBE` lief beim Boot von selbst (`C:\AUTO`); im Desktop dreimal Datei > Neuer Ordner, je mit Stoppuhr | Messung: Werte liegen vor | drei Stoppuhrzeiten |

`P0REP.NAP` in Phase A **nicht** starten (wartet hier auf Tasten).

## Wechsel A → B (Pi aus, Karte an den Mac — Lead/Anwender)

1. **Zuerst `C:` komplett kopieren** (Phase-A-Logs: `HZPROBE.LOG`, `AWPROBE.LOG`, `DCPROBE.LOG` —
   `DCPROBE` schreibt sein Log bei **jedem** Boot neu).
2. **Kern und `cmdline.txt` zusammen tauschen:** Befehl aus PITEST01 "Stand Teil 3" mit `--active kernelb.img`.
   Er kopiert `kernelb.img` auf `kernel8.img` **und** `cmdline-b.txt` (`serial=on`) auf `cmdline.txt`, sichert
   die bisherigen beiden (= `kernela.img`, `cmdline-a.txt`) nach `<Lauf>/card-b/card-before/C/`, prueft jede
   Datei per sha256 nach dem Wiedereinhaengen und schreibt ein neues Manifest (`kernel_active kernelb.img`,
   `boot_cmdline serial=on`). Ohne Mac-Skript: `kernelb.img` auf `kernel8.img` kopieren (ersetzen),
   `cmdline.txt` durch die eine Zeile `serial=on` ersetzen, beides im Kopf notieren. `config.txt` bleibt
   unberuehrt: die Firmware laedt die dort mit `kernel=` genannte Datei, hier `kernel8.img`
   (`boot/rpi4/config.txt:9@kernel`; Raspberry-Pi-Doku, github.com/raspberrypi/documentation @ 7376533a, Option
   `kernel`; lokal `references/rpi4-boot/config-txt-boot.adoc:20-22`), und liest die Kommandozeile aus
   `cmdline.txt` der Startpartition (Option `cmdline`, Vorgabe `cmdline.txt`,
   `references/rpi4-boot/config-txt-boot.adoc:16-18`); der Kern holt sie per Mailbox und sucht `serial=on`
   (`kernel/src/bootargs.rs:22@SERIAL_ON`).
3. UART-Adapter anstecken, Terminal 115200 8N1 (`docs/HW-TEST-LOG.md`), Mitschrift **vor** dem Einschalten starten.

## Phase B — Kern `kernelb.img` (`crash-nowait`, `fs-io-trace`), `cmdline.txt` mit `serial=on`, Adapter

| Schritt | Tun | PASS wenn | Festhalten |
|---|---|---|---|
| **B1** FS26 T1 | einschalten, bis Desktop | serielles Protokoll vollstaendig | ganzes Protokoll (`gemdos 0x39`-Zeilen `dur=`, `dev w=…`, `W lba= us=`, `bios 0x04`) |
| **B2** FS26 T2 | C: oeffnen, dreimal Datei > Neuer Ordner (Name, Return) | Protokoll liegt vor | welcher der drei Durchgaenge langsam war |
| **B3** P0REP | `C:\P0REP.NAP` doppelklicken, **nicht eingreifen** (QEMU: ca. 15 min; Pi: unbekannt); Absturzberichte laufen von selbst durch | Schlusszeile **"P0PAR-REP: PASS n/n"** | Foto Schlusszeile; bei `FAIL` oder `STOP` Foto |

## Nach dem Lauf

Pi aus, Karte an den Mac: `C:` komplett kopieren (`P0PAR.LOG`, `DCPROBE.LOG`, `CARDMAN.TXT`, `cmdline.txt`),
serielles Protokoll, alle Fotos, dieses Blatt mit Eintraegen. Lead: `tools/sync-card.sh verify` (gleiche Karte, gleiche
Optionen, `--active kernelb.img`) und sha256 des Kartenabbilds nach dem Lauf; Ablage in einem Lauf unter `measurements/`.
