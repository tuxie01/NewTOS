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
| Hosttests (`cargo xtask test`) | **2239 / 0 / 1** (gemessen 2026-11-06 mit HW08 Fix, Developer, +22 Tests; davor 2217 am 2026-11-06 mit FS31, Developer, +17 Tests; davor 2200 am 2026-11-06 mit HW08 Punkte 1+2, Developer, +5 Tests; davor 2195 am 2026-11-06 mit V44, Developer, +5 Tests; davor 2190 am 2026-11-06 mit V33, Developer, +30 Tests; davor 2160 am 2026-11-06 mit V30, Developer, +40 Tests; davor 2120 am 2026-11-06 mit V32, Developer, +4 Tests; davor 2116 am 2026-11-06 mit V29, Developer, +20 Tests; davor 2096 am 2026-11-06 mit V28, Developer, +30 Tests; davor 2066 am 2026-11-06 mit V25 laut Recent Decisions; davor 2033 am 2026-11-05 mit V20, Developer, +20 Tests; davor 2013 am 2026-11-05 mit V26, Developer, +9 Tests; davor 2004 am 2026-11-05 mit V19, Tester; davor 1974 am 2026-11-04 mit PROC08 Schirmbericht, +6 Tests; davor 1929 mit PROC08 `crash-walk`, +7 Tests; davor 1922 mit XBIOS12, +4 Tests; davor 1918 mit FS21, +8 Tests; davor 1910 mit MEM11, +3 Tests; davor 1907 mit FS28, +2 Tests; davor 1905 mit TOOL29, +6 xtask-Tests; davor 1899 mit A126, +4 Tests; davor 1895 mit SDK08, kein neuer Test; davor 1895 mit COOK02 `_IDT`, kein neuer Test; davor 1895 mit XBIOS04, +1 Test; davor 1894 mit BIOS09, +1 Test; davor 1893 mit BIOS17, +1 Test; davor 1892 mit BIOS16, +4 Tests; davor 1888 mit BIOS08, +7 Tests; davor 1881 mit XBIOS05, +9 Tests, Basis laut Lead 1872; davor 1859 mit BIOS07, +9 Tests; davor 1850 mit FS23, +3 Tests; davor 1847 mit BIOS04, +4 Tests; davor 1843 mit XBIOS02, +1 Test; davor 1842 mit BIOS06, +2 Tests; davor 1840 mit BIOS01 Rest, +7 Tests; davor 1833 mit TOOL19, +5 Tests; davor 1828 mit ARCH06, +4 Tests; davor 1824 mit XBIOS18, +1 Test; davor 1823 mit TOOL21, +2 Tests; davor 1821 mit TOOL17, +5 Tests; davor 1816 mit XBIOS15, +8 Tests; davor 1808 mit BIOS15, +18 Tests; davor 1790 mit FS22, +7 Tests; davor 1783 mit XBIOS01, +16 Tests; davor 1767 mit FS20, +7 Tests; davor 1760 mit BIOS01 S2, +9 Tests; davor 1751 mit TOOL07 Rest, +3 xtask-Tests; davor 1748 mit TOOL12, +3 deps-Tests; davor 1745 mit TOOL10; TOOL09 +16, TOOL10 +3 xtask-Tests; vorher 1726 / 0 / 1 auf `75527af8`, `--fast` ohne den einen Doctest) |
| SDK-Hosttests (`sdk/run-host-tests.sh`) | **38 / 0 / 0** (gemessen 2026-11-07 mit A142, Developer, +2 `inline-rsh-host`, `hypview-prepare-host`; davor 36 am 2026-11-07 mit A140, Developer, +1 `hypview-win-host`; davor 35 am 2026-10-30 mit SDK08, kein neuer Test: Rebuild-Paritaet von `rt-host`/`ldbl-host` blendet nur `.comment` aus, Host-clang `clang-2100.4.1.1`; dazwischen 33/2/0 nach dem clang-Update; davor 35 mit XBIOS05, +1 `bconmap-layout`; davor 34 mit SDK07, +1 `td-nohard-host`; davor 33 mit SDK04, +3 `sysvars-layout`, `hypview-ref-host`, `zview-color-host`; davor 30 mit SDK04 C6, +1 `td-cookie-host`; davor 29 mit SDK05, +1 `cookie-host`; davor 28 mit SDK03, +1 `sysinfo-host`; davor 27 mit XBIOS01, +1 `iorec-layout`) |
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
  **[REL07](../TASK/REL07.md) Release 0.7** (**veroeffentlicht 2026-11-04**: Tag `v0.7` auf `d7711c42`, GitHub-Pre-Release mit `NewTOS-0.7.zip` sha256 `a5191394…`; Pi-Abnahme des Kerns 2026-11-04; Tester informiert (#3, #8), #9 geschlossen, REL07 erledigt; Paket mit `tools/release/package-release.sh`, `measurements/measurement-2026-11-04-rel07-final/RUNINFO.TXT`; Schritt 1 Version aus einer Quelle `797e0078`; Anwender 2026-11-04; nach der PROC08-Leiter, mit `crash-walk`, Abnahme auf der Pi des Anwenders, dann Ragnar informieren). **PROC08 wieder aufgetreten 2026-11-04** (Foto, `measurements/measurement-2026-11-04-proc08-recur/RUNINFO.TXT`): Schirmbericht `model=valid table=valid at=fault` = Kandidat B; Mechanismus-Forschung `docs/analysis/proc08-candidate-b-2026-11-04.md` (Fenster gefunden, Ausloeser nicht); **zurueckgestellt "to be tested"** (Anwender: 60 zView-Starts mit Fix ohne Absturz, vorher mehrmals hintereinander); Fix umgesetzt 2026-11-04 (`01eb7709`), Tester PASS (`measurements/measurement-2026-11-04-proc08-fix-test/RUNINFO.TXT`), Pi-Abnahme offen; Leiter im Schirmbericht umgesetzt 2026-11-04 (Commit dieser Zeile, `measurements/measurement-2026-11-04-proc08-ladder/RUNINFO.TXT`). **Zuerst (Anwender 2026-11-05):** [V18](../TASK/V18.md) (die 26 VDI-Kandidaten aus TOOL32 am Orakel messen, drei Teile, dann Fix-Tickets je Befund; Teil 1 gemessen 2026-11-05 `2e22799b`: 9 Befunde, Fix-Tickets [V19](../TASK/V19.md) (erledigt 2026-11-05, `506643fb`, Tester PASS `measurements/measurement-2026-11-05-v19-test/RUNINFO.TXT`) → [V26](../TASK/V26.md) (Nebenbefund V19, Speicherweg griff in ungepruefte Zeile, P2; erledigt 2026-11-05, `1c84f4d2`+`87f1684e`, Anwenderentscheidung a: EmuTOS-Geometrie, Schnitt nur hinter `fd_h`; Tester PASS `measurements/measurement-2026-11-05-v26-test/RUNINFO.TXT`) → [V20](../TASK/V20.md) (`abline`, erledigt 2026-11-05, `bec04bab`; Anwenderentscheidung A: Umlauf nur im logischen Schirm; Tester PASS `measurements/measurement-2026-11-05-v20-test/RUNINFO.TXT`, AES/Desktop-Szenen pixelgleich; Pi: XOR-Rahmen am Schirmrand ins Testszenario) → [V21](../TASK/V21.md) (erledigt 2026-11-06, `497773f9`, Saat-Warteschlange wie EmuTOS; Tester PASS mit Ueberlauf-Fall `measurements/measurement-2026-11-06-v21-test/RUNINFO.TXT`; Notbremse offen beim Anwender) → [V22](../TASK/V22.md) (erledigt 2026-11-06, `7108bdff`, Tester PASS `measurements/measurement-2026-11-06-v22-test/RUNINFO.TXT`) → [V23](../TASK/V23.md) (erledigt 2026-11-06, `ee67903f`, Kappung zentral in `dispatch_in`; Tester PASS `measurements/measurement-2026-11-06-v23-test/RUNINFO.TXT`; Folgeticket [TOOL34](../TASK/TOOL34.md) geparkt) → [V24](../TASK/V24.md) (erledigt 2026-11-06, `caf77fab`, Anwenderentscheidung B; Tester PASS `measurements/measurement-2026-11-06-v24-test/RUNINFO.TXT`) → [V25](../TASK/V25.md) (erledigt 2026-11-06, `294272e2`, frueher Ausstieg von `curve`; Tester PASS `measurements/measurement-2026-11-06-v25-test/RUNINFO.TXT`; Teil 1 damit bis auf Entscheidung C orakelgleich) → V18 Teil 2 gemessen 2026-11-06 `eba4122a`: [V28](../TASK/V28.md) (erledigt 2026-11-06, `f1006588`, Tester PASS `measurements/measurement-2026-11-06-v28-test/RUNINFO.TXT`; erledigt damit auch [V36](../TASK/V36.md)) → [V29](../TASK/V29.md) (erledigt 2026-11-06, `b4e419e5`, Tester PASS `measurements/measurement-2026-11-06-v29-test/RUNINFO.TXT`) → [V30](../TASK/V30.md) (erledigt 2026-11-06, `858e5f68`, Tester PASS `measurements/measurement-2026-11-06-v30-test/RUNINFO.TXT`; AES-Bilder bytegleich; Pi: umlaufender Text ueber den Cache-Clean-Pfad ins Testszenario) → [V33](../TASK/V33.md) (erledigt 2026-11-06, `877a14b9`, Tester PASS `measurements/measurement-2026-11-06-v33-test/RUNINFO.TXT`; Folge [V44](../TASK/V44.md); vor der naechsten Karte QED, TeraDesk, HypView, zView neu linken; Pi: Text ins Testszenario) → [V44](../TASK/V44.md) (erledigt 2026-11-06, `6e96e91c`, Tester PASS `measurements/measurement-2026-11-06-v44-test/RUNINFO.TXT`; Pi: Fbench auf neuem Kernel gegen `e56021e7`) → [HW08](../TASK/HW08.md) (P1, neu 2026-11-06, Anwender: vor allen weiteren Text-Tickets: Speichern scheitert sporadisch bis zum Neustart, zeitweise langsam, Hypothese Anwender: Timeout der Karte; schliesst HW05, HW06 ein; Punkte 1+2 erledigt 2026-11-06, `e421bd5d`, `measurements/measurement-2026-11-06-hw08/RUNINFO.TXT`; Pi-Lauf offen, Entwurf `measurements/measurement-2026-11-06-hw08/PI-RUN.TXT`; Fix danach, Anwenderentscheidung A/B im Ticket) → [FS31](../TASK/FS31.md) (erledigt 2026-11-06, `20f87d1d`, Tester PASS `measurements/measurement-2026-11-06-fs31-test/RUNINFO.TXT`; Loeschen auf einem Laufwerk traf offene Dateien auf einem anderen, in Release 0.7 enthalten; Warnung an die Tester offen beim Anwender; Folge [FS32](../TASK/FS32.md), geparkt) → fuer Release [REL071](../TASK/REL071.md): [A140](../TASK/A140.md) (#22; umgesetzt 2026-11-07, Patch `sdk/overlays/hypview/dl_win.c`, `measurements/measurement-2026-11-07-a140/RUNINFO.TXT`; Tester und Pi offen), [A141](../TASK/A141.md) (#23; gemessen 2026-11-07: EmuTOS-gleich, Ursache in der Ressource, Entscheidung A/B1/B2/C beim Anwender), [A142](../TASK/A142.md) (#21; Bytefolge in cflibs Bildern, andere Wurzel, offen), Paket und Pi-Abnahme (Anwender 2026-11-06) → [PORT12](../TASK/PORT12.md) (XGDOS, neu 2026-11-06, Anwender: frueher einplanen; Stufe 1 erledigt 2026-11-06 `measurements/measurement-2026-11-06-port12/RUNINFO.TXT`, ADR-Entscheidung beim Anwender, kein Kernelcode davor) → [V31](../TASK/V31.md) → [V32](../TASK/V32.md) (erledigt 2026-11-06, `937d6361`, Tester PASS `measurements/measurement-2026-11-06-v32-test/RUNINFO.TXT`; Folge [DESK71](../TASK/DESK71.md)) → [V34](../TASK/V34.md) → [V35](../TASK/V35.md) → [V37](../TASK/V37.md) → [V38](../TASK/V38.md) → [V27](../TASK/V27.md) (P4), Anwenderentscheidungen A/B/C in V18; Teil 2/3 danach). **Reihenfolge nach der Prioritaetenrunde (Anwender 2026-11-04):** [A127](../TASK/A127.md) (Ursachenforschung 2026-11-04 ohne Nachstellung, `docs/analysis/a127-root-cause-2026-11-04.md`; D1 umgesetzt 2026-11-04, `bae52dbd`, Tester PASS `measurements/measurement-2026-11-04-a127-d1-test/RUNINFO.TXT`; Folge: Klick-Auswahl im Menue wirkt nach dem Doppelklick-Fenster, am Orakel gemessen EmuTOS-gleich 2026-11-05 `measurements/measurement-2026-11-04-a127-oracle/RUNINFO.TXT`; Pi des Anwenders 2026-11-05 ok (`measurements/measurement-2026-11-05-a127-d1-card/RUNINFO.TXT`); D2/D3 offen bis Ragnars Rueckmeldung) → [A128](../TASK/A128.md) (Dateiauswahl: Schieber wie EmuTOS, Rad als Ergaenzung; GitHub #9; umgesetzt 2026-11-04, `df2c7501`; Tester PASS 2026-11-04, `measurements/measurement-2026-11-04-a128-test/RUNINFO.TXT`; offen echte Pi im Testszenario; Folgeticket [TOOL31](../TASK/TOOL31.md), P3, geparkt) → [A129](../TASK/A129.md) (erledigt 2026-11-04: Pixel EmuTOS-gleich, kein Fehler) → [A132](../TASK/A132.md) (umgesetzt 2026-11-04, `be3c06a9`, Tester PASS; Maske im Titel der Dateiauswahl) → [A133](../TASK/A133.md) (umgesetzt 2026-11-04, `fd6c7e3f`, Tester PASS zusammen mit A132 `measurements/measurement-2026-11-04-a133-test/RUNINFO.TXT`; TEDINFO-Text im Schreibmodus des Farbworts) → [A135](../TASK/A135.md) (neu, P3: `G_BOXCHAR` ebenso) → [A136](../TASK/A136.md) (neu, P3: Luecken der Drop-down-Box wie EmuTOS) → [A134](../TASK/A134.md) (neu, P3: Laufwerksklick/Fehlerzweig der Dateiauswahl) → [USB03](../TASK/USB03.md) (geparkt 2026-11-05 bis Ragnars Rueckmeldung zu #19 und zum USB-Stand; Anwender: baut teils aufeinander auf) → [ARCH03](../TASK/ARCH03.md)-Rest (Vorschlag 2026-11-05, nicht bestaetigt: Stufe 2b nach 1.0, `asid=on` ins Testszenario) → [TOOL32](../TASK/TOOL32.md) (VDI-Zuordnung in `mapping.tsv`, erledigt 2026-11-05; 26 Abweichungs-Kandidaten im Ticket, ungemessen) → [TOOL33](../TASK/TOOL33.md) (AES-Zuordnung, erledigt 2026-11-05; 47 Kandidaten im Ticket, ungemessen); dann EmuTOS-Konformitaet (A119–A125, DESK40/49/51–55/61/63/67–69, FS12, FS15–FS17, INPUT02, RST02) → Ton fuer 1.0 (SND01, SND02, H04). Daneben: die nur auf der Pi offenen Abnahmen (A16, A37, DESK34, DESK35, HW03, R02–R05, SER01, V11, V12, DESK43, BIOS09, FS26 T1–T3, >15 USB-Befehle) als ein Testszenario fuer die Tester; die „ERLEDIGT“-Tickets ohne Quittung einmal gezielt durchsehen. Ausbau nach 1.0 und Werkzeug-Tickets bleiben liegen. Aus der Issue-Durchsicht 2026-11-04 ans Ende: [A130](../TASK/A130.md) (#2 Fenster-Rollbalken, P2; am Orakel gemessen 2026-11-05, nachgestellt, Befunde 001–004 in `docs/audits/2026-11-05-a130-slider.md`; erledigt 2026-11-05: Fix `d9e44143`, Tester PASS `measurements/measurement-2026-11-05-a130-test/RUNINFO.TXT`, Pi des Anwenders ok `measurements/measurement-2026-11-05-a130-card/RUNINFO.TXT`), [A131](../TASK/A131.md) (#7 Fensterschatten, P3; am Orakel gemessen 2026-11-05: Schatten hellgrau statt `W_BOX`-Randfarbe; Anwenderentscheidung 2026-11-05 wie EmuTOS, hebt die vom 2026-09-10 auf; umgesetzt 2026-11-05 `837e1001`, Tester PASS `measurements/measurement-2026-11-05-a131-test/RUNINFO.TXT`; erledigt 2026-11-05: Pi des Anwenders ok `measurements/measurement-2026-11-05-a131-card/RUNINFO.TXT`), [DESK70](../TASK/DESK70.md) (#12 Textprogramm-Ausgabe, erst EmuTOS pruefen, P3). Aus den V30-Nebenbefunden ans Ende (2026-11-06): gemessen [V39](../TASK/V39.md) (`F_LIGHT`), [V40](../TASK/V40.md) (`LSTLIN`), [V41](../TASK/V41.md) (Ursache belegt 2026-11-06, `de1d12dd`, neun Stellen; keine App zeichnet Effekte in Replace, Fix ans Ende, Anwender), [V45](../TASK/V45.md) (aus V41), [FS33](../TASK/FS33.md) (aus HW08, zweiter Handle), [V42](../TASK/V42.md) (P4, 4096+ Zeichen), [A143](../TASK/A143.md), [A144](../TASK/A144.md); aus dem Fbench-Bericht `docs/analysis/fbench-functions-2026-11-07.md` [V46](../TASK/V46.md), [V47](../TASK/V47.md), [A145](../TASK/A145.md), [V48](../TASK/V48.md) (P4); nur gelesen [V43](../TASK/V43.md) (P4), [A137](../TASK/A137.md), [A138](../TASK/A138.md), [A139](../TASK/A139.md), zuerst messen.
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
  [H01](../TASK/H01.md) (wartet auf Hardware); [HW05](../TASK/HW05.md) (Befund erledigt, Abnahme mit dem Pi-Lauf von [HW08](../TASK/HW08.md)); zurueckgestellt:
  [T13](../TASK/T13.md).
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
| 2026-11-07 | **[A142](../TASK/A142.md) (GitHub #21): QED zeichnet Check-Boxen, Radio-Buttons und Alert-Icons wieder richtig, HypView sein QED.HYP-Logo (`2f6ea7eb`, `56fd81a1`, `6e43e35a`, Developer; Tester 2026-11-07 in QEMU abgenommen, `measurements/measurement-2026-11-07-a142-test/RUNINFO.TXT`, Pi offen).** Warum: cflibs `inline.rsh` haelt seine 44 Bildtabellen als Bytes in m68k-Folge, VDI und AES lesen native Woerter, also tauschten die beiden 8-Pixel-Haelften jedes Worts den Platz. Fix im Overlay: Worttabellen mit denselben Zahlen, Upstream unveraendert. Auf der Karte betrifft das nur `QED.NAP`. Dasselbe Muster aus einer Datei: einfarbige `.HYP`-Bilder gingen ungewandelt an `vr_trnfm`, Fix im HypView-Patch. Danach gefahrene Glyphen und Logo 0 Pixel Abweichung zum Orakel. `sdk/run-host-tests.sh` 36 -> 38 / 0 / 0; `cargo xtask test` nicht betroffen. |
| 2026-11-07 | **[A141](../TASK/A141.md) (GitHub #23): HypViews Icon-Farben sind EmuTOS-gleich, die Ursache steckt in der Ressource (Commit dieser Zeile, Developer; `measurements/measurement-2026-11-07-a141/RUNINFO.TXT`).** Warum: Ticket aus einem Screenshot, erst messen. `tosdump/probes/icprobe.c` (neu, dauerhaft) und die echte m68k-HypView auf dem Orakel, Falcon Truecolor: 59 Faelle, 0 verschieden, Leiste pixelgleich. Die 8-Plane-Icons von `hyp_view.rsc` sind fuer die NVDI-Palette gezeichnet (`references/freemint/xaaes/src.km/pal/nvdi.pal`), EmuTOS und NewTOS nehmen die TOS-Standardpalette. Kein Produktcode geaendert; Optionen A/B1/B2/C im Ticket, Entscheidung beim Anwender. [A142](../TASK/A142.md) hat eine andere Wurzel: Bytefolge in cflibs eingebauten Bildern (`sdk/cflib/overlay/inline.rsh`). Hosttests unberuehrt. |
| 2026-11-07 | **ADR-034: `references/tos4x` als Lesevergleich (Commit dieser Zeile, Anwender).** Warum: Der Anwender will DMA-Sound und VDI in TOS 4.04, dem Falcon-TOS, gegen EmuTOS vergleichen. EmuTOS bleibt Spezifikation und Orakel; gelesen wird Tag `build/tos404`, nur wo ein Ticket es nennt; nichts wird kopiert oder zitiert; ein Unterschied ist ein Befund, gemessen wird am Orakel, Abweichen von EmuTOS entscheidet der Anwender. Folge: NewTOS kann nicht mehr behaupten, dass niemand die Atari-Quellen gesehen hat; die Tickets halten fest, wo gelesen wurde. Keine Lizenzdatei im Baum, rechtlich nicht bewertet. |
| 2026-11-07 | **[A140](../TASK/A140.md) (GitHub #22): HypView als Accessory stuerzt beim zweiten Oeffnen nicht mehr ab (`fba96623`, Developer; Tester 2026-11-07 in QEMU abgenommen, `measurements/measurement-2026-11-07-a140-test/RUNINFO.TXT`, Pi offen).** Warum: Nachgestellt in QEMU, gleiche Stelle wie im Absturzbericht (`text+00006eac`). Der Fehler steckt in Upstream-HypView: `Win` zeigt nach dem Schliessen weiter auf den freigegebenen Fensterblock, und AC_OPEN liest ihn in `find_file`. EmuTOS laesst freigegebenen Speicher lesbar, die Stelle laeuft dort zufaellig durch; NewTOS sperrt ihn wie FreeMiNT (ADR-027), Gegenprobe `memprot=off` ohne Absturz. Kernel unveraendert. Fix im Patch `sdk/overlays/hypview/dl_win.c`, `Win` wird beim Freigeben NULL. Rot belegt: neuer `hypview-win-host`, 9 von 27 Pruefungen scheitern mit Upstream, 0 mit dem Patch. QEMU: sechsmal oeffnen und schliessen ohne Absturz, Speicherschutz an und strict (`measurements/measurement-2026-11-07-a140/RUNINFO.TXT`). SDK-Hosttests 36/0/0. Das naechste Paket muss HypView neu bauen. |
| 2026-11-06 | **[HW08](../TASK/HW08.md) Fix: Ein Kartenfehler blockiert spaeteres Speichern nicht mehr, und `Fwrite` schreibt wie EmuTOS nicht mehr je Aufruf zurueck (`16bfa004`; Tester 2026-11-07 in QEMU abgenommen, `measurements/measurement-2026-11-06-hw08-fix-test/RUNINFO.TXT`, Pi-Lauf offen).** Warum: Ein Sektor, den nach drei Versuchen niemand mehr schrieb, liess jedes `Fcreate` bis zum Neustart scheitern; ein `CMD12` an eine Karte ohne laufende Uebertragung setzte `ILLEGAL_COMMAND`, das naechste `CMD24` galt als abgelehnt, die Karte blieb in `rcv` (Pi-Protokoll). Jetzt: `WRITEBACK_ATTEMPTS` gestrichen (Anwender Weg B, benannte Abweichung in `docs/COMPATIBILITY.md`); Treiber fragt mit `CMD13`, `CMD12` nur an `rcv`/`data`, Datenkommando ohne Antwort bekommt beide Resets, nach einem Fehler wartet das naechste Datenkommando auf `tran`; der Dispatcher antwortet mit dem ersten Geraetefehler des Aufrufs wie `osif` (`Fcreate` -2 statt -36, auch `Dfree`, dessen falsche Zahl weg ist). Am Orakel: Schreibkommandos je Muster gleich EmuTOS bis auf die Verdraengung, Prozessende ohne `Fclose` gleich. Nicht mitgefixt: zweiter Handle auf wachsende Datei sieht Laenge 0, EmuTOS teilt die `DFD`. Lauf `measurements/measurement-2026-11-06-hw08-fix/RUNINFO.TXT`. [HW06](../TASK/HW06.md) erledigt: Eigenschaft der Karte. |
| 2026-11-06 | **[FS31](../TASK/FS31.md): Loeschen auf einem Laufwerk trifft keine offene Datei auf einem anderen mehr (Commit dieser Zeile, Developer).** `ixdel` nannte eine offene Datei nur nach Verzeichnis-Startcluster und Slot; jede Wurzel ist 0, gleich alte Ordner beginnen beim gleichen Cluster. Rot belegt: genau die Kodes des Pi-Laufs, `Fwrite` -36, `Fclose` -65. Schlimmer als der Anlass: Kopieren oder Verschieben ueber eine bestehende Datei auf ein anderes Laufwerk liest bei gleicher Nummer 0 Byte ohne Fehler, in NEWDESK und TeraDesk, beim Verschieben sind danach beide Inhalte weg. Am Orakel gemessen, Hatari faehrt zwei Laufwerke (`HATARI_ORACLE_GEMDOS_DRIVE`): EmuTOS trifft das andere Laufwerk nicht, Log nach dem Fix bytegleich, Sonde `tosdump/probes/xdprobe.c`. Alle Schluessel im BDOS durchgesehen, nur `ixdel` war falsch. Tester und Pi offen; tosdiff-Fall [FS32](../TASK/FS32.md). `measurements/measurement-2026-11-06-fs31/RUNINFO.TXT`. Hosttests 2217/0/1; Warnungen 64/93, Clippy 73/121, je Basis gleich. |
| 2026-11-06 | **[PORT12](../TASK/PORT12.md) Stufe 1: XGDOS laeuft als Bibliothek auf NewTOS, ohne Kernelarbeit (Commit dieser Zeile, Anwender).** Warum: frueh wissen, was der Port kostet, bevor ein ADR den Kernel beruehrt. Die Upstream-Tests laufen in QEMU unveraendert, alle Ausgaben, drei PDF und das Metafile bytegleich zum Mac. Gepatcht (`sdk/overlays/xgdos/`) laedt ein Programm Speedo-Fonts und zeichnet ueber das echte VDI, 0 Pixel Abweichung zum Mac oberhalb der Systemfontzeile, und schreibt eine PDF auf die Karte, bytegleich. Vektor 34 per `Setexc` wird gespeichert und nie gerufen (gemessen); residenter Speicher ist fuer andere Prozesse auch bei `memprot=strict` nutzbar. Beleg `measurements/measurement-2026-11-06-port12/RUNINFO.TXT`. Offen: Anwender entscheidet Weg A oder B fuer das ADR. Kein Kernelcode, keine Host-Testzahl betroffen. Nebenbefund: `Fcreate` lehnt Namen ueber 8.3 ab, EmuTOS kuerzt (gelesen), nicht getickert. |
| 2026-11-06 | **`tools/sd-bench.py`: SD-Karten am Mac messen, Beleg fuer [HW08](../TASK/HW08.md) (Commit dieser Zeile, Anwender).** Schreibt nur Temp-Dateien auf ein gemountetes, wechselbares FAT-Volume, misst Durchsatz und einzelne Ausreisser, zeigt mit `sudo` Partitionen und BPB. Erstes Ergebnis `measurements/measurement-2026-11-06-hw08-mac/RUNINFO.TXT`: Die Testkarte braucht fuer 4 KiB ueber 256 MiB verteilt p50 999 ms, max 1991 ms, innerhalb 256 KiB 8,6 ms; eine zweite Karte im selben Leser 3,9 ms, max 38 ms. Der Leser allein ist also nicht die Ursache, Dateisystem und Fuellstand sind nicht getrennt, der Pi-Pfad ist ungemessen. Kein Produktcode. |
| 2026-11-06 | **[HW08](../TASK/HW08.md) Punkte 1+2: Ursache des Haengens bis zum Neustart in QEMU nachgestellt, Pi-Lauf offen, kein Fix (`e421bd5d`, Commit dieser Zeile).** Debug-Features `sd-diag` (Strg+Alt+F9 zeigt Treiberzaehler, Fehlerprotokoll mit LBA, Zeiten, Cache-Zaehler, dirty Sektoren auf dem Schirm, nur aus dem RAM) und `sd-fault` (Fehler im Treiber einspeisen); ohne sie Kern bytegleich. Sonde `tosdump/probes/wfprobe.c` beidseitig, `sdk/examples/fswbench.c` fuer die Schreibmessung. Gemessen, `measurements/measurement-2026-11-06-hw08/RUNINFO.TXT`: Glied Cache (FAT-Sektor nach 3 Fehlschlaegen aufgegeben, jedes `flush` meldet ihn, `Fcreate` -10 bis Neustart auch bei wieder guter Karte) reproduziert 32768 Byte und 0-Byte-Dateien; EmuTOS haengt nie, verliert dafuer die Puffer. Neu: Glied Treiber (CMD12 ausserhalb einer Uebertragung, naechstes R1 `ILLEGAL_COMMAND` als Ablehnung gewertet, Karte bleibt im Empfangszustand, alles `CTO` bis Neustart); auf der echten Karte ungeprueft. Ausloeser auf der Pi offen; Limit nicht angefasst. |
| 2026-11-06 | **[V44](../TASK/V44.md): Text ist wieder schnell, schneller als vor V33 (Commit dieser Zeile).** Ursache schichtweise gemessen (`-icount`, Gastbefehle): die acht Woerter je Glyphenzeile aus V33 kosten 85 von 96 Ticks, die 512 Zeilen rund 10; das Backend ist es nicht. Jetzt `tos-api/src/vdi/text.rs`: Glyphenbau generisch ueber die Zeilenbreite, `fits_narrow` waehlt je Glyphe einwortige Zeilen fuer Zellen bis 64 × 64 (alle eingebauten, jeder Effekt), sonst acht Woerter; Grenzen aus V33 unveraendert, kein Assembler, keine Direct-Blit-Schicht. `txbench` gegen die Basis `587ddbd5`, QEMU, Wanduhr, 8 Runden: plain ×0,77, bold ×0,79, light+skew ×0,85, kurze Strings ×0,79 (vor dem Fix ×1,36, ×1,58, ×1,23, ×1,32); Pi ungemessen. Bytegleich zum Stand davor: Probe, alle `VDI.BIN`/`RASTER.BIN`; `AES.BIN` nur das bekannte Adresswort, `tosdiff` 0 (`measurements/measurement-2026-11-06-v44/RUNINFO.TXT`). Hosttests +5, 2195/0/1. Bau beide Boards ok; Warnungen tos-api 64 → 64, Kernel 93 → 93 (Bau), Clippy 73 → 73, 121 → 121. Tester offen. |

## Notes

The project is intentionally over-documented at the start. The premise:
spec-first development against a fixed, finite, well-documented API
prevents the implementation drift that ruins long-running OS hobby
projects.
