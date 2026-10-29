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
| Hosttests (`cargo xtask test`) | **1840 / 0 / 1** (gemessen 2026-10-29 mit BIOS01 Rest, +7 Tests; davor 1833 mit TOOL19, +5 Tests; davor 1828 mit ARCH06, +4 Tests; davor 1824 mit XBIOS18, +1 Test; davor 1823 mit TOOL21, +2 Tests; davor 1821 mit TOOL17, +5 Tests; davor 1816 mit XBIOS15, +8 Tests; davor 1808 mit BIOS15, +18 Tests; davor 1790 mit FS22, +7 Tests; davor 1783 mit XBIOS01, +16 Tests; davor 1767 mit FS20, +7 Tests; davor 1760 mit BIOS01 S2, +9 Tests; davor 1751 mit TOOL07 Rest, +3 xtask-Tests; davor 1748 mit TOOL12, +3 deps-Tests; davor 1745 mit TOOL10; TOOL09 +16, TOOL10 +3 xtask-Tests; vorher 1726 / 0 / 1 auf `75527af8`, `--fast` ohne den einen Doctest) |
| SDK-Hosttests (`sdk/run-host-tests.sh`) | **33 / 0 / 0** (gemessen 2026-10-29 mit SDK04, +3 `sysvars-layout`, `hypview-ref-host`, `zview-color-host`; davor 30 mit SDK04 C6, +1 `td-cookie-host`; davor 29 mit SDK05, +1 `cookie-host`; davor 28 mit SDK03, +1 `sysinfo-host`; davor 27 mit XBIOS01, +1 `iorec-layout`) |
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
  [XBIOS17](../TASK/XBIOS17.md) (qemu-virt `Bconout(1)`; erst
  Bericht, P3) →
  die uebrigen Audit-Tickets BIOS04–BIOS14, XBIOS02–XBIOS14 (S2
  zuerst; BIOS05 und XBIOS03 sind ACCEPTED nach ADR-031, XBIOS10 nach ARCH05) →
  [TOOL13](../TASK/TOOL13.md), [TOOL14](../TASK/TOOL14.md),
  [TOOL15](../TASK/TOOL15.md) → [TOOL03](../TASK/TOOL03.md) → DUMPGDOS →
  [FS19](../TASK/FS19.md) → [MEM11](../TASK/MEM11.md) (Erzwingung) →
  [TOOL01](../TASK/TOOL01.md); [SDK02](../TASK/SDK02.md), [SDK06](../TASK/SDK06.md)
  (P3) bei Gelegenheit.
  Daneben ein Researcher zur Zeit, ohne Produktaenderung: Erhebung zu
  TOOL17-Gruppen B (XHDI/SCSIDRV), C (OS-Pool/DND-Baum), F (Startfunktionen):
  wer nutzt sie (laeuft).
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
| 2026-10-29 | **[BIOS01](../TASK/BIOS01.md) Rest: BIOS-STOR-002 bis -007 behoben (Commit dieser Zeile).** Nach freigegebener Spec und EmuTOS `bios/blkdev.c`: 003 Anzahl 0 durchlaeuft die Bereichspruefung (`-8`); 007 NULL-Puffer auf A:/B: liefert 0; 004 `Getbpb` lehnt Sektorgroessen ueber der beim Start ermittelten Grenze ab (`scan` wie EmuTOS `pun_info_setup` — die Laufzeitgrenze stand in der Spec als „open“, zur Bestaetigung beim Anwender); 005 Sektorzahl 0 ergibt `numcl` 0; 006 nach gescheitertem `Getbpb` liefert logisches `Rwabs` `ESECNF` (`recsiz` 0); 002 `RW_NOTRANSLATE` adressiert die SD als physische Einheit 26, jede andere `EUNDEV` (`kernel/src/selftest_emmc.rs` und `apitest` BB13–BB16 umgestellt). Bestaetigende Faelle alle EQUAL: `bios-r2` 0/51, `x004` 0/10, `x005` 0/13, `x006` 0/11, `phys` 0/12, `x001` 0/8; Endzustaende der zerstoerenden Tabellen gleich dem Orakel. Kein vorher gleicher Fall neu verschieden (`bios-r1`, `gemdos-mc-*`). Offen in der Spec: Skalierung bei `recsiz` > 512 (Schritt 7). Hosttests 1840/0/1 (Lead-Gegenlauf gleich). Beleg `measurements/measurement-2026-10-29-bios01-rest/RUNINFO.TXT`. |
| 2026-10-29 | **[TOOL19](../TASK/TOOL19.md): `DISKUNIT.TXT`/`PROGEXT.TXT` aus einer Profildefinition, aufgezeichnet, ohne Standardwert (Commit dieser Zeile).** Neue Datei `tosdump/profiles.tsv` je Runner und Profil (Orakel `disk`: 18/PRG, NewTOS: 26/NAP); Orakel-Skript und `cargo xtask tosdump` lesen sie zur Laufzeit, tosdiff baut sie ein. Aufgezeichnet in `profile.txt` bzw. `tosdump-run.txt` (zurueckgelesen von der Karte), LOG-Format unveraendert. Abgewiesen (Exit 2 / „nothing run“ / FAILED): fehlende Datei, falscher Wert, LOG ohne passenden Profilwert; tosdiff prueft vor jedem Vergleich und Selfcheck (`--check-profile`). Gemessen: alle Abweisungen und je ein positiver Lauf beidseitig; Referenzen unberuehrt. Hosttests 1833/0/1 (Lead-Gegenlauf gleich), `tosdump` host-test gruen. Beleg `measurements/measurement-2026-10-29-tool19/RUNINFO.TXT`. |
| 2026-10-29 | **[TOOL18](../TASK/TOOL18.md): Cookie-Jar auf dem Pin gemessen, alle drei Profile (`9b882e8d`).** Neuer Falltyp `jar:<TAG>` (Wert, „absent“ unterscheidbar von `0xFFFFFFFF`) und `jar:*` (Anzahl und Reihenfolge), Spec in `docs/TOS-REFERENCE-DUMPER.md` §6. Orakel: 12 Cookies (`_CPU _VDO _FPU _MCH _SWI _SND _FRB _AKP _IDT _FDC XHDI SCSI`), falcontc = disk bytegleich; NewTOS 5 (`NTOS _MCH _IDT _AKP EdDI`), 15 von 22 Faellen verschieden. `docs/COMPATIBILITY.md` „Cookie jar vs. the oracle“ mit den gemessenen Werten: 8 Unterschiede ACCEPTED durch ADR-012 und den EdDI-Eintrag, **7 OPEN** beim Anwender (`_VDO`, `_SWI`, `_FRB`, `_AKP`, `_IDT`, `XHDI`, `SCSI`). Sechs verschobene `sdk/`-Anker in zwei Analysen nachgezogen. `tosdump` host-test 40 Referenzen; Hosttests 1828/0/1. Beleg `measurements/measurement-2026-10-29-tool18/RUNINFO.TXT`. |
| 2026-10-29 | **[SDK04](../TASK/SDK04.md) Developer-Teil: SDK-Typen fester Breite, HypView `.REF` und zView `remap_color` behoben (Commit dieser Zeile).** Overlay `sdk/libc/overlay/include/mint/sysvars.h`: Seite-0-Zellen als `volatile` 32-Bit, `OSHEADER` mit 48 Byte, `NEWTOS_COOKIE` mit Schritt 8 — alles per `_Static_assert` gegen `docs/api/` geprueft; `ostruct.h` bekommt Asserts fuer `_MD`/`_MPB` (Layout A aus BIOS06) und `_KBDVECS`. Erzeugter Code byte-gleich (`libc.a` und alle sechs Nutzer von `sysvars.h`). HypView `.REF` las Magie und Struktur als `long`: jede Datei war „Illegal REF file“; Patch liest big-endian aus den Bytes (Host-Test mit echten `.REF`: vorher 18/19 Fehler, nachher 0). `.BGH` toter Code, nur vermerkt. zView `remap_color` nahm Big-Endian an (Rot→Gruen, Blau→Schwarz; latent, nur 4/8 Bitplanes): Patch, Host-Test vorher 38/41 Fehler, nachher 0. `05-abi-pitfalls.md` berichtigt (`Kbdvbase`-Zeiger, kein „press any key“ fuer `.NCP`). SDK-Hosttests 33/0/0 (Lead-Gegenlauf gleich). |
| 2026-10-29 | **[ARCH06](../TASK/ARCH06.md): `_MCH` = `0xFFFFFFFF` (Commit dieser Zeile); `_VDO` gemessen, Entscheidung beim Anwender.** `MCH_NOHARD` in `tos-api/src/cookie.rs`, alle Kopien nachgezogen (Selbsttest, Kommentare, Spec, `COOK01`/`X03` mit Nachtrag). Gemessen auf QEMU raspi4b: Zeigerform `0xFFFFFFFF` (4 Byte geschrieben, `Getcookie` nullerweitert), Wertform `-1` — gleich „nicht gefunden“, also mehrdeutig; mintlib `sysinfo`/`uname` liefern „unknown“ ohne Absturz (SDK03); zView-Zweig bleibt tot. `_VDO`: ohne den Cookie nimmt TeraDesk den ST-Zweig — Overscan ist im Video-Dialog freigeschaltet, Haken + OK endet mit Bus Error (Schreiben auf `0xFFFC00`, ACIA, nach `Super(0L)`); mit eingeschleustem Falcon-`_VDO` ruft es `VsetMode`/`VgetMonitor` und erhaelt die Funktionsnummern `0x58`/`0x59`, OK loest einen harmlosen AES-Neustart aus. Gruppe E in `mapping.tsv` eingeordnet. Hosttests 1828/0/1 (Lead-Gegenlauf gleich); `docs/HARDWARE-PROFILE.md` behaelt seinen alten doc-check-Rueckstand (29 vorher und nachher). Beleg `measurements/measurement-2026-10-29-arch06/RUNINFO.TXT`. |
| 2026-10-29 | **[SDK04](../TASK/SDK04.md) Fund C6: TeraDesk `find_cookie` liest das Jar richtig (Commit dieser Zeile).** Unter LP64 lief TeraDesk mit 16-Byte-Schritt durch das Cookie-Jar und fand nur Cookies im geraden Slot mit Wert 0 (und lieferte dann den Wert des Nachbarn). Benannte Patch-Stelle im TeraDesk-Overlay: Slot als zwei `uint32_t`, Schritt 8 Byte, Wert null-erweitert wie `Getcookie`. Sonde `TDCOOKIE.NCP` (TeraDesks `cookie.c` unveraendert gebaut) auf QEMU raspi4b: vorher 3/12, nachher 12/12 wie `Getcookie`; TeraDesk startet unveraendert bis zum Desktop. `docs/sdk/05-abi-pitfalls.md` B2 und `docs/api/bios/_cookie-jar.md` berichtigt (die Begruendung „TeraDesks Weg funktioniert“ stand an zwei Stellen). Neuer Hosttest `td-cookie-host` (ohne Patch 30/41 Fehler). SDK-Hosttests 30/0/0 (Lead-Gegenlauf gleich). Beleg `measurements/measurement-2026-10-29-sdk04-c6/RUNINFO.TXT`. |
| 2026-10-29 | **[SDK05](../TASK/SDK05.md): cflib `getcookie` baut Cookie-Namen richtig (Commit dieser Zeile).** cflib baute den Namen ueber eine Union aus `char[4]` und `long`; auf Little-Endian AArch64 kam er verdreht heraus, die oberen 4 Byte der LP64-`long` blieben unbestimmt — `getcookie` fand nichts. Benannte Patch-Stelle in `sdk/cflib/overlay/cookie.c` (Upstream unveraendert): Name per Schiebeoperationen, ganze `long` definiert. Sonde `CKPROBE.NCP` gegen das echte `cflib.a` auf QEMU raspi4b: vorher 1/6 Cookies wie `Getcookie`, nachher 6/6. QED (Speicherabzug) liest `_IDT` jetzt `0x112F` statt des Ersatzwerts `0x112E` — sichtbar aendert sich nichts, QED wertet nur die Datumsreihenfolge aus, die bei beiden gleich ist (Annahme des Tickets war abgeleitet). Neuer Hosttest `cookie-host` (ohne Patch 17/26 Fehler). SDK-Hosttests 29/0/0 (Lead-Gegenlauf gleich). Beleg `measurements/measurement-2026-10-29-sdk05/RUNINFO.TXT`. |
| 2026-10-29 | **[SDK03](../TASK/SDK03.md): mintlib `sysinfo(SI_PLATFORM)` im Overlay korrigiert (Commit dieser Zeile).** Zwei benannte Patch-Stellen in `sdk/libc/overlay/unix/sysinfo.c` (Upstream unveraendert): das fehlende Komma hinter `"cfevb"` (sonst „cfevbatariclone“ und Lesen hinter dem Feldende fuer jede unbekannte Familie) und ein Zweig fuer `_MCH` = `0xFFFFFFFF` ohne ColdFire (sonst `strlen(NULL)`), Name „unknown“ wie EmuTOS' `guess_machine_name()`. Neuer Hosttest `sysinfo-host` (ASan, echtes `getcookie.c`, 15 Faelle): ohne Patch 5 Fehler (SEGV bzw. falscher Name), mit Patch 0. Upstream-Bericht als Vorschlag in der Quittung, nicht gesendet. SDK-Hosttests 28/0/0 (Lead-Gegenlauf gleich). |
| 2026-10-29 | **[XBIOS18](../TASK/XBIOS18.md): `kbdvec`-Haken laeuft auch, waehrend ein Prozess auf AUX wartet (Commit dieser Zeile).** Vorher bekam der Haken eines in `Bconin(1)`/`Cauxin` wartenden Prozesses seine Tastenbytes auf qemu-virt erst mit dem naechsten seriellen Byte. Jetzt weicht das AUX-Warten aus wie `read_key_loop` (`give_way`): Trap-Neustart, der Haken laeuft, dann wieder warten — kein Busy-Spin, ADR-029 unveraendert; nur qemu-virt (rpi4-Kern bytegleich). Sonde `axprobe` dauerhaft unter `tosdump/probes/`, beidseitig: Orakel und NewTOS je 2 Haken-Aufrufe vor dem seriellen Byte (vorher NewTOS 0); auf qemu-virt laeuft die Sonde als eingebetteter Desktop aus einer Scratch-Verdrahtung (`measurements/measurement-2026-10-29-xbios18/method/embed-probe-scratch.diff`). Keine neue Abweichung. Hosttests 1824/0/1 (Lead-Gegenlauf gleich). Beleg `measurements/measurement-2026-10-29-xbios18/RUNINFO.TXT`. |
| 2026-10-29 | **[TOOL21](../TASK/TOOL21.md): ADR-Belege mit Fundstelle, Gruppe A als Hardware, AUX-Pfad neu eingeordnet (Commit dieser Zeile).** `design:ADR-NNN@<datei>:<zeile>` ist Pflicht; der Hosttest prueft, dass die Zeile im Abschnitt des ADR liegt (20 Zeilen umgestellt). DSP, DMA-Ton, TT-Shifter und VIDEL (78 Zeilen) sind `design:hardware:` nach Anwenderentscheidung (Ausnahme von der TOOL17-Regel „Trap-Platz nie allein wegen Hardware“, in der README vermerkt). AUX-Pfad nach XBIOS15: 8 Definitionen jetzt `mapped`, Sendepfad und `rsconfB`/`rsconf_dummy` bleiben `unmapped` mit Notiz. Abdeckung bios: mapped 169, design 477, unmapped 143 (vorher 161/399/229). Hosttests 1823/0/1 (Lead-Gegenlauf gleich). |

## Notes

The project is intentionally over-documented at the start. The premise:
spec-first development against a fixed, finite, well-documented API
prevents the implementation drift that ruins long-running OS hobby
projects.
