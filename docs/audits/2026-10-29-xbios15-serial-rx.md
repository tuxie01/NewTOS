# Audit XBIOS15 Schritt 1 — wohin ein seriell empfangenes Byte geht

## 1. Kopf

| | |
|---|---|
| Scope | TASK/XBIOS15 „Arbeit“ Schritt 1: Ziel eines seriell empfangenen Bytes — Ring von Geraet 0 (`Iorec(0)`), Ring des auf AUX gemappten Ports (ueber `Bconmap(-2)`) oder Tastaturring (`Iorec(1)`); dazu `Bconstat(1)`, `Bconstat(2)`, `Bconin(1)`/`Bconin(2)` fuer ein Byte |
| Datum | 2026-10-29 |
| NewTOS | `2487bbea`, gemessen als `git archive` im Scratchpad (sauber); Haupttree nicht gemessen (Entwickler an FS22, HEAD waehrend des Laufs → `0d3bb485`, ohne Aenderung an `kernel/src/console.rs`, `tos-api/src/xbios/iorec.rs`, `tos-api/src/bios/chardev.rs`) |
| EmuTOS | `1a7484aba1bfba23e2c0899e243b6d8177fe41df` |
| ROM | `323c87bbff65096e54a8dc00b474f6924a950ebf25cc9513bd53d495a2d2750c` (`tools/oracle/etos512de-1a7484ab.img`) |
| Hatari | 2.6.1, Binary `953f4e78…6f6`, `tools/oracle/hatari.cfg` `c8f4998a…e46` |
| Kanarienvogel | ORACLE.md §6: `canary tt: OK`, `canary falcontc: OK` |
| QEMU | 11.0.2; Boards `raspi4b` (`-append serial=on`) und `virt` (die Maschine von `cargo xtask run`) |
| Kernel | rpi4 `5e78d746…6e56`, rpi4-diag `10e98313…5b89`, qemu-virt `8b5a3638…77e6` (alle `--release --features bios-arg-trace`) |
| Karten | `tools/mkcard.sh`, je Lauf eine; `optional_missing none` (4 Manifeste) |
| Belege | `measurements/measurement-2026-10-29-xbios15-rx/RUNINFO.TXT`, Laeufe unter `measurements/measurement-2026-10-29-xbios15-rx/runs/` |

Methode: eine Sonde (`measurements/measurement-2026-10-29-xbios15-rx/method/serrx.c`), fuer
m68k und AArch64 gebaut; sie dumpt vorher/nachher die Records und Ringe, wartet 400
`Vsync()`, waehrend der Wartezeit kommen drei Bytes `61 5a 31` ueber die serielle Leitung.
Einspeisung: Hatari `--rs232-in` (tt, ST-MFP, Bconmap-Geraet 6) bzw. `--scc-b-in` (falcontc,
SCC Kanal B, Geraet 7), jeweils mit der zugehoerigen Ausgabeoption (ohne sie schaltet Hatari
RS-232 ab); QEMU: serial0 als TCP-Chardev, Bytes 0,3 s nach der Bereitmeldung der Sonde.
Jeder Lauf zweimal (bytegleich), dazu je Seite ein Kontrolllauf ohne Bytes.

## 2. Findings

### XBIOS-SERRX-001 — qemu-virt: serielles RX landet im Tastaturring statt im Ring von Geraet 0

- **Schwere:** S3 · **Status:** MEASURED · **Route:** Developer
- **Lokalisierung:** LOCALIZED: `serial_rx_byte`, qemu-virt-Zweig
  (`kernel/src/console.rs:159@serial_rx_byte`, ruft `tos-api/src/xbios/iorec.rs:369@push_key`).
  Die einzige `--down`-Abhaengigkeit von `push_serial_iorec` ist `incr_tail`, ohne
  NewTOS-Gegenstueck (`docs/analysis/deps/mapping.tsv`); die Zielseite (`push_key`, Ring-
  arithmetik) ist durch `xbios-flush-r1` gleich gemessen
  (`measurements/measurement-2026-10-29-xbios01/RUNINFO.TXT`). Die Routing-Entscheidung
  selbst steht in `serial_rx_byte`. Beobachtet ueber `Iorec(0)`, `Iorec(1)`, `Bconstat(1)`,
  `Bconstat(2)`, `Bconin(2)`.
