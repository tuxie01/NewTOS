# PITEST01 — Testkarte und Ablauf fuer die Tester (echte Pi 4)

Status: **OFFEN** · Prioritaet: **P2** · Typ: Developer (Sonden, Debug-Feature, Werkzeug) — Anwenderentscheidung 2026-10-31

Grundlage: `docs/analysis/tester-scenario-draft-2026-10-31.md` (Entwurf, Schritte
A0–A6, B1–B2).

## Anwenderentscheidungen 2026-10-31

1. **Automatische Wiederholung von A1 (P0PAR): Option (c).** Ein Debug-Feature
   nur im Phase-B-Kern: ein Absturzbericht wird protokolliert und das Programm
   ohne Tastenwarten beendet (`crash=nowait`-Verhalten), dazu eine
   P0PAR-Variante, die jeden Fall vielfach wiederholt und ein Protokoll auf
   `C:` schreibt. Der Release-Kern bleibt unberuehrt (Feature aus =
   byte-gleich bis auf `panic::Location`, CLAUDE.md "Testing").
2. **Kartenweg:** neues Skript, das die Build-Ausgabe auf eine **bestehende**
   Pi-Karte (mit Firmware-Startpartition) abgleicht, jede Datei per sha256
   gegen die Build-Ausgabe prueft und ein Kartenmanifest mit `host_clang` und
   Kern-Commit schreibt. Kein voller Abbildbau.
3. **HZPROBE** fuer A2 (`_hz_200` im Anwendermodus) und ein **einseitiges
   Ablaufblatt** fuer die Tester.
4. Melden, wenn alles bereit ist; **die Karte erst nach Bestaetigung des
   Anwenders bespielen.**

## Arbeit

- Teil 1 (Developer): Debug-Feature + P0PAR-Wiederholungsvariante + HZPROBE,
  in QEMU raspi4b geprueft.
- Teil 2 (Developer): Karten-Abgleichskript (gegen ein Abbild im Scratch
  getestet, nie gegen eine echte Karte) + Ablaufblatt.

## Abnahme

- QEMU: P0PAR-Dauerlauf und HZPROBE laufen ohne Tester-Eingriff und schreiben
  ihr Protokoll; Release-Kern byte-gleich (Feature-Pruefung).
- Abgleichskript: Probe gegen ein Scratch-Abbild mit Firmware-Partition;
  Manifest mit `host_clang` und Kern-Commit; absichtlich verfaelschte Datei
  wird erkannt.
- Hosttests gruen.

[Gemeinsame Regeln](README.md) gelten.

## Stand Teil 1 (2026-10-31, umgesetzt, nicht committet)

- **Debug-Feature `crash-nowait`** (`kernel/Cargo.toml`, Standard aus):
  `report_and_wait` (`kernel/src/arch/aarch64/crash.rs:82@report_and_wait`)
  gibt den Absturzbericht wie bisher auf Schirm und serieller Leitung aus, isst
  wartende Tasten wie `dopanic` und kehrt dann ohne Aufforderung und ohne
  Tastenwarten zurueck; der Aufrufer beendet das Programm wie immer mit
  `Pterm(-1)` (`kill_program`). Mit Feature aus ist der Kern auf rpi4 und
  qemu-virt (release und dev) byte-gleich mit `HEAD`; Phase-B-Kern:
  `cargo xtask build --board rpi4 --release --features crash-nowait,fs-io-trace`.
- **`P0REP.NAP`** (`sdk/examples/p0rep.c`, Kind unveraendert `P0KID.NAP`):
  P0PAR-Faelle `RwSxErWTKL.` je Runde, Standard 1000 Runden (Befehlszeile
  1..100000), gleiche Soll-Exitcodes wie P0PAR, dazu die Eigenzustandspruefung
  nach `L`; Zaehler je Fall in `C:\P0PAR.LOG` (nach Runde 1, alle 100 Runden,
  am Ende), Schirm `P0PAR-REP: PASS n/n` bzw. `FAIL ...`. Nur fuer den
  `crash-nowait`-Kern: dauern in Runde 1 zwei Fehlerfaelle laenger als
  4 × Grundlauf + 1 s, wartete der Bericht auf eine Taste — `STOP`, rc 2 (auf
  dem Release-Kern nach zwei beantworteten Abfragen). Das Ablaufblatt laesst
  P0REP nur in Phase B zu.
