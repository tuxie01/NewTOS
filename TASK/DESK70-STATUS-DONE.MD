# DESK70 — Quittung: CoreMark haelt sein Ergebnis bis zur Taste, der Desktop bleibt wie EmuTOS

Datum 2026-11-07 · Beleg
[measurements/measurement-2026-11-07-desk70-build/](../measurements/measurement-2026-11-07-desk70-build/RUNINFO.TXT)
· Messung Punkt 1 `docs/audits/2026-11-07-desk70-tos-exit.md` (`7d700cf3`)
· Ticket [DESK70](DESK70.md) · GitHub #12

- **Ausgangslage:** Weder EmuTOS' Desktop noch NEWDESK warten nach dem Ende
  eines Textprogramms (gemessen, `measurements/measurement-2026-11-07-desk70/RUNINFO.TXT`).
  Der Desktop bleibt wie er ist (Anwender). Damit CoreMarks Ergebnis lesbar
  bleibt, wartet das Programm selbst.
- **Code:** `portable_fini` in `sdk/overlays/coremark/newtos/core_portme.c`.
  Nach dem vorhandenen `fflush(stdout)`: `Dgetpath(buf, 0)`; ist das aktuelle
  Verzeichnis nicht die Wurzel, folgen `Press any key to exit.` und
  `Crawcin()`. `Crawcin` heisst: jede Taste, kein Echo, kein Abbruch mit
  Ctrl-C, der Rueckgabewert bleibt der des Benchmarks. Das liegt hinter
  `stop_time()` und hinter der letzten Ergebniszeile, also ausserhalb der
  Zeitmessung. Nur `core_portme.c` geaendert. Die MD5-Pruefung der sechs
  eingefrorenen Dateien in `sdk/apps/build-coremark.sh` lief gruen
  (`measurements/measurement-2026-11-07-desk70-build/build/build-coremark.log`, 0 Warnungen).
- **Warum die Wurzel „AUTO“ heisst:** Ein `\AUTO\`-Programm startet in der
  Wurzel des Bootlaufwerks. `run_auto_program` setzt kein Verzeichnis
  (EmuTOS `bios/bios.c:685-695`), das Kind erbt die Verzeichnisse von
  `initial_basepage` (`bdos/proc.c:396-410`), Spec
  `docs/api/gemdos/process/_run-ownership-curdir.md` 4.2 und 6.5;
  `Dgetpath` liefert dort `""` (`docs/api/gemdos/directory/47-dgetpath.md`).
  Der Desktop setzt vor dem Start das Verzeichnis des Programms oder des
  obersten Fensters (`desk/desksupp.c:880-915`), das Programm erbt es ueber
  `sh_apdir` (`aes/gemshlib.c:109-125,174-175`; Spec 4.4). Grenzen: Eine
  Kopie in der Wurzel wartet auch vom Desktop nicht (bisheriges Verhalten),
  ein Start aus EmuCON in einem Unterordner wartet. Beides steht im
  Kommentar.
- **Bestehende Messlaeufe:** `COREVAL.NCP`/`CORE.NCP` aus `C:\AUTO`
  (`tools/make-sd-image.sh` `SD_COREMARK_AUTO=1`, Methoden von sdk01a-test,
  sdk01j, con04) laufen weiter ohne Halt durch.
- **Text richtiggestellt:** die vier Stellen aus dem Audit
  (`sdk/apps/build-coremark.sh` „WHY .NCP“, `docs/sdk/06-image-and-testing.md`
  Abschnitt Doppelklick, `docs/sdk/00-quickstart.md` Schritt 3,
  `docs/benchmarks/coremark.md` Schritt 4 und AUTO-Absatz). Dazu der
  CoreMark-Kommentar in `tools/make-sd-image.sh`, der sonst nach der Aenderung
  falsch gewesen waere („Nothing holds the screen“). Dazu zwei durch die
  laengeren Kommentare verschobene Zeilenzitate in
  `docs/sdk/02-prerequisites.md`. Datierte Schnappschuesse unberuehrt.
- **QEMU raspi4b (Developer):** `a-auto`: Ohne jede Eingabe laufen beide aus
  AUTO, kein `Press any key`, COREVAL startet im selben Stempel wie die
  letzte Ergebniszeile von CORE, danach `desktop up`. `b-desk`: Doppelklick
  auf `C:\COREMARK\CORE.NCP` (Injektor nur im Scratch-Klon), Ergebnis und
  Aufforderung stehen 53,5 s, nach RETURN `Pterm'd`, rc=0, `desktop up`.
  Der Desktop ist danach pixelgleich mit dem Bild vor dem Klick
  (sha1 `8c61653971b8`).
- **Builds:** Kein Rust-Code geaendert (kernel, tos-api, tos-types, xtask
  unberuehrt), daher kein `cargo build`/`clippy`/`cargo xtask test` noetig.
  Die beiden Kerne des Laufs stammen aus einem Scratch-Klon von `7d700cf3`.

Offen: Tester-Abnahme. Echte Pi gehoert ins Testszenario: Doppelklick auf
`CORE.NCP`, Ergebnis steht, Taste fuehrt zurueck. Antwort auf #12 gibt der
Anwender. `COREVAL.NCP` vom Desktop ist nicht eigens gestartet; es nimmt
denselben Codepfad.
