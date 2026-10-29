# STATUS — NewTOS

> Single source of truth for project state. Updated with every milestone.
> Edit only via PR; do not edit in random places. Date everything.

## Current state

### Stand 2026-10-26

Zweig `main`, letztes Release-Tag `v0.6.1` (NewTOS 0.6.1 Alpha, `d5e9424c`,
2026-10-22; Kernel auf echter Pi 4 abgenommen, `docs/HW-TEST-LOG.md`).
Vorgeschlagenes Ziel weiterhin `base-1`
(`docs/analysis/inventory-2026-10-08.md` §3, Definition offen).
Verlauf bis heute: [docs/archive/status-log-2026-10-26.md](archive/status-log-2026-10-26.md)
(Geschichte, kein Beleg).

**Desktop:** ADR-022 — der Desktop wird **1:1 gegen EmuTOS** gebaut, durch die
AES-Falle, ohne Umgehungen. Seit DESK43 (erledigt 2026-10-16, U10 `f6f90202`,
Tester PASS `87436d52`, ADR-026) ist er die Bibliothek `programs/desktop/`
(nur `newtos-gem` + `tos-types`, Umgehung = Compilerfehler) und bootet als
EL0-Programm NEWDESK; der Kernel linkt keinen Desktop mehr, ein Ladefehler
faellt auf EmuCON zurueck. Rust-Programme fuer NewTOS: `programs/`
(Laufzeit `newtos-gem`, Bau `cargo xtask build-programs`).

**Auf echter Pi 4 abgenommen:** Boot, Desktop, TeraDesk (Start und sauberes
Beenden), QED aus TeraDesk heraus, EmuCON, **USB-Maus samt Mausrad**,
Dateikopieren ueber Partitionsgrenzen (FS04), Doom (40–75 fps, `kbdvec`
KEYS02), Tastenwiederholung, `.NTP`-Parameterdialog (DESK47), die vier
nativen Endungen aus TeraDesk (DESK41), Fensteroeffnen (DESK50), zView;
die Vorfuehrkarte vom 2026-10-08 mit Speicherschutz an (ARCH03 bis zu den
Nacharbeiten nach E9). Seit 2026-10-22 laut `docs/HW-TEST-LOG.md` dazu: GH1
und HypView, der Release-Kernel 0.6.1, USB01 F1/F2/F8 und CON04 (VT52 in
`Bconout(2)`).

**ARCH03 ist umgesetzt und abgenommen** (ADR-028, VA = PA; Tester auf `main`
2026-10-13, memprot on/off/strict + `asid=on`,
`measurements/measurement-2026-10-13-arch03-acceptance/`). Offen daraus laut
[ARCH03](../TASK/ARCH03.md): Stufe 2b (`Pexec` 100+), `asid=on` auf echter Pi.

**Basislinien** (bei jedem Arbeitsschritt zu halten):

| | |
|---|---|
| Hosttests (`cargo xtask test`) | **1790 / 0 / 1** (gemessen 2026-10-29 mit FS22, +7 Tests; davor 1783 mit XBIOS01, +16 Tests; davor 1767 mit FS20, +7 Tests; davor 1760 mit BIOS01 S2, +9 Tests; davor 1751 mit TOOL07 Rest, +3 xtask-Tests; davor 1748 mit TOOL12, +3 deps-Tests; davor 1745 mit TOOL10; TOOL09 +16, TOOL10 +3 xtask-Tests; vorher 1726 / 0 / 1 auf `75527af8`, `--fast` ohne den einen Doctest) |
| SDK-Hosttests (`sdk/run-host-tests.sh`) | **27 / 0 / 0** (gemessen 2026-10-29 mit XBIOS01, +1 `iorec-layout`) |
| `sh tools/desk-drift-probe.sh` | **0** (unsafe 20; gemessen 2026-10-26) |
| APITEST (`#Z`, memprot on/off/strict) | **109 / 0 / 5** (letzter Lauf `measurements/measurement-2026-10-26-a118-fix/RUNINFO.TXT`) |

Bau- und Clippy-Warnungen stehen hier ohne Zahl: sie wurden bei dieser
Bereinigung nicht nachgemessen. Sie duerfen nicht steigen; jeder Arbeitsschritt
nennt seine Vorher-/Nachher-Zahl in seiner Zeile unter *Recent Decisions*.

**Offen (Stand 2026-10-26, aus der Statuszeile der Tickets ohne DONE-Quittung):**