- **`HZPROBE.NAP`** (`tosdump/probes/hzprobe.c`, reine Logik in
  `hzprobe-rules.h`, Hosttest `tosdump/tests/test_hzprobe.c`; Bau
  `make -C tosdump hzprobe-newtos`, QEMU-Lauf `tosdump/probes/run-hzprobe.sh`):
  liest `$4BA` 30 s lang (Befehlszeile 4..3600) je 64-mal im Anwender- und im
  Super-Zustand im Wechsel, Bezug `Tgettime` (Generic-Timer-Zaehler, nicht der
  Tick); PASS-Regel im Kopf der Sonde (kein Schritt > 2, Summe ± 1 %, kein
  Rueckschritt, Zaehlen in beiden Zustaenden, gueltige `Super`-Marken);
  `C:\HZPROBE.LOG`.
- **QEMU raspi4b** (`measurements/measurement-2026-10-31-pitest01-part1/RUNINFO.TXT`):
  `crash-nowait` + P0REP `PASS 12000/12000` (1000 Runden, 5000 Berichte, keine
  Taste); Release-Kern + Injektor: `STOP` nach den Faellen `w`, `x`; HZPROBE
  auf Release- und `crash-nowait`-Kern `PASS ticks=6000/6000 maxstep=1`.
  Hosttests 1918/0/1; Warnungen rustc 93/64, Clippy 121/74 wie Basis.
- **Fund F1 (nicht behoben, fuer den Lead):** `$4BA` wird nur in die Seite 0
  des laufenden Prozesses geschrieben (`tos-api/src/bios/tick.rs:108@publish`).
  Ein Elternprozess liest nach `Pexec` bis zu seinem naechsten Tick (≤ 5 ms)
  den Wert von vor dem Kindlauf; die erste P0REP-Fassung sah deshalb fuer ein
  7,5-s-Tastenwarten keine Zeit vergehen (RUNINFO Abschnitt 5). EmuTOS hat
  eine globale Zelle. P0REP liest deshalb erst am naechsten Tick.
- Offen: Teil 2 (Kartenabgleich-Skript, Ablaufblatt); die echte Pi.

## Stand Teil 2 (2026-10-31, umgesetzt, nicht committet)

- **`tools/sync-card.sh`** (Shell wie `tools/mkcard.sh`/`tools/sd-manifest.sh`, kein xtask-Code): gleicht die
  Build-Ausgabe auf eine **bestehende** Pi-Karte ab. `kernels` baut die beiden Kerne und schreibt `KERNELS.TXT`
  (sha256, Features, Commit, Dirty-Flags); `sync` ist ohne `--write` ein Trockenlauf (neu/geaendert/gleich je
  Datei per sha256, Fremddateien als Warnung), mit `--write` Kopie, Unmount/Mount, Rueckleseprobe jeder Datei,
  Manifest nach `<run>/card.manifest` und `C:\CARDMAN.TXT`; `verify` prueft nur. Formatiert nie, loescht nie
  (ausser den AppleDouble-Dateien, die macOS beim eigenen Schreiben anlegt), schreibt nie Firmware, verweigert
  falsche Datentraeger (Name, Mountpunkt, FAT, Firmware-Dateien, `C:` = Partition 1).
- **Kernwechsel per Dateitausch:** auf der Karte `kernela.img` (Release) und `kernelb.img` (`crash-nowait`,
  `fs-io-trace`), `kernel8.img` = der aktive. `config.txt` bleibt unberuehrt; das Skript verweigert, wenn
  `config.txt` einen anderen `kernel=` oder `os_prefix=` setzt (Raspberry-Pi-Doku, Optionen `kernel`/`os_prefix`,
  Quelle in der RUNINFO).
