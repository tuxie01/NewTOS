# Audit DESK70 (Punkt 1) — Textprogramm vom Desktop gestartet: was passiert nach dem Programmende

## 1. Kopf

| | |
|---|---|
| Scope | Eine Messfrage aus `TASK/DESK70.md` Punkt 1 (GitHub #12): Wartet der Desktop nach dem Ende eines vom Desktop gestarteten Textprogramms (`.TOS`/`.TTP` bzw. `.NCP`/`.NTP`) auf eine Taste oder eine Zeit, wie lange bleibt der Text stehen, was zeigt der Schirm danach — EmuTOS gegen NewTOS. |
| Datum | 2026-11-07 |
| NewTOS | `df2a6554ac7f35a089e2b768de55fb46a9232f71`, sauber (Messung in einem frischen Klon dieses Commits im Scratch) |
| EmuTOS | `1a7484aba1bfba23e2c0899e243b6d8177fe41df`, ROM `tools/oracle/etos512de-1a7484ab.img`, sha256 `323c87bb…2d2750c` |
| Hatari | 2.6.1, `tools/oracle/hatari.cfg`, leeres HOME |
| Board | QEMU 11.0.2 `raspi4b`, `serial=on` |
| Abbilder | Orakel `d70.img` `cdf197b6…`, `d70z.img` `c0914309…`; NewTOS `card.img` `58b13b03…`, `cardz.img` `f8f757b8…` (Manifeste im Lauf) |
| Lauf | `measurements/measurement-2026-11-07-desk70/RUNINFO.TXT` |
| Canary | tt OK, falcontc OK, disk OK (Testabbild unverändert); NewTOS-Harness (TOOL09) läuft: `measurements/measurement-2026-11-07-desk70/preflight/tosdump-run.txt` |

**Ergebnis in einem Satz:** Beide Seiten warten nicht. Der Textschirm wird spätestens ein Bild nach der letzten Ausgabe gelöscht, und der Desktop baut sich ohne Taste und ohne Pause wieder auf. Danach ist der Schirm pixelgleich mit dem Stand vor dem Start. NewTOS verhält sich hier wie EmuTOS. Abweichend sind nur vier Text-Stellen im Repo, die eine Tastenabfrage versprechen.

## 2. Findings

Am Verhalten gibt es keinen Befund. Die folgenden vier Befunde betreffen Text im Repo, der der Messung widerspricht.

### DESK70-001 — S4 — MEASURED — `sdk/apps/build-coremark.sh` verspricht eine Tastenpause

- **Ort:** `sdk/apps/build-coremark.sh:34-40@launch_native`. Zitat: *„`.NCP` gets the VT52 text screen and -- crucially … -- the desktop's "[ Press any key to return to the desktop ]" pause after the program exits (programs/desktop/src/main.rs, launch_native), so the result stays on screen."*
- **Widerspruch:** Gemessen gibt es weder bei EmuTOS noch bei NewTOS eine solche Pause (Abschnitt 6). `launch_native` gibt es in `programs/desktop/src/` nicht mehr. Der Desktop startet über `programs/desktop/src/pro.rs:122@pro_run`. Ein `.NCP` bleibt nur dann stehen, wenn das Programm selbst auf eine Taste wartet.
- **Belege:** Läufe `n-m0ncp-r1/-r2` und `o-m0tos-r1/-r2` in `measurements/measurement-2026-11-07-desk70/`.
- **Lokalisierung:** LOCALIZED: Kommentar im Bau-Skript, kein Funktionsbefund.
- **Route:** Developer. Das Ticket sieht die Korrektur dieses Kommentars ohnehin vor.

### DESK70-002 — S4 — MEASURED — `docs/sdk/06-image-and-testing.md` beschreibt einen gelöschten Startweg

- **Ort:** `docs/sdk/06-image-and-testing.md:282-286`. Die Stelle nennt die Zeilen 467-500 von `programs/desktop/src/main.rs` als `launch_native` und sagt: *„`.NCP` clears the screen, runs, waits for a key (`console_pause_for_key`, `:610`), and redraws the desktop."*
- **Widerspruch:** Es gibt keine Tastenwartung (gemessen). `launch_native` und `console_pause_for_key` existieren nicht mehr (DESK43 U3, `TASK/DESK43-U3-STATUS-DONE.MD`). Auch die zitierten Zeilen gehören zu einem anderen Stand.
- **Route:** Researcher (Doku).

### DESK70-003 — S4 — MEASURED — `docs/sdk/00-quickstart.md` verspricht eine Tastenwartung

- **Ort:** `docs/sdk/00-quickstart.md:314-315`. Zitat: *„A `.NCP` runs in the console and waits for a key before the desktop comes back."*
- **Widerspruch:** Gemessen kehrt der Desktop ohne Taste zurück, wenn das Programm selbst nicht wartet (Lauf `n-m0ncp-r1`: Text gelöscht 4 ms nach der letzten Ausgabe, Desktop nach 0,29 s).
- **Route:** Researcher (Doku).

### DESK70-004 — S4 — MEASURED — `docs/benchmarks/coremark.md`: „result stays up until a key is pressed"

- **Ort:** `docs/benchmarks/coremark.md:96-98`. Zitat: *„double-click `CORE.NCP`. Banner, then ~30 s of deliberately blank screen …, then the result, which stays up until a key is pressed."*
- **Widerspruch:** Gemessen bleibt nichts stehen. CoreMarks Port (`sdk/overlays/coremark/newtos/core_portme.c`) wartet nicht auf eine Taste. Genau das berichtet #12, und auf der echten Pi bestätigt es `measurements/measurement-2026-11-07-hw08-pi4-card/RUNINFO.TXT`.
- **Route:** Researcher (Doku).

Für alle vier gilt:

- **Eingabeprüfung:** Bei EmuTOS zeigt der Aufruf-Trace `Pexec(0, "C:\D70M0.TOS", [0]"", …)` und die vier `Cconws` des Programms. Bei NewTOS zeigt der serielle Log `sh_ldapp: starting C:\D70M0.NCP, sh_nextapp=0, sh_isgem=0` und die vier Textzeilen. Der Startweg ist auf beiden Seiten `shel_write(SHW_EXEC, isgem=0)` → NORMAL_APP.
- **EmuTOS-Quelle:** `aes/gemshlib.c:645` (`dos_exec`), danach `:650-684` ohne Ein- oder Ausgabe. Der Wechsel in den Grafikmodus erfolgt über `sh_chgrf`/`sh_tographic` (`:513-523`, `:223-239`). `desk/deskpro.c:52-64` (`pro_run`) kehrt sofort zurück.
- **NewTOS-Quelle:** `kernel/src/shell_loop.rs:422@sh_ldapp`, `kernel/src/shell_loop.rs:309@sh_chgrf`, `kernel/src/shell_loop.rs:351@sh_tographic`.
- **Spec:** `docs/api/aes/shell-loop.md` ist nicht betroffen. `docs/sdk/05-abi-pitfalls.md:578-583` steht schon richtig („no pause when it ends"; Korrektur aus SDK04).
- **Retest:** keiner im Code. Eine Doku-Korrektur braucht keinen Messlauf.

## 3. Offene Tickets im Scope

| Ticket | Messung | Ergebnis |
|---|---|---|
| `TASK/DESK70.md` Punkt 1 („erst messen, was EmuTOS' Desktop nach dem Ende eines TOS-Programms tut") | Läufe `o-*` gegen `n-*` | **EQUAL**: EmuTOS kehrt sofort zurück, NewTOS auch. Laut Ticket ist NewTOS damit konform; #12 wird mit Erklärung beantwortet (EmuCON oder ein Programm, das selbst wartet). Die Lesart im Abschnitt „Stand 2026-11-07" (keine Abfrage in `desk/`, `aes/`, `bdos/`) bestätigt die Messung. |

## 4. Coverage-Matrix

„EQUAL" heißt hier: per Sonde auf beiden Seiten gemessen und gleich. Ein tosdiff-Fall existiert dafür nicht und ist auch nicht möglich. Ein Dumper kann den Schirm nach seinem eigenen `Pterm` nicht beobachten, und der Ablauf „Desktop startet Programm" liegt außerhalb jeder Falltabelle. Darum lautet das Urteil MEASURED, nicht CONFIRMED.

| EmuTOS-Funktion | NewTOS (mapping.tsv) | Schicht | Status | Läufe |
|---|---|---|---|---|
| `pro_run` / `pro_exec` (`desk/deskpro.c:35-64`) | `programs/desktop/src/pro.rs:122@pro_run` (nicht in mapping.tsv) | desk | EQUAL (Sonde) | o-m0tos-r1/r2, n-m0ncp-r1/r2 |
| `do_aopen` (`desk/desksupp.c:843-1001`), `.TOS` gegen `.TTP` | `open_item`s App-Arm und `programs/desktop/src/main.rs:1083@pro_run_native` (Doku `programs/desktop/src/main.rs:857@do_aopen`; nicht in mapping.tsv) | desk | EQUAL (Sonde): `.TTP`/`.NTP` öffnet zuerst den Parameterdialog, nach dem Programmende verhalten sich beide gleich | o-m0ttp, n-m0ntp |
| `opn_appl` (Parameterdialog) | `programs/desktop/src/inf.rs:4592@opn_appl` | desk | nur als Vorstufe gesehen, nicht Gegenstand | o-m0ttp, n-m0ntp |
| `sh_write` (`aes/gemshlib.c:157`) | `tos-api/src/aes/shell.rs:495@shel_write` | aes | EQUAL (Sonde; EmuTOS-Trace `shel_write(intin: 0x1,0x0,0x1)`) | o-trace, n-* |
| `sh_ldapp` (`aes/gemshlib.c:592-689`) | `kernel/src/shell_loop.rs:422@sh_ldapp` | aes | EQUAL (Sonde): keine Eingabe zwischen Programmende und Desktopstart | alle |
| `sh_main`, `sh_chgrf`, `sh_tographic` (`:692-731`, `:513-523`, `:223-239`) | `kernel/src/shell_loop.rs:246@sh_main`, `kernel/src/shell_loop.rs:309@sh_chgrf`, `kernel/src/shell_loop.rs:351@sh_tographic` | aes | EQUAL (Sonde): gleiche Bildfolge (Text, weiß mit Sanduhr, grüne Fläche von oben, Menüzeile, Fenster), Ende pixelgleich mit dem Stand vor dem Start | alle |
| `process_inf2` `#Z 00` (`aes/geminit.c:438-463`) | `tos-api/src/aes/init.rs:437@process_inf2` | aes | EQUAL (Sonde), Kontrolle auf dem unveränderten Kernel | o-zauto, n-zauto |

Bug-Dichte: 0 bestätigte Verhaltensfehler bei 7 gemessenen Funktionen. Die 4 Befunde sind Doku-Befunde (S4).

## 5. Neue Fälle

Keine tosdump-Falltabelle, der Grund steht in Abschnitt 4. Die Sonde ist wiederholbar und liegt vollständig in `measurements/measurement-2026-11-07-desk70/method/`:

- `d70prog.c`: eine Quelle für beide Seiten, drei Modi.
- Orakel: `hatari-run.sh` (Fifo-Steuerung, AVI jedes Bildes) und `avi-frames.py` (emulierte Uhr aus den Audio-Chunks).
- NewTOS: `mk-injector.py`/`build-inj.sh` (Injektor nur im Scratch) und `qemu-run.py` (QMP-Dumps etwa alle 3 ms).
- Beide Seiten: `classify.py` als gemeinsame Einstufung der Schirme.

## 6. Gemessen vs. offen

**Gemessen.** Doppelklick auf beiden Seiten skriptgesteuert:

- Orakel: Mauszeiger über EmuTOS' eigene Alt+Pfeil-Mausemulation, dann `hatari-event doubleclick`.
- NewTOS: IKBD-Injektor im Scratch-Kernel.
- Zur Kontrolle zusätzlich `#Z`-Autorun ohne jede Eingabe, auf NewTOS mit dem **unveränderten** Release-Kernel.

| | EmuTOS (emulierte s, 50,4 Bilder/s) | NewTOS (Host-s, Dumps etwa alle 3 ms) |
|---|---|---|
| Text sichtbar, sofortiges `Pterm` (M0, zwei Läufe) | 4 Bilder (≈ 80 ms vom ersten Zeichen bis zum Löschen); der volle Text höchstens 1 Bild | ≈ 3–5 ms ab dem ersten Zeichen |
| Letzte Ausgabe → Schirm gelöscht | ≤ 2 Bilder (≤ 40 ms) | ≤ 5 ms |
| Gelöscht → Desktop fertig | 2,91–2,93 s (6 Läufe) | 0,25–0,30 s (6 Läufe) |
| Mit 4–6 s Verzögerung im Programm (M1) | Text 5,07 s sichtbar, dann sofort gelöscht | Text 4,21 s sichtbar, dann sofort gelöscht |
| Kontrolle: Programm wartet in `Cconin` (M2), keine Taste | Text bleibt 40,2 s (emuliert) stehen, RETURN löst | Text bleibt 59,4 s stehen, RETURN löst |
| `.TTP`/`.NTP` | Parameterdialog, RETURN, danach wie `.TOS` | Parameterdialog, RETURN, danach wie `.NCP` |
| `#Z 00` Autorun | wie Doppelklick, danach Desktop | wie Doppelklick, danach Desktop |
| Schirm danach | pixelgleich mit dem Bild vor dem Doppelklick (Hash) | pixelgleich mit dem Bild vor dem Doppelklick (Hash) |
| Aufrufe zwischen `Pterm` und Desktop | Trace: `Setexc`, `Dsetdrv`, `Drvmap`, `Dsetpath`, `Setexc`, `Bconout` ESC H ESC f ESC E, `Mxalloc`, `Pexec(7)`, `Pexec(6)`, `appl_init`, keine Konsoleneingabe | serieller Log: `Pterm'd` → `starting desktop NEWDESK` nach 0–25 ms |

Die Dauern unterscheiden sich um den Faktor 10. Das ist Rechengeschwindigkeit (68030 mit 16 MHz emuliert gegen QEMU). Auf keiner Seite steckt darin eine Wartezeit: Der Text ist in beiden Fällen schon gelöscht, bevor der Neuaufbau beginnt.

**Offen bzw. bewusst nicht gemessen:**

- CoreMark selbst; laut Auftrag genügt ein kleines Programm.
- Ein `gemdos-trace` auf NewTOS. Der Startweg ist durch den seriellen Log und die Zeitfolge belegt; ein GEMDOS-Trace der Konsolenaufrufe zwischen `Pterm` und Desktop fehlt auf dieser Seite.
- Die echte Pi 4 (Abschnitt 7).

## 7. Real-Pi4-Liste

- Sichtbarkeitsdauer des Textes auf echter Hardware nach dem Programmende. Laut Messung liegt sie im Millisekundenbereich. Der Anwenderlauf vom 2026-11-07 (`measurements/measurement-2026-11-07-hw08-pi4-card/RUNINFO.TXT`: „keine Tastenabfrage, zurück zum Desktop") passt dazu, ein neuer Pi-Gang ist dafür nicht nötig.

## 8. Akzeptable Abweichungen

- Die Dauer des Neuaufbaus (EmuTOS etwa 2,9 s, NewTOS etwa 0,3 s) ist reine Geschwindigkeit. Die Bildfolge und das Endbild sind gleich. Über die API ist sie nicht beobachtbar.

## 9. Out-of-scope observations

- NEWDESK zeigt `.NCP`/`.NTP` im Fenster mit Dokument-Icon, EmuDESK zeigt `.TOS`/`.TTP` bei gleicher Reihenfolge der INF-Zeilen mit Programm-Icon. Beleg: `measurements/measurement-2026-11-07-desk70/screendumps/m0-launch-emutos-vs-newtos.png`.
- Der Fenstertitel lautet bei NEWDESK `C:\`, bei EmuDESK `C:\*.*`, für dieselbe `#W`-Zeile. Beleg: dasselbe Bild.
- Die Cursorblinkperiode beträgt bei EmuTOS 30 VBL (0,595 s bei 50,4 Hz), bei NewTOS 0,5 s fest. Beleg: `measurements/measurement-2026-11-07-desk70/oracle/o-m1tos.frames.txt` gegen `measurements/measurement-2026-11-07-desk70/newtos/n-m1ncp/screens.txt`.
- Ein einziges Mal (Erkundungslauf, Bild nicht aufbewahrt) zeigte NewTOS für unter 5 ms ein grünes Rechteck in Fenstergröße auf dem weißen Schirm. In den 6 aufbewahrten Läufen trat es nicht wieder auf.
- `tools/make-sd-image.sh:550-553@launch_native` zitiert die gelöschte `launch_native` als Grund, warum der Desktop kein `Dsetpath` macht.
- `docs/analysis/deps/mapping.tsv` hat keine einzige Zeile für die Schicht `desk/`. `cargo xtask deps pro_run --up` meldet deshalb `unmapped` (Route Researcher).
- `docs/api/desktop/_hw-test-2026-09-17.md:1293-1296` verspricht ebenfalls die Pause. Das ist ein datierter Schnappschuss und damit historisch, kein Befund.

## 10. Cleanup

- `AUDIT-TEMP`: keine Fundstelle außerhalb von `docs/`, auch nicht im Laufordner. `measurements/measurement-2026-11-07-desk70/method/mk-injector.py` setzt den Marker erst zur Laufzeit zusammen. Der Injektor lag nur im Scratch-Klon, `host_env.rs` wurde nach jedem Bau zurückkopiert und per `cmp` geprüft. Der Klon wurde danach gelöscht.
- Produktbaum: nicht angefasst. Neu im Baum sind nur der Laufordner und dieser Bericht.
- Fremde Änderungen gesehen: keine (`git status` war zu Beginn leer).
