# DESK70 — Quittung: CoreMark haelt sein Ergebnis bis zur Taste, der Desktop bleibt wie EmuTOS

Datum 2026-11-07 · Beleg
[measurements/measurement-2026-11-07-desk70-build2/](../measurements/measurement-2026-11-07-desk70-build2/RUNINFO.TXT)
(gilt), erster Bau
[measurements/measurement-2026-11-07-desk70-build/](../measurements/measurement-2026-11-07-desk70-build/RUNINFO.TXT)
· Messung Punkt 1 `docs/audits/2026-11-07-desk70-tos-exit.md` (`7d700cf3`)
· Ticket [DESK70](DESK70.md) · GitHub #12

- **Ausgangslage:** Weder EmuTOS' Desktop noch NEWDESK warten nach dem Ende
  eines Textprogramms (gemessen, `measurements/measurement-2026-11-07-desk70/RUNINFO.TXT`).
  Der Desktop bleibt wie er ist (Anwender). Damit CoreMarks Ergebnis lesbar
  bleibt, wartet das Programm selbst.
- **Code:** `portable_fini` in `sdk/overlays/coremark/newtos/core_portme.c`.
  Nach dem vorhandenen `fflush(stdout)` folgen immer `Press any key to exit.`
  und `Crawcin()` (Anwenderentscheidung 2026-11-07, zweiter Bau; der erste
  Bau `729bab3b` wartete nur ausserhalb der Wurzel, per `Dgetpath`, damit
  AUTO-Laeufe durchliefen). `Crawcin` heisst: jede Taste, kein Echo, kein Abbruch mit
  Ctrl-C, der Rueckgabewert bleibt der des Benchmarks. Das liegt hinter
  `stop_time()` und hinter der letzten Ergebniszeile, also ausserhalb der
  Zeitmessung. Nur `core_portme.c` geaendert. Die MD5-Pruefung der sechs
  eingefrorenen Dateien in `sdk/apps/build-coremark.sh` lief gruen
  (`measurements/measurement-2026-11-07-desk70-build2/build/build-coremark.log`, 0 Warnungen).
- **Kein AUTO mehr:** AUTO ist fuer Treiber, CoreMark ist ein Programm vom
  Desktop (Anwender 2026-11-07). Ein wartendes Programm in `\AUTO\` hielte
  den Bootvorgang an, deshalb ist der Schalter `SD_COREMARK_AUTO` aus
  `tools/make-sd-image.sh` entfernt. Kein lebendes Skript nutzte ihn; die
  vergangenen Laufverzeichnisse, die ihn nennen oder CoreMark selbst nach
  AUTO kopieren (u. a. sdk01a-test, fs18-test, desk70-build, desk70-test),
  bleiben unveraendert; ihre Skripte gelten nur fuer ihre damaligen Binaries. Ein
  Messlauf ohne Tastatur startet `CORE.NCP` per `#Z 00 C:\COREMARK\CORE.NCP@`
  in `C:\NEWDESK.INF`, liest das Ergebnis von der seriellen Leitung und
  beendet QEMU.
- **Text richtiggestellt:** die vier Stellen aus dem Audit
  (`sdk/apps/build-coremark.sh` „WHY .NCP“, `docs/sdk/06-image-and-testing.md`
  Abschnitt Doppelklick, `docs/sdk/00-quickstart.md` Schritt 3,
  `docs/benchmarks/coremark.md` Schritt 4 und AUTO-Absatz; im zweiten Bau
  auf „wartet immer“ gebracht, dazu in `docs/sdk/06-image-and-testing.md`
  Schalterliste und Manifest-Beispiel ohne `SD_COREMARK_AUTO`). Dazu der
  CoreMark-Kommentar in `tools/make-sd-image.sh`, der sonst nach der Aenderung
  falsch gewesen waere („Nothing holds the screen“). Dazu zwei durch die
  laengeren Kommentare verschobene Zeilenzitate in
  `docs/sdk/02-prerequisites.md`. Datierte Schnappschuesse unberuehrt.
- **QEMU raspi4b (Developer), zweiter Bau** (`measurements/measurement-2026-11-07-desk70-build2/RUNINFO.TXT`):
  `z-key`: `#Z`-Start von `C:\COREMARK\CORE.NCP`, Ergebnis und Aufforderung
  stehen 15,6 s ohne Eingabe, nach RETURN (Injektor nur im Scratch-Klon)
  `Pterm'd`, rc=0, `desktop up`. `z-nokey`: ohne Eingabe steht der volle
  Bericht vor `Press any key to exit.` auf der seriellen Leitung, der Schirm
  bleibt 48 s bis zum Ende von QEMU, kein `desktop up`.
- **QEMU raspi4b, erster Bau** (`measurements/measurement-2026-11-07-desk70-build/RUNINFO.TXT`):
  `b-desk`: Doppelklick auf `C:\COREMARK\CORE.NCP`, Ergebnis und
  Aufforderung stehen 53,5 s, nach RETURN zurueck, Desktop danach pixelgleich
  mit dem Bild vor dem Klick (sha1 `8c61653971b8`). Sein AUTO-Teil (`a-auto`)
  gilt nicht mehr.
- **Builds:** Kein Rust-Code geaendert (kernel, tos-api, tos-types, xtask
  unberuehrt), daher kein `cargo build`/`clippy`/`cargo xtask test` noetig.
  Die Kerne stammen aus Scratch-Klonen (`7d700cf3`, zweiter Bau `60a9f39a`).

Offen: Tester-Abnahme. Echte Pi gehoert ins Testszenario: Doppelklick auf
`CORE.NCP`, Ergebnis steht, Taste fuehrt zurueck. Antwort auf #12 gibt der
Anwender. `COREVAL.NCP` vom Desktop ist nicht eigens gestartet; es nimmt
denselben Codepfad.