- **Warteschlange (Anwender 2026-10-29), in dieser Reihenfolge.** Regel: ein
  neues Werkzeug-Ticket kommt hinter den Produkt-Fix, den es verzoegern wuerde,
  ausser der Fix ist ohne das Werkzeug nicht abnehmbar.
  Developer, einer zur Zeit: [BIOS15](../TASK/BIOS15.md) (ADR-031: Haken-Warnungen, Besitz, `prt_*`) →
  [XBIOS15](../TASK/XBIOS15.md) (serielles RX in den Ring von Geraet 0) →
  [TOOL17](../TASK/TOOL17.md) (`mapping.tsv`-Klassen, Abdeckung je Schicht;
  danach die Retest-Listen von BIOS01/FS20/FS22/XBIOS01 neu klassifizieren) →
  [TOOL20](../TASK/TOOL20.md) (Sondenfaelle `pxprobe`/`gcprobe` nach tosdiff) →
  [SDK03](../TASK/SDK03.md) (`sysinfo`) → [SDK05](../TASK/SDK05.md)
  (cflib-Cookie-Name verdreht, QED `_IDT`; Sonde zuerst) →
  [SDK04](../TASK/SDK04.md) Fund C6 (TeraDesk `find_cookie`) →
  [ARCH06](../TASK/ARCH06.md) (`_MCH` = `0xFFFFFFFF`, `_VDO` messen) →
  [SDK04](../TASK/SDK04.md) Developer-Teil (Typen fester Breite, Fixes nach
  Messung) → [TOOL18](../TASK/TOOL18.md) (Cookie-Falltabelle, alle drei
  Profile) → [TOOL19](../TASK/TOOL19.md) (`DISKUNIT.TXT`-Pruefungen) →
  BIOS01 Rest → die uebrigen Audit-Tickets BIOS04–BIOS14, XBIOS02–XBIOS14 (S2
  zuerst; BIOS05 und XBIOS03 sind ACCEPTED nach ADR-031, XBIOS10 nach ARCH05) →
  [TOOL13](../TASK/TOOL13.md), [TOOL14](../TASK/TOOL14.md),
  [TOOL15](../TASK/TOOL15.md) → [TOOL03](../TASK/TOOL03.md) → DUMPGDOS →
  [FS19](../TASK/FS19.md) → [MEM11](../TASK/MEM11.md) (Erzwingung) →
  [TOOL01](../TASK/TOOL01.md); [SDK02](../TASK/SDK02.md), [SDK06](../TASK/SDK06.md)
  (P3) bei Gelegenheit.
  Daneben ein Tester bzw. Auditor zur Zeit, ohne Produktaenderung:
  XBIOS15 Schritt 1 (Orakel-Messung: wohin serielles RX geht; jederzeit).
- **P1:** [MEM12](../TASK/MEM12.md) — umgesetzt und auf QEMU abgenommen (Seite 0 nur im Super-Zustand beschreibbar); offen nur der Lauf auf der echten Pi.
  [USB01](../TASK/USB01.md) — F1/F2/F3/F8 auf echter Hardware
  bestaetigt (Pi 2026-10-23, zwei Steckpositionen; Ragnars Lauf 2026-10-25,
  vier Funkempfaenger an beiden Buchsen); offen: mehr als 15 Befehle am VL805,
  Report-only-Maeuse, Rueckfragen zum Rad der DW 9000 und zur Menueleiste.
  [ARCH03](../TASK/ARCH03.md) Rest (siehe oben). [A45](../TASK/A45.md) ist
  blockiert (siehe *Blocked*).
- **P1, umgesetzt, nur Pi- bzw. Anwendungsabnahme offen** (laut Statuszeile):
  A16, A37 (anwendungsseitig), DESK34, DESK35, HW03 (drei Pi-Gegenproben),
  R02–R05, SER01, V11, V12, DESK43 (Pi-Gegenprobe des EmuCON-Rueckfalls bei
  defektem NEWDESK).
- **P2:** [MEM11](../TASK/MEM11.md) (Anwenderadressen nur zufaellig < 2 GiB;
  Debug-Pruefung am Rueckgabeweg aktiv, Erzwingung offen; tosdiff-Pruefung wartet auf tosdump-Format v3), [SDK01](../TASK/SDK01.md) (A–L erledigt bzw. umgesetzt, G gestrichen;
  offen nur die Abnahme von K unter Linux/Homebrew), [USB03](../TASK/USB03.md),
  [SND01](../TASK/SND01.md), [SND02](../TASK/SND02.md),
  [NET01](../TASK/NET01.md) (Vorrecherche, Ziel 1.2), [HW04](../TASK/HW04.md)
  (teilweise), [W01](../TASK/W01.md), [TOOL01](../TASK/TOOL01.md) (`cargo xtask doctor`, Projekt-venv), [TOOL03](../TASK/TOOL03.md) (reproduzierbare Messabbilder und Dumper-Bauten, vor dem DUMPGDOS-Abbildvergleich), [TOOL04](../TASK/TOOL04.md) (Testlaufzeit: schnelle und volle Stufe), [TOOL05](../TASK/TOOL05.md) (doc-check auch fuer Doku, die eine geaenderte Datei zitiert); geparkt: [H04](../TASK/H04.md),
  [H01](../TASK/H01.md) (wartet auf Hardware); zurueckgestellt:
  [HW05](../TASK/HW05.md), [T13](../TASK/T13.md).