- **Messung** (Sonde, beidseitig; kein tosdump-Fall, siehe Abschnitt 5):

  | | Orakel tt | Orakel falcontc | NewTOS qemu-virt |
  |---|---|---|---|
  | `Bconmap(-1)` | 6 | 7 | `0x2c` (Funktionsnummer) |
  | `Iorec(0)` = `maptab[mapped-6].iorec` | ja (`0x4A54`) | ja (`0x478A`) | nicht pruefbar: `Bconmap(-2)` = `0x2c` |
  | Ring Geraet 0 / gemappter Port | `hd 0 tl 3`: `61 5a 31` | `hd 0 tl 3`: `61 5a 31` | `hd 0 tl 0`, leer |
  | Tastaturring `Iorec(1)` | leer | leer | `hd 0 tl 12`: `00000061 0000005a 00000031` |
  | `Bconstat(1)` | -1 | -1 | 0 |
  | `Bconstat(2)` | 0 | 0 | -1 |
  | `Bconin(1)` | `0x61` | `0x61` | nicht gerufen (`Bconstat(1)` 0) |
  | `Bconin(2)` | nicht gerufen | nicht gerufen | `0x00000061` |

  Belege: `measurements/measurement-2026-10-29-xbios15-rx/runs/summary.txt`,
  `measurements/measurement-2026-10-29-xbios15-rx/runs/oracle-tt-1/SERRX.LOG`, `measurements/measurement-2026-10-29-xbios15-rx/runs/oracle-falcontc-1/SERRX.LOG`,
  `measurements/measurement-2026-10-29-xbios15-rx/runs/newtos-virt-1/SERRX-serial.LOG` (je Lauf 2 bytegleich; Kontrollen `*-none` leer).
- **Eingangspruefung:** bestanden. NewTOS `bios-arg-trace` und Hatari `--trace bios,xbios`
  zeigen dieselben Aufrufe mit denselben Argumenten bis nach der Wartezeit
  (`measurements/measurement-2026-10-29-xbios15-rx/runs/oracle-*-trace/probe-calls.txt`, `measurements/measurement-2026-10-29-xbios15-rx/runs/newtos-virt-1/SERIAL.txt`); danach verzweigt
  die Sonde nach den (verschiedenen) Ergebnissen.
- **EmuTOS:** `references/emutos/bios/serport.c:350-359` (`mfp_rs232_rx_interrupt_handler`,
  ohne `CONF_SERIAL_CONSOLE` → `push_serial_iorec`, `:331-343`, Ring `iorec1.in`); Falcon:
  `scc_rx_interrupt_handler` `:647-683`, Push in `iorecB.in` (`:676`); welcher Port AUX ist:
  `:966-977`, `Iorec(0)` = `rs232iorecptr` `:1003`, `references/emutos/bios/xbios.c:340`.
  Im gepinnten Build ist `CONF_SERIAL_CONSOLE` 0 (`references/emutos/include/config.h:1422-1429`,
  `docs/api/_emutos-config.md`).
- **NewTOS:** `kernel/src/console.rs:159@serial_rx_byte` legt jedes Byte mit Scancode 0 in den
  IKBD-Ring; der Kommentar dort (`kernel/src/console.rs:152-159@serial_rx_byte`) erklaert qemu-virt
  zum `CONF_SERIAL_CONSOLE = 1`-Profil.
- **Spec-Status:** `docs/api/xbios/0e-iorec.md` (Abschnitt „NewTOS implementation“, E2)
  beschreibt das Verhalten („qemu-virt serial bytes enter as `ascii` with scancode 0“) — als
  Ist-Beschreibung, nicht als Vertrag. `docs/COMPATIBILITY.md` (Absatz „`Iorec`: the key queue
  is the IKBD `IOREC`“) fuehrt es **nicht** als Abweichung; dort steht nur, dass die Records
  der Geraete 0 und 2 leer bleiben. Ohne Eintrag ist die Abweichung nicht ACCEPTED.
- **Fehlerszenario:** auf qemu-virt liest ein Terminalprogramm `Bconstat(1)`/`Bconin(1)` und
  bekommt nie ein Byte; dieselben Bytes erscheinen als Tastendruecke (`Bconin(2)`, `Cconin`,
  AES-Tastaturereignisse) mit Scancode 0. Ein Programm, das den Tastaturring mit
  `ibufhd = ibuftl` leert, verwirft serielle Daten.
