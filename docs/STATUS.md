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
| Hosttests (`cargo xtask test`) | **1895 / 0 / 1** (gemessen 2026-10-30 mit COOK02 `_IDT`, kein neuer Test; davor 1895 mit XBIOS04, +1 Test; davor 1894 mit BIOS09, +1 Test; davor 1893 mit BIOS17, +1 Test; davor 1892 mit BIOS16, +4 Tests; davor 1888 mit BIOS08, +7 Tests; davor 1881 mit XBIOS05, +9 Tests, Basis laut Lead 1872; davor 1859 mit BIOS07, +9 Tests; davor 1850 mit FS23, +3 Tests; davor 1847 mit BIOS04, +4 Tests; davor 1843 mit XBIOS02, +1 Test; davor 1842 mit BIOS06, +2 Tests; davor 1840 mit BIOS01 Rest, +7 Tests; davor 1833 mit TOOL19, +5 Tests; davor 1828 mit ARCH06, +4 Tests; davor 1824 mit XBIOS18, +1 Test; davor 1823 mit TOOL21, +2 Tests; davor 1821 mit TOOL17, +5 Tests; davor 1816 mit XBIOS15, +8 Tests; davor 1808 mit BIOS15, +18 Tests; davor 1790 mit FS22, +7 Tests; davor 1783 mit XBIOS01, +16 Tests; davor 1767 mit FS20, +7 Tests; davor 1760 mit BIOS01 S2, +9 Tests; davor 1751 mit TOOL07 Rest, +3 xtask-Tests; davor 1748 mit TOOL12, +3 deps-Tests; davor 1745 mit TOOL10; TOOL09 +16, TOOL10 +3 xtask-Tests; vorher 1726 / 0 / 1 auf `75527af8`, `--fast` ohne den einen Doctest) |
| SDK-Hosttests (`sdk/run-host-tests.sh`) | **35 / 0 / 0** (gemessen 2026-10-29 mit XBIOS05, +1 `bconmap-layout`; davor 34 mit SDK07, +1 `td-nohard-host`; davor 33 mit SDK04, +3 `sysvars-layout`, `hypview-ref-host`, `zview-color-host`; davor 30 mit SDK04 C6, +1 `td-cookie-host`; davor 29 mit SDK05, +1 `cookie-host`; davor 28 mit SDK03, +1 `sysinfo-host`; davor 27 mit XBIOS01, +1 `iorec-layout`) |
| `sh tools/desk-drift-probe.sh` | **0** (unsafe 20; gemessen 2026-10-26) |
| APITEST (`#Z`, memprot on/off/strict) | **109 / 0 / 5** (letzter Lauf `measurements/measurement-2026-10-26-a118-fix/RUNINFO.TXT`) |

Bau- und Clippy-Warnungen stehen hier ohne Zahl: sie wurden bei dieser
Bereinigung nicht nachgemessen. Sie duerfen nicht steigen; jeder Arbeitsschritt
nennt seine Vorher-/Nachher-Zahl in seiner Zeile unter *Recent Decisions*.

**Offen (Stand 2026-10-26, aus der Statuszeile der Tickets ohne DONE-Quittung):**