- **„ERLEDIGT“ ohne DONE-Quittung — gelten als offen, bis gemessen
  (Phase-5-Triage; Anwender 2026-10-26):** A06, A08, A09, A26, A116, ACC07,
  CON01, CON02, DESK39, DESK45, DESK47, FS02, FS07, FS14, FS18, GH1,
  LIBC-BUF, MEM01, MEM03, MEM04, MEM07, PROC01, REF04, SPEC01, V01, V02,
  V04–V10, V13–V15 (36); dazu sieben mit nur Teil- oder Sammelquittung:
  DESK43 (Quittungen bis U9-3-5), FS08 (je Teilschritt), MEM05 (A und B),
  R02–R05 (`R02-R05-STATUS-DONE.MD`). Liste per Skript aus der Statuszeile
  und den Quittungsdateien in `TASK/`, Stand 2026-10-26.
- **Umfang 1.0 (Anwender 2026-10-23):** FAT16 bleibt das einzige Dateisystem
  (FAT32/exFAT nach 1.0); **Ton gehoert dazu** — SND01 (Falcon-DMA-Sound),
  SND02 (HDMI), H04 (Klingel, Tastenklick, `Dosound`), alle P2.
- **P3 / P4:** 57 Tickets P3 und 4 Tickets P4 mit Status OFFEN, dazu 80 P3 und
  1 P4 im BACKLOG, 3 TEILWEISE, 3 IDEE. Darunter: ARCH03 Stufe 2b, SDL01,
  RTC01, REF03, REF05 (GIC-Quellen), REF06 (fehlende Track-B-Quellen), T15,
  DESK49, DESK51–DESK55, DESK61, DESK63, DESK65, DESK66, DESK67, DESK69, FS12
  (26 Laufwerke), FS15, FS16, FS17, LIBC-IOCTL, A119–A125, RST02, INPUT02,
  CON03, ARCH04, USB02, MEM08, MEM10, [KEYS03](../TASK/KEYS03.md),
  [T18](../TASK/T18.md), [FS19](../TASK/FS19.md) (`Fsfirst`-Reihenfolge, erst messen); zurueckgestellt COOK01; Ideen MEM06,
  IO01.
- **Luecke ohne Ticket — `shel_find` Stufe 4 (`PATH=`):** von EmuTOS'
  vierstufiger `findfile`-Ordnung (`gemshlib.c:406-472`) sind Stufe 1
  (App-Verzeichnis), 2 (Pfad wie uebergeben) und 3 (Wurzel des aktuellen
  Laufwerks) umgesetzt (`tos-api/src/aes/shell.rs:1074-1117`). Stufe 4 ist nicht
  gebaut (`tos-api/src/aes/shell.rs:1119`), aber nicht mehr blockiert: die
  AES-Umgebung mit `PATH=` gibt es seit PPARENT (`tos-api/src/aes/shell.rs:1030-1034`).
  Die Existenzprobe `read_file` ist fragil gegenueber einer `Fsfirst`-Probe
  (verfehlt Verzeichnis-Treffer, ignoriert Wildcards).
- **Wunsch ohne Ticket — Mehrbank-RAM:** `dtb.rs::parse_memory` liest nur die
  erste `/memory@…`-Bank (`kernel/src/arch/aarch64/dtb.rs:29`); auf der Pi
  sind das 768 MiB TT-RAM, die uebrigen ~3 GiB bleiben ungenutzt. Kein
  Blocker (768 MiB reichen ausdruecklich); gewuenscht: alle `reg`-Eintraege,
  diskontinuierliches TT-RAM, Seitentabellen fuer die hohe Region.

**Serielle Ausgabe, seit 2026-10-02 (SER01):** auf der Pi standardmaessig
**aus**; `serial=on` in `cmdline.txt` schaltet sie ein. QEMU-raspi4b-Laeufe
brauchen `-append serial=on`, alte Messskripte den Shim `tools/qemu-serial-on`.

**Regel zur SD-Karte, seit 2026-09-30:** `tools/make-sd-image.sh` baut das
**QEMU-Abbild**, **nicht** die echte Karte. Die physische Karte ist
handgepflegt; auf sie kommt **nur**, was der Anwender ausdruecklich nennt.

**QEMU-Laeufe:** QEMU-Sockets relativ im Laufverzeichnis (Anwenderwunsch
2026-10-16). Lange QEMU-Serien nur am Netzteil (Akku-Schlaf verfaelscht
Zeiten); Screendumps an Logmarken, ein QEMU zur Zeit. Scratch-Regel: CLAUDE.md
(*Placement rules*).

**Arbeitsweise:** ein Task pro Agent, seriell, nie parallel. Der Lead baut
nicht selbst; er prueft nach, committet und schreibt hier. Worker fassen
weder Karte noch Pi an und committen nur, wenn der Lead es verlangt.

---

## Phase Status

> Phasenplan seit 2026-05-03 nach den EmuTOS-Modulen (`bios/` / `bdos/` /
> `vdi/` / `aes/` / `desk/`); die alte Nummerierung liegt in
> `phases/_archive/`. Ein m68k-Emulator ist keine Phase dieses Plans; er
> bleibt eine spaetere Option fuer `.PRG`/`.TOS`/`.TTP` (CLAUDE.md).
> Gearbeitet wird heute in Tickets (`TASK/`). Die frueheren
> Langfassungen der Zeilen stehen im Archiv (Abschnitt *Phase Status —
> Langfassung*).