- **Schwere-Begruendung:** S3, weil nur auf dem Emulator-Board qemu-virt und nur bei
  seriellem Eingang; der falsche Wert selbst ist ein normaler API-Pfad. Hebt der Lead
  qemu-virt zum gleichrangigen Ziel, ist es S2.
- **Retest:** `cargo xtask deps push_serial_iorec --up` = `mfp_rs232_rx_interrupt_handler`,
  `mfp_rs232_rx_interrupt` (beide ohne Gegenstueck); dazu `scc_rx_interrupt_handler` (Falcon,
  siehe XBIOS-SERRX-004). Faelle: diese Sonde auf qemu-virt (Methode im Lauf), die
  Ticket-Retests `bios-r1`, `xbios-chardev-r1`, `xbios-ptr-r1`, `xbios-flush-r1`.
- **Hinweis zum Fix (nicht entschieden):** NewTOS' AUX ist „Port vorhanden, kein Kabel“
  (`tos-api/src/bios/chardev.rs:115@AuxNoCable`, Bconstat 0, Bconin 0). Ein Byte in den Ring
  von Geraet 0 zu legen, ohne dass `Bconstat(1)`/`Bconin(1)` ihn lesen, waere eine zweite
  Inkonsistenz (Ring gefuellt, Bconstat 0). Der Fix braucht eine Entscheidung zu AUX auf
  qemu-virt (Treiber ueber den Ring von Geraet 0, oder dokumentiertes Serienkonsolen-Profil
  in COMPATIBILITY.md) — Lead/Anwender.

### XBIOS-SERRX-002 — qemu-virt: Scancode 0 statt der Tabellensuche von `push_ascii_ikbdiorec`

- **Schwere:** S3 · **Status:** PLAUSIBLE · **Route:** Developer (nur falls das
  Serienkonsolen-Profil bleibt; mit dem Fix von 001 gegenstandslos)
- **Lokalisierung:** `kernel/src/console.rs:159@serial_rx_byte` (gleiche Stelle wie 001).
- **Gemessen:** NewTOS legt `0x00000061` ab (Scancode 0, Shift 0). **Nicht gemessen:** was
  EmuTOS im Profil `CONF_SERIAL_CONSOLE = 1` ablegt — das gepinnte ROM hat es nicht.
  Quelle: `references/emutos/bios/ikbd.c:192-231` sucht den Scancode in `norm`, dann `shft`
  (mit `MODE_LSHIFT`), dann Ctrl-A..Z (mit `MODE_CTRL`); fuer `a` waere das Scancode `0x1E`
  (Hypothese aus der Quelle).
- **Fall, der es klaert:** dieselbe Sonde gegen einen EmuTOS-Bau `1a7484ab` mit
  `CONF_SERIAL_CONSOLE=1` (nicht der Pin; Entscheidung, ob ein solches Zweit-ROM zulaessig ist,
  liegt beim Researcher/Lead).

### XBIOS-SERRX-003 — Begruendung „qemu-virt hat kein anderes Konsolen-Eingabegeraet“ ist widerlegt

- **Schwere:** S4 · **Status:** MEASURED · **Route:** Researcher
- Der Kommentar `kernel/src/console.rs:152-159@serial_rx_byte` begruendet das
  `CONF_SERIAL_CONSOLE = 1`-Profil damit, dass qemu-virt kein anderes Eingabegeraet habe.
  Im Standardlauf (`cargo xtask run`-Maschine: `qemu-xhci`, `usb-kbd`, `usb-mouse`) meldet der
  Kernel `[usb] HID configured: slot 1 ep 0x81 dci 3 (keyboard)`
  (`measurements/measurement-2026-10-29-xbios15-rx/runs/newtos-virt-1/SERIAL.txt`). EmuTOS'
  `CONF_SERIAL_CONSOLE` heisst „exclusively the serial port input“
  (`references/emutos/include/config.h:1422-1426`); NewTOS mischt beide Quellen in einen Ring.
  Die Begruendung steht auch im Modulkopf (`kernel/src/console.rs:14@serial_rx_byte`) und in
  `docs/api/xbios/0e-iorec.md` E2 — alle Kopien mitziehen.

