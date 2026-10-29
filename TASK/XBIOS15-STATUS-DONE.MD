# XBIOS15 — Quittung: serielles RX in den Ring von Geraet 0, AUX liest ihn (qemu-virt); `cargo xtask run` mit Fenster

Datum 2026-10-29 · Beleg
[measurements/measurement-2026-10-29-xbios15/](../measurements/measurement-2026-10-29-xbios15/RUNINFO.TXT)
· Ticket [XBIOS15](XBIOS15.md) · Audit `docs/audits/2026-10-29-xbios15-serial-rx.md`
(SERRX-001 Abnahmetabelle, -003, -004) · Spec `docs/api/xbios/0e-iorec.md`
(Abschnitt "NewTOS implementation", "XBIOS15")

- **Ring von Geraet 0 (Option (a)):** `tos-api/src/xbios/iorec.rs:519@push_serial`
  (EmuTOS `push_serial_iorec`, `bios/serport.c:331-343`, und der Push von
  `scc_rx_interrupt_handler`, `:670-678`: `tail` zuerst, voll → verworfen, 255
  Bytes, Byte mit dem `datamask` des Records maskiert wie der SCC-Pfad `:671`),
  `:479@pop_serial` (`get_iorecbuf`, `:171-189`: `head` zuerst, dann das Byte),
  `:485@serial_pending` (`bconstat_iorec`, `:191-199`). Wie alle Ringe vor jeder
  Nutzung geprueft/repariert, IRQs maskiert.
- **Routing:** `kernel/src/console.rs:169@serial_rx_byte` (qemu-virt) legt jedes
  Byte nur noch dort ab; der Tastaturring wird auf beiden Boards nur vom
  IKBD-Dekoder gefuellt. raspi4b unveraendert (verwirft,
  `kernel/src/console.rs:196@serial_rx_byte`).
- **AUX-Treiber (nur qemu-virt):** `kernel/src/console.rs:216@AuxSerial`, beim Boot
  vor dem RX-Interrupt in Slot 1 der Geraetetabelle gesetzt (`install_aux`,
  `kernel/src/lib.rs`): `Bconstat(1)` -1 solange ein Byte wartet, `Bconin(1)`
  wartet auf ein Byte (IRQs offen, `wfi`, Tastatur/Maus werden weiter gepumpt)
  und liefert es nullerweitert — wie EmuTOS `bconin1`. `Bconout(1)`/`Bcostat(1)`
  unveraendert (1 verworfen / -1, delegiert an `AuxNoCable`).
- **SERRX-003 (Begruendung berichtigt, alle Kopien):** Modulkopf und
  `serial_rx_byte` in `kernel/src/console.rs`; `docs/api/xbios/0e-iorec.md`
  (E1-Absatz, E2-Absatz, E5, neuer Absatz XBIOS15, Out-of-scope-Punkt
  „gegenstandslos“); `docs/api/bios/02-bconin.md` (Absatz „CON input is USB-HID“
  nannte qemu-virt das `CONF_SERIAL_CONSOLE = 1`-Profil); Kommentare, die
  qemu-virt-Konsoleneingabe dem PL011 zuschrieben: `kernel/src/host_env.rs`
  (7 Stellen), `kernel/src/drivers/gic.rs`, `kernel/src/drivers/qemu_virt/pl011.rs`,
  `kernel/src/initinfo.rs` (Diagnose: Quelltag `PUSH_SRC_SERIAL` entfernt, er
  konnte nicht mehr auftreten), `tos-api/src/xbios/iorec.rs` (Modulkopf).
- **AUX-Doku nachgezogen (qemu-virt-Zusatz, datiert):** `docs/api/bios/01-bconstat.md`,
  `02-bconin.md`, `docs/api/gemdos/console/03-cauxin.md`,
  `docs/api/gemdos/file/3f-fread.md` (Punkt 1a; R17 gilt nur noch auf der Pi),
  `docs/COMPATIBILITY.md` (`Iorec`-Absatz, Punkt (3)), Doc-Kommentare
  `tos-api/src/bios/chardev.rs`, `tos-api/src/gemdos/console.rs` (`cauxin`).
- **Unverdrahteter Selbsttest** `kernel/src/selftest_console.rs` F2: auf qemu-virt
  haette `Cauxin` jetzt gewartet; dort wird vorher ein Byte in den Ring gelegt
  und `Cauxin() == 'Q'` geprueft (rpi4 unveraendert `== 0`).
- **Ratsche (`docs/analysis/deps/mapping.tsv`):** `push_serial_iorec` → `serial_rx_byte`
  + `push_serial`; `scc_rx_interrupt_handler` → `pl011_rx_drain`, `serial_rx_byte`,
  `push_serial` (SERRX-004); `mfp_rs232_rx_interrupt_handler` → `pl011_rx_drain`;
  `bconstat1`/`bconin1` → `AuxSerial::bconstat`/`bconin`; `bconstat_iorec` →
  `serial_pending`; `bconin_iorec` → `aux_read_blocking`; `get_iorecbuf` →
  `pop_serial`/`pop_byte`; `incr_tail` → `push_byte`. `cargo xtask deps
  --check-mapping`: 1032 Zeilen, alle aufgeloest. Ohne Gegenstueck bleibt
  `mfp_rs232_rx_interrupt` (Assembler-Einsprung).