| Phase | Titel | Status | Stand | Einzelheiten |
|-------|-------|--------|-------|--------------|
| 00 | EmuTOS Source Analysis & Extraction | abgeschlossen | 2026-05-02 | `docs/api/_phase00-completion.md` |
| 01 | Boot + Early Console | abgeschlossen; bootet auf echter Pi 4 | 2026-09-04 | `phases/PHASE-01-boot-uart.md` |
| 02 | GEMDOS Console (Trap 1) | D1–D5 umgesetzt, Konsole auf echter Pi in Betrieb; bewusst ausgenommen: `console_peek_key` gibt dem Geraet keine Runde (`kernel/src/host_env.rs:87-89`) | 2026-09-04 | `phases/PHASE-02-gemdos-console.md` |
| 03 | BIOS HW Layer | abgeschlossen | 2026-09-04 | `phases/PHASE-03-bios-hw-layer.md` |
| 04 | BDOS Memory Layer (EmuTOS `bdos/umem.c`, `bdos/iumem.c`) | abgeschlossen | 2026-05-03 | `phases/PHASE-04-bdos-memory.md`, ADR-025, ADR-027 |
| 05 | High-Half Migration & Process ABI | abgeschlossen; D7 `Supexec` mit X01 entschieden (Super/Supexec als SDK-Overlay) | 2026-10-17 | `phases/PHASE-05-high-half-migration.md`, [X01](../TASK/X01.md) |
| 06 | BDOS Process | abgeschlossen; `PE_BASEPAGE`/`GOTHENFREE`/`BASEPAGEFLAGS` mit ARCH03 Stufe 2 (`32c6171c`), Scheduler nach EmuTOS `rlr`/`drl` mit A115 (`e7754b16`) | 2026-10-25 | `phases/PHASE-06-bdos-process.md`, ADR-028, ADR-029 |
| 07 | BDOS Misc (Time) | abgeschlossen, 200-Hz-Systemtick; Tastenwiederholung gepollt (KEYS01); `etv_timer`-Aufruf und `dosound` ohne Gegenstueck (`tos-api/src/bios/tick.rs`) | 2026-10-03 | `phases/PHASE-07-bdos-misc.md` |
| 08 | BIOS Block Devices (Trap 13) | abgeschlossen; Geraetetabelle mit FS10/FS11 | 2026-10-13 | `phases/PHASE-08-bios-block-devices.md` |
| 09 | BDOS File System | FAT16 komplett; FAT12-Schreiben ausserhalb des Umfangs, FAT32/exFAT nach 1.0 | 2026-10-19 | `phases/PHASE-09-bdos-filesystem.md`, [FS18](../TASK/FS18.md) |
| 10 | BIOS Screen | abgeschlossen; VT52-Auswertung in `Bconout(2)` mit CON04 | 2026-10-23 | `phases/PHASE-10-bios-screen.md`, [CON04](../TASK/CON04.md) |
| 11 | XBIOS (Trap 14) | substanziell fertig; dazu KEYS02 (`kbdvec`-Haken, Pi 2026-10-04), X01, X04; Rest-Vektoren unter *Blocked* | 2026-10-17 | `phases/PHASE-11-xbios.md` |
| 12 | VDI | komplett (~70 Opcodes); Pi-Abnahme R02–R05 offen | 2026-09-21 | `phases/PHASE-12-vdi.md` |
| 13 | AES | komplett; laufende EmuTOS-Angleichung in A-Tickets, zuletzt A118 | 2026-10-26 | `phases/PHASE-13-aes.md` |
| 14 | Desktop | EL0-Programm NEWDESK, 1:1 nach EmuTOS (ADR-022, ADR-026), Quelle `programs/desktop/`; offene DESK-Tickets P3 | 2026-10-16 | `phases/PHASE-14-desktop.md`, [DESK43](../TASK/DESK43.md) |
| 15 | First C Source Port | laufend: QED, TeraDesk, zView, Doom, HypView; SDK fuer Fremde (SDK01) | 2026-10-22 | `phases/PHASE-15-first-source-port.md`, `docs/sdk/` |
| 16 | USB-HID Input (xHCI + VL805) | laeuft auf echter Pi 4; offen USB01 (P1), USB03 (P2), USB02 (P3) | 2026-10-25 | `phases/PHASE-16-usb-hid-input.md`, [USB01](../TASK/USB01.md) |

## Open ADRs

| ID            | Topic                                   | Offen fuer |
|---------------|-----------------------------------------|-----------|
| ADR-BDOS-Lock | BDOS-Serialisierungsmodell auf SMP; kein ADR in `docs/MEMORY.md` | Mehrkern-Ausbau |

## Blocked

