# BIOS09 — `Bconin` auf AUX, MIDI und Bconmap-Ports kehrt sofort mit 0 zurueck

Status: **UMGESETZT** 2026-10-30, Pi-Messung im spaeteren Testszenario (MEM12-Schritt entfernt, Anwender 2026-10-30) · Prioritaet: **P2** · Typ: Developer — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-bios-core.md`

## Sachverhalt

**BIOS-CORE-008 — S3 — MEASURED.** Beleg: Bericht Abschnitt 2, Lauf `measurements/measurement-2026-10-28-bios-core-audit/RUNINFO.TXT`.

- `measurements/measurement-2026-10-28-bios-core-audit/runs/HANG-EMUTOS-tt-1`, `-3`, `-6` (`BCHANG.LOG`): Kontrollhandle 0 kehrt zurueck (`H0 after ret=0`, `references/emutos/bios/parport.c:123` `bconin0`), `Bconin(1)`, `(3)`, `(6)` nicht — 20 s Wanduhr bei Fast-Forward, Trace endet im Aufruf. `measurements/measurement-2026-10-28-bios-core-audit/runs/HANG-NEWTOS-raspi4b`: alle vier `after ret=00000000`.
- Lokalisiert: `tos-api/src/bios/chardev.rs:133@AuxNoCable` (`bconin` → 0), `tos-api/src/bios/chardev.rs:103@Absent`.
- EmuTOS-Stellen: `references/emutos/bios/serport.c:260` `bconin1` (Iorec-Warteschleife), `references/emutos/bios/midi.c:35` `bconin3` (`while(!bconstat3())`).
- Spec/Entscheidung: `docs/api/gemdos/file/3f-fread.md` FS11 Punkt 1a nennt das Nichtblockieren fuer AUX ausdruecklich „documented divergence“ (auch im Code, `tos-api/src/bios/chardev.rs:128-132@AuxNoCable`); kein COMPATIBILITY-Eintrag.
- Fehlerbild: Terminal- oder MIDI-Eingabeprogramme lesen einen endlosen Strom von NUL-Bytes statt zu warten.

## Arbeit

1. Entscheidung: FS11 Punkt 1a in `docs/COMPATIBILITY.md` aufnehmen oder auf Blockieren aendern (mit Folgen fuer Programme ohne Eingang: Haenger).
2. Developer-Ticket erst nach der Entscheidung. Kein tosdump-Fall (EmuTOS blockiert, unbeaufsichtigte Tabelle ausgeschlossen).

## Abnahme

- Bestaetigender Fall: Lauf-Familie HANG (`BCHANG.LOG`) mit Handle 1/3/6 gleich dem Orakel (bei „Blockieren“), sonst COMPATIBILITY-Eintrag und ACCEPTED; kein Tabellenfall moeglich (Bericht Abschnitt 5).
- Kein vorher gleicher Fall in `bios-r1.tsv`, `bios-core-r1.tsv`, `xbios-chardev-r1.tsv`, `xbios-system-r1.tsv` weicht neu ab.
- `cargo xtask test` gruen.
- `docs/STATUS.md` und Spec im selben Commit; `cargo xtask doc-check` der geaenderten Doku 0 Fehler.

## Entscheidung (Anwender, 2026-10-30)

Option (a): auf rpi4 warten `Bconin(1)` und damit `Bconin(6)` (Bconmap-Eintrag
AUX, BIOS08) wie EmuTOS `bconin1` (`references/emutos/bios/serport.c:260`), so
wie qemu-virt seit XBIOS15. Der MIDI-Teil (`Bconin(3)`) ist seit BIOS04
erledigt. Kein COMPATIBILITY-Eintrag; FS11 Punkt 1a ("documented divergence")
entfaellt fuer `Bconin` und wird in Spec und Code-Kommentar richtiggestellt.

**Waehrend des Wartens laufen keine anderen Prozesse.** Es laufen nur die
`kbdvec`-Haken des wartenden Prozesses (Abgabe per `give_way`, wie XBIOS18):
die Tastatur wird weiter gepumpt, Bytes fuer einen `kbdvec`-Haken brechen die
Wartezeit ab, der Haken laeuft, der Trap startet neu. So auch EmuTOS (IKBD-
Interrupt springt waehrend `bconin1` durch `kbdvec`,
`references/emutos/bios/aciavecs.S:407-414`). Strg+Alt+Entf bleibt wirksam.

## Empfangspfad auf rpi4 (Lead, 2026-10-30, Stand `f065a705` vor dem Fix; Zeilen nachgezogen) — vorhanden, Bytes wurden verworfen

- RX-Interrupt wird auf beiden Boards eingeschaltet, unabhaengig von
  `serial=on` (der Schalter gilt nur fuer TX):
  `kernel/src/lib.rs:371@enable_rx_interrupt`,
  `kernel/src/drivers/bcm2711/pl011.rs:186@enable_rx_interrupt` (RXIM+RTIM).
- IRQ-Eintrag: `kernel/src/drivers/gic.rs:210@pl011_rx_drain` →
  `kernel/src/console.rs:146@pl011_rx_drain`.
- Endpunkt rpi4: `kernel/src/console.rs:181@serial_rx_byte` verwirft das Byte
  (nur `initinfo-diag` zaehlt es). qemu-virt: `kernel/src/console.rs:169@serial_rx_byte`
  → `iorec::push_serial`.
- Lesen: `kernel/src/console.rs:194@AuxSerial`, Warten
  `kernel/src/console.rs:231@aux_read_blocking`, Einbau
  `kernel/src/console.rs:254@install_aux` (Aufruf
  `kernel/src/lib.rs:367@install_aux`) — alle `#[cfg(feature = "board-qemu-virt")]`.
