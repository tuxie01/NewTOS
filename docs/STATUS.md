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
| Hosttests (`cargo xtask test`) | **1918 / 0 / 1** (gemessen 2026-10-31 mit FS21, +8 Tests; davor 1910 mit MEM11, +3 Tests; davor 1907 mit FS28, +2 Tests; davor 1905 mit TOOL29, +6 xtask-Tests; davor 1899 mit A126, +4 Tests; davor 1895 mit SDK08, kein neuer Test; davor 1895 mit COOK02 `_IDT`, kein neuer Test; davor 1895 mit XBIOS04, +1 Test; davor 1894 mit BIOS09, +1 Test; davor 1893 mit BIOS17, +1 Test; davor 1892 mit BIOS16, +4 Tests; davor 1888 mit BIOS08, +7 Tests; davor 1881 mit XBIOS05, +9 Tests, Basis laut Lead 1872; davor 1859 mit BIOS07, +9 Tests; davor 1850 mit FS23, +3 Tests; davor 1847 mit BIOS04, +4 Tests; davor 1843 mit XBIOS02, +1 Test; davor 1842 mit BIOS06, +2 Tests; davor 1840 mit BIOS01 Rest, +7 Tests; davor 1833 mit TOOL19, +5 Tests; davor 1828 mit ARCH06, +4 Tests; davor 1824 mit XBIOS18, +1 Test; davor 1823 mit TOOL21, +2 Tests; davor 1821 mit TOOL17, +5 Tests; davor 1816 mit XBIOS15, +8 Tests; davor 1808 mit BIOS15, +18 Tests; davor 1790 mit FS22, +7 Tests; davor 1783 mit XBIOS01, +16 Tests; davor 1767 mit FS20, +7 Tests; davor 1760 mit BIOS01 S2, +9 Tests; davor 1751 mit TOOL07 Rest, +3 xtask-Tests; davor 1748 mit TOOL12, +3 deps-Tests; davor 1745 mit TOOL10; TOOL09 +16, TOOL10 +3 xtask-Tests; vorher 1726 / 0 / 1 auf `75527af8`, `--fast` ohne den einen Doctest) |
| SDK-Hosttests (`sdk/run-host-tests.sh`) | **35 / 0 / 0** (gemessen 2026-10-30 mit SDK08, kein neuer Test: Rebuild-Paritaet von `rt-host`/`ldbl-host` blendet nur `.comment` aus, Host-clang `clang-2100.4.1.1`; dazwischen 33/2/0 nach dem clang-Update; davor 35 mit XBIOS05, +1 `bconmap-layout`; davor 34 mit SDK07, +1 `td-nohard-host`; davor 33 mit SDK04, +3 `sysvars-layout`, `hypview-ref-host`, `zview-color-host`; davor 30 mit SDK04 C6, +1 `td-cookie-host`; davor 29 mit SDK05, +1 `cookie-host`; davor 28 mit SDK03, +1 `sysinfo-host`; davor 27 mit XBIOS01, +1 `iorec-layout`) |
| `sh tools/desk-drift-probe.sh` | **0** (unsafe 20; gemessen 2026-10-26) |
| APITEST (`#Z`, memprot on/off/strict) | **109 / 0 / 5** (letzter Lauf `measurements/measurement-2026-10-26-a118-fix/RUNINFO.TXT`) |

Bau- und Clippy-Warnungen stehen hier ohne Zahl: sie wurden bei dieser
Bereinigung nicht nachgemessen. Sie duerfen nicht steigen; jeder Arbeitsschritt
nennt seine Vorher-/Nachher-Zahl in seiner Zeile unter *Recent Decisions*.

**Offen (Stand 2026-10-26, aus der Statuszeile der Tickets ohne DONE-Quittung):**