- **KEYS02 / D7-Luecke: IKBD-Vektoren ausser `kbdvec` werden nicht gerufen (2026-10-04).** `Kbdvbase()` liefert je Prozess die volle Tabelle (`0x9A0`), alle Slots enthalten gueltige EL0-Stubs; ein Programm-Haken in `midivec`, `vkbderr`, `vmiderr`, `statvec`, `mousevec`, `clockvec`, `joyvec`, `midisys`, `ikbdsys` oder `mousexvec` wird aber **nicht aufgerufen**. Maus kernelintern ueber `vdi::feed_relative`, Extra-Maustasten ueber `bios::vectors::mousexvec`; `ikbdsys` nur fuer den Wiederholungsstopp gelesen (D4). Es fehlen: Mauspakete an `mousevec` (Format A4), Joystick an `joyvec` (Klaerung HID), `statvec`/`clockvec` (brauchen `Ikbdws`), MIDI (keine Hardware). Spec `docs/api/xbios/22-kbdvbase.md` B7/C6. Weitere Grenzen: `Cconrs`/Konsolen-`Fread`/`v_string` weichen bei eingehaengtem Haken nicht aus (Programm haengt dort); Upcalls sichern keine FP/SIMD-Register (Haken muss ohne sie auskommen).

- **A45: `appl_tplay` und `appl_trecord` sind ohne die AES-Forkqueue nicht
  implementierbar (2026-09-13).** Beide sind heute Stubs im Sinne von
  CLAUDE.md Regel 2 — hartkodierter Rueckgabewert, Argumente ignoriert. Regel 2
  laesst dafuer genau zwei Wege, implementieren oder ehrlich abweisen und hier
  eintragen; dies ist der zweite. **Die fehlende Abhaengigkeit ist die
  Forkqueue**, nicht die Funktion selbst: EmuTOS zeichnet in `ap_trecd` gar
  nicht auf, sondern setzt `gl_recd`/`gl_rlen`/`gl_rbuf`
  (`aes/gemaplib.c:209-213`), und das Mitschreiben passiert in `forker()`
  (`aes/gemdisp.c:123-146`); `ap_tplay` speist die Ereignisse ueber
  `forkq`/`dsptch` zurueck und tauscht fuer `MCHNG` zusaetzlich die
  VDI-Cursor- und Bewegungsvektoren (`aes/gemaplib.c:126-166`). NewTOS' AES hat
  keine Forkqueue — dieselbe Luecke, die `tos-api/src/aes/mod.rs` an `chkkbd`
  fuer den Tastenpfad dokumentiert. Sie gehoert zur kooperativen Kernelarbeit
  (Meilenstein 2b); landet sie, sind diese beiden Funktionen ihre ersten
  Abnehmer. **Kein Fehlercode erfunden:** die AES kennt hier keine Konvention
  dafuer, und ein Wert, den kein TOS zurueckgibt, waere eine neue Abweichung
  anstelle der alten — `appl_tplay`s `1` ist EmuTOS' Dispatcher-Vorgabe
  (`aes/gemsuper.c:85,109-110`), `appl_trecord`s `0` ist woertlich wahr (null
  aufgezeichnete Ereignisse). **Keine Zielanwendung braucht den Ereignisband:**
  ein einziger Treffer im ganzen `sdk/`, die Deklaration in
  `sdk/gemlib/overlay/mt_gem.h.in`, kein Aufruf. Ticket `TASK/A45.md`.

## Recent Decisions

