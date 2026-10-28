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
| Hosttests (`cargo xtask test`) | **1748 / 0 / 1** (gemessen 2026-10-28 mit TOOL12, +3 deps-Tests; davor 1745 mit TOOL10; TOOL09 +16, TOOL10 +3 xtask-Tests; vorher 1726 / 0 / 1 auf `75527af8`, `--fast` ohne den einen Doctest) |
| SDK-Hosttests (`sdk/run-host-tests.sh`) | **26 / 0 / 0** (gemessen 2026-10-26) |
| `sh tools/desk-drift-probe.sh` | **0** (unsafe 20; gemessen 2026-10-26) |
| APITEST (`#Z`, memprot on/off/strict) | **109 / 0 / 5** (letzter Lauf `measurements/measurement-2026-10-26-a118-fix/RUNINFO.TXT`) |

Bau- und Clippy-Warnungen stehen hier ohne Zahl: sie wurden bei dieser
Bereinigung nicht nachgemessen. Sie duerfen nicht steigen; jeder Arbeitsschritt
nennt seine Vorher-/Nachher-Zahl in seiner Zeile unter *Recent Decisions*.

**Offen (Stand 2026-10-26, aus der Statuszeile der Tickets ohne DONE-Quittung):**

- **Warteschlange (Anwender 2026-10-28), in dieser Reihenfolge.** Regel: ein
  neues Werkzeug-Ticket kommt hinter den Produkt-Fix, den es verzoegern wuerde,
  ausser der Fix ist ohne das Werkzeug nicht abnehmbar.
  Developer, einer zur Zeit: [TOOL07](../TASK/TOOL07.md) Rest (Variante B; laeuft) →
  [BIOS01](../TASK/BIOS01.md) S2
  (BIOS-STOR-001; Rueckgabewerte + Abbildvergleich, braucht TOOL16 nicht;
  Retest-Luecken aus dem heutigen `mapping.tsv` im Ticket) →

  [TOOL16](../TASK/TOOL16.md) (Dumper liest durch Rueckgabezeiger) → das S1
  `Iorec` ([XBIOS01](../TASK/XBIOS01.md)) → [BIOS15](../TASK/BIOS15.md)
  (ADR-031: Haken-Warnungen, Besitz, `prt_*`) → [SDK03](../TASK/SDK03.md)
  (`sysinfo`) → [ARCH06](../TASK/ARCH06.md) (`_MCH` = `0xFFFFFFFF`, `_VDO`
  messen) → [SDK04](../TASK/SDK04.md) Developer-Teil (Typen fester Breite,
  Fixes nach Messung) → [TOOL17](../TASK/TOOL17.md)
  (`mapping.tsv`-Klassen, Abdeckung je Schicht; danach BIOS01-Liste neu
  klassifizieren) → [TOOL18](../TASK/TOOL18.md) (Cookie-Falltabelle, alle
  drei Profile) → BIOS01
  Rest → die uebrigen Audit-Tickets BIOS04–BIOS14, XBIOS02–XBIOS14 (S2
  zuerst; BIOS05 und XBIOS03 sind ACCEPTED nach ADR-031, XBIOS10 nach ARCH05) →
  [TOOL13](../TASK/TOOL13.md), [TOOL14](../TASK/TOOL14.md),
  [TOOL15](../TASK/TOOL15.md) → [TOOL03](../TASK/TOOL03.md) → DUMPGDOS →
  [FS19](../TASK/FS19.md) → [MEM11](../TASK/MEM11.md) (Erzwingung) →
  [TOOL01](../TASK/TOOL01.md); [SDK02](../TASK/SDK02.md) (P3) bei Gelegenheit.
  Daneben ein Auditor bzw. Researcher zur Zeit, in eigenem Arbeitsbaum:
  [SDK04](../TASK/SDK04.md) Erhebung (LP64-Zugriffe auf TOS-Strukturen) → Vertrag `Getmpb` (BIOS06) →
  Spec-Tickets der Audits.
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
| 2026-10-28 | **Maschinenidentitaet entschieden ([ARCH05](../TASK/ARCH05.md)): Nachtrag ADR-012, `_MCH` = `0xFFFFFFFF`; neue Tickets [ARCH06](../TASK/ARCH06.md), [SDK03](../TASK/SDK03.md), [SDK04](../TASK/SDK04.md), [TOOL18](../TASK/TOOL18.md).** Erhebung (`docs/analysis/machine-identity-adr-proposal-2026-10-28.md`): der vorlaeufige Wert `0x00050000` ist die registrierte ARAnyM-Kennung; kein portiertes Programm vergleicht `_MCH` numerisch; EmuTOS schaltet NVRAM, Videl, DSP, RTC ueber Hardware-Flags. Anwender: `_MCH` = `0xFFFFFFFF` (EmuTOS „keine Atari-Hardware“), `NTOS` ist die Kennung; ADR-Hinweis, dass `-1` je nach Vorzeichen „unter ST“ oder „ueber Falcon“ liest, also jeder kuenftige Port auf numerische Vergleiche geprueft wird; `docs/api/bios/_cookie-jar.md` berichtigt. `_CPU`/`_FPU`/`_SND`/`_FDC` bleiben weg, `_VDO` erst messen (TeraDesk-ST-Zweig). mintlibs `sysinfo(SI_PLATFORM)` (NULL, fehlendes Komma) im Overlay korrigieren. Cookie-Falltabelle auf dem Pin fuer `tt`/`falcontc`/`disk`, ACCEPTED-Abweichungen mit gemessenen Werten. [XBIOS10](../TASK/XBIOS10.md): kein NVRAM, Antwort wie EmuTOS ohne Chip, gemessen mit einem neuen ST-Profil. LP64: dritter Fall derselben Klasse (TeraDesk `hdv_*`/`_sysbase`, Cookie-Jar mit `long`-Paaren in HighWire/TeraDesk) — systematische Erhebung ueber alle Ports, Messung je Fund, SDK-Typen fester Breite (SDK04). |
| 2026-10-28 | **ADR-031 „OS hooks installed by programs“ ([BIOS03](../TASK/BIOS03.md)); Umsetzung [BIOS15](../TASK/BIOS15.md); TOOL17 hinter XBIOS01.** Bestand (`docs/analysis/os-hooks-adr-proposal-2026-10-28.md`): von den portierten Programmen haengt nur TeraDesk Haken ein, die NewTOS nicht bedient. Anwender: bedient bleiben `kbdvec`, `etv_term`, `vex_wheelv`, `Supexec`, `G_USERDEF`; alle anderen nicht, mit einer Warnung je Haken und Prozess (nie still; neu auch fuer `vex_butv/motv/curv/timv`). `hdv_*` im ADR getrennt: Floppy-Nutzung = fehlende Hardware (Nicht-Ziel), Nicht-Floppy (RAM-Disk, Cache) = Ausloeser fuer volle Unterstuetzung; kein App-Patch. BIOS-CORE-002 und XBIOS-CHARDEV-003 ACCEPTED mit dem ADR als Beleg. Besitz beim einhaengenden Prozess, `Pterm` stellt nur oben in der XBRA-Kette zurueck, `Ptermres` behaelt; Seite 0 nur zum Warnen lesen, beim Verlassen des Super-Zustands; `prt_stat`/`prt_vec` = `just_rts`-Stub. `docs/COMPATIBILITY.md` „OS hooks“. Erhebung `Iorec`: Luecke „kein lokaler Quelltext“ fuer das Standardabbild per Konstruktion leer (alle 31 Programme aus Quellen im Baum); Pruefung als Verfahren in `.claude/agents/researcher.md`. TOOL17 klassifiziert Floppy-Funktionen als `design:hardware:floppy`. |
| 2026-10-28 | **[TOOL17](../TASK/TOOL17.md): Abdeckung der Zuordnung EmuTOS → NewTOS messbar machen; Ratsche.** Anwender: `mapping.tsv` trennt „kein Gegenstueck mit Absicht“ (Beleg: ADR, z. B. ADR-022 EmuDESK, ADR-030 Line-A, oder Hardware) von „noch nicht abgebildet“ — heute stehen beide als `-` mit freier Notiz (671 Zeilen), `vdi`/`aes`/`desk` haben keine Zeilen. `cargo xtask deps` meldet die Abdeckung je Schicht (abgebildet / mit Absicht / nicht abgebildet). CLAUDE.md „Testing“: jeder Fix ergaenzt die Zeilen der Funktionen, die er beruehrt; eine Retest-Menge ist erst vollstaendig, wenn ihre `--up`-Menge in den beruehrten Schichten keine nicht abgebildete Funktion hat, sonst listet das Ticket sie. TOOL17 kommt vor BIOS01 S2. |
| 2026-10-28 | **`Iorec`-Vertrag entschieden ([XBIOS01](../TASK/XBIOS01.md), `docs/api/xbios/0e-iorec.md`); Lokalisierung vor jedem Audit-Fix.** Anwender: ein gemeinsamer Ring ersetzt `RX_RING`, sein Inhalt ist Programmdaten, der Kernel validiert `head`/`tail` und `buf`/`size` bei jeder Nutzung; 32-Bit-Eintraege mit Shift zum Push-Zeitpunkt (schliesst [BIOS11](../TASK/BIOS11.md)); Zustellung ohne Trap vorerst nicht, ausser die Erhebung findet ein Programm, das den Ring ohne Trap pollt (dann per Timer wie EmuTOS per Interrupt); ersetztes `buf`/`size` je nach Erhebung kernel-eigen mit einmaliger Warnung oder EmuTOS-treu nur in global abgebildetem Speicher; Geraete 0 und 2 leer wie das Orakel; `kbdvec`-Argument im Haken-ADR [BIOS03](../TASK/BIOS03.md). Abnahme XBIOS01: `offsetof`-Pruefung des SDK-`_IOREC` gegen die Spec und Flush (`ibufhd = ibuftl` → `Bconstat(2) == 0`) gemessen auf beiden Seiten — dafuer schreibt [TOOL16](../TASK/TOOL16.md) auch durch den Rueckgabezeiger. CLAUDE.md „Testing“: vor jedem Fix aus einem Audit-Ticket lokalisiert der Lead per `cargo xtask deps --down` + `mapping.tsv` und traegt NewTOS-Funktion und `--up`-Retest-Menge ins Ticket ein; fuer BIOS01 (BIOS-STOR-001) erledigt: `bios/mod.rs@rwabs`/`@mediach`, `bpb.rs@getbpb`, Retest 83/85/26 NewTOS-Gegenstuecke. |
| 2026-10-28 | **[TOOL12](../TASK/TOOL12.md): Abhaengigkeitsgraph Track B — Trap #2 je Opcode, Kantenart `vector` (Commit dieser Zeile).** Der Graph fuehrte von `gem`/`gsx2` je eine „not resolved“-Kante zu jedem Trap-#2-Handler und kannte keine Aufrufe ueber Systemvektoren: `deps lrwabs --down` endete bei `protect_wlwwwl` (BIOS-STOR-009). `tools/deps/gen-deps.py` verfolgt den AES/VDI-Opcode in gimple bis zum `crysbind`-Switch bzw. `jmptb1/2` (+87 `table`, +91 `layer`, die sechs alten Kanten entfallen) und leitet `vector`-Kanten aus Ablagen von Funktionsadressen in Slots ab (gimple, Initialisierer, `Setexc`, Assembler, Adressen aus `tosvars.ld`): 114 Kanten, 34 Slots. `lrwabs --down` 1 → 155; `--up`-Mengen schrumpfen, weil die alte Sammelkante jede AES/Desk-Funktion mitzog (`getrec --up` 539 → 382, Abschlusskriterium-Test gruen). `mapping.tsv`: `blkdev_*` n:m, 21 VT52-Handler mit Gegenstueck in `tos-api/src/vdi/escape.rs`. Luecken in `tools/deps/README.md`. `--check` bytegleich; Hosttests 1748/0/1 (Lead-Gegenlauf gleich). |
| 2026-10-28 | **Korrektur [SDK02](../TASK/SDK02.md); gemeinsames Haken-ADR [BIOS03](../TASK/BIOS03.md); Sondenregel fuer MEASURED-Fixes; [TOOL16](../TASK/TOOL16.md); [ARCH05](../TASK/ARCH05.md).** Anwender: mintlibs `RW_*` sind Bitnummern, `1 << RW_x` ergibt die Masken der Spec 1/2/4/8 — Werte bleiben (Quellkompatibilitaet), nur Kommentar und Bau-Pruefung; die Lesart des vorigen Eintrags („`RW_WRITE` 0 macht Schreiben zum Lesen“, „Fix auf Masken“) ist zurueckgenommen; SDK02 P3, kein Vorlauf vor BIOS01. BIOS03 wird ein ADR fuer alle von Programmen eingehaengten OS-Haken (`hdv_*`, `etv_timer`, `Kbdvbase`-Haken u. a.) mit Tabelle je Haken, Entscheidung nach Bestand in den portierten Programmen; Regel: nie still, ein nicht bedienter Haken wird einmal gewarnt. CLAUDE.md „Testing“: ein Fix fuer einen nur per Sonde gemessenen Befund darf die Sonde als Abnahmefall nehmen, wenn sie dauerhaft unter `tosdump/` liegt, beidseitig gegen das gepinnte Orakel laeuft und den Eingabe-Trace hat; Folgeticket nach tosdiff. TOOL16 (P1): Dumper folgt Rueckgabezeigern mit Layout aus `docs/api/`, Voraussetzung fuer die Abnahme von XBIOS01 und BIOS06. XBIOS10: ADR-012 regelt nur, dass `_MCH` eine eigene Kennung ist (Wert `0x0005_0000` im Code vorlaeufig) — erst ARCH05 (welche Maschine NewTOS meldet und was daraus folgt). |
| 2026-10-28 | **Tickets fuer die Funde der Audits BIOS-Kern, XBIOS-Zeichengeraete, XBIOS-System: [BIOS04](../TASK/BIOS04.md)–[BIOS14](../TASK/BIOS14.md), [XBIOS01](../TASK/XBIOS01.md)–[XBIOS14](../TASK/XBIOS14.md).** Researcher-Entwurf, je Ticket Befund-ID, Schwere, Urteil, Messwerte beider Seiten und Beleg; S1/S2 = P1, S3 = P2, S4 = P3. Das S1 `Iorec` (XBIOS01) folgt laut Anwender direkt auf BIOS01 S2; davor legt ein Researcher den Vertrag des IOREC-Abbilds fest. Haken, die NewTOS nie ruft (`etv_timer` BIOS05, `Kbdvbase`-Haken XBIOS03), warten auf die Entscheidung, ob [BIOS03](../TASK/BIOS03.md) ein gemeinsames ADR wird. Zusammengelegt nur CORE-004/005/006 (BIOS07, ein Zweig von `Setexc`) und CORE-012 mit CHARDEV-008 (BIOS14, Graph); Nebenbeobachtungen der Berichte (`Ongibit`, `Tsetdate`, VDI und `mousevec`, Absturzbildschirm, US-Tabelle des Orakels) nicht getickert. Abnahmefaelle der MEASURED-Befunde ohne tosdiff-Fall sind vorerst Sonden (fehlende Capture-Form „durch den Rueckgabezeiger“, BIOS13 Punkt 7). |
| 2026-10-28 | **Entscheidungen zu [BIOS02](../TASK/BIOS02.md), [TOOL07](../TASK/TOOL07.md), [TOOL08](../TASK/TOOL08.md); neue Tickets [SDK02](../TASK/SDK02.md), [TOOL12](../TASK/TOOL12.md)–[TOOL15](../TASK/TOOL15.md).** Spec BIOS-Speicher freigegeben (`d7b2d9f8` Entwurf): 001, 003, 005 EmuTOS folgen, 007 angleichen; 002 = die SD der Pi ist physische Einheit 26 (EmuTOS-Zaehlung SD/MMC), jede andere Einheit `EUNDEV`, Falltabellen per Token aus der Profildefinition (Orakel 18, NewTOS 26), keine Suchlogik; der BIOS01-Fall fuer 001 belegt „schreibt nichts“ per Abbildvergleich. TOOL07: Variante B, Runner-Regel „zerstoerende Tabelle → frischer Plattenklon“, nach jeder solchen Tabelle Abbildvergleich beider Seiten (gleicher Endzustand samt kaputtem Bootsektor), Vorspann `+set` nur in `!disk … destructive`; Platten-Kanarienvogel auf `bios-r2.tsv`, sobald zweimal bitgleich. TOOL08: Zeilennummern angenommen, stehende Regel in CLAUDE.md „Testing“ (Debug-Feature aus = Code/Daten gleich bis auf `panic::Location`), gepruefter xtask-Befehl TOOL13. SDK: mintlibs `ostruct.h` (Upstream 0.60.1 und Overlay) hat `RW_*` als Bitnummern (`RW_WRITE` 0, `RW_NOTRANSLATE` 3); die Audit-Sonde nutzte Zahlenwerte, also keine Artefakte; kein SDK-Programm nutzt die Header-Konstanten (`apitest` definiert 8 selbst) — Fix auf Masken mit Bau-Pruefung SDK02 vor BIOS01. TOOL12 Track B laeuft (Developer); TOOL14 Trace-Betriebsart des Orakels, TOOL15 mtools in `oracle.env` und `--fast-forward` als Profilparameter. Altes `kernel8.img` im Wurzelverzeichnis geloescht. |
| 2026-10-28 | **[TOOL11](../TASK/TOOL11.md): `PTR_RAM` ab `0x400`, Laeufer mit denselben Grenzen (`55d11bd0`).** Seit `ff1da029` galt jede gerade Nicht-Null unter 2 GiB als Zeiger; NewTOS' Funktionsnummern (`Iorec` `0x0E`, `Bconmap(-2)` `0x2C`) bestanden und verdeckten das fehlende `Iorec`. Untergrenze `sysvars_start` (`0x400`): alle 25 Orakel-Referenzen bestehen, `Keytbl`/`Setexc` von NewTOS bleiben gueltig; der Laeufer folgt einem Rueckgabezeiger (`r:`) nur innerhalb der Grenzen. DUMPBIOS `bios-r1` jetzt 46/50 (`tt`/`falcontc`) statt 42/46 abweichend, neu genau 98–100 und 184; `xbios-chardev-r1` 21–23, 44. Keine Referenz neu aufzunehmen, Kanarienvoegel OK. Beleg `measurements/measurement-2026-10-28-tool11/RUNINFO.TXT`. |
| 2026-10-28 | **[TOOL10](../TASK/TOOL10.md): `kernel8.img` neben dem ELF (`04ed0b76`); PTR_RAM-Vorschlag fuer [TOOL11](../TASK/TOOL11.md) (`ed83bb69`).** `cargo xtask build`/`run` schrieben die Karten-Kernel ins Wurzelverzeichnis, entgegen CLAUDE.md; jetzt `target/aarch64-unknown-none-softfloat/<debug\|release>/kernel8.img`, der Bau nennt den Pfad. `cargo xtask tosdump --tree` nimmt nur das gerade gebaute Abbild, auch bei Commits vor der Aenderung. CLAUDE.md nennt den Pfad. Hosttests 1745/0/1. TOOL11 Schritt 1 (Researcher, `docs/analysis/ptr-ram-rule-2026-10-28.md`): Untergrenze `0x400` (`sysvars_start`) fuer `PTR_RAM`; alle Orakelwerte bestehen, neu abweichend nur `Iorec` und `Bconmap(-2)`; der Laeufer muss dieselben Grenzen pruefen, bevor er einem Rueckgabezeiger folgt. Beleg `measurements/measurement-2026-10-28-tool10/RUNINFO.TXT`. |

## Notes

The project is intentionally over-documented at the start. The premise:
spec-first development against a fixed, finite, well-documented API
prevents the implementation drift that ruins long-running OS hobby
projects.