- Laut `docs/MEMORY.md` QUIRK-002-Korrektur und Messung 2026-09-04 wird die
  PL011-RX-IRQ auf der echten Pi zugestellt.
- **Bekannte Falle:** ohne Adapter ist GPIO 15 (RXD0) ein Eingang **ohne
  Pull** (`kernel/src/drivers/bcm2711/pl011.rs:93@configure_uart0_pins`) und
  taktet beim Start ein unechtes `0x00` ein (Kommentar
  `kernel/src/console.rs:184-194@serial_rx_byte`). Mit dem Routing landet es
  im AUX-Ring: das erste `Bconin(1)` ohne Kabel kehrt einmal mit `0` zurueck
  statt zu warten. Ein Pull-up auf GPIO 15 (Leerlaufpegel der UART ist High)
  waere eine Hardware-Aenderung nach BCM2711-Datenblatt (Track B) und ist
  **nicht** Teil dieses Tickets ohne Anwenderentscheidung; der Pi-Schritt
  unten macht die Wirkung sichtbar.

## Lokalisierung (Lead, 2026-10-30)

- `bconin1`, `bconin_restartable` (Trap-Weg), `bconstat1`, `bconinB`,
  `bconin_iorec`, `push_serial_iorec`, `mfp_rs232_rx_interrupt_handler`:
  `mapping.tsv` alle `mapped` (Zeilen 904-941, 987), Anmerkung jeweils
  "rpi4: AuxNoCable" bzw. "rpi4 dropped" — der Fix aktualisiert diese Zeilen
  (Ratsche).
- `--down` `bconin1`: 4 Funktionen, `bios` 100 % `mapped`, vollstaendig.
- `--up` (Wiedertestmenge, Schichten bios/bdos): `bconin`, `biostrap`,
  `xauxin` (`Cauxin`), `conin`, `getch`, `xconin`, `xnecin`, `xrawcin`,
  `xrawio`, `cgets`, `conbrk`, `conout`, `cookdout`, `backsp`, `enter`,
  `newline`, `osif`, `prt_line`, `tabout`, `xconout`, `xconrs`, `xconws`.
  `unmapped` in den beruehrten Schichten: keine. Vollstaendig: ja.