- **Warteschlange (Anwender 2026-10-29), in dieser Reihenfolge.** Regel: ein
  neues Werkzeug-Ticket kommt hinter den Produkt-Fix, den es verzoegern wuerde,
  ausser der Fix ist ohne das Werkzeug nicht abnehmbar.
  **Vor-Start-Pruefung je Ticket (Anwender 2026-10-30):** Produkt-Ticket —
  ist ein Programm auf der Karte oder ein in diesem Repo geplanter Port
  betroffen? Werkzeug-Ticket — blockiert es die Abnahme eines Produkt-Fixes?
  Sonst parken, mit Grund hier.
  **Geparkt 2026-10-31 (Anwender) nach dieser Pruefung:**
  [TOOL13](../TASK/TOOL13.md) (kein anstehender Fix bringt ein Debug-Feature;
  Regel bisher von Hand belegt), [TOOL14](../TASK/TOOL14.md) (Sonden laufen
  ohne Trace-Betriebsart), [TOOL15](../TASK/TOOL15.md) (Echtzeit-Sonden
  schalten fast-forward selbst ab; mtools-Version faengt die sha256-Pruefung),
  [TOOL03](../TASK/TOOL03.md) (nur fuer DUMPGDOS/FS19 noetig), DUMPGDOS (kein
  Ticket; GEMDOS-Faelle laufen ueber Sonden und `gemdos-mc-*`),
  [FS19](../TASK/FS19.md) (nur Programme mit DTA in unabgebildetem Speicher),
  [MEM11](../TASK/MEM11.md) (heute nicht erreichbar; ein Hosttest haelt die
  Speicheraufteilung unter 2 GiB fest, damit eine Aenderung laut scheitert),
  [TOOL01](../TASK/TOOL01.md) (blockiert keine Abnahme),
  [FS25](../TASK/FS25.md) (Frage nur ans Orakel).
  Ebenfalls geparkt 2026-10-31 (P3-Durchsicht):
  [BIOS13](../TASK/BIOS13.md) (reine Doku-/Ledger-Pflege; kein Programm, kein offener Fix),
  [BIOS14](../TASK/BIOS14.md) (Werkzeugluecke; blockiert keinen offenen Fix),
  [XBIOS07](../TASK/XBIOS07.md) (Doku-Korrektur ohne Nutzer),
  [XBIOS11](../TASK/XBIOS11.md) (kein Kartenprogramm haengt am Seed-Ursprung (einziger Nutzer TeraDesk-Formatieren)),
  [XBIOS12](../TASK/XBIOS12.md) (von RTC01 abgeloest; kein Nutzer von XBIOS Gettime gefunden),
  [XBIOS13](../TASK/XBIOS13.md) (reine Spec-Pflege),
  [XBIOS14](../TASK/XBIOS14.md) (Orakelgrenze; blockiert keinen Fix),
  [XBIOS16](../TASK/XBIOS16.md) (kein Programm ruft Jdisint/Jenabint/Mfpint),
  [XBIOS19](../TASK/XBIOS19.md) (kein Programm ruft Bioskeys),
  [KEYS04](../TASK/KEYS04.md) (kein Kartenprogramm pollt Kbshift (nur apitest)),
  [INPUT03](../TASK/INPUT03.md) (kein Programm braucht die Maus-Tastatur-Emulation),
  [PROC07](../TASK/PROC07.md) (undokumentierte CP/M-Reliquie; gemlib sendet nur Opcodes != 0),
  [FS27](../TASK/FS27.md) (nur beschaedigte Medien),
  [SPEC02](../TASK/SPEC02.md) (reine Nachpflege; offene Arbeit liest die Stellen nicht),
  [SPEC03](../TASK/SPEC03.md) (reine Aufraeumarbeit; der Hook blockiert die Altlast nicht),
  [TOOL22](../TASK/TOOL22.md) (Tabellenpflege, haengt an PROC07),
  [TOOL23](../TASK/TOOL23.md) (Debug-Feature; wieder aufnehmen, wenn ein AES-Fix den Mitschnitt braucht),
  [TOOL24](../TASK/TOOL24.md) (Debug-Feature, blockiert nichts),
  [TOOL25](../TASK/TOOL25.md) (zugehoeriger Fix (BIOS07) per Sonde abgenommen),
  [TOOL26](../TASK/TOOL26.md) (zugehoeriger Fix (BIOS16) per Sonde abgenommen),
  [TOOL28](../TASK/TOOL28.md) (Handregel in CLAUDE.md deckt es ab),
  [TOOL30](../TASK/TOOL30.md) (zugehoeriger Fix (A126) per Sonde abgenommen),
  [SDK02](../TASK/SDK02.md) (kein Nutzer der Konstanten),
  [SDK06](../TASK/SDK06.md) (kein Programm liest einen Cookie mit gesetztem Bit 31).
  [FS21](../TASK/FS21.md) (P2, Archivbit beim `Fclose`; umgesetzt 2026-10-31, Commit dieser Zeile). Wartet auf Anwender: [BIOS12](../TASK/BIOS12.md) (D1–D3 `Setexc`, D1 `Kbshift`).
  Developer, einer zur Zeit:
  die S2-Audit-Tickets [XBIOS02](../TASK/XBIOS02.md) (erledigt 2026-10-29, `7bc047a6`),
  [BIOS04](../TASK/BIOS04.md) (erledigt 2026-10-29, Commit der Quittung) →
  [FS23](../TASK/FS23.md) (erledigt 2026-10-29, Commit der Quittung), [FS24](../TASK/FS24.md) (ACCEPTED unter ADR-033, Anwender 2026-10-30) →
  die uebrigen Audit-Tickets [BIOS07](../TASK/BIOS07.md) (erledigt 2026-10-29, Commit der Quittung),
  BIOS08–BIOS14 ([BIOS10](../TASK/BIOS10.md) erledigt 2026-10-29, Commit der Quittung; [BIOS08](../TASK/BIOS08.md) erledigt 2026-10-30, Commit der Quittung), XBIOS04–XBIOS14 ([XBIOS05](../TASK/XBIOS05.md) erledigt 2026-10-29, Commit der Quittung; BIOS05 und XBIOS03
  sind ACCEPTED nach ADR-031, XBIOS10 nach ARCH05) →
  [FS26](../TASK/FS26.md) (Ordner anlegen ~3 s auf der Pi; Schichtmessung in
  QEMU zuerst, Pi-Messung ins Testszenario; Anwender 2026-10-31) →
  [MEM11](../TASK/MEM11.md)-Hosttest (Speicheraufteilung < 2 GiB; erledigt 2026-10-31, Commit dieser Zeile) → [FS29](../TASK/FS29.md) (P3, geparkt 2026-10-31: braucht ein `Rwabs`-Sektor-0-Werkzeug), [FS30](../TASK/FS30.md) (P3, geparkt: `..`-Zeit, kein Programm liest sie),
  [PROC07](../TASK/PROC07.md), [COOK02](../TASK/COOK02.md) (geparkt; Punkt 3 `_IDT` = Orakel umgesetzt 2026-10-30, Commit der Quittung),
  [TOOL22](../TASK/TOOL22.md), [XBIOS19](../TASK/XBIOS19.md), [TOOL23](../TASK/TOOL23.md),
  [SPEC02](../TASK/SPEC02.md), [TOOL24](../TASK/TOOL24.md), [FS27](../TASK/FS27.md), [TOOL25](../TASK/TOOL25.md) (P3); [BIOS16](../TASK/BIOS16.md) (erledigt 2026-10-30, `9c4c29c5`) → [BIOS17](../TASK/BIOS17.md) (erledigt 2026-10-30, Commit der Quittung) → [BIOS09](../TASK/BIOS09.md) (umgesetzt 2026-10-30, Commit der Quittung; offen der Pi-Schritt in der MEM12-Prueffolge, Punkt 10) → [XBIOS04](../TASK/XBIOS04.md) (umgesetzt 2026-10-30, Commit der Quittung) → [SDK08](../TASK/SDK08.md) (erledigt 2026-10-30, Commit der Quittung) → [A126](../TASK/A126.md) (umgesetzt 2026-10-30, Commit dieser Zeile) → [TOOL29](../TASK/TOOL29.md) (umgesetzt 2026-10-30, Commit dieser Zeile; Hook-Installation durch den Lead) → [FS28](../TASK/FS28.md) (Anwender 2026-10-30; geparkt bis ein Port sie braucht, weil kein Programm der Karte betroffen ist: [XBIOS06](../TASK/XBIOS06.md)-Fix, [XBIOS08](../TASK/XBIOS08.md), [XBIOS09](../TASK/XBIOS09.md), [XBIOS10](../TASK/XBIOS10.md)) → [FS28](../TASK/FS28.md) (erledigt 2026-10-30, Commit dieser Zeile), [A126](../TASK/A126.md); [TOOL26](../TASK/TOOL26.md) (P3); [FS28](../TASK/FS28.md), [A126](../TASK/A126.md) (P2), [KEYS04](../TASK/KEYS04.md), [INPUT03](../TASK/INPUT03.md) (P3); [SDK02](../TASK/SDK02.md), [SDK06](../TASK/SDK06.md)
  (P3) bei Gelegenheit; [SDK08](../TASK/SDK08.md) (P2, SDK-Paritaet nach clang-Update; erledigt 2026-10-30);
  geparkt bis MIDI/IO01: [UART01](../TASK/UART01.md); [INPUT04](../TASK/INPUT04.md) (P3, 0xF6-Paketlaenge,
  ruht; vor jedem IKBD-Interpreter); [TOOL27](../TASK/TOOL27.md) (P3,
  rsprobe nach tosdiff, geparkt); [TOOL28](../TASK/TOOL28.md) (P3, host_clang ins RUNINFO); [TOOL30](../TASK/TOOL30.md) (P3, akprobe nach tosdiff). Zurueckgestellt (Anwender 2026-10-29):
  [DOOM01](../TASK/DOOM01.md) (DOOM-Freeze im Menue, nur auf der Pi).
  Daneben ein Researcher zur Zeit, ohne Produktaenderung: — (frei).