| Date       | Decision                                            |
|------------|-----------------------------------------------------|
| 2026-10-29 | **XBIOS15 Schritt 1 gemessen (`b57d32d6`) und entschieden; FS22-Folgepunkte; BIOS15-Umfang.** Audit `docs/audits/2026-10-29-xbios15-serial-rx.md`: das Orakel legt serielle Bytes in den Ring von Geraet 0 (= gemappter Port: MFP auf tt, SCC B auf falcontc), NewTOS qemu-virt mit Scancode 0 in den Tastaturring. Anwender: Option (a) — auf qemu-virt RX in den Ring von Geraet 0, ein AUX-Treiber liest daraus, Abnahme gleich dem Orakel, raspi4b unveraendert; SERRX-002 abgelehnt (kein Zweit-ROM, ein Pin), SERRX-003 (falsche Begruendung in `console.rs` und Spec) im Ticket, SERRX-004 (`scc_rx_interrupt_handler`) in TOOL17. Vor dem Fix geprueft: der Injektor speist IKBD ein, automatische qemu-virt-Laeufe nutzen seriell nur fuer Ausgabe; betroffen ist nur `cargo xtask run` (qemu-virt), wo im Terminal Getipptes heute als Tastatur ankommt — dem Anwender berichtet. FS22: Retest-Mengen und Ergebnisse der zusaetzlichen Pfade im Ticket (mit Medienwechsel nur per Hosttest belegt); Folgeticket [TOOL20](../TASK/TOOL20.md) (Sonden nach tosdiff) nach TOOL17. BIOS15: `Mfpint`/`Xbtimer` warnen und liefern den Rueckgabewert des Orakels (gemessen). |
| 2026-10-29 | **[FS22](../TASK/FS22.md): ein Neustart fuer alle Dateipfade nach Medienwechsel (FS20 Option B, Commit dieser Zeile).** `gemdos::restartable` ist der eine Neustart von `osif`: Fehlerkontext leeren, Aufruf ausfuehren, bei Medienwechsel Ergebnis verwerfen, Laufwerk neu anmelden, wiederholen; hoechstens 3 Neustarts, dann `E_CHNG` (EmuTOS unbegrenzt; Abweichung in `docs/COMPATIBILITY.md`). Darunter laufen `gemdos::dispatch`, `Pexec`-Laden (Trap, `launch.rs`, Accessories; Environment und TPA werden vor dem Neustart freigegeben wie `references/emutos/bdos/proc.c:319-332`), `bdos::fs::read_file` (Ressourcen, `shel_find`, `GEM.RSC`, `NEWDESK.INF`, `KEYBOARD.KBD`) und — ueber die Lokalisierung hinaus, gleiche Klasse — `find::list_dir` (Dateiauswahl, Accessory-Lader, AUTO-Ordner) sowie die direkten GEMDOS-Aufrufe der AES-Shell (`Dsetpath`, `Dgetpath`, `Dsetdrv`, `Fdelete`). Spec `docs/api/gemdos/_state.md` §4a, „Open [FS20]“ aufgeloest. Sonde `pxprobe` dauerhaft unter `tosdump/probes/`, beidseitig mit Eingabe-Trace: `Pexec` nach Schreiben von Sektor 0 liefert 42 wie das Orakel (vorher `EFILNF`), DTA nach `Fsfirst` gleich. Keine neue Abweichung (alle Plattentabellen, `bios-r1`, `gcprobe`). Hosttests 1790/0/1 (Lead-Gegenlauf gleich). Beleg `measurements/measurement-2026-10-29-fs22/RUNINFO.TXT`. |
| 2026-10-29 | **XBIOS01 und BIOS11 abgenommen (`a32921d5`); Folgepunkte.** Anwender: beide geschlossen. `Rsconf` schreibt Baudrate/Flusssteuerung nicht in den Record von Geraet 0 → Teilpunkt in [XBIOS06](../TASK/XBIOS06.md) (MEASURED-Sonde beidseitig: `Rsconf`, dann Feld-Dump `Iorec(0)`). Neues Ticket [XBIOS15](../TASK/XBIOS15.md) (P2): auf `qemu-virt` landen serielle Bytes mit Scancode 0 im Tastaturring; zuerst am Orakel messen, wo serielles RX landet (erwartet Geraet-0-Ring, EmuTOS `push_serial_iorec`); lokalisiert. [MEM12](../TASK/MEM12.md) Prueffolge echte Pi um USB-Tastatur → `Iorec(1)`-Ring und K16 mit physischer Umschalttaste ergaenzt. |
| 2026-10-29 | **[XBIOS01](../TASK/XBIOS01.md): `Iorec` verdrahtet, der IKBD-`IOREC` ist die Tastenschlange; schliesst [BIOS11](../TASK/BIOS11.md) (Commit dieser Zeile).** `Iorec(0/1/2)` liefert Zeiger in eine gemeinsame, fuer jeden Prozess EL0-les-/schreibbare Seite `0x7D000` (LP64-`_IOREC`, Geraet 0 56 Byte, Werte wie das Orakel), sonst `-1` (vorher: Funktionsnummer `0x0E`, Bus Error beim Flush). `RX_RING` entfaellt: Dekoder, Wiederholung, qemu-virt-Seriell, `Bconin(2)`/GEMDOS-Konsole, AES und `Fread` auf `CON:` gehen ueber den Ring (E1); Eintraege 32 Bit mit Shift zum Push-Zeitpunkt (E2, K16 jetzt `0x02300042` wie EmuTOS). Der Kernel prueft den Record vor jeder Nutzung und nach jedem Trap, repariert ersetztes `buf`/`size` und kaputte Indizes und warnt einmal je Prozess (E4 (a)); keine Zustellung ohne Trap (E3 (a)); Geraete 0/2 leer (E5); `kbdvec`-Haken bekommt den IOREC in `x1` (E6). Gemessen: `xbios-ptr-r1` 24-31, `bios-r1` 98-102, `xbios-chardev-r1` 21-27 jetzt EQUAL (tt und falcontc), kein vorher gleicher Fall in fuenf Tabellen neu verschieden; neue Tabelle `xbios-flush-r1.tsv` mit drei Tasten im Ring vor dem Flush (Orakel per Hatari `--cmd-fifo`, NewTOS per Injektor) beidseitig 0/207 DIFF; XCIOFL ohne Bus Error, Sonde I01-I08 gleich bis auf Adressen/`sizeof`. `sdk/libc/tests/iorec-layout.c` prueft `_IOREC` gegen die Spec. Beleg `measurements/measurement-2026-10-29-xbios01/RUNINFO.TXT`. |
| 2026-10-29 | **FS20 Option B: [FS22](../TASK/FS22.md).** Pruefung `docs/analysis/osif-restart-coverage-2026-10-29.md` (`7dc40b74`): EmuTOS hat einen Neustart in `osif` (`bdos/bdosmain.c:449`, `:455-486`), AES und Desktop rufen ueber `trap #1`, `Pexec` raeumt in einem verschachtelten `setjmp` auf (`bdos/proc.c:319-332`). Anwender: Option B — ein gemeinsamer Neustart-Helfer fuer alle kernel-internen Dateipfade (`Pexec`-Laden, `read_file`-Nutzer) und den `dispatch`-Neustart, Aufraeumen je Pfad vor dem Neustart, Wiederholungsgrenze (z. B. 3) mit `E_CHNG` als dokumentierte Abweichung (EmuTOS unbegrenzt); Sonden beidseitig: `Pexec` nach Schreiben von Sektor 0, `Fsfirst`-DTA nach dem Neustart. P2, nach XBIOS01. |
| 2026-10-29 | **[TOOL16](../TASK/TOOL16.md): Dumper liest und schreibt durch zurueckgegebene Zeiger (Commit dieser Zeile).** Strukturlayouts stehen einmal in der Spec (Block `tosdump-layout` in `docs/api/structures/iorec.md` und `md.md`, je Feld Typ, Offset m68k/LP64, Vergleichsklasse); die Falltabelle traegt eine Kopie, der Hosttest `layoutcheck` weist jede Abweichung ab. Capture `r:<layout>` folgt einem Zeiger nur innerhalb der `PTR_RAM`-Grenzen (sonst Grund-Slot, kein Lesen); `+put <feld>=<feld>` schreibt nur in Tabellen mit `!write return`. `Getmpb` ist innerhalb einer Super-Klammer zugelassen. Neue Tabelle `xbios-ptr-r1.tsv` (9 Faelle), Referenzen tt/falcontc je zweimal bitgleich; NewTOS 6/9 DIFF wie erwartet (Iorec liefert `0x0E` und wird nicht gelesen, `Getmpb` im Mischformat). Grenze: der Ring war leer, der Flush-Fall belegt das Leeren eines vollen Rings noch nicht (in XBIOS01 vermerkt). Bestehende Tabellen mit dem neuen Dumper am Orakel unveraendert. `tosdump` host-test 32 Referenzen, Hosttests 1767/0/1. Beleg `measurements/measurement-2026-10-29-tool16/RUNINFO.TXT`. |
| 2026-10-29 | **[FS20](../TASK/FS20.md): GEMDOS meldet ein Laufwerk nach Medienwechsel neu an (S1, Commit dieser Zeile).** Nach einem geaenderten Bootsektor schrieb GEMDOS mit alter Geometrie in einen anderen Cluster. Jetzt wie EmuTOS `osif`: Mediach-Test beim Puffertreffer und vor jedem Geraetezugriff, Neuanmeldung mit `offree` (offene Dateien → `EIHNDL`), `freetree` (aktuelle Verzeichnisse → Wurzel), `mark_bcbs_invalid` (Puffer verworfen), `Getbpb` + `log_media` und Neustart des Aufrufs; Spec `docs/api/gemdos/_state.md` §4a. Abnahme nach Sondenregel: `gcprobe` dauerhaft unter `tosdump/probes/`, beidseitig mit Eingabe-Trace: A07 0, B04 32676, B05 0, Cluster 3 in Sektor 277 wie das Orakel (vorher 2/32672/2, Sektor 293). Einzige Abbild-Abweichung: `FA_ARCHIVE` beim Schliessen fehlt (Altbefund, [FS21](../TASK/FS21.md)). Kein vorher gleicher Fall neu verschieden. Designfrage beim Anwender: Kernel-Dateileser ausserhalb von `dispatch` (`Pexec`, `read_file`) melden nicht neu an. Hosttests 1767/0/1 (Lead-Gegenlauf gleich), `mapping.tsv` um `mark_bcbs_invalid` u. a. ergaenzt. Beleg `measurements/measurement-2026-10-28-fs20/RUNINFO.TXT`. |
| 2026-10-29 | **ABI-Pruefung umgesetzt; Entscheidungen zu BIOS06, BIOS12, XBIOS05; neue Tickets [SDK05](../TASK/SDK05.md), [SDK06](../TASK/SDK06.md).** Pruefung (`docs/analysis/abi-assumptions-check-2026-10-28.md`, `d47be1b4`): Regel „LONG in `x0` vorzeichenerweitert“ (`tos-api/src/trap.rs:6@TrapArgs`) mit Abweichungen `Rsconf`, `Gettime`, `Setexc`-Platzhaltern und der Zeigerform von `Getcookie` (Nebeneffekt, kein Nutzer); `MCH_NOHARD` ist `references/emutos/include/cookie.h:50`; kein Port ruft ueber die `Bconmap`-MAPTAB. Anwender: Nachtrag ADR-012 und Portier-Checkliste auf den belegten Stand abgeschwaecht; Regel und Abweichungen in `docs/COMPATIBILITY.md`, ebenso `_MCH` als ACCEPTED und die MAPTAB-Zeiger (aus EL0 nicht aufrufbar, laut; wiedervorlegen bei Nutzer). BIOS06: Layout A, statischer Schnappschuss, `Getmpb(NULL)` erst am Orakel messen, Sonde G00–G06 bis TOOL16. BIOS12: Vorzeichenerweiterung bleibt, Abweichungen werden behoben (BIOS07, XBIOS06, XBIOS09, SDK06); `-1`-Abfrage auf den unteren 32 Bit (P3, Sonde zuerst); `num >= 0x200` liefert 0; Seite 0 traegt alle `num < 0x200`, nicht bediente Vektoren auf `just_rts`; `Kbshift` ein echtes Byte; ADR-031-Zeile zu `etv_timer` (EmuTOS-eigene Kette `tikfrk`/`tick_int`). XBIOS05: eine Tabelle wie EmuTOS auf dem ST, Orakelzeile `Bconmap(-2)` mit 8 + 24 Byte. XBIOS06: Schluss zum TSR-Byte gegen die Quelle geprueft (EmuTOS nutzt Bit 7 selbst als „Sender leer“, `serport.c:277`). cflib baut Cookie-Namen per Byte-Union und findet auf Little-Endian `_IDT` nicht (QED nimmt `0x112E`; abgeleitet) — SDK05, Sonde zuerst; SDK04 sucht dieselbe Klasse in allen Ports. Die Kommentare „zero-extended“ in `system.rs` aendern sich mit dem Rsconf-Fix (sie beschreiben den heutigen Code richtig). |
| 2026-10-28 | **FS20 vor allem anderen; `DISKUNIT.TXT` unter Bedingungen ([TOOL19](../TASK/TOOL19.md)); TeraDesk `find_cookie` vor ARCH06; Portier-Checkliste.** Anwender: [FS20](../TASK/FS20.md) (S1, GEMDOS schreibt nach geaendertem Bootsektor mit alter Geometrie) kommt als Naechstes, abgenommen nach Sondenregel (dauerhafte Sonde unter `tosdump/`, Bootsektor per `+set` auf frischem Klon aendern, GEMDOS-Schreiben, Abbildvergleich beider Seiten), spaeter nach DUMPGDOS; Lokalisierung im Ticket (`ckdrv` → `fat.rs@mount`, `log_media` → `fat.rs@log_in`, `getbcb` → `cache.rs`; `mark_bcbs_invalid` ohne Gegenstueck). `DISKUNIT.TXT` als Quelle der Einheit ist in Ordnung, wenn sie immer aus der Profildefinition entsteht, im RUNINFO steht und der Runner bei fehlender oder unpassender Einheit abweist, ohne Standardwert (TOOL19). TeraDesks `find_cookie` (16-Byte-Schritt, SDK04 C6) wird vor der `_VDO`-Messung von [ARCH06](../TASK/ARCH06.md) behoben. `Getcookie` erweitert mit Nullen: `0xFFFFFFFF` ist fuer native Ports gross positiv, numerische `_MCH`-Vergleiche nehmen den Pfad „ueber Falcon“ — im Nachtrag zu ADR-012 und als Punkt in `docs/sdk/08-porting-checklist.md`. |
| 2026-10-28 | **[BIOS01](../TASK/BIOS01.md) S2: forcechange-Handshake (BIOS-STOR-001, Commit dieser Zeile); S1-Fund [FS20](../TASK/FS20.md).** Ein logisches Schreiben ab Sektor 0 setzt je Laufwerk ein Flag vor der Bereichspruefung; danach liefern `Rwabs` (auch mit `RW_NOMEDIACH`) `E_CHNG` und `Mediach` 2, bis `Getbpb` (auch die BDOS-Anmeldung) es loescht — Pruefreihenfolge nach Spec. `bios-r2` W01–W05, X13, X14 und die 001-Faelle in x004–x006 jetzt EQUAL (`bios-r2` 3/51 verschieden statt 10: 003, 007); kein vorher gleicher Fall neu verschieden (`bios-r1` tt/falcontc, alle Plattentabellen). Neue Tabelle `bios-r2-x001.tsv` (Sektor 0 mit geaendertem Byte ohne `RW_NOMEDIACH`) belegt per Abbildvergleich „schreibt nichts“: vorher 4/8 verschieden und Sektor 0 geaendert, nachher 0/8, Testpartition = Original = Orakel. GEMDOS-Folge per Sonde `gcprobe` beidseitig: **S1** — nach einem geaenderten Bootsektor schreibt NewTOS' GEMDOS mit alter Geometrie in einen anderen Cluster (vor und nach dem Fix; `Volume::mount` hat keinen Medienwechsel-Pfad); und seit dem Fix bleibt `Mediach` 2 nach einem GEMDOS-Zugriff stehen, wo EmuTOS neu anmeldet. Beides FS20. Hosttests 1760/0/1 (Lead-Gegenlauf gleich), `tosdump` host-test 30 Referenzen, `mapping.tsv` um die `blkdev_*`-Zeilen ergaenzt. Beleg `measurements/measurement-2026-10-28-bios01-s2/RUNINFO.TXT`. |

## Notes

The project is intentionally over-documented at the start. The premise:
spec-first development against a fixed, finite, well-documented API
prevents the implementation drift that ruins long-running OS hobby
projects.