### XBIOS-SERRX-004 — Mapping: Empfangspfad des Zielprofils (Falcon, SCC B) ohne Eintrag; `push_ascii_ikbdiorec` kein Knoten

- **Schwere:** S4 · **Status:** MEASURED (Falcon-Teil) · **Route:** Researcher
- Gemessen auf falcontc: AUX ist Bconmap-Geraet 7, `Iorec(0)` = `maptab[1].iorec` (SCC B),
  die Bytes kommen ueber `scc_rx_interrupt_handler`
  (`measurements/measurement-2026-10-29-xbios15-rx/runs/oracle-falcontc-1/SERRX.LOG`, B05/A05). `docs/analysis/deps/mapping.tsv` fuehrt
  `scc_rx_interrupt_handler` als „no counterpart“, waehrend `push_serial_iorec` (MFP-Pfad,
  TT/ST) auf `serial_rx_byte` zeigt. Fuer das Falcon030-Zielprofil ist `serial_rx_byte` das
  Gegenstueck von `scc_rx_interrupt_handler` (Push-Teil), nicht nur von `push_serial_iorec`.
- `cargo xtask deps push_ascii_ikbdiorec --up`: „no node of docs/analysis/deps/edges.tsv“ — im
  gepinnten Build herauskompiliert, aber genau dessen Verhalten implementiert NewTOS auf
  qemu-virt; der Graph kann diese Kopplung nicht zeigen.
- Die Ticket-Lokalisierung (TASK/XBIOS15.md) nennt nur den MFP-Pfad; fuer falcontc gilt der
  SCC-Pfad.

## 3. Offene Tickets im Scope

| Ticket | Ergebnis | Fall |
|---|---|---|
| XBIOS15 | **DIFF** — Erwartung bestaetigt: Orakel fuellt den Ring von Geraet 0 (= Ring des gemappten Ports, tt MFP, falcontc SCC B), Tastaturring leer; NewTOS qemu-virt legt die Bytes mit Scancode 0 in den Tastaturring, Geraet-0-Ring leer | Sonde `serrx`, Laeufe `measurements/measurement-2026-10-29-xbios15-rx/runs/oracle-*`, `measurements/measurement-2026-10-29-xbios15-rx/runs/newtos-virt-*` |

## 4. Coverage-Matrix

Zeilen = beobachtete Groesse je NewTOS-Board, Bezug ist das Orakel (tt und falcontc
gleich).

| EmuTOS | NewTOS (Mapping) | Board | Status | Fall |
|---|---|---|---|---|
| `push_serial_iorec` / `scc_rx_interrupt_handler` (Routing) | `serial_rx_byte` | qemu-virt | DIFF (XBIOS-SERRX-001) | serrx A01/A02 |
| `push_serial_iorec` / `scc_rx_interrupt_handler` (Routing) | `serial_rx_byte` | raspi4b | ACCEPTED (Byte erreicht `serial_rx_byte`, wird verworfen; COMPATIBILITY E5 (a) + AUX „no cable“) | serrx + diag |
| `iorec` (Geraet-0-Ring) | `iorec` | qemu-virt | DIFF (001) | A01 |
| `iorec` (Geraet-0-Ring) | `iorec` | raspi4b | ACCEPTED (E5 (a)) | A01 |
| `iorec` (Tastaturring) | `iorec` / `push_key` | qemu-virt | DIFF (001) | A02 |
| `iorec` (Tastaturring) | `iorec` / `push_key` | raspi4b | EQUAL (leer) | A02 |
| `bconstat1` | `AuxNoCable::bconstat` | qemu-virt | DIFF (001) | A03 |
| `bconstat1` | `AuxNoCable::bconstat` | raspi4b | ACCEPTED (AUX „no cable“, `docs/api/bios/01-bconstat.md`) | A03 |
| `bconin1` | `AuxNoCable::bconin` | beide | UNMEASURED: NewTOS meldet nie ein Byte, die Sonde ruft `Bconin(1)` nur nach `Bconstat(1) != 0` (EmuTOS blockiert auf leerem Ring) | C01 |
| `bconstat2` | `key_pending` | qemu-virt | DIFF (001) | A04 |
| `bconstat2` | `key_pending` | raspi4b | EQUAL (0) | A04 |
| `bconin2` | `pop_key` | qemu-virt | DIFF (001; Orakel ruft nicht, kein Byte) | D01 |
| `bconin2` | `pop_key` | raspi4b | EQUAL (beide: kein Zeichen, nicht gerufen) | D01 |
| `bconmap` (`-1`, `-2`) | `bconmap` | beide | DIFF, ausserhalb des Scopes (Funktionsnummer `0x2c`; XBIOS-CHARDEV-005) | H01, B05/A05 |

