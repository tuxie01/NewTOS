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
| Hosttests (`cargo xtask test`) | **1726 / 0 / 1** (gemessen 2026-10-28 auf `75527af8`; `cargo xtask test --fast` 1726 / 0 / 0, ohne den einen Doctest) |
| SDK-Hosttests (`sdk/run-host-tests.sh`) | **26 / 0 / 0** (gemessen 2026-10-26) |
| `sh tools/desk-drift-probe.sh` | **0** (unsafe 20; gemessen 2026-10-26) |
| APITEST (`#Z`, memprot on/off/strict) | **109 / 0 / 5** (letzter Lauf `measurements/measurement-2026-10-26-a118-fix/RUNINFO.TXT`) |

Bau- und Clippy-Warnungen stehen hier ohne Zahl: sie wurden bei dieser
Bereinigung nicht nachgemessen. Sie duerfen nicht steigen; jeder Arbeitsschritt
nennt seine Vorher-/Nachher-Zahl in seiner Zeile unter *Recent Decisions*.

**Offen (Stand 2026-10-26, aus der Statuszeile der Tickets ohne DONE-Quittung):**

- **Warteschlange (Anwender 2026-10-28, nach dem Audit „BIOS-Speicher“), in dieser Reihenfolge.**
  Developer, einer zur Zeit: [TOOL07](../TASK/TOOL07.md) Rest (BIOS-STOR-004–006,
  Option beim Anwender) →
  [TOOL08](../TASK/TOOL08.md) (BIOS/XBIOS-Argumente im Tracer) →
  [TOOL11](../TASK/TOOL11.md) (`PTR_RAM` laesst Funktionsnummern durch) →
  [TOOL09](../TASK/TOOL09.md) (xtask-Befehl fuer tosdump-Laeufe auf NewTOS) →
  [TOOL10](../TASK/TOOL10.md) (`kernel8.img` nicht ins Wurzelverzeichnis) →
  [BIOS01](../TASK/BIOS01.md) (Fixes BIOS-Speicher, S2 zuerst, jeder erst
  nach Bestaetigung durch TOOL07) → Track B: Trap #2 aufloesen, Kanten fuer
  Vektoraufrufe (`hdv_*`, BIOS-STOR-009) und das Abschlusskriterium belegen
  (`cargo xtask deps getrec --up` samt Kanten ueber die osif-Tabelle) →
  [TOOL03](../TASK/TOOL03.md) → DUMPGDOS → [FS19](../TASK/FS19.md) →
  [MEM11](../TASK/MEM11.md) (Erzwingung) → [TOOL01](../TASK/TOOL01.md).
  Daneben ein Auditor bzw. Researcher zur Zeit, in eigenem Arbeitsbaum: die
  XBIOS-Gruppen ohne Plattenzugriff von unten nach oben (BIOS-Kern und
  XBIOS-Zeichengeraete erledigt, Funde noch nicht getickert) →
  [BIOS02](../TASK/BIOS02.md) (Spec, Einheit der Pi-SD) →
  [BIOS03](../TASK/BIOS03.md) (`hdv_*`: ADR-Vorschlag, Entscheidung beim
  Anwender).
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
| 2026-10-28 | **Audit XBIOS-Zeichengeraete und Tastatur (`05bd0be2`).** `Midiws`, `Iorec`, `Rsconf`, `Keytbl`, `Bioskeys`, `Ikbdws`, `Kbdvbase`, `Kbrate`, `Bconmap` auf `b81d0ddc`; neue Tabelle `tosdump/cases/xbios-chardev-r1.tsv` mit Pin-Referenzen, Sonden fuer das, was der Waechter abweist. Urteil FINDINGS: S1=1 S2=2 S3=3 S4=2; gleich nur `Keytbl` und `Kbrate`. S1: `Iorec` ist nicht verdrahtet und liefert `0x0E`; das verbreitete Tastaturpuffer-Leeren (`io->ibufhd = io->ibuftl`) beendet das Programm mit Bus Error in Seite 0. S2: `Bioskeys` nicht verdrahtet; Haken in `mousevec`/`statvec`/`clockvec`/`joyvec` der `Kbdvbase`-Tabelle werden nie gerufen. `Keytbl` → `0x768` ist ein gueltiger Zeiger (Frage aus [TOOL11](../TASK/TOOL11.md)), `Iorec`/`Bconmap(-2)` sind keine. Bericht `docs/audits/2026-10-28-xbios-chardev.md`, Lauf `measurements/measurement-2026-10-28-xbios-chardev-audit/RUNINFO.TXT`. Funde noch nicht getickert. |
| 2026-10-28 | **[TOOL07](../TASK/TOOL07.md) Etappe 1: DUMPBIOS-Plattenrunde (`0495d622`).** DUMPBIOS wies `Rwabs`/`Getbpb`/`Mediach` ab, das Speicher-Audit konnte nur sondieren. Eine `!disk`-Tabelle laesst sie jetzt zu: Vorspann nur lesend, Schreiben nur als Write-back des zuletzt gelesenen Sektors, das Testabbild behaelt jedes Byte. `tosdump/cases/bios-r2.tsv` (51 Faelle) auf dem Profil `disk` (zwei Laeufe bytegleich, Abbild unveraendert); `tools/mkcard.sh` mit `MKCARD_TESTDISK` legt dasselbe Abbild als Partition 1 und die Programme auf Partition 2. NewTOS: 10 von 51 DIFF — BIOS-STOR-001 (forcechange), -003 (Anzahl 0), -007 (Puffer NULL auf A:/B:) per tosdiff bestaetigt; `Drvmap` EXACT gleich (`0xC`). Offen: -004 bis -006 brauchen gepatchte Bootsektoren und einen Rueckweg ueber die physische Einheit, Optionen im Ticket. Zitatanker in den Audit-Berichten nachgezogen, die TOOL06/TOOL07 verschoben hatten (die Luecke aus [TOOL05](../TASK/TOOL05.md)). Beleg `measurements/measurement-2026-10-28-tool07/RUNINFO.TXT`. |
| 2026-10-28 | **Audit BIOS-Kern ohne Plattenzugriff (`ae695977`).** `Getmpb`, `Bconstat`, `Bconin`, `Bconout`, `Setexc`, `Tickcal`, `Bcostat`, `Kbshift` auf `1680d5b2`, gemessen aus einem eingefrorenen Arbeitsbaum, waehrend TOOL06 im Hauptbaum lief. Neue Tabelle `tosdump/cases/bios-core-r1.tsv` (44 Faelle) mit Pin-Referenzen; Sonden, wo der DUMPBIOS-Waechter abweist. Urteil FINDINGS: S2=3 S3=7 S4=2, fuenf per tosdiff CONFIRMED, sechs per Sonde MEASURED; nur `Bconstat` und `Tickcal` gleich. S2: MIDI/IKBD melden nie bereit und verweigern Ausgabe; ein per `Setexc(0x100)` eingehaengter `etv_timer`-Haken wird nie gerufen; Kern und SDK-Header sind sich ueber das `MD`-Layout von `Getmpb` uneinig. Bericht `docs/audits/2026-10-28-bios-core.md`, Lauf `measurements/measurement-2026-10-28-bios-core-audit/RUNINFO.TXT`. Funde noch nicht getickert. |
| 2026-10-28 | **[TOOL06](../TASK/TOOL06.md): Orakel-Plattenprofil `disk` (`dedde4de`).** `tools/hatari-oracle.sh disk` mit `HATARI_ORACLE_DISK=<Abbild>`: Falcon030, Floppys aus, ein reproduzierbares FAT16-Testabbild (`tools/mktestdisk.sh`) als IDE-Master C:, der Dumper auf Hataris GEMDOS-Laufwerk D:; `Drvmap` dort `0xC`. Der Trace belegt, dass EmuTOS das Abbild selbst liest. `tt`/`falcontc` unveraendert, §6-Kanarienvogel OK; eigener Kanarienvogel des Profils in `docs/analysis/ORACLE.md` §6 ohne `Random` und `Gettime` (beide schwanken je Lauf). Auditor-Rolle verweist auf das Profil. Beleg `measurements/measurement-2026-10-28-tool06/RUNINFO.TXT`. |
| 2026-10-28 | **Tickets aus dem Audit BIOS-Speicher; Plattenmessung zuerst; Auditor-Rolle; DUMPBIOS 47 → 42 erklaert.** Anwender 2026-10-28: bottom-up heisst BIOS-Speicher vor DUMPGDOS; ein Fix beginnt erst, wenn die DUMPBIOS-Plattenrunde den Befund bestaetigt (der Fall ist danach Regressionsfall), S2 zuerst ([BIOS01](../TASK/BIOS01.md)); Spec und Einheit der Pi-SD [BIOS02](../TASK/BIOS02.md); ob umgehaengte `hdv_*`-Vektoren wirken, ist eine Zielfrage (RAM-Disks, Caches) — ADR zuerst wie bei Line-A ([BIOS03](../TASK/BIOS03.md)). Die drei Luecken, die das naechste Speicher-Audit blockieren, kommen nach vorn: Orakel-Plattenprofil als zweites Profil mit eigenem Kanarienvogel, bestehende unveraendert ([TOOL06](../TASK/TOOL06.md)), DUMPBIOS-Plattenrunde mit getrennter Programmpartition und `Drvmap` exakt je Profil ([TOOL07](../TASK/TOOL07.md)), BIOS-Argument-Tracer ([TOOL08](../TASK/TOOL08.md)); dazu [TOOL09](../TASK/TOOL09.md) (Befehl fuer den Pre-flight) und [TOOL10](../TASK/TOOL10.md) (`kernel8.img` lag im Wurzelverzeichnis). `.claude/agents/auditor.md`: Urteilsklasse MEASURED (beidseitig per Sonde, wiederholbar, Belegpfad) zwischen PLAUSIBLE und CONFIRMED; Programm auf eigener Partition, Abbildvergleich nur der Testpartition; Pre-flight 2 nennt das heutige Verfahren. 47 → 42 (`tt`) bzw. 51 → 46: dieselben fuenf Faelle in beiden Profilen (98–100 `Iorec`, 109 `Keytbl`, 184 `Bconmap(-2)`), NewTOS-Werte unveraendert; gekippt durch die `PTR_RAM`-Regel aus `ff1da029`, nicht durch NewTOS. Dabei bestehen `0x0E` und `0x2C`, die Funktionsnummern selbst — [TOOL11](../TASK/TOOL11.md). |
| 2026-10-28 | **Audit BIOS-Speicher (`Rwabs`, `Getbpb`, `Mediach`, `Drvmap`), erste Triage der DUMPBIOS-Funde (`7edfd7cc`).** Bericht `docs/audits/2026-10-28-bios-storage.md`, Lauf `measurements/measurement-2026-10-28-bios-storage-audit/RUNINFO.TXT`; Kanarienvogel OK. DUMPBIOS verweigert drei der vier Aufrufe und das Orakel hat kein BIOS-Blockgeraet, darum maß eine Sonde beide Seiten auf derselben FAT16-Karte: `Drvmap` gleich, die drei anderen verschieden — S2: forcechange-Handshake beim Schreiben von Sektor 0 fehlt (`Mediach` immer 0); S3: `RW_NOTRANSLATE` per Laufwerksbuchstabe statt physischer Einheit, Anzahl 0 ohne Bereichspruefung, `Getbpb`-Grenzen, `Rwabs` nach gescheitertem `Getbpb`, `hdv_*`-Vektoren wirkungslos (nicht gemessen); S4: Spec veraltet, Graph. Alle PLAUSIBLE, keiner per tosdiff bestaetigt; Tickets noch nicht angelegt. Ablauf-Probe: neun Punkte (Plattenprofil im Orakel, BIOS-Argument-Tracer, Pre-flight-Wortlaut u. a.). |
| 2026-10-28 | **[TOOL04](../TASK/TOOL04.md): `cargo xtask test --fast` / `--full`; die Testlaufzeit lag an zwei Host-Ursachen, nicht an den Tests. (`6c943310`).** Messlaeufe `measurements/measurement-2026-10-27-tool04/RUNINFO.TXT` und `measurements/measurement-2026-10-28-tool04/RUNINFO.TXT`. (1) Jedes neu gelinkte Testbinary wartete beim ersten Start 12–80 s auf die Gatekeeper/XProtect-Pruefung (162 Binaries: 6472 s); seit der Anwender Terminal unter Datenschutz & Sicherheit → Entwicklertools zugelassen hat (2026-10-28), zusammen 1,6 s. (2) `target/debug/deps` waechst je tos-api-Aenderung um ~3 000 Objektdateien (inkrementell + `split-debuginfo` unpacked) und raeumt nie auf (516 186 Eintraege); jeder rustc-Prozess liest das ganze Verzeichnis beim Start (`SearchPath::new`): Neubinden nach einer tos-api-Aenderung 165–192 s statt ~5 s in einem frischen Target, kausal belegt mit 500 000 leeren Dateien (4,5 s → 212 s). Bereinigung bzw. ein `[profile.test]` ohne dieses Wachstum ist Anwenderentscheidung (Messwerte je Variante im Lauf). Stufen (`tools/xtask/src/test_tiers.rs`): schnell = alle Unit- und alle 159 Integrationstests ohne die Doctests (diese kosten 16 von 18,7 s eines Laufs ohne Aenderung und fuehren keinen Test aus), nennt am Ende das Ausgelassene; voll = alles. Beide gruen: 1721/0/0 bzw. 1721/0/1, nach einer tos-api-Aenderung 198 s bzw. 209 s. Kein Test verloren (Namensabgleich im Lauf); STATUS-Basislinie 1670 → 1721 = +45 Tests seit `5a21cee7` und 6 neue Stufentests. Integrationstests nicht zusammengelegt: 85 von 159 installieren den prozessglobalen Host. Ohne die Entwicklertools-Ausnahme (VS Code, ssh, launchd) gelten wieder ~2 h je Lauf. Warnungen unveraendert (xtask clippy: 0 neue). Nachtrag (`75527af8`, `366f2da4`; Anwenderentscheidung 2026-10-28, Lauf `measurements/measurement-2026-10-28-tool04-profile/RUNINFO.TXT`): `[profile.test] split-debuginfo = "packed"` im Workspace-`Cargo.toml` (nur Host-Testbauten; Kernelbauten dev/release unberuehrt, per `--unit-graph` belegt); `target/debug/deps` bleibt ueber 7 Neubindungen nach `touch tos-api/src/lib.rs` flach bei 3996 Eintraegen, je 13,8–14,1 s (voll wie schnell), ohne Aenderung 3,7 s; volle Debuginfo im `.dSYM` je Testbinary (lldb loest Zeilen in Test und tos-api auf). `cargo xtask test` warnt in beiden Stufen ueber 20 000 Eintraegen (`warn_if_deps_bloated`, `tools/xtask/src/test_tiers.rs`), Lauf bleibt gruen; 5 neue Tests → 1726/0/1. |
| 2026-10-28 | **Warteschlange in STATUS.md; B4 nach MEM12 berichtigt; [TOOL05](../TASK/TOOL05.md).** Die vereinbarte Reihenfolge stand bisher nur im Auto-Memory; sie steht jetzt unter *Offen* (Anwender 2026-10-28). `docs/sdk/05-abi-pitfalls.md` B4 beschrieb `Super`/`Supexec` noch als EL0-No-op und zitierte die Zeilen 804–812 von `osbind.h`, hinter dem Dateiende (775 Zeilen seit `60d3c4fd`); jetzt der Super-Zustand nach MEM12 mit verankerten Zitaten, `docs/sdk/` wieder doc-check-sauber. Die Luecke dahinter: doc-check vor dem Commit prueft nur selbst bearbeitete Doku, nicht die, die eine geaenderte Datei zitiert — TOOL05. Remote-Branch `mem12` geloescht (in `main` enthalten). Kanarienvogel des Orakels ausdruecklich definiert (`docs/analysis/ORACLE.md` §6: `vdi-all.tsv`, `tt` und `falcontc`, Dumper-Build `19b7`, erwartete sha256 der format-3-Pin-Referenzen, ein Befehl); die alte Fassung verglich mit den format-2-`EMUTOS14`-Referenzen, die kein heutiger Dumper mehr erzeugt. Lauf 2026-10-28 auf `d8e9628c`: beide OK (`measurements/measurement-2026-10-28-oracle-canary/RUNINFO.TXT`); der Auditor-Pre-flight verweist darauf. |
| 2026-10-27 | **MEM12: Seite 0 fuer EL0 nur im Super-Zustand beschreibbar, nie ausfuehrbar (`34530791`, `60d3c4fd`).** `Super`/`Supexec` sind wieder Kernaufrufe mit einem Super-Zustand je Prozess (`docs/api/_super-state.md`); jedes Umschalten schreibt den Seite-0-Eintrag neu und invalidiert den TLB. Stubs in einer gemeinsamen, nur ausfuehrbaren Seite; Kbdvbase-Tabelle und USERDEF-PARMBLK in einer privaten RW-Seite je Prozess; Tastenbytes an den innersten gestarteten Prozess, Accessory-Haken werden gemeldet; der Kern liest aus Seite 0 nur `conterm` Bits 0–3 und `memvalid*`. SDK-Makros wieder Traps, alle Programme neu gebaut. QEMU raspi4b: `super-r1.tsv` 0 von 9 Faellen verschieden, P0PAR besteht (beide TLB-Richtungen), HypView-Taste, TeraDesk Reboot und Shift+Esc, DOOM-Tastatur laufen in ihren Super-Klammern, alle Kartenprogramme starten. Offen: Lauf auf der echten Pi (Prueffolge in [MEM12](../TASK/MEM12.md)), danach die Karte mit allen neu gebauten Programmen. Beleg: `measurements/measurement-2026-10-27-mem12-acceptance/RUNINFO.TXT`. |
| 2026-10-27 | **Vergleichsregeln muessen am Orakel bestehen; `PTR_RAM` verlangt eine gerade Adresse (`ff1da029`).** `tosdiff --selfcheck` wendet jede relationale Regel (Adresspruefung, kept check, `NONZERO`/`HANDLE`/`PTR_RAM`) auf eine Orakel-Referenz allein an; ein Verstoss ist ein Regel- oder Tabellenfehler fuer den Researcher, nie ein NewTOS-Fund. `tosdump/tests/test_selfcheck.sh` prueft so alle 18 gepinnten Referenzen in den Hosttests. Anlass: `PTR_RAM` verlangte ein Vielfaches von 4, die EmuTOS-BIOS-Referenz verletzte das selbst (WORD-ausgerichtete m68k-Strukturen); jetzt gerade, ungerade bleibt Fehler. Dazu: Laeufe, die ein gebootetes Abbild beschreiben, halten dessen sha256 nach dem Lauf fest (CLAUDE.md); [TOOL03](../TASK/TOOL03.md) macht Messabbilder reproduzierbar (PRNG mit Startwert im Manifest, feste Zeitstempel). |

## Notes

The project is intentionally over-documented at the start. The premise:
spec-first development against a fixed, finite, well-documented API
prevents the implementation drift that ruins long-running OS hobby
projects.
