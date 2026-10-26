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
| Hosttests (`cargo xtask test`) | **1670 / 0 / 1** (gemessen 2026-10-26 auf `5a21cee7`) |
| SDK-Hosttests (`sdk/run-host-tests.sh`) | **26 / 0 / 0** (gemessen 2026-10-26) |
| `sh tools/desk-drift-probe.sh` | **0** (unsafe 20; gemessen 2026-10-26) |
| APITEST (`#Z`, memprot on/off/strict) | **109 / 0 / 5** (letzter Lauf `measurements/measurement-2026-10-26-a118-fix/RUNINFO.TXT`) |

Bau- und Clippy-Warnungen stehen hier ohne Zahl: sie wurden bei dieser
Bereinigung nicht nachgemessen. Sie duerfen nicht steigen; jeder Arbeitsschritt
nennt seine Vorher-/Nachher-Zahl in seiner Zeile unter *Recent Decisions*.

**Offen (Stand 2026-10-26, aus der Statuszeile der Tickets ohne DONE-Quittung):**

- **P1:** [MEM12](../TASK/MEM12.md) — EL0 darf Seite 0 beschreiben (`UserRwExec`); Leser und Stubs klaeren, dann schreibgeschuetzt oder ausgeblendet.
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
  (teilweise), [W01](../TASK/W01.md); geparkt: [H04](../TASK/H04.md),
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
- **P3 / P4:** 56 Tickets P3 und 4 Tickets P4 mit Status OFFEN, dazu 80 P3 und
  1 P4 im BACKLOG, 3 TEILWEISE, 3 IDEE. Darunter: ARCH03 Stufe 2b, SDL01,
  RTC01, REF03, REF05 (GIC-Quellen), REF06 (fehlende Track-B-Quellen), T15,
  DESK49, DESK51–DESK55, DESK61, DESK63, DESK65, DESK66, DESK67, DESK69, FS12
  (26 Laufwerke), FS15, FS16, FS17, LIBC-IOCTL, A119–A125, RST02, INPUT02,
  CON03, ARCH04, USB02, MEM08, MEM10, [KEYS03](../TASK/KEYS03.md),
  [T18](../TASK/T18.md); zurueckgestellt COOK01; Ideen MEM06,
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
| 2026-10-26 | **`tools/make-sd-image.sh` bricht bei fehlendem Programm ab; MXPROBE laeuft beim Boot (`6292ed53`).** Ein Bootlauf baute die Karte ohne MXPROBE und meldete das nur, die MEM11-Pruefungen wurden nie erreicht. Jetzt wird jede gelistete Eingabe vor dem Abbild geprueft; fehlt eine, endet das Skript mit Datei und Baubefehl (Baureihenfolge: [`sdk/06-image-and-testing.md`](sdk/06-image-and-testing.md)). Optional bleiben nur zView `SAMPLES` und TeraDesk `cicons.rsc` (so in ihren Bauskripten vorgesehen). MXPROBE ruft `Malloc`, `Mxalloc` und `Fgetdta` auf und liegt standardmaessig in `\AUTO\`; Dev-Kern raspi4b: `MXPROBE: PASS`, keine Panik, Desktop. Folge: ohne die ungetrackten QED-, TeraDesk- und CoreMark-Quellen unter `sdk/apps/` ist kein Abbild mehr baubar. |
| 2026-10-26 | **NewTOS-Tabellen in v3 unter QEMU, MEM11-Nachgang, MEM12.** Alle fuenf NewTOS-Falltabellen auf `0bc2d4fb` in QEMU raspi4b (640x480) als v3 aufgenommen (`844a9872`, `measurements/measurement-2026-10-26-newtos-v3/RUNINFO.TXT`); die alten `NEWTOS-*` stammen von der Pi mit 1920x1080 und bleiben. Funde fuer die Triage: AES `global[3..4]` gleicher LONG in anderer Wortreihenfolge in jedem Fall, `global[10]` 4 gegen 16; VDI 13/182 und 4/120 Faelle, darunter die Farbzahl; RASTER nicht vergleichbar, solange die EmuTOS-RASTER-Referenz v2 ist. [MEM11](../TASK/MEM11.md): Debug-Pruefungen nur fuer Kerninvarianten — `Fgetdta` prueft nur noch die vom Kern vergebene DTA; neuer Fall `Fsetdta(0xDEAD0000)` + `Fsfirst` beendet das Programm wie EmuTOS nach dem Busfehler, der Kern laeuft weiter (`471aa74f`). [MEM12](../TASK/MEM12.md) (P1): EL0 darf Seite 0 heute beschreiben; Ticket klaert Leser und Stubs (`0bc2d4fb`). |
| 2026-10-26 | **tosdump-Format v3: Zeigerbreite und Wortreihenfolge im Kopf (`d42675e5`).** Die Feldklasse address war auf v2-Records nicht pruefbar, weil EmuTOS (Hatari) und NewTOS Zeiger in verschiedener Wortreihenfolge schreiben und der Record das nicht sagt. tosdiff prueft jetzt jede Adresszelle auf `< 0x80000000` ([MEM11](../TASK/MEM11.md)) und meldet Verletzungen getrennt von Wertdifferenzen; v2 bleibt lesbar mit „not checkable“, v3 gegen v2 wird abgewiesen. Zwoelf gepinnte EmuTOS-Referenzen in v3 neu aufgenommen, Payload byte-gleich zu v2 (`593ae23e`, `measurements/measurement-2026-10-26-ref07-v3/RUNINFO.TXT`). **Offen:** die zwei RASTER-Referenzen bleiben v2 — ihre MFDB-Zeigerzellen zeigen ins Textsegment des Dumpers, das seit `9b67b6e4` gewachsen ist, byte-gleich nicht mehr reproduzierbar; alle `NEWTOS-*`-Aufnahmen sind v2 und werden gegen die v3-Referenzen abgewiesen, bis sie neu aufgenommen sind. |
| 2026-10-26 | **Bereinigung, dritte und vierte Runde (Anwenderentscheidungen).** `cargo xtask doc-check` laesst datierte Schnappschuesse aus (CLAUDE.md: Geschichte wie das Archiv), kennt die Praefixe ET/NT/FM/KA u. a., loest eindeutige Dateinamen auf und prueft EmuTOS-Zitate am Orakel-Pin (`aae5725c`, `cfebf54a`, `86608e41`); Rueckstand 3308 → 1851. docs/api/ ist doc-check-sauber, nicht mehr belegbare Zitate als `cite stale 2026-10-26` markiert (`ff6e5cd2`, `4659824b`, `cbac4e82`). ADR-030 angenommen: keine Line-A-Emulation, kein exportierter LINEA-Block, `__aline` bleibt NULL; kein portiertes Programm und keine NewTOS-Komponente liest Line-A (`a06bed63`, `117342b0`); mintlib `linea.c` ist per ADR-030 aus `libc.a` ausgeschlossen, nicht als Baufehler gefuehrt (`ef48e0da`); NEWDESK liest Aufloesung und Tasten ueber das VDI ([DESK69](../TASK/DESK69.md), `83537484`). [MEM11](../TASK/MEM11.md): eine Anwenderadresse ab `0x8000_0000` kommt als negativer LONG zurueck und liest sich wie ein Fehlercode; der Debug-Bau haelt jetzt an, wo `Malloc`/`Mxalloc`/`Fgetdta` einen solchen Zeiger ausgeben wuerden (`ecd4ddb5`, `3c5205ed`); die Erzwingung an den Abbildungsstellen bleibt offen. tosdump-Format v3 (Zeigerbreite und Wortreihenfolge im Kopf) fuer die Adresspruefung beschlossen, noch nicht gebaut. |
| 2026-10-26 | **Bereinigung, zweite Runde (Anwenderentscheidungen).** Vorrang in CLAUDE.md: ADR > CLAUDE.md > STATUS.md > Memory; ein Memory, das einem ADR widerspricht, wird korrigiert (`1a1c008c`). Registernamen-Regel an die Treiber angepasst, keine Umbenennungen. ADR-030 „keine Line-A-Emulation“ als **Proposed** mit drei offenen Fragen (`075bdddd`). Tickets KEYS03 (`#K` gehoert EmuDESK, Layoutwahl braucht eine eigene Zeile) und T18 (veraltete Code-Kommentare) (`9944ab1b`). ARCHITECTURE-ADR-Liste, Lizenz GPLv2+ in REQUIREMENTS, Zitat eines nie geschriebenen ADRs, Links auf ein geloeschtes Memory (`fbeda572`). Nightly-Update-Regel als nicht freigegeben markiert (`5e74a839`). ERLEDIGT-Tickets ohne DONE-Quittung gelten als offen (`9485459a`). `cargo xtask doc-check` prueft jetzt ganz `docs/` ohne `docs/archive/` und jede `ADR-NNN`-Nennung gegen die Ueberschriften in docs/MEMORY.md; CLAUDE.md, STATUS.md und docs/MEMORY.md sind sauber, der Rest hat 3308 Altfehler (vor allem `docs/analysis/`). |
| 2026-10-26 | **Bereinigung STATUS.md, Projekt-Memory, CLAUDE.md nach Triage** ([`analysis/cleanup-triage-2026-10-26.md`](analysis/cleanup-triage-2026-10-26.md), `70585217`). STATUS.md fuehrt nur noch den Stand und die 10 neuesten Eintraege; die 516 aelteren, der historische Text und die alten Phasenzeilen stehen woertlich in [`archive/status-log-2026-10-26.md`](archive/status-log-2026-10-26.md) (`340e8232`). Wissen, das nur im Log stand, ist vorher umgezogen: ADR-028 (ARCH03, loest die doppelte ADR-025 auf), ADR-029, ADR-013-Nachtrag (`8624372d`), vier COMPATIBILITY-Eintraege (`a641c130`), Spec-, Dumper- und Toolchain-Notizen (`b181b544`, `2f35b936`, `0399d421`), Tickets A125/REF06 (`f242696e`). docs/MEMORY.md: Statuszeilen abgeloester ADRs, QUIRK-002 widerlegt, Zitate aufloesbar (`86f413be`, `def5d3e2`). CLAUDE.md an Code und Arbeitsweise angeglichen (`5a21cee7`, `21e9dae6`). Neu: `cargo xtask doc-check` prueft Pfade und `file:line` in CLAUDE.md, STATUS.md und docs/MEMORY.md (`c8e67a47`); `docs/archive/` ist Geschichte, nie Beleg. **Offen:** `#K` in NEWDESK.INF doppelt belegt (Tastaturlayout gegen Menue-Shortcuts, `docs/COMPATIBILITY.md`), ohne Ticket. |
| 2026-10-26 | **Oracle-Basis (Phase 0): EmuTOS-Referenz auf `1a7484ab` gepinnt, gegen die `EMUTOS14-*`-Referenzen kalibriert (`9b67b6e4`, `5aa2f30a`, `92402900`, `fc686f78`).** Die Referenzen stammen vom 1.4-Release-ROM (`5805c0a4…`, Tag `978e3756`), die Spec-Quelle liegt 71 Commits spaeter. Entscheidung des Users: Pin auf `1a7484ab`, vorher je eine Variable: Kontrolle (Release-ROM, heutiger Dumper, Hatari 2.6.1) 13/14 bitgleich + RASTER 0/59 (nur Dumper-Zeigerfelder), mit bereinigter `tools/oracle/hatari.cfg` byteweise wie mit der vollen Nutzerkonfig; eigener gcc-7.5-Bau von 1.4 (`67bc59a6…`) **0 von 938 Faellen** — einziges abweichendes Feld `ap_3resv` = `&D` (`aes/gemsuper.c:98`, 0xBE04 → 0xBD5C), seitdem Feldklasse **address** in `tosdump/core/record.c` (Grep: kein NewTOS-Code/Test haengt am Wert); Uebersetzungen 502/502 wie im Release; Pin-Bau (`323c87bb…`, festgeschrieben als `tools/oracle/etos512de-1a7484ab.img`, Neubau = Gegenprobe) gegen 1.4-Bau: 3 Faelle, alle `wind_get WF_OWNER` = EmuTOS `acb7f3b4`, nichts unzuordenbar; Canary `vdi-all` tt + falcontc bitgleich. Eine Konfig `tools/oracle/oracle.env` fuer Bauskript, `tools/hatari-oracle.sh`, `tools/hatari-qed.sh`, `tools/emutos-config.sh` (CONF-Tabelle vorher/nachher byteidentisch), `tosdump/Makefile`. Neue Referenzen `tosdump/reference/EMUTOS-1a7484ab-*`, `EMUTOS14-*` bleiben. **Offen:** ~~Umstellung der Rust-Konformitaetstests auf die neuen Dateien macht 4 WF_OWNER-Tests rot — nur Test-Buchhaltung (`DELTAS` und `wf_owner_answers_…` halten den 1.4-Dump fest; ohne `DELTAS` stimmen die drei Faelle mit dem Pin-Dump), Befund fuer den Developer~~ — erledigt mit `cf4d9b33` (Konformitaetstests lesen die `EMUTOS-1a7484ab-*`-Dumps, `DELTAS` leer); address-Klasse relational fuer Phase 1. `docs/analysis/ORACLE.md`, `measurements/measurement-2026-10-26-oracle/RUNINFO.TXT`. |
| 2026-10-26 | **A118-Nacharbeit — abgelehnter Wiederaufruf gibt die uebergebene Sperre weiter (`3cb21e31`).** Tester (`f42f3719`, `measurements/measurement-2026-10-25-a118-test/`): das Tor der Fensteropcodes 101/102/103/105 wird beim Wiederausfuehren eines geparkten Aufrufs neu ausgewertet; aendert der Halter das Fenster zwischendurch, lehnt der Aufruf ab und die ihm uebergebene Sperre blieb bei ihm (`(1, B)`). EmuTOS prueft vor dem Warten (`aes/gemwmlib.c:1415-1422,1443-1450,1632-1638`), der Geweckte ist Halter und gibt per `END_UPDATE` ab; jetzt `window::end_unused_grant` (benannt: EmuTOS liefe den Rumpf auf dem vorher geprueften Fenster). Tests fuer die vier Mutations-Ueberlebenden (Klammern `close_window`/`graf_dragbox`, geparkte Rechteck-Waits `evnt_multi`/`evnt_mouse`), Mutationsprobe 5/5 rot. Hosttest 16/20 auf `f42f3719`, 20/20 neu; `cargo xtask test` 1657/0/1 → 1664/0/1; Warnzeilen/Clippy unveraendert; QEMU GH1-Serie 41/41 + std/A117/TeraDesk-Ende/a118/APITEST 41/41 pixelgleich, APITEST 109/0/5 in on/off/strict. `measurements/measurement-2026-10-26-a118-fix/RUNINFO.TXT`. |
| 2026-10-25 | **A118 — die restlichen Nehmer der `wind_update`-Sperre nach EmuTOS (`ded51d9e`).** Die GH1-Liste je Stelle gegen EmuTOS geprueft: `wm_open`/`wm_close`/`wm_delete` (offen)/`wm_set` (`aes/gemwmlib.c:1422,1450,1473,1638`), `gr_rubwind`/`gr_dragbox` hinter `graf_rubberbox`/`graf_dragbox` (`aes/gemgrlib.c:205,250`), `ap_exit` (`aes/gemaplib.c:253-258`, ueber `wait_for_accs` gehalten), `fs_input`→`fm_do`→`fm_own` (`aes/gemfslib.c:718`, Sperre **und** Maus), `fm_error` der Shell-Schleife, Quick-Check `rlr == gl_mowner` (`aes/gemevlib.c:185`) — alle bestaetigt; `w_redraw_desktop` entfaellt (Funktion fehlt ganz, kein fehlender Nehmer). **Wirkung:** Hosttest `aes_a118_lock_takers` 10/13 rot auf `17cab8b6`; QEMU: ein Doppelklick neben der Dateiauswahl eines Accessorys selektierte danach das C:-Icon des Desktops (der Klick ging an NEWDESK), jetzt nicht; die uebrigen Punkte latent fuer NEWDESK/TeraDesk/QED/SYSACC. **Fix:** Dispatcher wartet (`update_block`) vor 19/70/71/90/91 und `opcode_takes_lock` (101/102/103/105, genau wenn EmuTOS `BEG_UPDATE` erreicht), Rumpfe klammern `update_beg`/`update_end`, `fsel` unter `fm_own`, `appl_exit` einmal je `AC_CLOSE`-Runde, `input::owns_mouse` fuer Quick-Check und geparkte Rechtecke, Shell-Schleife ruft `form_error` per GEM-Trap aus EL1 (parkt wie jeder Client). Benannt: `fsel` wartet vor dem Zeichnen; Kind-Sweep zaehlt bei fremdem Halter oben drauf. **Zahlen:** Warnzeilen 159 = 159, Clippy tos-api 74 / kernel 121 unveraendert, `cargo xtask test` 1644/0/1 → 1657/0/1; QEMU raspi4b GH1-Serie 41/41 + std 26/26 pixelgleich, A117 7/7, TeraDesk Start+Beenden, QED Start+Beenden, APITEST on/off/strict 109/0/5 = 109/0/5. Nebenbefunde (offen): `ap_exit`s `all_run()` danach, `graf_*box` ohne `gr_rubwind`/`gr_dragbox`, `MU_M1`-Arm trotz gefeuertem Quick-Check. Pi offen. `measurements/measurement-2026-10-25-a118/RUNINFO.TXT`, `TASK/A118-STATUS-DONE.MD`. |
| 2026-10-25 | **A115 — Scheduler waehlt nach EmuTOS' Bereitliste `rlr`/`drl` statt Slot-Reihenfolge (`e7754b16`).** Wirkung belegt: Hosttest `sched_a115_ready_list` 0/3 auf `587e0f68` (ein per `appl_write` Geweckter lief schon beim Abschluss-Dispatch des Schreibers statt hinter ihm; zwei in einem Durchgang Geweckte liefen in Slot- statt umgekehrter Weckreihenfolge; Accessories starteten in Lade- statt umgekehrter `pstart`-Reihenfolge), QEMU-Sonde f2: Basis ACCB, ACCC, MAIN — neu MAIN, ACCC, ACCB. EmuTOS `disp`/`disp_act`/`mwait_act`/`schedule` (`aes/gemdisp.c:74-93,181-253`), `pstart`/`insert_process` (`gempd.c:125-145`), `signal` (`gemasync.c:56-57`). Nachbau in `sched.rs` „The ready list“: `runnable` bleibt Mitgliedschaft, Reihenfolge per `rlr`-Schluessel und `drl`-Stapel, der EL1-Kernel-Client ist ein Eintrag; jeder `runnable`-Schreiber laeuft ueber `sched` (auch Testfixturen und unverdrahtete Selftests). Spec `event.md` (A115-Bindung), `accessories.md`. Hosttests 1641 → 1644, Bau/Clippy unveraendert, APITEST 109/0/5 (on/off/strict), NEWDESK/TeraDesk/QED/A117-Reihen pixelgleich. `TASK/A115-STATUS-DONE.MD`, `measurements/measurement-2026-10-25-a115/RUNINFO.TXT`. Offen: echte Pi, Tester. |

## Notes

The project is intentionally over-documented at the start. The premise:
spec-first development against a fixed, finite, well-documented API
prevents the implementation drift that ruins long-running OS hobby
projects.