Zaehlung (ohne die Bconmap-Zeile ausserhalb des Scopes): EQUAL 3, ACCEPTED 3, DIFF 6, UNMEASURED 1
von 13. Alle 6 DIFF gehen auf die eine Wurzel XBIOS-SERRX-001 zurueck. Bug-Dichte: 0
CONFIRMED / 6 gemessene Funktionen (`serial_rx_byte`, `iorec`, `bconstat1`, `bconstat2`,
`bconin2`, `push_key`) — 1 MEASURED-Bug.

## 5. Neue Faelle

Keine Referenzdatei angelegt. Vorgeschlagene Tabelle `xbios-serrx-r1.tsv` (unter `tosdump/cases/`)
(DUMPBIOS; nicht angelegt, weil qemu-virt kein Laufwerk fuer eine Tabelle hat und
`cargo xtask tosdump` keine serielle Eingabe kennt — beides waere Werkzeugarbeit, TOOL-Ticket):

```
!layout	iorec	14/24	ibuf:p@0/0,ibufsiz:w@4/8,ibufhd:w@6/10,ibuftl:w@8/12,ibuflow:w@10/14,ibufhi:w@12/16
!layout	extiorec	34/56	in:[iorec]@0/0,out:[iorec]@14/24,baudrate:b@28/48,flowctrl:b@29/49,ucr:b@30/50,datamask:b@31/51,wr5:b@32/52:IGNORE
14:0x2C	w:-1	EXACT	-	Bconmap(-1): the mapped AUX device (tt 6, falcon 7)
14:0x0E	w:0	PTR_RAM	r:extiorec	Iorec(0) before the bytes: in empty
14:0x0E	w:1	PTR_RAM	r:iorec	Iorec(1) before: empty
13:1	w:1	EXACT	-	Bconstat(1) before: 0
13:1	w:2	EXACT	-	Bconstat(2) before: 0
14:0x25	-	IGNORE	-	Vsync() wait 1/400 (the harness injects 61 5a 31 here)
...	(400 rows)
14:0x0E	w:0	PTR_RAM	r:extiorec	Iorec(0) after: in head 0, tail 3 -- bios/serport.c:331-343
14:0x0E	w:1	PTR_RAM	r:iorec	Iorec(1) after: head == tail
13:1	w:1	EXACT	-	Bconstat(1) after: -1 -- bios/serport.c:191-199,255-258
13:1	w:2	EXACT	-	Bconstat(2) after: 0
```

Was die Tabelle nicht ausdrueckt (Ringinhalt, `Bconin(1)`, der Weg ueber `Bconmap(-2)`), bleibt
bei der Sonde. Ein `bconmap`/`maptab`-Layout fuer die TOOL16-Erfassung gibt es noch nicht; die
Sonde liest `BCONMAP`/`MAPTAB` nach `references/emutos/bios/serport.h:103-116` (gemessen tt:
`maptabsize` 4, `mapped_device` 6; falcontc: 3, 7) — Vorschlag an den Researcher, das Layout in
`docs/api/structures/` aufzunehmen, wenn eine Tabelle es braucht.

## 6. Gemessen vs. offen

Gemessen (beide Seiten, je 2 Laeufe bytegleich + Kontrolle ohne Bytes + ein Trace-Lauf):
Orakel tt und falcontc, NewTOS qemu-virt und raspi4b (`serial=on`), plus raspi4b mit
Drop-Markierung. Bconmap(-1) in jedem Lauf; der Ring des gemappten Ports per `Bconmap(-2)` auf
dem Orakel (auf NewTOS nicht erreichbar, `Bconmap(-2)` = `0x2c`).

Offen: tosdump-Bestaetigung (CONFIRMED) — siehe Abschnitt 5; `Bconin(1)` auf NewTOS mit
anstehendem Byte (es steht nie eines an); EmuTOS mit `CONF_SERIAL_CONSOLE=1` (002); echte Pi 4.