- **Warteschlange (Anwender 2026-10-29), in dieser Reihenfolge.** Regel: ein
  neues Werkzeug-Ticket kommt hinter den Produkt-Fix, den es verzoegern wuerde,
  ausser der Fix ist ohne das Werkzeug nicht abnehmbar.
  Developer, einer zur Zeit:
  die S2-Audit-Tickets [XBIOS02](../TASK/XBIOS02.md) (erledigt 2026-10-29, `7bc047a6`),
  [BIOS04](../TASK/BIOS04.md) (erledigt 2026-10-29, Commit der Quittung) →
  [FS23](../TASK/FS23.md) (erledigt 2026-10-29, Commit der Quittung), [FS24](../TASK/FS24.md) (ACCEPTED unter ADR-033, Anwender 2026-10-30) →
  die uebrigen Audit-Tickets [BIOS07](../TASK/BIOS07.md) (erledigt 2026-10-29, Commit der Quittung),
  BIOS08–BIOS14 ([BIOS10](../TASK/BIOS10.md) erledigt 2026-10-29, Commit der Quittung; [BIOS08](../TASK/BIOS08.md) erledigt 2026-10-30, Commit der Quittung), XBIOS04–XBIOS14 ([XBIOS05](../TASK/XBIOS05.md) erledigt 2026-10-29, Commit der Quittung; BIOS05 und XBIOS03
  sind ACCEPTED nach ADR-031, XBIOS10 nach ARCH05) →
  [TOOL13](../TASK/TOOL13.md), [TOOL14](../TASK/TOOL14.md),
  [TOOL15](../TASK/TOOL15.md) → [TOOL03](../TASK/TOOL03.md) → DUMPGDOS →
  [FS19](../TASK/FS19.md) → [MEM11](../TASK/MEM11.md) (Erzwingung) →
  [TOOL01](../TASK/TOOL01.md) → [FS26](../TASK/FS26.md) (Ordner anlegen
  ~3 s auf der Pi, erst messen) → [FS25](../TASK/FS25.md) (Messung),
  [PROC07](../TASK/PROC07.md), [COOK02](../TASK/COOK02.md) (geparkt; Punkt 3 `_IDT` = Orakel umgesetzt 2026-10-30, Commit der Quittung),
  [TOOL22](../TASK/TOOL22.md), [XBIOS19](../TASK/XBIOS19.md), [TOOL23](../TASK/TOOL23.md),
  [SPEC02](../TASK/SPEC02.md), [TOOL24](../TASK/TOOL24.md), [FS27](../TASK/FS27.md), [TOOL25](../TASK/TOOL25.md) (P3); [BIOS16](../TASK/BIOS16.md) (erledigt 2026-10-30, `9c4c29c5`) → [BIOS17](../TASK/BIOS17.md) (erledigt 2026-10-30, Commit der Quittung) → [BIOS09](../TASK/BIOS09.md) (umgesetzt 2026-10-30, Commit der Quittung; offen der Pi-Schritt in der MEM12-Prueffolge, Punkt 10) → [XBIOS04](../TASK/XBIOS04.md) (umgesetzt 2026-10-30, Commit der Quittung) → XBIOS06/08/09/10 → [FS28](../TASK/FS28.md), [A126](../TASK/A126.md); [TOOL26](../TASK/TOOL26.md) (P3); [FS28](../TASK/FS28.md), [A126](../TASK/A126.md) (P2), [KEYS04](../TASK/KEYS04.md), [INPUT03](../TASK/INPUT03.md) (P3); [SDK02](../TASK/SDK02.md), [SDK06](../TASK/SDK06.md)
  (P3) bei Gelegenheit; [UART01](../TASK/UART01.md) (P2, neu 2026-10-30: Pi-Messung
  in der MEM12-Prueffolge Punkt 11 zuerst); [INPUT04](../TASK/INPUT04.md) (P3, 0xF6-Paketlaenge,
  ruht; vor jedem IKBD-Interpreter); [TOOL27](../TASK/TOOL27.md) (P3,
  rsprobe nach tosdiff). Zurueckgestellt (Anwender 2026-10-29):
  [DOOM01](../TASK/DOOM01.md) (DOOM-Freeze im Menue, nur auf der Pi).
  Daneben ein Researcher zur Zeit, ohne Produktaenderung: — (frei).
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