- **Liste:** `measurements/measurement-2026-10-31-pitest01-part2/method/pitest01-card.list` (Sonden in `C:\`,
  `DCPROBE` als `C:\AUTO\DCPROBE.NCP`, Alltagsprogramme unter `C:\Apps\<APP>\` wie auf der Karte laut
  ihrer letzten lokalen Sicherung vom 2026-10-08). Neubau aller Programme:
  `measurements/measurement-2026-10-31-pitest01-part2/method/build-card-programs.sh`.
- **Probe nur gegen ein Scratch-Abbild** (`measurements/measurement-2026-10-31-pitest01-part2/RUNINFO.TXT`):
  5 Verweigerungen, Trockenlauf schreibt nichts, Schreiben + Rueckleseprobe 51/51, im **abgehaengten** Abbild
  verfaelschtes Byte von `verify` erkannt (exit 1) und repariert, Phasenwechsel, `fsck_msdos` sauber.
- **Ablaufblatt:** `docs/analysis/tester-run-sheet-2026-10-31.md` (eine Seite, Phasen A/B, P0REP nur in B).

**Befehl fuer den Lead** (erst nach Commit und mit Bestaetigung des Anwenders; `R` = neuer Laufordner):

```sh
R=measurements/measurement-YYYY-MM-DD-pitest01-card   # Datum einsetzen
L=measurements/measurement-2026-10-31-pitest01-part2/method/pitest01-card.list
measurements/measurement-2026-10-31-pitest01-part2/method/build-card-programs.sh $R/logs   # "ALL-OK"
tools/sync-card.sh kernels $R/kernels kernelb.img=crash-nowait,fs-io-trace kernela.img=none
diskutil list     # Mountpunkt + Name der Startpartition (= C:) ablesen, nicht raten
B=/Volumes/BOOT   # <- abgelesener Mountpunkt; Name unten = abgelesener Datentraegername
C=(--list "$L" --boot "$B" BOOT --drive C "$B" BOOT --kernels "$R/kernels")
tools/sync-card.sh sync "${C[@]}" --active kernela.img                          # 1. Trockenlauf lesen
tools/sync-card.sh sync "${C[@]}" --active kernela.img --run-dir $R/card-a --write   # 2. Phase A
grep -n "#Z" "$B/NEWDESK.INF"      # muss leer bleiben (kein Autostart)
# Wechsel A->B (nach dem Kopieren von C:):
tools/sync-card.sh sync "${C[@]}" --active kernelb.img --run-dir $R/card-b --write
```

- Offen: echte Karte und echte Pi; HypView braucht auf der Karte eine `.HYP` und `HYP_VIEW.CFG` (A4), die
  Karte hatte am 2026-10-08 kein HypView; `cmdline.txt` fehlte dort (fuer Phase B neu anlegen mit
  `serial=on`); die Pfade der Liste gegen den Trockenlauf pruefen.

## Anwenderentscheidungen 2026-10-31 (nach Teil 2)

- Es gibt **eine** Karte, die Testkarte; eine "Alltagskarte" gibt es nicht
  (Wortlaut hier und im Ablaufblatt berichtigt). Den vorhandenen
  `kernel8.img` vor dem Schreiben sichern.
- HypView mit `HYP_VIEW.CFG` und einer `.HYP`-Datei kommt auf die Karte (A4).
- `cmdline.txt` wechselt mit dem Kern beim Uebergang A→B: Phase A ohne,
  Phase B mit `serial=on` (`--allow-boot-config` nur dafuer). Begruendung des
  Leads: `serial=on` schaltet die UART-Ausgabe ein, und `write_byte` wartet
  auf freien Platz im Sende-FIFO (`kernel/src/drivers/bcm2711/pl011.rs:165-171@write_byte`),
  der sich mit der Baudrate leert, auch ohne Adapter — Phase A wuerde dadurch
  langsamer (A0-Bootzeit, FS26 T3); aus dem Code abgeleitet, nicht gemessen.
- Volume-Namen vor dem Trockenlauf per `diskutil list`; die Ausgabe des
  Trockenlaufs geht vor `--write` an den Anwender.