- **`cargo xtask run` (Option (b)):** qemu-virt startet standardmaessig mit
  `-display cocoa` (Tastatur/Maus = USB im Fenster), das Terminal ist die
  serielle Leitung = AUX; `--headless` = `-display none`; `--window` bleibt
  akzeptiert (= Standard); beide zusammen → Fehler. raspi4b unveraendert (ohne
  Anzeige, `--window` fuer das Fenster; `--headless` nennt den Standard).
  `tools/xtask/src/main.rs` `run_shows_window` + 3 Hosttests, Hilfetext, Meldung
  beim Start. Doku: `README.md`, `docs/sdk/06-image-and-testing.md`. Gefahren:
  `cargo xtask run --headless` bis `[desk] desktop up`, Meldungen wie erwartet;
  `--window --headless` → Fehler. Das Fenster selbst (cocoa) nicht gestartet.

## Abnahme

- **Sonde `serrx` qemu-virt (2 Laeufe bytegleich + Kontrolle):** `Bconstat(1)` -1,
  `Bconstat(2)` 0, Ring von Geraet 0 `hd 0 tl 3: 61 5a 31`, Tastaturring leer,
  `Bconin(1)` `0x61`, danach `hd 1 tl 3`. Normierter Vergleich mit
  `measurements/measurement-2026-10-29-xbios15-rx/runs/oracle-tt-1/SERRX.LOG`: verschieden
  nur `sizeof` (F1), Adressen (F4) und `Bconmap(-1/-2)` (XBIOS05, ausserhalb).
  Gleich dem Orakel.
- **raspi4b (2 Laeufe + Kontrolle):** `SERRX.LOG` bytegleich mit dem Lauf vor dem
  Fix (`bcee3b6f…`): verworfen, beide Ringe leer, `Bconstat(1)` 0.
- **Retest-Tabellen:** `bios-r1`, `xbios-chardev-r1`, `xbios-ptr-r1`,
  `xbios-flush-r1` (mit drei Tasten ueber den Injektor) gegen tt und falcontc,
  vorher (`git archive 18e8c115`) und nachher: dieselben Faelle verschieden
  (41/45 von 147, 12 von 23, 1 von 9, 0 von 207), **kein Fall neu verschieden**
  (`measurements/measurement-2026-10-29-xbios15/runs/before-after.txt`).
- **Desktop:** qemu-virt bis `[desk] desktop up (0 drives)`, Screendump; QMP
  `send-key` Ctrl+Alt+Del ueber die `usb-kbd` erreicht den IKBD-Dekoder
  (`[reset] keyboard Ctrl+Alt+Del (Warm) requested`, „no restart mechanism on
  this board; ignored“), Desktop laeuft weiter. raspi4b bis zum Desktop durch
  die Tabellenlaeufe (Runner endet auf „desktop up“).
- **Hosttests:** `cargo xtask test` 1816/0/1 (Baseline 1808/0/1; +4 Unit
  `iorec`, +1 Integration `xbios01_iorec`, +3 xtask); `make -C tosdump
  host-test` gruen; SDK 27/0/0.
- **Bau/Clippy:** beide Boards, auch mit `--features initinfo-diag`; Warnungen
  tos-api 64 / kernel 93 vorher = nachher; clippy tos-api 74, kernel 121 (beide
  Boards) vorher = nachher; geaenderte Dateien 0 Treffer.
- **doc-check:** bearbeitete Docs 0 Fehler (nach RUNINFO); in den 98 Docs/Tickets,
  die geaenderte Dateien zitieren, 0 Fehler mit Bezug auf diese Dateien (17
  verschobene Anker nachgezogen); der Rest ist der alte Rueckstand.

## Offen / fuer den Lead

- **Bconin(1) blockiert auf qemu-virt** (EmuTOS-treu, `bconin1`); vorher kehrte
  es dort mit 0 zurueck. Ein Programm, das auf qemu-virt `Fread` auf AUX/stdaux
  oder `Cauxin` ohne Daten ruft, wartet jetzt. Kein Nutzer gefunden
  (`programs/`, `sdk/apps`, `sdk/examples`). Bitte bestaetigen.
- **`CLAUDE.md`** (Zeile „Integration runs in QEMU via `cargo xtask run`…“) und
  **`.claude/agents/tester.md`** (beschreibt `xtask run` mit `-display none`)
  nicht angefasst (Konfiguration) — Formulierungsvorschlag im Bericht an den
  Lead. Erinnerung „QEMU window view command“ ebenso.
- `docs/STATUS.md` nicht bearbeitet (Auftrag).