## 7. Real-Pi4-Liste

- Pi 4, `serial=on`, USB-Seriell-Adapter an GPIO 14/15: Bytes senden waehrend ein Programm
  `Bconstat(2)` pollt — Tastaturring muss leer bleiben, `Bconstat(1)` 0 (gemessen nur in QEMU).
- Pi 4 ohne Adapter (offener RXD0): das Stoerbyte `0x00` darf weder im Tastaturring noch in
  `Iorec(0)` erscheinen (Drop-Pfad wie in QEMU gemessen).

## 8. Akzeptable Abweichungen

- **raspi4b: Byte verworfen, Ring von Geraet 0 leer, `Bconstat(1)` 0.** Belegt durch
  `docs/COMPATIBILITY.md`, Absatz „`Iorec`: the key queue is the IKBD `IOREC`“, Punkt (3)
  („records of devices 0 (RS-232) and 2 (MIDI) … stay empty — no driver fills them (E5 (a))“),
  und AUX „port present, no cable“ (`docs/api/bios/01-bconstat.md`, „AUX (1) revised
  2026-10-13“; `tos-api/src/bios/chardev.rs:115@AuxNoCable`). Verhalten wie dokumentiert:
  gemessen leer/0, Drop in `serial_rx_byte` nachgewiesen (`measurements/measurement-2026-10-29-xbios15-rx/runs/newtos-rpi4-diag/SERIAL.txt`:
  `<AUDIT-RX:61><AUDIT-RX:5a><AUDIT-RX:31>`), Lauf mit Bytes = Kontrolllauf.
- Adressen und `sizeof(_IOREC)` (24 statt 14) unterscheiden sich gemaess Vertrag F1.

## 9. Out-of-scope observations

- `Bconmap(-1)`/`Bconmap(-2)` liefern auf NewTOS `2487bbea` die Funktionsnummer `0x2c`
  (bekannt, XBIOS-CHARDEV-005 / TASK/XBIOS05).
- Waehrend eines qemu-virt-Laufs beendete ein fremder Prozess QEMU per SIGTERM
  (`measurements/measurement-2026-10-29-xbios15-rx/runs/newtos-virt-2-killed`); parallel laufende Agenten raeumen offenbar `qemu-system`-Prozesse ab.

## 10. Cleanup

- Temporaere Verdrahtung nur in den Scratch-Baeumen (nie im Haupttree):

  ```
  kernel/embed.rs (qemu-virt-Baum)
  49c49,50
  < pub const EMBEDDED: &[Embedded] = &[Embedded { name: "NEWDESK", file: "NEWDESK.NAP" }];
  ---
  > // AUDIT-TEMP(xbios15-serial-rx): the serrx probe is started as the desktop (qemu-virt has no drive)
  > pub const EMBEDDED: &[Embedded] = &[Embedded { name: "NEWDESK", file: "SERRX.NAP" }]; // AUDIT-TEMP(xbios15-serial-rx)

  kernel/src/console.rs (rpi4-Baum, nur rpi4-diag-Kernel)
  186a187,188
  > #[cfg(not(feature = "board-qemu-virt"))]
  > const HEXD: &[u8; 16] = b"0123456789abcdef"; // AUDIT-TEMP(xbios15-serial-rx)
  188a191,192
  >     // AUDIT-TEMP(xbios15-serial-rx): make the drop visible on the serial line
  >     for c in [b'<', b'A', b'U', b'D', b'I', b'T', b'-', b'R', b'X', b':', HEXD[(_b >> 4) as usize], HEXD[(_b & 15) as usize], b'>'] { UART0.write_byte(c); } // AUDIT-TEMP(xbios15-serial-rx)
  ```

  Beide Dateien danach byte-gleich zu `git show 2487bbea:<datei>` (geprueft); die
  Scratch-Baeume wurden geloescht.
- `AUDIT-TEMP` ausserhalb `docs/` und `.claude/` im Haupttree: keine Fundstelle.
- Haupttree: keine Datei ausserhalb des neuen Laufverzeichnisses und dieses Berichts
  geaendert; fremde Aenderungen gesehen: die FS22-Arbeit des Entwicklers (jetzt committet,
  `b2eb3e0e`, `0d3bb485`), nicht angefasst.
