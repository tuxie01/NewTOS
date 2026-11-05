# STATUS — NewTOS

> Single source of truth for project state. Updated with every milestone.
> Edit only via PR; do not edit in random places. Date everything.

## Current state

### Stand 2026-10-26

Zweig `main`, letztes Release-Tag `v0.7` (NewTOS 0.7 Alpha, `d7711c42`, veroeffentlicht 2026-11-04); davor `v0.6.1` (NewTOS 0.6.1 Alpha, `d5e9424c`,
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
| Hosttests (`cargo xtask test`) | **1974 / 0 / 1** (gemessen 2026-11-04 mit PROC08 Schirmbericht, +6 Tests; davor 1929 mit PROC08 `crash-walk`, +7 Tests; davor 1922 mit XBIOS12, +4 Tests; davor 1918 mit FS21, +8 Tests; davor 1910 mit MEM11, +3 Tests; davor 1907 mit FS28, +2 Tests; davor 1905 mit TOOL29, +6 xtask-Tests; davor 1899 mit A126, +4 Tests; davor 1895 mit SDK08, kein neuer Test; davor 1895 mit COOK02 `_IDT`, kein neuer Test; davor 1895 mit XBIOS04, +1 Test; davor 1894 mit BIOS09, +1 Test; davor 1893 mit BIOS17, +1 Test; davor 1892 mit BIOS16, +4 Tests; davor 1888 mit BIOS08, +7 Tests; davor 1881 mit XBIOS05, +9 Tests, Basis laut Lead 1872; davor 1859 mit BIOS07, +9 Tests; davor 1850 mit FS23, +3 Tests; davor 1847 mit BIOS04, +4 Tests; davor 1843 mit XBIOS02, +1 Test; davor 1842 mit BIOS06, +2 Tests; davor 1840 mit BIOS01 Rest, +7 Tests; davor 1833 mit TOOL19, +5 Tests; davor 1828 mit ARCH06, +4 Tests; davor 1824 mit XBIOS18, +1 Test; davor 1823 mit TOOL21, +2 Tests; davor 1821 mit TOOL17, +5 Tests; davor 1816 mit XBIOS15, +8 Tests; davor 1808 mit BIOS15, +18 Tests; davor 1790 mit FS22, +7 Tests; davor 1783 mit XBIOS01, +16 Tests; davor 1767 mit FS20, +7 Tests; davor 1760 mit BIOS01 S2, +9 Tests; davor 1751 mit TOOL07 Rest, +3 xtask-Tests; davor 1748 mit TOOL12, +3 deps-Tests; davor 1745 mit TOOL10; TOOL09 +16, TOOL10 +3 xtask-Tests; vorher 1726 / 0 / 1 auf `75527af8`, `--fast` ohne den einen Doctest) |
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
  [FS21](../TASK/FS21.md) (P2, Archivbit beim `Fclose`; umgesetzt 2026-10-31, Commit dieser Zeile). Wartet auf Anwender: [BIOS12](../TASK/BIOS12.md) erledigt 2026-10-31 (Quittung). [XBIOS12](../TASK/XBIOS12.md) umgesetzt 2026-10-31 (ohne RTC startet die Uhr beim Baudatum des Kerns, 00:00:00, wie EmuTOS `DEFAULT_DATETIME`, bis RTC01; Commit dieser Zeile). Geparkt: [TICK01](../TASK/TICK01.md) (bis Tester K6 aus DOOM01 sehen).
  [PITEST01](../TASK/PITEST01.md) (P2, Testkarte und Ablauf fuer die Tester an der echten Pi; **erledigt 2026-11-02**, Quittung; HypView-Startseite = Einstellung in `HYP_VIEW.CFG`, zView-Fensterschliessen vom Anwender nicht nachvollziehbar, kein Ticket): Teil 1 umgesetzt 2026-10-31 (Debug-Feature `crash-nowait`, `P0REP.NAP`, `HZPROBE.NAP`), Teil 2 umgesetzt 2026-10-31 (Kartenabgleich `tools/sync-card.sh`, Ablaufblatt `docs/analysis/tester-run-sheet-2026-10-31.md`), Teil 3 umgesetzt 2026-10-31 (eine Karte; Sicherung von `kernel8.img`, `cmdline.txt`, `config.txt` und jeder ersetzten Datei vor dem Schreiben; HypView mit `HYP_VIEW.CFG` und `ZVIEW.HYP`; `cmdline.txt` wechselt mit dem Kern per `--cmdline`, Phase A ohne, Phase B mit `serial=on`; Liste und Befehl `measurements/measurement-2026-10-31-pitest01-part3/RUNINFO.TXT`, PITEST01 "Stand Teil 3"); Karte gefuellt und Pi-Laeufe A, B, C1 gelaufen 2026-10-31 (`measurements/measurement-2026-10-31-pitest01-phase-a/RUNINFO.TXT`, `measurements/measurement-2026-10-31-pitest01-phase-b/RUNINFO.TXT`, `measurements/measurement-2026-10-31-pitest01-c1/RUNINFO.TXT`); Funde: [DOOM01](../TASK/DOOM01.md), [PROC08](../TASK/PROC08.md) (neu, P1), zView-Fensterschliessen (ohne Ticket); Kartenstand in PITEST01 "Stand nach den Pi-Laeufen".
  [PROC08](../TASK/PROC08.md) (P1, neu 2026-10-31, Anwender: Ticket anlegen): Rueckkehr aus `Pexec(4)` fand die Code-Seite des Elternprozesses nicht (zView auf der Pi, einmalig); Ursachenforschung zuerst. **Naechstes Ticket (Anwender 2026-11-02)**, vor DOOM01. Ursachenforschung 2026-11-02 ohne Beleg (`docs/analysis/proc08-root-cause-2026-11-02.md`); Debug-Feature `crash-walk` gebaut (`371c50fd`); Pi-Lauf 2026-11-04 mit `serial=on memprot=verify` ohne Wiederauftreten (`measurements/measurement-2026-11-02-proc08-pi/RUNINFO.TXT`). **Wartet auf Wiederauftreten** (Anwender 2026-11-04): `crash-walk` schreibt seine Diagnose jetzt auch in den Absturzbericht auf dem Schirm (Commit dieser Zeile); die Karte faehrt diesen Kern im Alltag mit leerer `cmdline.txt`, bei einem Absturz macht der Anwender ein Foto. Danach Prioritaeten gemeinsam durchgehen (Anwender 2026-11-04), dann weiter in der Warteschlange.
  **[REL07](../TASK/REL07.md) Release 0.7** (**veroeffentlicht 2026-11-04**: Tag `v0.7` auf `d7711c42`, GitHub-Pre-Release mit `NewTOS-0.7.zip` sha256 `a5191394…`; Pi-Abnahme des Kerns 2026-11-04; Tester informiert (#3, #8), #9 geschlossen, REL07 erledigt; Paket mit `tools/release/package-release.sh`, `measurements/measurement-2026-11-04-rel07-final/RUNINFO.TXT`; Schritt 1 Version aus einer Quelle `797e0078`; Anwender 2026-11-04; nach der PROC08-Leiter, mit `crash-walk`, Abnahme auf der Pi des Anwenders, dann Ragnar informieren). **PROC08 wieder aufgetreten 2026-11-04** (Foto, `measurements/measurement-2026-11-04-proc08-recur/RUNINFO.TXT`): Schirmbericht `model=valid table=valid at=fault` = Kandidat B; Mechanismus-Forschung `docs/analysis/proc08-candidate-b-2026-11-04.md` (Fenster gefunden, Ausloeser nicht); **zurueckgestellt "to be tested"** (Anwender: 60 zView-Starts mit Fix ohne Absturz, vorher mehrmals hintereinander); Fix umgesetzt 2026-11-04 (`01eb7709`), Tester PASS (`measurements/measurement-2026-11-04-proc08-fix-test/RUNINFO.TXT`), Pi-Abnahme offen; Leiter im Schirmbericht umgesetzt 2026-11-04 (Commit dieser Zeile, `measurements/measurement-2026-11-04-proc08-ladder/RUNINFO.TXT`). **Reihenfolge nach der Prioritaetenrunde (Anwender 2026-11-04):** [A127](../TASK/A127.md) (Ursachenforschung 2026-11-04 ohne Nachstellung, `docs/analysis/a127-root-cause-2026-11-04.md`; D1 umgesetzt 2026-11-04, `bae52dbd`, Tester PASS `measurements/measurement-2026-11-04-a127-d1-test/RUNINFO.TXT`; Folge: Klick-Auswahl im Menue wirkt nach dem Doppelklick-Fenster, am Orakel gemessen EmuTOS-gleich 2026-11-05 `measurements/measurement-2026-11-04-a127-oracle/RUNINFO.TXT`; Pi des Anwenders 2026-11-05 ok (`measurements/measurement-2026-11-05-a127-d1-card/RUNINFO.TXT`); D2/D3 offen bis Ragnars Rueckmeldung) → [A128](../TASK/A128.md) (Dateiauswahl: Schieber wie EmuTOS, Rad als Ergaenzung; GitHub #9; umgesetzt 2026-11-04, `df2c7501`; Tester PASS 2026-11-04, `measurements/measurement-2026-11-04-a128-test/RUNINFO.TXT`; offen echte Pi im Testszenario; Folgeticket [TOOL31](../TASK/TOOL31.md), P3, geparkt) → [A129](../TASK/A129.md) (erledigt 2026-11-04: Pixel EmuTOS-gleich, kein Fehler) → [A132](../TASK/A132.md) (umgesetzt 2026-11-04, `be3c06a9`, Tester PASS; Maske im Titel der Dateiauswahl) → [A133](../TASK/A133.md) (umgesetzt 2026-11-04, `fd6c7e3f`, Tester PASS zusammen mit A132 `measurements/measurement-2026-11-04-a133-test/RUNINFO.TXT`; TEDINFO-Text im Schreibmodus des Farbworts) → [A135](../TASK/A135.md) (neu, P3: `G_BOXCHAR` ebenso) → [A134](../TASK/A134.md) (neu, P3: Laufwerksklick/Fehlerzweig der Dateiauswahl) → [USB03](../TASK/USB03.md) (geparkt 2026-11-05 bis Ragnars Rueckmeldung zu #19 und zum USB-Stand; Anwender: baut teils aufeinander auf) → [ARCH03](../TASK/ARCH03.md)-Rest; dann EmuTOS-Konformitaet (A119–A125, DESK40/49/51–55/61/63/67–69, FS12, FS15–FS17, INPUT02, RST02) → Ton fuer 1.0 (SND01, SND02, H04). Daneben: die nur auf der Pi offenen Abnahmen (A16, A37, DESK34, DESK35, HW03, R02–R05, SER01, V11, V12, DESK43, BIOS09, FS26 T1–T3, >15 USB-Befehle) als ein Testszenario fuer die Tester; die „ERLEDIGT“-Tickets ohne Quittung einmal gezielt durchsehen. Ausbau nach 1.0 und Werkzeug-Tickets bleiben liegen. Aus der Issue-Durchsicht 2026-11-04 ans Ende: [A130](../TASK/A130.md) (#2 Fenster-Rollbalken, P2), [A131](../TASK/A131.md) (#7 Fensterschatten, P3), [DESK70](../TASK/DESK70.md) (#12 Textprogramm-Ausgabe, erst EmuTOS pruefen, P3).
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
  [DOOM01](../TASK/DOOM01.md) (DOOM-Freeze im Menue, nur auf der Pi) — erneut zurueckgestellt 2026-11-02 (Anwender: erst PROC08); Diagnosestand 2026-10-31: Freeze nur mit dem Accessory X-Shot (wartet ohne Timer auf `MU_KEYBD`) und nur ohne `serial=on`; Ursache im Kern, offen; naechster Schritt in DOOM01 "Stand 2026-11-02".
  Daneben ein Researcher zur Zeit, ohne Produktaenderung: — (frei).
- **P1:** [MEM12](../TASK/MEM12.md) — erledigt 2026-10-30 (Anwender: Pi-Lauf 2026-10-29, Kern `58e35ac8`); `P0PAR` ist erster Schritt in [DOOM01](../TASK/DOOM01.md) und im spaeteren Testszenario.
  [USB01](../TASK/USB01.md) — erledigt 2026-11-04 (Quittung; F1/F2/F3/F8 und
  USBSNAP auf echter Hardware bestaetigt). Rest: [USB03](../TASK/USB03.md)
  (Report-only-Maeuse, Rad der B100, Taste 3; P1 seit 2026-11-04),
  [A127](../TASK/A127.md) (neu: Menueeintraege wirken erst nach mehreren
  Klicks, Markierung bleibt schwarz; Ragnar in #3; Ursache zuerst), mehr als
  15 Befehle am VL805 ins Testszenario, [USB04](../TASK/USB04.md) (P3).
  [ARCH03](../TASK/ARCH03.md) Rest (siehe oben). [A45](../TASK/A45.md) ist
  blockiert (siehe *Blocked*).
- **P1, umgesetzt, nur Pi- bzw. Anwendungsabnahme offen** (laut Statuszeile):
  A16, A37 (anwendungsseitig), DESK34, DESK35, HW03 (drei Pi-Gegenproben),
  R02–R05, SER01, V11, V12, DESK43 (Pi-Gegenprobe des EmuCON-Rueckfalls bei
  defektem NEWDESK).
- **P2:** [MEM11](../TASK/MEM11.md) (Anwenderadressen nur zufaellig < 2 GiB;
  Debug-Pruefung am Rueckgabeweg aktiv, Erzwingung geparkt; die Aufteilung unter 2 GiB halten seit 2026-10-31 Bau-Zusicherungen beider Boards und `tos-api/tests/mem11_layout_below_2gib.rs` fest; tosdiff-Pruefung wartet auf tosdump-Format v3), [SDK01](../TASK/SDK01.md) (A–L erledigt bzw. umgesetzt, G gestrichen;
  offen nur die Abnahme von K unter Linux/Homebrew),
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
  CON03, ARCH04, USB02, [USB04](../TASK/USB04.md), MEM08, MEM10, [KEYS03](../TASK/KEYS03.md),
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
| 16 | USB-HID Input (xHCI + VL805) | laeuft auf echter Pi 4; USB01 erledigt 2026-11-04; offen USB03 (P1), USB02 (P3), USB04 (P3) | 2026-11-04 | `phases/PHASE-16-usb-hid-input.md`, [USB03](../TASK/USB03.md) |

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
| 2026-11-05 | **[A127](../TASK/A127.md) D1 gegen das Orakel gemessen: konform (Commit dieser Zeile).** Die Klick-Verzoegerung im Menue (Tester: 40–42 `_hz_200`-Ticks) ist EmuTOS' Doppelklick-Fenster: gemessen mit dem neuen Probe `tosdump/probes/mnprobe.c` auf EmuTOS 1a7484ab/Hatari 2.6.1, head `bae52dbd` und base `4d44a616`; Klick in der Desktop-Warte EmuTOS 48–52 / head 48–52 / base 0 Ticks (Doppelklick-Index 2), 40–44 / 40–44 / 0 (Index 3); Ziehen ueberall 0; base kehrt nach der Auswahl 175–179-mal mit `MU_BUTTON` zurueck, EmuTOS und head einmal. Spec `docs/api/aes/menu.md` Punkt 9 praezisiert (`gl_bpend` bleibt nach einer durch Nachricht beendeten Mehrfachklick-Warte stehen, A127-ORC-001). `measurements/measurement-2026-11-04-a127-oracle/RUNINFO.TXT`, `docs/audits/2026-11-05-a127-menu-click-latency.md`. |
| 2026-11-04 | **[A127](../TASK/A127.md) D1: Die Menueverfolgung nimmt die linke Taste als Ereignis wie EmuTOS `mn_do` (Commit dieser Zeile).** Anwenderentscheidung 2026-11-04 (EmuTOS-konform angleichen). Vorher tastete `menu::track` den Tastenpegel je Durchlauf ab, ein Druecken+Loslassen im selben Durchlauf ging verloren (A127 Lauf C: drei Klicks ohne Wirkung). Jetzt je Runde eine Warte wie `ev_multi(…, buparm, …)`, erfuellt von `post_button` mit der ersten passenden Flanke (`gemevlib.c:185-198,238-239`, `geminput.c:344,463-483`); Spec `docs/api/aes/menu.md` Punkt 9 ergaenzt, Punkt 3 berichtigt. Folge wie EmuTOS: bei offener Mehrfachklick-Warte wirkt ein Menueklick erst nach dem Doppelklick-Fenster. QEMU Lauf C: erster Klick wirkt; Laeufe A/B/E/F unveraendert. `aes_a73`-Fall 1a haelt den Druck jetzt ueber das Fenster (sonst reihenfolgeabhaengig). Hosttests 1974/0/1; `measurements/measurement-2026-11-04-a127-d1/RUNINFO.TXT`. Ob Ragnars Menuebefund damit weg ist, zeigt erst seine Rueckmeldung. |
| 2026-11-04 | **[A133](../TASK/A133.md): TEDINFO-Text (`G_TEXT`, `G_BOXTEXT`, `G_FTEXT`, `G_FBOXTEXT`) im Schreibmodus des Farbworts wie EmuTOS (Commit dieser Zeile).** Bit 7 waehlt `MD_REPLACE` (Zeichenzellen in Stift 0), sonst transparent; 3D-hohl-weiss und `SELECTED` mit `changecol` schalten auf transparent wie `gemoblib.c:586-588,702-706`. Vorher immer transparent; Spec `docs/api/aes/object-form.md` ergaenzt (Kasten A133). Pixel: Dateibox gegen das Orakel 0 Abweichungen (A132-Rest 140), eigener TEDINFO-Probedialog 0 (vorher 1916), Desktop-Startbild unveraendert. Hosttests 1970/0/1; `measurements/measurement-2026-11-04-a133/RUNINFO.TXT`. `G_BOXCHAR` folgt als [A135](../TASK/A135.md). |
| 2026-11-04 | **[A132](../TASK/A132.md): Die Dateiauswahl zeigt die Maske im Titel ueber der Liste wie EmuTOS (Commit dieser Zeile).** Gefunden bei der A129-Messung (einzige Pixelabweichung der Dateibox gegen das Orakel). Spec ergaenzt aus `gemfslib.c` (`set_mask`, `fs_pspec`, Maske bei Oeffnen, Neulisten, Ordner/Schliesser, Laufwerkswechsel); fuer Aufrufer sichtbar: Pfad mit `\` am Ende kommt mit `*.*` zurueck, `C:*.C` wird `C:\*.C`, Laufwerkswechsel behaelt die Maske (vorher `*.*`, Ordnerwechsel kappte sie auf 15 Zeichen). Pixelvergleich: Glyphen gleich, 140 Restpixel durch transparenten Text statt `MD_REPLACE` -> [A133](../TASK/A133.md); weitere Lesebefunde -> [A134](../TASK/A134.md). Hosttests 1965/0/1; `measurements/measurement-2026-11-04-a132/RUNINFO.TXT`. Auch: [A129](../TASK/A129.md) geschlossen, Kanten EmuTOS-gleich. |
| 2026-11-04 | **[REL07](../TASK/REL07.md): NewTOS 0.7 (Alpha) veroeffentlicht (Tag `v0.7` auf `d7711c42`, GitHub-Pre-Release, `NewTOS-0.7.zip` sha256 `a51913942d7c8f10427e5cf5e8ce001fc4514f744a98265457e5245a8ecbbbba`).** Anwenderentscheidungen 2026-11-04: Version 0.7, Kern mit `crash-walk` samt Leiter, Ball entfaellt, Rueckmeldung nur ueber GitHub Issues, vor dem Push Abnahme auf der Pi (*„soweit passt alles, die Version wird auch richtig reported“*, `docs/HW-TEST-LOG.md`). Paket erstmals mit `tools/release/package-release.sh` (alles aus sauberem Stand neu gebaut, feste Dateitabelle, kein `TODO-REL` in den Texten, Namensliste gegen 0.6.1); hochgeladenes ZIP nach dem Herunterladen pruefsummengleich. Texte `docs/release/v0.7/`, Faktenliste `docs/analysis/rel07-changes-2026-11-04.md`. |
| 2026-11-04 | **[REL07](../TASK/REL07.md) Schritt 1: Produktversion aus einer Quelle, jetzt 0.7 (Commit dieser Zeile).** Anwenderentscheidung 2026-11-04 ("mach es richtig"): `[workspace.package] version` in der Wurzel-`Cargo.toml`, kernel/tos-api/tos-types erben sie; `VERSION_STRING` daraus zur Bauzeit (Anzeige wie EmuTOS `version.mk`: `x.y` bei Patch 0, sonst `x.y.z`; Pre-Release-Suffix bricht den Bau ab); `ver` nutzt sie; der Desktop liest die Version zur Laufzeit ueber `Ssystem(2019)` statt seiner Kopie, bei Fehlschlag bleibt der Wert leer wie EmuTOS `inf_disk` beim Volume-Label. TOS-sichtbare Versionen unveraendert. QEMU: Bootschirm, About und `ver` zeigen "NewTOS 0.7"; Hosttests 1958/0/1; `measurements/measurement-2026-11-04-rel07-version/RUNINFO.TXT`. |
| 2026-11-04 | **[PROC08](../TASK/PROC08.md): `crash-walk` bekommt die Leiter (Commit dieser Zeile).** Anwenderentscheidung 2026-11-04, eigener Auftrag nach dem Fix: Schlaegt der erste `AT S1E0R` fehl, versucht der Absturzbehandler nacheinander zweiten `AT`, `AT` auf `SP_EL0`- und Privatseite, `dsb`, `dc civac` der Deskriptoren, `tlbi vaale1is`, `vaae1is`, `vmalle1is` und fragt nach jedem Schritt die MMU neu; eine Schirm- und eine serielle Zeile `ladder: … => fixed-by <Schritt>`/`repeat`/`none` trennen die Kandidaten R1–R4 (`docs/analysis/proc08-candidate-b-2026-11-04.md`, Lesetabelle in PROC08 "Stand Schirmbericht"). Ohne Feature bytegleich mit `233e404e` auf beiden Boards (release und dev); QEMU zeigt `=> none` bzw. `not run, AT ok` (kein Walk-Cache); Hosttests 1958/0/1. |
| 2026-11-04 | **[PROC08](../TASK/PROC08.md): Kindtabellen erst nach dem Umschalten auf die Boot-Tabellen freigeben, `isb` zwischen `msr ttbr0_el1` und `tlbi` (Commit dieser Zeile).** Anwenderentscheidung 2026-11-04 nach dem Foto des Schirmberichts (`measurements/measurement-2026-11-04-proc08-recur/RUNINFO.TXT`: `model=valid table=valid at=fault` = Kandidat B) und der Mechanismus-Forschung (`docs/analysis/proc08-candidate-b-2026-11-04.md`): `finish_terminate` gab die Tabellen des Kindes frei, waehrend `TTBR0_EL1` noch dessen Wurzel nannte, und `restore_boot_ttbr0`/`switch_ttbr0_kernel`/der Stage-2-Boot liessen das `isb` vor dem `tlbi` weg (ARM ARM `IRBMTM`, `IBLDZX`/`ICPZSM`; A72-TRM 5.6). Ausloeser des Nachladens nicht gefunden; QEMU bildet den Walk-Cache nicht nach, Abnahme nur auf der Pi. QEMU raspi4b 820 Modulstarts mit `memprot=verify` (auch `asid=on`), qemu-virt Boot: kein Absturz, kein Haenger; Hosttests 1952/0/1; `measurements/measurement-2026-11-04-proc08-fix/RUNINFO.TXT`. QUIRK-001-Kommentar berichtigt (das `kprintln!` fehlt seit `c31f4637`). |
| 2026-11-04 | **[A128](../TASK/A128.md): Schieber der Dateiauswahl wie EmuTOS, Mausrad nur dort als NewTOS-Ergaenzung, `graf_slidebox` wie EmuTOS (Commit dieser Zeile).** Anwenderentscheidungen 2026-11-04 (Spec `docs/api/aes/resource-fileselector.md` "A128"): Klick in die Leiste blaettert 9 Zeilen, Ziehen ueber `gr_slidebox` mit XOR-Rahmen, 3D-Angleichung und LP64-Baumzugriff; Rad 1 Zeile je Raststufe, nur ueber Liste und Leiste, kein `WM_ARROWED` an ein Fenster dahinter (`docs/COMPATIBILITY.md`); Daumen mit `mul_div_round`, Auswahl faellt beim Blaettern weg. Korrektur des freigegebenen Specs bei der Umsetzung: `ob_actxywh` vergroessert 3D-Objekte (`references/emutos/aes/gemoblib.c:313-336`), der Daumenrand gehoert zum Daumen. Sonde `tosdump/probes/sbprobe.c` 13/13 gleich dem Orakel; Hosttests 1952/0/1; QEMU `measurements/measurement-2026-11-04-a128/RUNINFO.TXT`. Folgeticket [TOOL31](../TASK/TOOL31.md). |
| 2026-11-04 | **[USB01](../TASK/USB01.md) geschlossen, Reste verteilt (Commit dieser Zeile).** Anwenderentscheidung 2026-11-04 nach dem Abgleich von Ticket, STATUS und GitHub #3/#8: F1/F2/F3/F8 und USBSNAP sind auf echter Hardware bestaetigt (eigene Pi 2026-10-23/24, Ragnar 2026-10-25, `measurements/measurement-2026-10-25-usb01-ragnar2/RUNINFO.TXT`); die zwei Rueckfragen hatte Ragnar am 2026-10-26 beantwortet (Rad der DW 9000 geht; Menue haengt, Markierung bleibt schwarz). Reste: [USB03](../TASK/USB03.md) auf P1 (Report-only-Maeuse, dazu Taste 3), neues [A127](../TASK/A127.md) (P1, AES-Menuefehler aus #3, Ursache zuerst), neues Sammelticket [USB04](../TASK/USB04.md) (P3: F5-Rest, Hotplug, absolute Zeiger, Port-Reset), mehr als 15 Befehle am VL805 (Code seit `7e7fbafa`, gemessen hoechstens 13) ins Testszenario. |

## Notes

The project is intentionally over-documented at the start. The premise:
spec-first development against a fixed, finite, well-documented API
prevents the implementation drift that ruins long-running OS hobby
projects.