- **P1:** [MEM12](../TASK/MEM12.md) — erledigt 2026-10-30 (Anwender: Pi-Lauf 2026-10-29, Kern `58e35ac8`); `P0PAR` ist erster Schritt in [DOOM01](../TASK/DOOM01.md) und im spaeteren Testszenario.
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
  Debug-Pruefung am Rueckgabeweg aktiv, Erzwingung geparkt; die Aufteilung unter 2 GiB halten seit 2026-10-31 Bau-Zusicherungen beider Boards und `tos-api/tests/mem11_layout_below_2gib.rs` fest; tosdiff-Pruefung wartet auf tosdump-Format v3), [SDK01](../TASK/SDK01.md) (A–L erledigt bzw. umgesetzt, G gestrichen;
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
| 2026-10-31 | **[FS21](../TASK/FS21.md): `Fclose` einer geaenderten Datei setzt `FA_ARCHIVE` wie EmuTOS (Commit dieser Zeile).** Fund aus FS20: nach Fcreate/Fwrite/Fclose hatte der Eintrag auf dem Orakel `0x20`, auf NewTOS `0x00`. EmuTOS `ixclose` schreibt den Eintrag nur bei `O_DIRTY` neu und ODERt `FA_ARCHIVE` in das Attribut, das der Eintrag dann hat (`references/emutos/bdos/fsopnclo.c:530-550`); `O_DIRTY` setzen `Fcreate` (`:206`), ein Schreiben, das die Datei verlaengert oder ihr den ersten Cluster gibt (`references/emutos/bdos/fsio.c:42-46`, `references/emutos/bdos/fsfat.c:279-282`), und `Fdatime` setzen (`references/emutos/bdos/fsdir.c:698`); nur Oeffnen, Lesen oder Ueberschreiben innerhalb der Laenge laesst das Attribut stehen. Jetzt ODERt `flush_ofd` (`tos-api/src/bdos/fs/file.rs:736@flush_ofd`) das Bit ein, `fcreate` oeffnet sein OFD schmutzig, `fdatime` (setzen) markiert es. Abnahme raspi4b: FS20-Sonde `tosdump/probes/run-gcprobe.sh` Testpartition nach Nullen der Zeitstempel gleich dem Orakel (vorher Sektor 257 `0x04B` 20/00); Endzustaende `gemdos-mc-fs20` und `-fs24` jetzt gleich dem Orakel (vorher je das Attributbyte), `-fs22` gleich, `-paths` unveraendert (DELME.TXT, TOOL20 F3); tosdiff fs20 0, fs22 0, paths 1 (67), fs24 2 (76, 149), bios-r1 32/37 — alle wie vorher, keine neue Abweichung. Spec `docs/api/gemdos/file/3e-fclose.md` Randfall 8, Notizen in `3c-fcreate.md`, `57-fdatime.md`; `mapping.tsv` +1 (`ixclose` → `flush_ofd`); verschobener Anker `free_drive_files` nachgezogen. Hosttests 1918/0/1 (+8, `tos-api/tests/fs21_close_archive.rs`); rustc 93/64, Clippy 121/74 wie Basis. Beleg `measurements/measurement-2026-10-31-fs21/RUNINFO.TXT`. |
| 2026-10-31 | **[MEM11](../TASK/MEM11.md): Die Speicheraufteilung haelt jede Anwenderadresse unter 2 GiB — jetzt per Bau- und Hosttest festgehalten (Commit dieser Zeile).** Anwenderentscheidung 2026-10-31: die Erzwingung an der Abbildungsgrenze bleibt geparkt (heute nicht erreichbar), eine Aenderung der Aufteilung soll laut scheitern. Kern, zur Uebersetzungszeit auf beiden Boards: `TOS_MD_MAX_END <= USER_PTR_END` und die DRAM-Kappung von `discover`, jetzt `const fn` (`kernel/src/bios/machine/layout.rs:364@ttram_cap_end_for`, gleiche Rechnung), `<= USER_PTR_END` (`kernel/src/bios/machine/layout.rs:380@ttram_cap_end_for`) — damit liegen beide Pools und alles, was `usermem::expose` aus DRAM veroeffentlicht, unter 2 GiB; Pi-Bildschirmspeicher `BUS_TO_PHYS < USER_PTR_END` (`kernel/src/drivers/bcm2711/framebuffer.rs:72@BUS_TO_PHYS`). Host: `tos-api/tests/mem11_layout_below_2gib.rs` (Grenze = positiver `LONG`-Bereich; Seite 0, IOREC-/Stub-/private Seite und alle festen herausgegebenen Zeiger darunter; `bdos::mem::init` nimmt einen Pool bis `USER_PTR_END - 0x1000` an und lehnt einen bis `USER_PTR_END` ab). Gegenprobe: `TOS_MD_MAX_END = 0x8000_1000` → Kernbau scheitert auf rpi4 und qemu-virt (`evaluation panicked: MEM11: …`); `seed_md` ohne `checked_add` → Hosttest FAILED. Keine Laufzeitaenderung. Hosttests 1910/0/1 (+3); Bau beider Boards rustc 93/64, Clippy 121/74 wie Basis. Stand im Ticket. |
| 2026-10-31 | **[FS26](../TASK/FS26.md) Messung: "Neuer Ordner" in QEMU Schicht fuer Schicht (Commit dieser Zeile), kein Fix.** Desktop-Weg netto 8–11 ms; einziger Kartenzugriff ist `Dcreate` mit 19 Einzelsektor-Schreibungen (AUTO und GEM gleich, je 2–3 ms). EmuTOS (Hatari, gleiche Geometrie) braucht je `Dcreate` 19–24 Lese- plus 20 Schreibzugriffe: NewTOS greift nicht oefter zu. Die ~3 s auf der Pi sind damit in QEMU nicht erklaerbar; Kandidat (abgeleitet) ist die Schreiblatenz der Karte je Einzelsektor (FS04). Neues Debug-Feature `fs-io-trace` (`kernel/src/dbg_fsio.rs`), mit Feature aus byte-gleich bis auf `panic::Location`; Sonde `tosdump/probes/dcprobe.c`. Pi-Messwerte T1–T3 gehen ins Testszenario. Beleg `measurements/measurement-2026-10-31-fs26/RUNINFO.TXT`. |
| 2026-10-30 | **[FS28](../TASK/FS28.md): `Dsetpath` nach einer Neuanmeldung setzt den Pfad wie EmuTOS (Commit dieser Zeile).** Fund N1 aus FS24: nach einem Medienwechsel lieferte `Dsetpath(C:\SUB)` `E_OK`, `Dgetpath` danach die Wurzel (Orakel `\SUB`, `gemdos-mc-fs24` 87, 118, 150). Ursache per Hosttest belegt: `set_curdir` (`tos-api/src/bdos/fs/curdir.rs:229@set_curdir`) nahm zuerst den neuen Slot (den ersten freien) und pruefte erst danach das alte PD-Byte; nach `free_drive` nannte das veraltete Byte genau diesen Index, galt als lebendig, und `decr` gab den neuen Slot frei. Jetzt wird das alte Byte vor `incr` beurteilt, wie EmuTOS `xchdir` `olddir` vor `findit` liest (`references/emutos/bdos/fsdir.c:1129-1134`) und `ckdrv` das Byte auf einen Wurzel-Slot repariert (`references/emutos/bdos/fsdrive.c:121-130`); ein veraltetes Byte ist die Wurzel und gibt nichts frei. tosdump vorher/nachher (raspi4b): fs24 5 → 2 Abweichungen (76, 149 = FS24/ADR-033), fs20/fs22/paths/bios-r1 unveraendert, keine neue Abweichung; Hosttests 1907/0/1 (+2, `tos-api/tests/fs20_media_change.rs`). Spec `docs/api/gemdos/directory/3b-dsetpath.md`, `mapping.tsv` `xchdir` → `set_curdir`. Beleg `measurements/measurement-2026-10-30-fs28/RUNINFO.TXT`. |
| 2026-10-30 | **[TOOL29](../TASK/TOOL29.md): doc-check als git-pre-commit-Hook (Commit dieser Zeile).** Anwenderentscheidung 2026-10-30 nach `abc6276a` (ein Commit ging trotz doc-check-Fehler durch, weil die Befehlskette den Fehlercode verschluckte). `tools/git-hooks/pre-commit` ruft `cargo xtask doc-check --staged`; der Commit scheitert in `git commit` selbst, gleich wie die Befehlszeile verkettet ist. `--staged` liest den Index statt des Arbeitsbaums (Teil-Staging, `git commit -a`/`<pfad>` ueber `GIT_INDEX_FILE`; Zitatziele mit abweichender Arbeitskopie aus dem Index): es zaehlen nur Fehler, die der Commit einfuehrt (Anwenderentscheidung 2026-10-30, [SPEC03](../TASK/SPEC03.md)): in einer gestagten Doku (`CLAUDE.md`, `docs/**/*.md` ohne Archiv und datierte Schnappschuesse, `TASK/*.md`/`*.MD`, `RUNINFO.TXT`) ein Fehler auf einer neuen Zeile oder einer unveraenderten, deren Zitat bei `HEAD` aufloest; in jeder anderen Doku dieses Umfangs ein Zitat, das der Commit bricht (bei `HEAD` aufgeloest, im Index nicht: verschobener Anker, gekuerzte/geloeschte Datei, entfernte ADR). Altlasten werden gezaehlt, blockieren nie. Laufzeit je Commit in einem Wegwerf-Klon 0,56–0,89 s mit Doku oder Code, 0,08 s ohne (Vorfilter ohne cargo); die volle Variante ohne Zitat-Vorfilter 1,7–1,8 s bei gleichem Befund (`measurements/measurement-2026-10-30-tool29/RUNINFO.TXT`). Installation `cargo xtask install-hooks` (`core.hooksPath` = `tools/git-hooks`, verweigert das Verdecken anderer Hooks); Umgehen nur `git commit --no-verify`, fuer Agenten gesperrt durch `tools/hooks/no-verify-guard.sh`. Hosttests 1905/0/1 (+6). Clippy xtask unveraendert (1 Warnung bin, 8 test, keine in den beruehrten Dateien). Drei durch die eigene `main.rs`-Verschiebung gebrochene Anker nachgezogen (`docs/sdk/06-image-and-testing.md`, `TASK/MEM12.md`). |
| 2026-10-30 | **[A126](../TASK/A126.md): Die AES meldet den Tastenzustand nur mit Bits 0–3 wie EmuTOS (Commit dieser Zeile).** EmuTOS' AES hat eine Quelle: `gsx_kstate` → VDI `vq_key_s` → `gshift_s` = `Kbshift(-1) & 0x000f` (`references/emutos/aes/gemgsxif.c:432-436`, `references/emutos/vdi/vdi_input.c:110-113,183-187`); `chkkbd` legt den Wert in `kstate`, den `ev_rets` (`evnt_button`/`evnt_mouse`/`evnt_multi` `EV_KS`), `gr_mkstate` (`graf_mkstate`) und `handle_popup` (`mn_keystate`) melden. NewTOS las an diesen drei Stellen das ganze BIOS-Schaltbyte (CapsLock, seit BIOS10 auch Bits 5–7). Jetzt eine Quelle `tos-api/src/aes/gsxif.rs:286@gsx_kstate` (`kstate_of` fuer den Pop-up mit seinem Host), aus `fill_rets`, `graf_mkstate` und `popup_shown`; BIOS `Kbshift` und VDI unveraendert. Neue dauerhafte Sonde `tosdump/probes/akprobe.c` (CapsLock + Alt gehalten: `graf_mkstate`, `evnt_multi` MU_BUTTON mit rechter Taste, `evnt_button`, `evnt_multi` MU_KEYBD mit Alt+a; dann `Kbshift(0xf0)`/`Kbshift(0xff)` vom Programm): Orakel tt = falcontc; NewTOS vorher `0x18`/`0xf0`/`0xff`, nachher alle verglichenen Zeilen gleich dem Orakel, Eingabe per `bios-arg-trace` geprueft; `ksprobe` gleich dem BIOS10-Lauf. Ein tosdiff-Folgeticket fehlt noch (Lead). Specs `docs/api/aes/{event,window-graphics,menu,_state}.md`, `docs/api/bios/0b-kbshift.md` (offener Punkt erledigt); `mapping.tsv` +11 Zeilen (erste aes/vdi-Zeilen, `--down gsx_kstate` vollstaendig). Hosttests 1899/0/1 (+4); Bau beider Boards rustc 93/64, Clippy 121/74 wie Basis. Beleg `measurements/measurement-2026-10-30-a126/RUNINFO.TXT`. |
| 2026-10-30 | **[SDK08](../TASK/SDK08.md): Die Rebuild-Paritaet von `rt-host` und `ldbl-host` blendet nur die Compiler-Kennung `.comment` aus (Commit dieser Zeile).** Anwendervorgabe 2026-10-30. Ursache abschnittsweise bestaetigt: alle 1049 Archivteile (`libclang_rt.builtins.a` 151, `libm.a` 256, `libc.a` 642) mit dem neuen Host-clang `Apple clang 21.0.0 (clang-2100.4.1.1)` neu gebaut und als ELF verglichen — 1045 weichen nur in `.comment` ab (Inhalt und Groesse: ausgeliefert `clang-2100.3.34.2`), 4 Assembler-Teile byte-gleich, kein Code-, Daten-, Relokations- oder Symbolabschnitt verschieden; die Archive wurden nicht neu gebaut. `sdk/compiler-rt/tests/rt-host.sh` und `sdk/libc/tests/ldbl-host.sh` vergleichen jetzt mit `parity()`: byte-gleich, sonst nach `llvm-objcopy --remove-section=.comment` beider Seiten byte-gleich **und** gleiche Symboltabellen (das Entfernen nimmt das `$d`-Symbol in `.comment` mit; die Gegenprobe hatte dort ein invertiertes Byte uebersehen, vor dem Lauf behoben). Schritt 2 nennt den Host-clang und den clang der ausgelieferten Teile; eingebaute Gegenprobe: ein invertiertes Byte in `.text` einer Neubau-Kopie muss als Abweichung auffallen (beide: differ). Gegenprobe je Abschnitt an vier Teilen 30/30 wie erwartet (nur `.comment` ausgeblendet). `tools/sd-manifest.sh` schreibt eine Zeile `host_clang <clang --version, Zeile 1>` (Format v1 bleibt, Leser waehlen per Schluessel); `docs/sdk/06-image-and-testing.md`: RUNINFO.TXT nennt sie mit den `optional_missing`-Zeilen. SDK-Hosttests 33/2/0 → 35/0/0; Hosttests 1895/0/1 (= Basis, kein Rust-Code beruehrt). Beleg `measurements/measurement-2026-10-30-sdk08/RUNINFO.TXT`. |
| 2026-10-30 | **[COOK02](../TASK/COOK02.md) Punkt 3: `_IDT` ist `0x112E` wie das Orakel (Commit dieser Zeile).** Anwenderentscheidung 2026-10-30: NEWDESK nimmt bei der Datumseinstellung „Standard“ den Trenner aus `_IDT` (`programs/desktop/src/model.rs:194-201@rendered`, wie EmuTOS `references/emutos/desk/deskinf.c:140-145`); deshalb setzt `seed_defaults` jetzt `0x0000112E` (`.`, DD-MM-YY, 24 h; `tos-api/src/cookie.rs:226@IDT`) statt `0x0000112F` (`/`, „bring-up policy“). `_AKP` bleibt `0` (Fall 52 ACCEPTED). `jar-r1` Fall 53 jetzt EQUAL gegen tt, falcontc und disk (15 → 14 von 22 verschieden), kein anderer Fall veraendert; kein neu abweichender Fall in `bios-r1` (tt 32, falcontc 37 von 147) und `xbios-system-r1` (9/9 von 43), vorher `a8d875a7`. `docs/COMPATIBILITY.md` Fall 53 EQUAL; `docs/api/bios/_cookie-jar.md`, `docs/HARDWARE-PROFILE.md`, `docs/api/desktop/{deskinf,gembind}.md`, `docs/api/emucon/{04-builtins,05-config-strings-hardware}.md` nachgezogen. Hosttest `tos-api/tests/gemdos_ssystem_idt_cookie.rs` prueft `0x112E` und `.`; die Test-Keksdosen `sdk/cflib/tests/cookie-host.c`, `sdk/overlays/teradesk/tests/cookie-host.c` tragen den neuen Wert. Hosttests 1895/0/1 (= Basis); SDK-Hosttests 33/2/0: `rt-host` und `ldbl-host` scheitern nur an der Rebuild-Paritaet, weil der Host-clang jetzt `clang-2100.4.1.1` ist, die ausgelieferten Archive aber mit `clang-2100.3.34.2` gebaut sind (`.comment`), unabhaengig von COOK02; `cookie-host`, `td-cookie-host` PASS. Bau beider Boards rustc 93/64, Clippy 121/74 wie Basis. Beleg `measurements/measurement-2026-10-30-cook02-idt/RUNINFO.TXT`. |
| 2026-10-30 | **[XBIOS04](../TASK/XBIOS04.md): `Ikbdws` und `Bconout(4)` warnen einmal je Prozess „IKBD commands ignored“ (Commit dieser Zeile).** Anwenderentscheidung 2026-10-30, Option 1 der Researcher-Note `docs/analysis/xbios04-ikbdws-scope-2026-10-30.md`: kein IKBD-Interpreter, keine Antwortpakete; `Ikbdws` bleibt `void` (0), `Bconout(4,c)` 1, die Bytes verworfen. Neu in der ADR-031-Warnschlange (`tos-api/src/bdos/process/hooks.rs`): ein eigener Warnschluessel hinter den Haken (`note_ikbd_ignored`, `Kind::IkbdIgnored`), gemeinsam fuer beide Wege — einmal je Prozess, welcher Weg zuerst kommt; ein neuer Prozess, auch im selben Slot, warnt wieder. `docs/COMPATIBILITY.md` „IKBD commands are not interpreted“: Audit-Sonde W01–W04 ACCEPTED. Specs `docs/api/xbios/19-ikbdws.md` (veralteter Text „nicht verdrahtet, `EINVFN`“ und „PHASE-07 interpretiert“ berichtigt) und `docs/api/bios/03-bconout.md`; `mapping.tsv` +2 Zeilen (`ikbdws`, `bconout4` → `note_ikbd_ignored`). Kein neu abweichender Fall in `bios-r1`, `bios-core-r1`, `xbios-chardev-r1`, `xbios-system-r1` (tt/falcontc, vorher `9f71ded8`); `MDPROBE.LOG` byte-gleich mit dem BIOS04-Lauf, die Warnzeile steht genau einmal im seriellen Log. Hosttests 1895/0/1 (+1); Bau beider Boards rustc 93/64, Clippy 121/74 wie Basis. Beleg `measurements/measurement-2026-10-30-xbios04/RUNINFO.TXT`. |
| 2026-10-30 | **[BIOS09](../TASK/BIOS09.md): `Bconin(1)` und `Bconin(6)` warten auch auf der Pi wie EmuTOS' `bconin1` (Commit dieser Zeile).** Anwenderentscheidung Option (a), Audit-Fund BIOS-CORE-008: der AUX-Treiber von XBIOS15/XBIOS18 (`AuxSerial`, `aux_read_blocking`, `install_aux`) laeuft jetzt auf beiden Boards, `serial_rx_byte` legt jedes empfangene Byte auch auf rpi4 in den Eingangsring von `Iorec(0)` (vorher verworfen; `kernel/src/console.rs`). `Bconin(1)` — mit ihm `Bconin(6)`, `Cauxin` und `Fread` auf AUX — wartet auf ein Byte, mit oder ohne Kabel; waehrend des Wartens laeuft kein anderer Prozess, nur der `kbdvec`-Haken des wartenden (Neustart wie XBIOS18), Strg+Alt+Entf wirkt. Die Warteschleife steht jetzt in `tos-api/src/bios/chardev.rs` (`aux_read`, der Kern legt `wfi` darum) und ist auf dem Host getestet. FS11 Punkt 1a „documented divergence“ fuer `Bconin` widerrufen (kein COMPATIBILITY-Eintrag); `AuxNoCable` bleibt Tabellen-Vorgabe und Ausgabeseite. Keine Registeraenderung: ohne Adapter kann die offene RXD0-Leitung beim Start ein `0x00` einspielen, dann kehrt das erste `Bconin(1)` einmal mit 0 zurueck (Pi-Schritt in der MEM12-Prueffolge, Punkt 10, Sonde `tosdump/probes/awprobe.c`). Gemessen (QEMU raspi4b): Haengesonde `Bconin(1)`/`Bconin(6)` blockieren, BCHANG.LOG gleich dem Orakel tt; `awprobe` mit Injektor: 4 Haken-Aufrufe waehrend des Wartens, serielles `Q` beendet es, Strg+Alt+Entf startet neu; kein neu abweichender Fall in `bios-r1`, `bios-core-r1`, `xbios-chardev-r1`, `xbios-system-r1` (tt/falcontc, vorher `f065a705`). Specs `docs/api/bios/{01,02}`, `docs/api/gemdos/file/3f-fread.md` (1a), `docs/api/gemdos/console/{03,12}`, `docs/api/xbios/{0e,22,2c}`, `docs/COMPATIBILITY.md` (Iorec, AUX-Ausgabe); `mapping.tsv` 41 Zeilen nachgezogen (Anmerkungen „rpi4: AuxNoCable“/„rpi4 dropped“), +1 `aux_read`. Hosttests 1894/0/1 (+1); Warnungen unveraendert (rustc 93 kernel / 64 tos-api, clippy 121 / 74). Beleg `measurements/measurement-2026-10-30-bios09/RUNINFO.TXT`. |

## Notes

The project is intentionally over-documented at the start. The premise:
spec-first development against a fixed, finite, well-documented API
prevents the implementation drift that ruins long-running OS hobby
projects.