- **KEYS02 / D7-Luecke: IKBD-Vektoren ausser `kbdvec` werden nicht gerufen (2026-10-04).** `Kbdvbase()` liefert je Prozess die volle Tabelle (`0x9A0`), alle Slots enthalten gueltige EL0-Stubs; ein Programm-Haken in `midivec`, `vkbderr`, `vmiderr`, `statvec`, `mousevec`, `clockvec`, `joyvec`, `midisys`, `ikbdsys` oder `mousexvec` wird aber **nicht aufgerufen**. Maus kernelintern ueber `vdi::feed_relative`, Extra-Maustasten ueber `bios::vectors::mousexvec`; `ikbdsys` nur fuer den Wiederholungsstopp gelesen (D4). Es fehlen: Mauspakete an `mousevec` (Format A4), Joystick an `joyvec` (Klaerung HID), `statvec`/`clockvec` (brauchen einen IKBD-Interpreter; nach [XBIOS04](../TASK/XBIOS04.md) Option 1 gibt es keinen, `Ikbdws`/`Bconout(4)` warnen nur), MIDI (keine Hardware). Spec `docs/api/xbios/22-kbdvbase.md` B7/C6. Weitere Grenzen: `Cconrs`/Konsolen-`Fread`/`v_string` weichen bei eingehaengtem Haken nicht aus (Programm haengt dort); Upcalls sichern keine FP/SIMD-Register (Haken muss ohne sie auskommen).

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
| 2026-10-30 | **[COOK02](../TASK/COOK02.md) Punkt 3: `_IDT` ist `0x112E` wie das Orakel (Commit dieser Zeile).** Anwenderentscheidung 2026-10-30: NEWDESK nimmt bei der Datumseinstellung „Standard“ den Trenner aus `_IDT` (`programs/desktop/src/model.rs:194-201@rendered`, wie EmuTOS `references/emutos/desk/deskinf.c:140-145`); deshalb setzt `seed_defaults` jetzt `0x0000112E` (`.`, DD-MM-YY, 24 h; `tos-api/src/cookie.rs:226@IDT`) statt `0x0000112F` (`/`, „bring-up policy“). `_AKP` bleibt `0` (Fall 52 ACCEPTED). `jar-r1` Fall 53 jetzt EQUAL gegen tt, falcontc und disk (15 → 14 von 22 verschieden), kein anderer Fall veraendert; kein neu abweichender Fall in `bios-r1` (tt 32, falcontc 37 von 147) und `xbios-system-r1` (9/9 von 43), vorher `a8d875a7`. `docs/COMPATIBILITY.md` Fall 53 EQUAL; `docs/api/bios/_cookie-jar.md`, `docs/HARDWARE-PROFILE.md`, `docs/api/desktop/{deskinf,gembind}.md`, `docs/api/emucon/{04-builtins,05-config-strings-hardware}.md` nachgezogen. Hosttest `tos-api/tests/gemdos_ssystem_idt_cookie.rs` prueft `0x112E` und `.`; die Test-Keksdosen `sdk/cflib/tests/cookie-host.c`, `sdk/overlays/teradesk/tests/cookie-host.c` tragen den neuen Wert. Hosttests 1895/0/1 (= Basis); SDK-Hosttests 33/2/0: `rt-host` und `ldbl-host` scheitern nur an der Rebuild-Paritaet, weil der Host-clang jetzt `clang-2100.4.1.1` ist, die ausgelieferten Archive aber mit `clang-2100.3.34.2` gebaut sind (`.comment`), unabhaengig von COOK02; `cookie-host`, `td-cookie-host` PASS. Bau beider Boards rustc 93/64, Clippy 121/74 wie Basis. Beleg `measurements/measurement-2026-10-30-cook02-idt/RUNINFO.TXT`. |
| 2026-10-30 | **[XBIOS04](../TASK/XBIOS04.md): `Ikbdws` und `Bconout(4)` warnen einmal je Prozess „IKBD commands ignored“ (Commit dieser Zeile).** Anwenderentscheidung 2026-10-30, Option 1 der Researcher-Note `docs/analysis/xbios04-ikbdws-scope-2026-10-30.md`: kein IKBD-Interpreter, keine Antwortpakete; `Ikbdws` bleibt `void` (0), `Bconout(4,c)` 1, die Bytes verworfen. Neu in der ADR-031-Warnschlange (`tos-api/src/bdos/process/hooks.rs`): ein eigener Warnschluessel hinter den Haken (`note_ikbd_ignored`, `Kind::IkbdIgnored`), gemeinsam fuer beide Wege — einmal je Prozess, welcher Weg zuerst kommt; ein neuer Prozess, auch im selben Slot, warnt wieder. `docs/COMPATIBILITY.md` „IKBD commands are not interpreted“: Audit-Sonde W01–W04 ACCEPTED. Specs `docs/api/xbios/19-ikbdws.md` (veralteter Text „nicht verdrahtet, `EINVFN`“ und „PHASE-07 interpretiert“ berichtigt) und `docs/api/bios/03-bconout.md`; `mapping.tsv` +2 Zeilen (`ikbdws`, `bconout4` → `note_ikbd_ignored`). Kein neu abweichender Fall in `bios-r1`, `bios-core-r1`, `xbios-chardev-r1`, `xbios-system-r1` (tt/falcontc, vorher `9f71ded8`); `MDPROBE.LOG` byte-gleich mit dem BIOS04-Lauf, die Warnzeile steht genau einmal im seriellen Log. Hosttests 1895/0/1 (+1); Bau beider Boards rustc 93/64, Clippy 121/74 wie Basis. Beleg `measurements/measurement-2026-10-30-xbios04/RUNINFO.TXT`. |
| 2026-10-30 | **[BIOS09](../TASK/BIOS09.md): `Bconin(1)` und `Bconin(6)` warten auch auf der Pi wie EmuTOS' `bconin1` (Commit dieser Zeile).** Anwenderentscheidung Option (a), Audit-Fund BIOS-CORE-008: der AUX-Treiber von XBIOS15/XBIOS18 (`AuxSerial`, `aux_read_blocking`, `install_aux`) laeuft jetzt auf beiden Boards, `serial_rx_byte` legt jedes empfangene Byte auch auf rpi4 in den Eingangsring von `Iorec(0)` (vorher verworfen; `kernel/src/console.rs`). `Bconin(1)` — mit ihm `Bconin(6)`, `Cauxin` und `Fread` auf AUX — wartet auf ein Byte, mit oder ohne Kabel; waehrend des Wartens laeuft kein anderer Prozess, nur der `kbdvec`-Haken des wartenden (Neustart wie XBIOS18), Strg+Alt+Entf wirkt. Die Warteschleife steht jetzt in `tos-api/src/bios/chardev.rs` (`aux_read`, der Kern legt `wfi` darum) und ist auf dem Host getestet. FS11 Punkt 1a „documented divergence“ fuer `Bconin` widerrufen (kein COMPATIBILITY-Eintrag); `AuxNoCable` bleibt Tabellen-Vorgabe und Ausgabeseite. Keine Registeraenderung: ohne Adapter kann die offene RXD0-Leitung beim Start ein `0x00` einspielen, dann kehrt das erste `Bconin(1)` einmal mit 0 zurueck (Pi-Schritt in der MEM12-Prueffolge, Punkt 10, Sonde `tosdump/probes/awprobe.c`). Gemessen (QEMU raspi4b): Haengesonde `Bconin(1)`/`Bconin(6)` blockieren, BCHANG.LOG gleich dem Orakel tt; `awprobe` mit Injektor: 4 Haken-Aufrufe waehrend des Wartens, serielles `Q` beendet es, Strg+Alt+Entf startet neu; kein neu abweichender Fall in `bios-r1`, `bios-core-r1`, `xbios-chardev-r1`, `xbios-system-r1` (tt/falcontc, vorher `f065a705`). Specs `docs/api/bios/{01,02}`, `docs/api/gemdos/file/3f-fread.md` (1a), `docs/api/gemdos/console/{03,12}`, `docs/api/xbios/{0e,22,2c}`, `docs/COMPATIBILITY.md` (Iorec, AUX-Ausgabe); `mapping.tsv` 41 Zeilen nachgezogen (Anmerkungen „rpi4: AuxNoCable“/„rpi4 dropped“), +1 `aux_read`. Hosttests 1894/0/1 (+1); Warnungen unveraendert (rustc 93 kernel / 64 tos-api, clippy 121 / 74). Beleg `measurements/measurement-2026-10-30-bios09/RUNINFO.TXT`. |
| 2026-10-30 | **[BIOS17](../TASK/BIOS17.md): `swv_vec` haelt den Reset-Einstieg wie EmuTOS `reseth`; `RESET_STUB_*` heisst `RESET_ENTRY_*` (Commit dieser Zeile).** Anwenderentscheidung 2026-10-30 nach BIOS16: EmuTOS `bios_init` setzt `swv_vec` erst auf `just_rts`, an seinem Ende unbedingt auf `os_header.reseth` (`references/emutos/bios/bios.c:363`, `references/emutos/bios/bios.c:552`); NewTOS liess 0. Jetzt schreibt der Lader in jede Seite 0 bei `$46E` den Reset-Einstieg, den Wert von OSHEADER `reseth` (`tos-api/src/bdos/process/loader.rs:1242@RESET_ENTRY_VA`), und `PAGE0_HOOKS` hat ihn als Startwert (`tos-api/src/bdos/process/hooks.rs:284@SwvVec`), damit die BIOS15-Warnung fuer den Startwert schweigt; nie aufgerufen (ADR-031, kein Monitorwechsel). Umbenennung `RESET_STUB_VA`/`RESET_STUB_CODE` → `RESET_ENTRY_VA`/`RESET_ENTRY_CODE` und „reset stub“ → „reset entry“ in Code-Kommentaren und Specs, rein mechanisch: ein Kern nur mit der Umbenennung ist bytegleich mit `f2652440`. `vwprobe` V01 jetzt gleich dem Orakel (`is-reseth=1`, tt/falcontc); sonst bleibt der Unterschied zum Orakel E01/T02 wie bei BIOS16; `sxprobe`, `hkprobe`, MEM12 P0PAR unveraendert. Die Testprozesse der Hosttests BIOS07/15/16 bauen Seite 0 jetzt mit den `PAGE0_HOOKS`-Startwerten wie der Lader (sonst warnte jeder `swv_vec` = 0). Specs `docs/api/structures/osheader.md`, `docs/api/bios/_reset-warm-cold.md`, `docs/api/bios/05-setexc.md`, `docs/api/gemdos/154-ssystem.md`, `docs/api/_super-state.md`, `docs/COMPATIBILITY.md`; verschobene Anker nachgezogen; `mapping.tsv` +1 (`bios_init` → `populate_sysvars`). Hosttests 1893/0/1 (+1 `swv_vec_is_reseth_the_reset_entry`); Warnungen unveraendert (rustc 93 kernel / 64 tos-api, clippy 121 / 74). Beleg `measurements/measurement-2026-10-30-bios17/RUNINFO.TXT`. |
| 2026-10-30 | **[BIOS16](../TASK/BIOS16.md): eine direkt in Seite 0 geschriebene Vektorzelle `$008`–`$407` wird die `Setexc`-Zelle (Commit dieser Zeile).** Anwenderentscheidung 2026-10-30 (Option B, Besitz ja): schreibt ein Programm im Super-Zustand seine Seite-0-Kopie eines Vektors 2…0xFF, von `etv_timer` oder `etv_critic`, uebernimmt die Seite-0-Pruefung den Wert in die geteilte Zelle (`tos-api/src/bios/vectors.rs:286@adopt_page0_write`, aus `tos-api/src/bdos/process/hooks.rs:314@check_page0`), jede Seite 0 traegt ihn, `Setexc(n,-1)` liefert ihn wie EmuTOS (eine Zelle, `references/emutos/bios/bios.c:1070-1080`). Besitz wie ein `Setexc`-Schreiben (ADR-031 Regel 3: `Pterm` gibt den Vorwert zurueck, `Ptermres` behaelt), Warnung einmal je Haken und Prozess, nie aufgerufen. Damit schon der naechste Aufruf den Wert sieht, laeuft die Pruefung fuer einen Prozess im Super-Zustand auch **vor** jedem Trap (`hooks::before_trap`, vom Kern aus `rust_sync_handler`; Lead: bitte bestaetigen, die Entscheidung sagt „am Ende“). `$408` (`etv_term`, bedient) bleibt: Warnung, `Setexc(0x102,-1)`/`Pterm` nehmen den per `Setexc` gesetzten Wert. Neue dauerhafte Sonde `tosdump/probes/vwprobe.c`: W01–W04/S01/T01 jetzt gleich dem Orakel (tt = falcontc), vorher alt; T02 (Rueckgabe beim `Pterm`) und E01 (`$408`) als beobachtbare Unterschiede in `docs/COMPATIBILITY.md` („A vector cell written directly in page 0“); Eingabe per `bios-arg-trace`. **`swv_vec` nicht vorbelegt:** EmuTOS ueberschreibt `just_rts` (`references/emutos/bios/bios.c:363`) in `bios_init` mit `reseth` (`references/emutos/bios/bios.c:552`), gemessen V01 — Entscheidung beim Anwender. Kommentare `apitest.c`/`tick.rs` berichtigt. `bios-core-r1` 2/1, `super-r1` 0/0 vorher = nachher, `sxprobe`, `hkprobe`, P0PAR unveraendert. Spec `docs/api/bios/05-setexc.md`; `mapping.tsv` +2 (`adopt_page0_write`, `note_shared_write`). Hosttests 1892/0/1 (+4 `tos-api/tests/bios16_page0_write.rs`); Warnungen unveraendert (rustc 93 kernel / 64 tos-api, clippy 121 / 74). Beleg `measurements/measurement-2026-10-30-bios16/RUNINFO.TXT`. |
| 2026-10-30 | **[BIOS08](../TASK/BIOS08.md): BIOS-Handles ab 6 laufen ueber die `Bconmap`-Tabelle mit einem Eintrag wie EmuTOS (Commit dieser Zeile).** Profil aus der XBIOS05-Entscheidung (Option 1): EmuTOS nimmt in `Bconstat`/`Bconin`/`Bconout`/`Bcostat` jeden Handle ab 6 vor der Vektortabelle aus `bconmap_root` (`references/emutos/bios/bios.c:923-928,967-972,1000-1005,1152-1157`); mit `maptabsize` 1 ist Handle 6 = Eintrag 0 = AUX, ab 7 → 0. Vorher waren 6/7 das abwesende Geraet und 8/9 jenseits der Tabelle (alles 0). Jetzt leitet `chardev::device` (`tos-api/src/bios/chardev.rs`) wie EmuTOS um, `bconmap::entry_device` nennt den Port des Eintrags (Geraet 1, auch ein dort installierter Treiber); die Geraetetabelle endet bei 5. `Bconin(6)` ist der Trap-Eintrag von AUX: wartet auf qemu-virt wie `Bconin(1)` (nur Hosttest, Tester), auf raspi4b 0 sofort wie `Bconin(1)` (gemessen). `bios-r1` 69 `Bcostat(6)` jetzt EQUAL (tt, falcontc); 70 und `bios-core-r1` 65/66 bleiben verschieden = Portzahl, ACCEPTED in `docs/COMPATIBILITY.md` („`Bconmap`: one entry“, derselbe ROM liefert mit einem Eintrag 0, `references/emutos/bios/bios.c:1153-1154`, `references/emutos/bios/serport.c:965`). Kein neu abweichender Fall in `bios-r1`, `bios-core-r1`, `xbios-chardev-r1`, `xbios-system-r1`, `xbios-bconmap-r1` (je tt/falcontc, vorher `60824ec8`). Audit-Sonde B04 `Bconout(6)` 0 → 1 (= EmuTOS), B05 `Bconout(7)` 0 (EmuTOS 1, Portzahl). Specs `docs/api/bios/{01,02,03,08}` (BIOS08-Notizen, EmuTOS-Tabellen um den `BCONMAP_AVAILABLE`-Zweig berichtigt), `docs/api/xbios/2c-bconmap.md`, `docs/api/bios/_state.md`, `docs/api/gemdos/file/3f-fread.md`, `docs/api/bios/_dispatcher.md`, `docs/api/bios/_full-function-inventory.md`, `docs/HARDWARE-PROFILE.md`; apitest BC17/BC18 nachgezogen; `mapping.tsv` +8 Zeilen (`device`, `entry_device`). Hosttests 1888/0/1 (+7 `tos-api/tests/bios08_bconmap_handles.rs`; `fs11_device_rw` `bios_absent_profile` aufs neue Profil); Warnungen unveraendert (rustc 93 kernel / 64 tos-api, clippy 121 / 74). Beleg `measurements/measurement-2026-10-30-bios08/RUNINFO.TXT`. |
| 2026-10-29 | **[XBIOS05](../TASK/XBIOS05.md): `Bconmap` hat die Tabelle mit einem Eintrag wie EmuTOS auf einer Maschine ohne Falcon/TT (Commit dieser Zeile).** Anwenderentscheidung 2026-10-28 Option 1: `Bconmap(-1)` = 6, `Bconmap(-2)` = Zeiger auf `BCONMAP {maptab, 1, 6}`, jedes Geraet ausser 6 = 0, `Bconmap(6)` = 6 mit dem Rueckspeichern des Eintrags (`references/emutos/bios/serport.c:962-972,1082-1122`); vorher `0x2C` fuer jedes Argument. Neues Modul `tos-api/src/xbios/bconmap.rs`; Tabelle in der IOREC-Seite (EL0 les-/schreibbar, nie ausfuehrbar), LP64-Layout des SDK-`_BCONMAP`/`_MAPTAB` (`docs/api/structures/bconmap.md`, Pruefung `sdk/libc/tests/bconmap-layout.c`); `MAPTAB[0].Iorec` = `Iorec(0)`. Die fuenf Funktionszeiger zeigen auf nie ausfuehrbare Ziele: ein Aufruf endet im Crash-Bericht „Bconmap MAPTAB function (not callable)“ und `Pterm(-1)` (Sonde BMCALL). Der Kern antwortet aus eigenem Zustand, Schreiben in die Tabelle wird nicht uebernommen. Spec `docs/api/xbios/2c-bconmap.md` neu; `docs/COMPATIBILITY.md`: Falcon-`7`, `maptabsize` 4/3 und `Bconmap(7..9)` ACCEPTED je Profil, MAPTAB-Eintrag konkretisiert. Neue Tabelle `tosdump/cases/xbios-bconmap-r1.tsv` (CRC `DD429BF8`, `r:bconmap` 8+24 Byte m68k), Referenzen tt/falcontc je zweimal bytegleich: NewTOS nur `maptabsize` (beide) und `mapped_device` (Falcon) verschieden = Geraeteparameter. `bios-r1` 183–186 und `xbios-chardev-r1` 43–53 EQUAL gegen tt; gegen falcontc bleiben 183 bzw. 43 (7/6, ACCEPTED); kein vorher gleicher Fall in `bios-r1`, `bios-core-r1`, `xbios-chardev-r1`, `xbios-system-r1`, `xbios-ptr-r1` neu verschieden. Sonde XCPROBE B01–B24: gleich EmuTOS bis auf Adresse, Groesse und die fehlenden Ports 7–9. `mapping.tsv`: `bconmap`, `init_bconmap` `mapped`. Handles 6–9 unveraendert ([BIOS08](../TASK/BIOS08.md)). Hosttests 1881/0/1 (+9), SDK-Hosttests 35/0/0 (+1 `bconmap-layout`); Warnungen unveraendert (rustc 93 kernel / 64 tos-api, clippy 121 / 74). Beleg `measurements/measurement-2026-10-29-xbios05/RUNINFO.TXT`. |
| 2026-10-29 | **[FS24](../TASK/FS24.md) Schritt 1: `Dsetpath` nach Medienwechsel gegen das Orakel gemessen; Option B angehalten (Commit dieser Zeile).** Neue Tabelle `tosdump/cases/gemdos-mc-fs24.tsv` (84 Faelle, CRC `81113364`; Aufbau von `gemdos-mc-paths` Fall 67 plus V1a, V1b, V2, V3 des Researchers), Referenz `EMUTOS-1a7484ab-BIOS-mc-fs24-disk` (zwei Laeufe bytegleich, Selbstpruefung gruen). Das Orakel bestaetigt die Lesart: ein im DND-Baum geloggter Ordner wird ohne Plattenzugriff gefunden (`Mediach` bleibt 2), sonst trifft `Dsetpath` den Wechsel; V2 (`Fsfirst` einer Datei vor dem Ordner loggt nichts) 0, V3 (Ordner bleibt nach Pufferverkehr geloggt) 2. NewTOS `HEAD`: 5 von 84 verschieden (76, 87, 118, 149, 150). Option B — Sektor-Cache als Stellvertreter fuer „Ordner geloggt“ — kann V2 und V3 nicht treffen (abgeleitet aus `find.rs`/`cache.rs`, kein Prototyp): V2 wuerde neu verschieden (2 statt 0), V3 bliebe verschieden (0 statt 2). Fix nicht begonnen, Anwender entscheidet. Neuer Fund N1: nach einem Re-Login liest `Dgetpath` nach erfolgreichem `Dsetpath` die Wurzel (Faelle 87, 118, 150; `set_curdir` gibt den eben genommenen Slot als „alten“ frei). Spec `docs/api/gemdos/directory/3b-dsetpath.md` um die gemessenen Faelle ergaenzt; kein Produktcode geaendert. Beleg `measurements/measurement-2026-10-29-fs24/RUNINFO.TXT`. |
| 2026-10-29 | **[BIOS10](../TASK/BIOS10.md): `Kbshift` haelt das ganze Schaltbyte wie EmuTOS' `shifty` (Commit dieser Zeile).** Anwenderentscheidung D1 = 1 (`docs/api/bios/0b-kbshift.md`): statt fuenf Atomics (nur Bits 0–4) ein Byte im Tastaturdekoder (`tos-api/src/bios/shifty.rs`, `Shifty` + `key_event`, gehalten in `kernel/src/ikbd.rs`). `Kbshift(flag)` speichert alle acht Bits, `Kbshift(-1)` liefert sie; jedes IKBD-Tastenbyte aendert nur sein Bit, Clr/Home bzw. Insert mit Alt setzen `MODE_HOME`/`MODE_INSERT` (`bios/ikbd.c:876-883`), ihr Loslassen loescht sie (`:807-814`). Neue dauerhafte Sonde `tosdump/probes/ksprobe.c` (Setz-Arm K01–K13 wie die Audit-Sonde, Tastenpfad mit Injektor bzw. Hatari `--cmd-fifo`, Tasten aus `ksprobe-keys.txt`): vorher NewTOS K05 `0x1f`/K09 `0x01` und Tastenpfad ohne Bit 5/6 (PLAUSIBLE des Audits jetzt gemessen), nachher K01–K13, T01–T24, H01–H03 gleich dem Orakel (tt = falcontc), Eingaben per `bios-arg-trace` geprueft. `bios-r1` 77 EQUAL; kein neu abweichender Fall in `bios-r1`, `bios-core-r1`, `xbios-chardev-r1`, `xbios-system-r1` (tt/falcontc, vorher `8cc34fa0`). Offen (Befunde, nicht Teil von BIOS10): keine Tastatur-Mausemulation (Alt+Clr/Home und Alt+Insert landen als Tasten in der Schlange), AES-Tastenzustand nicht auf `0x0f` maskiert wie EmuTOS ueber `vq_key_s`, das Byte folgt der Eingabepumpe (`Kbshift` pumpt nicht). `mapping.tsv` +3 Zeilen. Hosttests 1872/0/1 (+13); Bau beider Boards rustc 93/64, Clippy 121/74 wie Basis. Beleg `measurements/measurement-2026-10-29-bios10/RUNINFO.TXT`. |
| 2026-10-29 | **[BIOS07](../TASK/BIOS07.md): `Setexc` antwortet aus Seite 0 wie EmuTOS (Commit dieser Zeile).** Anwenderentscheidungen D1–D3 und P3 (`docs/api/bios/05-setexc.md`) umgesetzt: Seite 0 traegt jedes `num < 0x200`. Die Vektoren und `etv_*` (`num <= 0x103`) sind eine Zelle fuer alle Prozesse (32 Bit, `VECTORS`), jede Seite 0 traegt ihre Kopie (Loader und jeder Schreibzugriff); darueber liest/schreibt `Setexc` die Systemvariable des Prozesses (vorher Schatten bis `0x11F`: `memvalid` las 0). Startwerte: nicht bediente Vektoren und `etv_timer`/`etv_critic`/`etv_term` = der `ret`-Stub (EmuTOS `just_rts`) statt 0 bzw. 64-Bit-Platzhalter, Vektor 0/1 = die ersten zwei LONGs des OSHEADER (EmuTOS `vecs_init`), `etv_xtra` 0. Antwort vorzeichenerweitert (D1), `-1`-Test auf den unteren 32 Bit (P3, vorher per Sonde `tosdump/probes/sxprobe.c` beidseitig gemessen), `num >= 0x200` 0 (D2). Eine direkt in Seite 0 geschriebene Kopie aendert die Zelle nicht (ADR-031 Regel 4, Warnung), dokumentiert. Abnahme: `bios-core-r1` 46–62 und `bios-r1` 53–57 EQUAL auf tt und falcontc (vorher 13 bzw. 2 Faelle DIFF), keine neue Abweichung in `bios-r1`, `bios-core-r1`, `xbios-chardev-r1`, `xbios-system-r1`; Sonden S01–S08 und `sxprobe` byte-gleich mit dem Orakel; BIOS15-Warnungen (`hkprobe`) und MEM12 (P0PAR PASS) nachgeprueft. `cargo xtask test` 1859/0/1 (+9); Bau beider Boards rustc 93/64, Clippy 121/74 wie Basis. Beleg `measurements/measurement-2026-10-29-bios07/RUNINFO.TXT`. |

## Notes

The project is intentionally over-documented at the start. The premise:
spec-first development against a fixed, finite, well-documented API
prevents the implementation drift that ruins long-running OS hobby
projects.