## Arbeit (Developer)

1. AUX-Treiber (`AuxSerial`, `aux_read_blocking`, `install_aux`, Routing in
   `serial_rx_byte`) auch fuer `board-rpi4`; Ausgabeseite bleibt wie bisher
   (`AuxNoCable`-Ausgabe). Keine Hardware-Registeraenderung.
2. Spec (`docs/api/gemdos/file/3f-fread.md` Punkt 1a, `docs/api/bios/` Bconin/
   Bconstat AUX) und Code-Kommentare (`AuxNoCable`, `serial_rx_byte`,
   `gic.rs`) richtigstellen; `AuxNoCable` bleibt nur, wo noch benutzt (sonst
   melden, nicht loeschen ohne Rueckfrage).
3. Hosttest: AUX-Eintrag auf rpi4-Profil wartet/liefert das Byte aus dem Ring
   (gleiches Muster wie die XBIOS15-Tests).
4. Permanente Sonde unter `tosdump/probes/` fuer den Pi-Schritt (MEM12
   Prueffolge Punkt 10): installiert einen `kbdvec`-Haken, der mitzaehlt und
   sichtbar macht, dass er waehrend `Bconin(1)` laeuft; ruft `Bconin(1)` ohne
   Kabel auf. In QEMU raspi4b mit dem Injektor vorab pruefen.
5. `cargo xtask test` gruen; Boot beider Boards bis Desktop.

## Abnahme (Anwender, 2026-10-30)

- Hosttest gruen.
- **MEASURED auf der echten Pi spaeter**, Schritt in der MEM12-Prueffolge:
  `Bconin(1)` blockiert ohne Kabel, Strg+Alt+Entf wirkt, der `kbdvec`-Haken
  feuert weiter.
- Kein vorher gleicher Fall in `bios-r1.tsv`, `bios-core-r1.tsv`,
  `xbios-chardev-r1.tsv`, `xbios-system-r1.tsv` weicht neu ab (raspi4b).

## Stand 2026-10-30

- Umgesetzt: AUX-Treiber auf beiden Boards (`kernel/src/console.rs@AuxSerial`,
  `kernel/src/lib.rs@install_aux`), Warteschleife nach
  `tos-api/src/bios/chardev.rs@aux_read` verschoben (Hosttest
  `tos-api/tests/xbios_kbdvecs.rs@aux_wait_reads_the_ring_and_gives_way_only_to_the_hook`).
- Hosttests 1894/0/1; vier Tabellen vorher/nachher ohne neue Abweichung;
  Haengesonde `BCHANG.LOG` byte-gleich `HANG-EMUTOS-tt-1`/`-6`.
  Beleg `measurements/measurement-2026-10-30-bios09/RUNINFO.TXT`.
- Sonde `tosdump/probes/awprobe.c` in QEMU raspi4b: Haken feuert waehrend
  `Bconin(1)`, Strg+Alt+Entf wirkt.
- Offen: echte Pi (MEM12 Punkt 10); GPIO-15-Falle (Anwenderentscheidung);
  apitest BC03 erwartet `Bconstat(1) == 0`, mit der Falle auf der Pi ohne
  Adapter evtl. `-1` (beides jetzt [UART01](UART01.md)).
- qemu-virt nach dem Umzug der Schleife: `tosdump/probes/run-axprobe.sh newtos-virt`
  dreimal, X01–X08 gleich XBIOS18 und dem Orakel; Beleg
  `measurements/measurement-2026-10-30-bios09-axprobe-virt/RUNINFO.TXT`.
- `tosdump/probes/run-axprobe.sh` Kopf "newtos-rpi4 … returns 0 at once"
  ist veraltet (SPEC02).

