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
| Hosttests (`cargo xtask test`) | **2273 / 0 / 2** (gemessen 2026-11-07 mit V35, Developer und Tester, +12 Tests, 1 ignoriert mit Grund (TPH, V52); davor 2261 am 2026-11-07 mit V34, Developer und Tester, +3 Tests; davor 2258 am 2026-11-07 mit V31, Developer und Tester, +17 Tests; davor 2241 am 2026-11-07 mit A141, Developer und Tester, +2 `rsc_recolour_tests`; davor 2239 am 2026-11-06 mit HW08 Fix, Developer, +22 Tests; davor 2217 am 2026-11-06 mit FS31, Developer, +17 Tests; davor 2200 am 2026-11-06 mit HW08 Punkte 1+2, Developer, +5 Tests; davor 2195 am 2026-11-06 mit V44, Developer, +5 Tests; davor 2190 am 2026-11-06 mit V33, Developer, +30 Tests; davor 2160 am 2026-11-06 mit V30, Developer, +40 Tests; davor 2120 am 2026-11-06 mit V32, Developer, +4 Tests; davor 2116 am 2026-11-06 mit V29, Developer, +20 Tests; davor 2096 am 2026-11-06 mit V28, Developer, +30 Tests; davor 2066 am 2026-11-06 mit V25 laut Recent Decisions; davor 2033 am 2026-11-05 mit V20, Developer, +20 Tests; davor 2013 am 2026-11-05 mit V26, Developer, +9 Tests; davor 2004 am 2026-11-05 mit V19, Tester; davor 1974 am 2026-11-04 mit PROC08 Schirmbericht, +6 Tests; davor 1929 mit PROC08 `crash-walk`, +7 Tests; davor 1922 mit XBIOS12, +4 Tests; davor 1918 mit FS21, +8 Tests; davor 1910 mit MEM11, +3 Tests; davor 1907 mit FS28, +2 Tests; davor 1905 mit TOOL29, +6 xtask-Tests; davor 1899 mit A126, +4 Tests; davor 1895 mit SDK08, kein neuer Test; davor 1895 mit COOK02 `_IDT`, kein neuer Test; davor 1895 mit XBIOS04, +1 Test; davor 1894 mit BIOS09, +1 Test; davor 1893 mit BIOS17, +1 Test; davor 1892 mit BIOS16, +4 Tests; davor 1888 mit BIOS08, +7 Tests; davor 1881 mit XBIOS05, +9 Tests, Basis laut Lead 1872; davor 1859 mit BIOS07, +9 Tests; davor 1850 mit FS23, +3 Tests; davor 1847 mit BIOS04, +4 Tests; davor 1843 mit XBIOS02, +1 Test; davor 1842 mit BIOS06, +2 Tests; davor 1840 mit BIOS01 Rest, +7 Tests; davor 1833 mit TOOL19, +5 Tests; davor 1828 mit ARCH06, +4 Tests; davor 1824 mit XBIOS18, +1 Test; davor 1823 mit TOOL21, +2 Tests; davor 1821 mit TOOL17, +5 Tests; davor 1816 mit XBIOS15, +8 Tests; davor 1808 mit BIOS15, +18 Tests; davor 1790 mit FS22, +7 Tests; davor 1783 mit XBIOS01, +16 Tests; davor 1767 mit FS20, +7 Tests; davor 1760 mit BIOS01 S2, +9 Tests; davor 1751 mit TOOL07 Rest, +3 xtask-Tests; davor 1748 mit TOOL12, +3 deps-Tests; davor 1745 mit TOOL10; TOOL09 +16, TOOL10 +3 xtask-Tests; vorher 1726 / 0 / 1 auf `75527af8`, `--fast` ohne den einen Doctest) |
| SDK-Hosttests (`sdk/run-host-tests.sh`) | **38 / 0 / 0** (gemessen 2026-11-07 mit A142, Developer, +2 `inline-rsh-host`, `hypview-prepare-host`; davor 36 am 2026-11-07 mit A140, Developer, +1 `hypview-win-host`; davor 35 am 2026-10-30 mit SDK08, kein neuer Test: Rebuild-Paritaet von `rt-host`/`ldbl-host` blendet nur `.comment` aus, Host-clang `clang-2100.4.1.1`; dazwischen 33/2/0 nach dem clang-Update; davor 35 mit XBIOS05, +1 `bconmap-layout`; davor 34 mit SDK07, +1 `td-nohard-host`; davor 33 mit SDK04, +3 `sysvars-layout`, `hypview-ref-host`, `zview-color-host`; davor 30 mit SDK04 C6, +1 `td-cookie-host`; davor 29 mit SDK05, +1 `cookie-host`; davor 28 mit SDK03, +1 `sysinfo-host`; davor 27 mit XBIOS01, +1 `iorec-layout`) |
| `sh tools/desk-drift-probe.sh` | **0** (unsafe 20; gemessen 2026-10-26) |
| APITEST (`#Z`, memprot on/off/strict) | **109 / 0 / 5** (letzter Lauf `measurements/measurement-2026-10-26-a118-fix/RUNINFO.TXT`) |

Bau- und Clippy-Warnungen stehen hier ohne Zahl: sie wurden bei dieser
Bereinigung nicht nachgemessen. Sie duerfen nicht steigen; jeder Arbeitsschritt
nennt seine Vorher-/Nachher-Zahl in seiner Zeile unter *Recent Decisions*.

**Offen (Stand 2026-10-26, aus der Statuszeile der Tickets ohne DONE-Quittung):**

- **Offen beim Anwender (Stand 2026-11-07, nach Release 0.7.1):** (1) Ticket fuer die
  47 AES-Kandidaten aus [TOOL33](../TASK/TOOL33.md) (Messung am Orakel wie
  [V18](../TASK/V18.md)), ans Ende der Warteschlange: ja oder nein. (2) Namen der
  Fbench-Tests mit leerem Fenster, daraus das Vergleichsticket (`FBENCH.PRG` im
  Orakel gegen QEMU je Test; Soll-Liste `docs/analysis/fbench-functions-2026-11-07.md`
  Abschnitt 6). (3) Tester (Ragnar, Wolfgang) zu 0.7.1 eigens informieren?
  (4) Stecker-Test aus
  REL071 nur vom Anwender als frueher getestet gemeldet, kein Beleg im Repo.
  Naechstes Produkt-Ticket der Warteschlange: [PORT12](../TASK/PORT12.md) (ADR-Entscheidung
  beim Anwender, kein Kernelcode davor). WFPROBE adressiert logische Partitionen nicht
  (liest den MBR-Eintrag Laufwerk minus C); kein Ticket ohne Anwender.

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
  **[REL07](../TASK/REL07.md) Release 0.7** (**veroeffentlicht 2026-11-04**: Tag `v0.7` auf `d7711c42`, GitHub-Pre-Release mit `NewTOS-0.7.zip` sha256 `a5191394…`; Pi-Abnahme des Kerns 2026-11-04; Tester informiert (#3, #8), #9 geschlossen, REL07 erledigt; Paket mit `tools/release/package-release.sh`, `measurements/measurement-2026-11-04-rel07-final/RUNINFO.TXT`; Schritt 1 Version aus einer Quelle `797e0078`; Anwender 2026-11-04; nach der PROC08-Leiter, mit `crash-walk`, Abnahme auf der Pi des Anwenders, dann Ragnar informieren). **PROC08 wieder aufgetreten 2026-11-04** (Foto, `measurements/measurement-2026-11-04-proc08-recur/RUNINFO.TXT`): Schirmbericht `model=valid table=valid at=fault` = Kandidat B; Mechanismus-Forschung `docs/analysis/proc08-candidate-b-2026-11-04.md` (Fenster gefunden, Ausloeser nicht); **zurueckgestellt "to be tested"** (Anwender: 60 zView-Starts mit Fix ohne Absturz, vorher mehrmals hintereinander); Fix umgesetzt 2026-11-04 (`01eb7709`), Tester PASS (`measurements/measurement-2026-11-04-proc08-fix-test/RUNINFO.TXT`), Pi-Abnahme offen; Leiter im Schirmbericht umgesetzt 2026-11-04 (Commit dieser Zeile, `measurements/measurement-2026-11-04-proc08-ladder/RUNINFO.TXT`). **Zuerst (Anwender 2026-11-05):** [V18](../TASK/V18.md) (die 26 VDI-Kandidaten aus TOOL32 am Orakel messen, drei Teile, dann Fix-Tickets je Befund; Teil 1 gemessen 2026-11-05 `2e22799b`: 9 Befunde, Fix-Tickets [V19](../TASK/V19.md) (erledigt 2026-11-05, `506643fb`, Tester PASS `measurements/measurement-2026-11-05-v19-test/RUNINFO.TXT`) → [V26](../TASK/V26.md) (Nebenbefund V19, Speicherweg griff in ungepruefte Zeile, P2; erledigt 2026-11-05, `1c84f4d2`+`87f1684e`, Anwenderentscheidung a: EmuTOS-Geometrie, Schnitt nur hinter `fd_h`; Tester PASS `measurements/measurement-2026-11-05-v26-test/RUNINFO.TXT`) → [V20](../TASK/V20.md) (`abline`, erledigt 2026-11-05, `bec04bab`; Anwenderentscheidung A: Umlauf nur im logischen Schirm; Tester PASS `measurements/measurement-2026-11-05-v20-test/RUNINFO.TXT`, AES/Desktop-Szenen pixelgleich; Pi: XOR-Rahmen am Schirmrand ins Testszenario) → [V21](../TASK/V21.md) (erledigt 2026-11-06, `497773f9`, Saat-Warteschlange wie EmuTOS; Tester PASS mit Ueberlauf-Fall `measurements/measurement-2026-11-06-v21-test/RUNINFO.TXT`; Notbremse offen beim Anwender) → [V22](../TASK/V22.md) (erledigt 2026-11-06, `7108bdff`, Tester PASS `measurements/measurement-2026-11-06-v22-test/RUNINFO.TXT`) → [V23](../TASK/V23.md) (erledigt 2026-11-06, `ee67903f`, Kappung zentral in `dispatch_in`; Tester PASS `measurements/measurement-2026-11-06-v23-test/RUNINFO.TXT`; Folgeticket [TOOL34](../TASK/TOOL34.md) geparkt) → [V24](../TASK/V24.md) (erledigt 2026-11-06, `caf77fab`, Anwenderentscheidung B; Tester PASS `measurements/measurement-2026-11-06-v24-test/RUNINFO.TXT`) → [V25](../TASK/V25.md) (erledigt 2026-11-06, `294272e2`, frueher Ausstieg von `curve`; Tester PASS `measurements/measurement-2026-11-06-v25-test/RUNINFO.TXT`; Teil 1 damit bis auf Entscheidung C orakelgleich) → V18 Teil 2 gemessen 2026-11-06 `eba4122a`: [V28](../TASK/V28.md) (erledigt 2026-11-06, `f1006588`, Tester PASS `measurements/measurement-2026-11-06-v28-test/RUNINFO.TXT`; erledigt damit auch [V36](../TASK/V36.md)) → [V29](../TASK/V29.md) (erledigt 2026-11-06, `b4e419e5`, Tester PASS `measurements/measurement-2026-11-06-v29-test/RUNINFO.TXT`) → [V30](../TASK/V30.md) (erledigt 2026-11-06, `858e5f68`, Tester PASS `measurements/measurement-2026-11-06-v30-test/RUNINFO.TXT`; AES-Bilder bytegleich; Pi: umlaufender Text ueber den Cache-Clean-Pfad ins Testszenario) → [V33](../TASK/V33.md) (erledigt 2026-11-06, `877a14b9`, Tester PASS `measurements/measurement-2026-11-06-v33-test/RUNINFO.TXT`; Folge [V44](../TASK/V44.md); vor der naechsten Karte QED, TeraDesk, HypView, zView neu linken; Pi: Text ins Testszenario) → [V44](../TASK/V44.md) (erledigt 2026-11-06, `6e96e91c`, Tester PASS `measurements/measurement-2026-11-06-v44-test/RUNINFO.TXT`; Pi: Fbench auf neuem Kernel gegen `e56021e7`) → [HW08](../TASK/HW08.md) (P1, neu 2026-11-06, Anwender: vor allen weiteren Text-Tickets: Speichern scheitert sporadisch bis zum Neustart, zeitweise langsam, Hypothese Anwender: Timeout der Karte; schliesst HW05, HW06 ein; Punkte 1+2 erledigt 2026-11-06, `e421bd5d`, `measurements/measurement-2026-11-06-hw08/RUNINFO.TXT`; Pi-Lauf offen, Entwurf `measurements/measurement-2026-11-06-hw08/PI-RUN.TXT`; Fix danach, Anwenderentscheidung A/B im Ticket; **erledigt 2026-11-07**, Quittung; alte Karte entfaellt, Anwender: defekt) → [FS31](../TASK/FS31.md) (erledigt 2026-11-06, `20f87d1d`, Tester PASS `measurements/measurement-2026-11-06-fs31-test/RUNINFO.TXT`; Loeschen auf einem Laufwerk traf offene Dateien auf einem anderen, in Release 0.7 enthalten; Warnung an die Tester offen beim Anwender; Folge [FS32](../TASK/FS32.md), geparkt) → fuer Release [REL071](../TASK/REL071.md): [A140](../TASK/A140.md) (#22; umgesetzt 2026-11-07, Patch `sdk/overlays/hypview/dl_win.c`, `measurements/measurement-2026-11-07-a140/RUNINFO.TXT`; Tester und Pi offen), [A141](../TASK/A141.md) (#23; gemessen 2026-11-07: EmuTOS-gleich, Ursache in der Ressource, Entscheidung A/B1/B2/C beim Anwender), [A142](../TASK/A142.md) (#21; Bytefolge in cflibs Bildern, andere Wurzel, offen), Paket und Pi-Abnahme (Anwender 2026-11-06; Paket 0.7.1 gebaut 2026-11-07, `measurements/measurement-2026-11-07-rel071-package/RUNINFO.TXT`, Pi-Abnahme 2026-11-07 bestanden `measurements/measurement-2026-11-07-rel071-pi/RUNINFO.TXT`; **veroeffentlicht 2026-11-07**: Tag `v0.7.1` auf `6baafc92`, GitHub-Pre-Release, #12/#21/#22/#23 beantwortet und geschlossen) → [PORT12](../TASK/PORT12.md) (XGDOS, neu 2026-11-06, Anwender: frueher einplanen; Stufe 1 erledigt 2026-11-06 `measurements/measurement-2026-11-06-port12/RUNINFO.TXT`, ADR-Entscheidung beim Anwender, kein Kernelcode davor) → [V31](../TASK/V31.md) (erledigt 2026-11-07, `17bcaf56`, Tester PASS `measurements/measurement-2026-11-07-v31-test/RUNINFO.TXT`; ZG012 aus V45 dabei orakelgleich) → [V32](../TASK/V32.md) (erledigt 2026-11-06, `937d6361`, Tester PASS `measurements/measurement-2026-11-06-v32-test/RUNINFO.TXT`; Folge [DESK71](../TASK/DESK71.md)) → [V34](../TASK/V34.md) (erledigt 2026-11-07, `5f299bf8`, Tester PASS `measurements/measurement-2026-11-07-v34-test/RUNINFO.TXT`; Folge [TOOL35](../TASK/TOOL35.md), geparkt) → [V35](../TASK/V35.md) (erledigt 2026-11-07, `788a8aa9`, Tester PASS `measurements/measurement-2026-11-07-v35-test/RUNINFO.TXT`; V18 Teil 2 damit orakelgleich bis auf V52) → **eingeschoben (Anwender 2026-11-07):** [V49](../TASK/V49.md) → [V50](../TASK/V50.md) → [V51](../TASK/V51.md) (Rasterkerne in NEON unter ADR-036, Vergleich MagiC/fVDI unter ADR-035; zweiter Worker im eigenen Worktree neben der V-Reihe) → [V37](../TASK/V37.md) → [V38](../TASK/V38.md) → [V27](../TASK/V27.md) (P4), Anwenderentscheidungen A/B/C in V18; Teil 2/3 danach). **Reihenfolge nach der Prioritaetenrunde (Anwender 2026-11-04):** [A127](../TASK/A127.md) (Ursachenforschung 2026-11-04 ohne Nachstellung, `docs/analysis/a127-root-cause-2026-11-04.md`; D1 umgesetzt 2026-11-04, `bae52dbd`, Tester PASS `measurements/measurement-2026-11-04-a127-d1-test/RUNINFO.TXT`; Folge: Klick-Auswahl im Menue wirkt nach dem Doppelklick-Fenster, am Orakel gemessen EmuTOS-gleich 2026-11-05 `measurements/measurement-2026-11-04-a127-oracle/RUNINFO.TXT`; Pi des Anwenders 2026-11-05 ok (`measurements/measurement-2026-11-05-a127-d1-card/RUNINFO.TXT`); D2/D3 offen bis Ragnars Rueckmeldung) → [A128](../TASK/A128.md) (Dateiauswahl: Schieber wie EmuTOS, Rad als Ergaenzung; GitHub #9; umgesetzt 2026-11-04, `df2c7501`; Tester PASS 2026-11-04, `measurements/measurement-2026-11-04-a128-test/RUNINFO.TXT`; offen echte Pi im Testszenario; Folgeticket [TOOL31](../TASK/TOOL31.md), P3, geparkt) → [A129](../TASK/A129.md) (erledigt 2026-11-04: Pixel EmuTOS-gleich, kein Fehler) → [A132](../TASK/A132.md) (umgesetzt 2026-11-04, `be3c06a9`, Tester PASS; Maske im Titel der Dateiauswahl) → [A133](../TASK/A133.md) (umgesetzt 2026-11-04, `fd6c7e3f`, Tester PASS zusammen mit A132 `measurements/measurement-2026-11-04-a133-test/RUNINFO.TXT`; TEDINFO-Text im Schreibmodus des Farbworts) → [A135](../TASK/A135.md) (neu, P3: `G_BOXCHAR` ebenso) → [A136](../TASK/A136.md) (neu, P3: Luecken der Drop-down-Box wie EmuTOS) → [A134](../TASK/A134.md) (neu, P3: Laufwerksklick/Fehlerzweig der Dateiauswahl) → [USB03](../TASK/USB03.md) (geparkt 2026-11-05 bis Ragnars Rueckmeldung zu #19 und zum USB-Stand; Anwender: baut teils aufeinander auf) → [ARCH03](../TASK/ARCH03.md)-Rest (Vorschlag 2026-11-05, nicht bestaetigt: Stufe 2b nach 1.0, `asid=on` ins Testszenario) → [TOOL32](../TASK/TOOL32.md) (VDI-Zuordnung in `mapping.tsv`, erledigt 2026-11-05; 26 Abweichungs-Kandidaten im Ticket, ungemessen) → [TOOL33](../TASK/TOOL33.md) (AES-Zuordnung, erledigt 2026-11-05; 47 Kandidaten im Ticket, ungemessen); dann EmuTOS-Konformitaet (A119–A125, DESK40/49/51–55/61/63/67–69, FS12, FS15–FS17, INPUT02, RST02) → Ton fuer 1.0 (SND01, SND02, H04). Daneben: die nur auf der Pi offenen Abnahmen (A16, A37, DESK34, DESK35, HW03, R02–R05, SER01, V11, V12, DESK43, BIOS09, FS26 T1–T3, >15 USB-Befehle) als ein Testszenario fuer die Tester; die „ERLEDIGT“-Tickets ohne Quittung einmal gezielt durchsehen. Ausbau nach 1.0 und Werkzeug-Tickets bleiben liegen. Aus der Issue-Durchsicht 2026-11-04 ans Ende: [A130](../TASK/A130.md) (#2 Fenster-Rollbalken, P2; am Orakel gemessen 2026-11-05, nachgestellt, Befunde 001–004 in `docs/audits/2026-11-05-a130-slider.md`; erledigt 2026-11-05: Fix `d9e44143`, Tester PASS `measurements/measurement-2026-11-05-a130-test/RUNINFO.TXT`, Pi des Anwenders ok `measurements/measurement-2026-11-05-a130-card/RUNINFO.TXT`), [A131](../TASK/A131.md) (#7 Fensterschatten, P3; am Orakel gemessen 2026-11-05: Schatten hellgrau statt `W_BOX`-Randfarbe; Anwenderentscheidung 2026-11-05 wie EmuTOS, hebt die vom 2026-09-10 auf; umgesetzt 2026-11-05 `837e1001`, Tester PASS `measurements/measurement-2026-11-05-a131-test/RUNINFO.TXT`; erledigt 2026-11-05: Pi des Anwenders ok `measurements/measurement-2026-11-05-a131-card/RUNINFO.TXT`), [DESK70](../TASK/DESK70.md) (#12 Textprogramm-Ausgabe, P3; am Orakel gemessen 2026-11-07: EmuTOS-Desktop wartet ebenso nicht, Desktop bleibt `7d700cf3`; CoreMark wartet selbst, immer, umgesetzt 2026-11-07 `729bab3b`, zweiter Bau nach Anwenderentscheidung `measurements/measurement-2026-11-07-desk70-build2/RUNINFO.TXT`; Tester PASS `measurements/measurement-2026-11-07-desk70-test2/RUNINFO.TXT`, Pi ok 2026-11-07, #12 beantwortet und geschlossen; erledigt). Aus den V30-Nebenbefunden ans Ende (2026-11-06): gemessen [V39](../TASK/V39.md) (`F_LIGHT`), [V40](../TASK/V40.md) (`LSTLIN`), [V41](../TASK/V41.md) (Ursache belegt 2026-11-06, `de1d12dd`, neun Stellen; keine App zeichnet Effekte in Replace, Fix ans Ende, Anwender), [V45](../TASK/V45.md) (aus V41), [FS33](../TASK/FS33.md) (aus HW08, zweiter Handle), [FS34](../TASK/FS34.md) (aus HW08, abgelehnter Datensektor), [V42](../TASK/V42.md) (P4, 4096+ Zeichen), [A143](../TASK/A143.md), [A144](../TASK/A144.md); aus dem Fbench-Bericht `docs/analysis/fbench-functions-2026-11-07.md` [V46](../TASK/V46.md), [V47](../TASK/V47.md), [A145](../TASK/A145.md), [V48](../TASK/V48.md) (P4); aus V34 [KEYS05](../TASK/KEYS05.md) (P4, Alt+Help `dumpflg`); aus V35 geparkt [V52](../TASK/V52.md) (P4, Handler schreiben in die `PTSIN`-Kopie; bis zum m68k-Thunk), [TOOL36](../TASK/TOOL36.md) (geparkt); bekannte Abweichungen nach Ausloeser: `docs/DEVIATIONS.md` (neu 2026-11-07); nur gelesen [V43](../TASK/V43.md) (P4), [A137](../TASK/A137.md), [A138](../TASK/A138.md), [A139](../TASK/A139.md), zuerst messen. Aus der Pi-Abnahme 0.7.1 (2026-11-07) geparkt: [A146](../TASK/A146.md) (Frage, Recherche: Abstand zwischen Fensterrahmen und Schatten).
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
  [H01](../TASK/H01.md) (wartet auf Hardware); [HW05](../TASK/HW05.md) (Befund erledigt, Abnahme offen: dieselbe `QED.NAP`-Kopie mit Stoppuhr wie vorher, ins Testszenario; [HW08](../TASK/HW08.md) erledigt 2026-11-07); zurueckgestellt:
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

**Arbeitsweise:** ein Task pro Agent; seit 2026-11-07 (Anwender) hoechstens zwei Worker gleichzeitig, mit getrennten Aufgaben und Dateien, die voneinander wissen. Der Lead baut
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
| 2026-11-07 | **[V35](../TASK/V35.md): VDI-Handler lesen `PTSIN` aus der lokalen Kopie wie EmuTOS `GSX_ENTRY` (`788a8aa9`, Developer; Tester PASS `44558a7a`, `measurements/measurement-2026-11-07-v35-test/RUNINFO.TXT`).** Warum: V18 Teil 2, Befund 013 am Orakel gemessen. EmuTOS kopiert `CONTRL[1]` Paare (gekappt auf 1024) vor der Handle-Pruefung in einen statischen Puffer, den alle Handler, Workstations und Prozesse teilen (`vdi/vdi_asm.S:79-108`); NewTOS las das Aufruferfeld. Anwenderentscheidung 2026-11-06: nur die Kopie, die Ueberlagerung mit internen Puffern nicht (`docs/COMPATIBILITY.md`). Drei eigene Aufrufer verliessen sich auf das alte Verhalten und sind offen korrigiert (Bootschirm `vdi_client::gtext` `CONTRL[1]` 0 -> 1, unverdrahtete Selbsttest-Demo, Testhilfe); kein Kartenprogramm betroffen (alle ueber gemlib). `vdi-v18-t2` 0/32, `vdi-all` 0/182, AES und Desktop bytegleich; Kosten in QEMU-icount 0. Hosttests 2261 -> 2273/0/2; Warnungen unveraendert. Folgen: [V52](../TASK/V52.md) geparkt (Handler schreiben in die Kopie, Anwender), [TOOL36](../TASK/TOOL36.md). Tester-Nebenbefund: Konsolenphase nach `appl_init` in QEMU ~7 % langsamer, nicht durch die Kopie, Ursache offen. |
| 2026-11-07 | **ADR-035 und ADR-036: MagiC und fVDI als Lesevergleich fuer Rastertechnik, NEON-Kerne im Raster-Backend; Tickets [V49](../TASK/V49.md)–[V51](../TASK/V51.md) (Commit dieser Zeile, Anwender).** Warum: Fbench Pi 4 gegen V4SA (Anwender-PDF) zeigt Fuellen, Fenster oeffnen/schliessen und Scrollen weit hinter einem 68080; gelesen (nicht gemessen): alle Flaechenfuellungen laufen pixelweise ueber `combine` -> `dyn put_pixel` -> `write_volatile`, der `softfloat`-Kern vektorisiert nie, `clean` macht `dc civac`, sobald ein Prozess `Physbase` haelt. Anwender: gleich Assembler statt erst Rust umbauen, wenn es Vorteile bringt. ADR-036: nur ganzzahlige NEON-Befehle im Raster-Backend, benutzte V-Register selbst sichern, skalare Referenz und bitgleiche Hosttests auf dem AArch64-Mac, Kern nur bei messbarem Gewinn auf der Pi. Ersetzt die Notiz "Assembler ganz viel spaeter". Quellen geklont: `references/magicmac` `90ae7dcc` (GPL-3), `references/fvdi` `5a1ce945`. Arbeitsweise ab jetzt (Anwender): zwei Worker mit getrennten Aufgaben, die voneinander wissen. |
| 2026-11-07 | **[V34](../TASK/V34.md): `Scrdmp` setzt `dumpflg` auf -1, Escape 17 (`v_hardcopy`) ruft `Scrdmp`, wie EmuTOS (`5f299bf8`, Developer; Tester PASS `038463fe`, `measurements/measurement-2026-11-07-v34-test/RUNINFO.TXT`).** Warum: V18 Teil 2, Befunde 011/012 am Orakel gemessen (T18A: EmuTOS -1/-1, NewTOS 5/5). EmuTOS `bios/xbios.c:490-494`, `vdi/vdi_esc.c:260-263`; `dump_vec` wird weiter nicht gerufen (ADR-031), gedruckt wird nichts. Spec zu EmuTOS korrigiert: D-VDI-4 ("Escape 17 No-op wegen fehlendem Drucker") zurueckgenommen, `docs/api/xbios/14-scrdmp.md`, `docs/api/vdi/_escape.md`, `docs/api/bios/_state.md`. Kein Karten-Programm ruft `Scrdmp`/`v_hardcopy`; AES und Desktop bytegleich. Hosttests 2258 -> 2261/0/1; Warnungen 64/93, Clippy 73/121, je Basis gleich. Folge: [TOOL35](../TASK/TOOL35.md) (tosdiff-Fall, geparkt), [KEYS05](../TASK/KEYS05.md) (Alt+Help zaehlt `dumpflg` nicht; `dumpflg` je Prozess statt global). |
| 2026-11-07 | **[V31](../TASK/V31.md): Skalierter Text bewegt die horizontale DDA wie EmuTOS, nur fuer sichtbare Glyphen und auch bei genauer Verdopplung (`17bcaf56`, Developer; Tester PASS `b1c35c9b`, `measurements/measurement-2026-11-07-v31-test/RUNINFO.TXT`).** Warum: V18 Teil 2, Befunde 007/008 am Orakel gemessen. NewTOS skalierte jede Glyphe vor der Clip-Entscheidung, Zeichen ausserhalb des Clip schoben `XDDA` weiter; bei `dda_inc == 0xFFFF` verdoppelte es exakt, EmuTOS driftet (K14, Anwenderentscheidung 2026-11-06: wie EmuTOS). Jetzt wie `text_blt` (`vdi/vdi_textblit.c:1301-1417`): Clip zuerst, Stiftschritt aus `char_resize`, `XDDA` nur in der Abstandsschleife von `scale`. Probe 933 Faelle, Kopf gegen Fix genau 9 anders, alle zum Orakel hin (T14E, T14K, T31A–E, ZG012; ZG013 485 -> 98 px, Rest Schraeg = V41); `vdi-v18-t2` nur noch 50/54 (V35), `vdi-all` 0/182. AES r2/r3/r4 und Desktop gleich zur Basis. Hosttests 2241 -> 2258/0/1; Warnungen 64/93, Clippy 73/121, je Basis gleich. Spec `docs/api/vdi/output/_text.md` ergaenzt. |
| 2026-11-07 | **[HW08](../TASK/HW08.md) erledigt: Ein Kartenfehler blockiert spaeteres Speichern auch auf der echten Pi nicht mehr (Fix `16bfa004`, Quittung; Commit dieser Zeile, Lead).** Warum: Die Pi-Laeufe lagen mit 0.7.1 vor, das Ticket war nur nicht geschlossen. Belege: FSWBENCH zweimal ohne Fehler, Fbench vollstaendig, Desktop-Kopie inhaltsgleich, WFPROBE t/d/u/b auf D: bestanden (`measurements/measurement-2026-11-07-hw08-pi5/RUNINFO.TXT`, `measurements/measurement-2026-11-07-hw08-pi6/RUNINFO.TXT`, `measurements/measurement-2026-11-07-rel071-pi/RUNINFO.TXT`). Schritt 2 mit der alten Karte entfaellt (Anwender: Karte defekt, auch im Mac). `nonrecover 1` in u2 ist erwartet: 200 ms Restbusy ueberdauern das Wiederherstellungsbudget von 100 ms, QEMU zaehlt gleich. Restbefund abgelehnter Datensektor (`a1`, `b4`) als [FS34](../TASK/FS34.md) ans Ende. Kein Code, Hosttests unberuehrt. |
| 2026-11-07 | **[REL071](../TASK/REL071.md): NewTOS 0.7.1 veroeffentlicht (Tag `v0.7.1` auf `6baafc92`, GitHub-Pre-Release mit `NewTOS-0.7.1.zip` sha256 `801ed08a…`; Pi-Abnahme `measurements/measurement-2026-11-07-rel071-pi/RUNINFO.TXT`, Paket `measurements/measurement-2026-11-07-rel071-final2/RUNINFO.TXT`).** Warum: Stabilitaets-Release nach FS31, HW08, A140, A141, A142, DESK70. Pi des Anwenders aus dem ZIP: Version, CoreMark (7330,89), QED-Dialoge, HypView als Programm und Accessory, Dateiauswahl, Schatten, Rollbalken, Menues, Kopieren, FSWBENCH; mit dem Testkern `sd-fault` WFPROBE t/d/u/b auf D: (F: ist eine logische Partition, die WFPROBE nicht adressiert), alle bestanden, `fsck` sauber. Texte nach Anwender 2026-11-07: README/LIESMICH nur Betriebssystem und SDK, Programm-Aenderungen in `CHANGES.TXT` im Programmordner (`6baafc92`, `tools/release/package-release.sh` packt `docs/release/v<Version>/apps/<DIR>.TXT`). Issues #12, #21, #22, #23 beantwortet und geschlossen. Nicht in dieser Sitzung gefahren: Stecker-Test (Anwender: frueher getestet), X-Shot, alte Karte. Nebenbei WFPROBE seitenweise (`1211488a`); geparkt [A146](../TASK/A146.md). |
| 2026-11-07 | **[A141](../TASK/A141.md) (GitHub #23): HypViews Werkzeugleiste zeigt die Icons in den Farben des Zeichners, so nah wie die TOS-Palette erlaubt (`857cf10d`, Developer; `measurements/measurement-2026-11-07-a141-fix/RUNINFO.TXT`, Tester und Pi offen).** Warum: Anwenderentscheidung B2, die Icons sind fuer die NVDI-Palette gezeichnet, ihre Anpassung gehoert zur Portierung. Patch `sdk/overlays/hypview/rsc/recolour-cicons.py` im Bau (`sdk/apps/build-hypview.sh`), Upstream unveraendert, Kernel, AES und VDI unberuehrt: jedes Pixel der 8-Plane-Daten, Register -> Pen -> `nvdi.pal` -> naechstes Register der TOS-Standardpalette (quadrierter euklidischer Abstand in VDI-Einheiten, bei Gleichstand das kleinste Register). Paletten mit Herkunft im Patch-Verzeichnis, `tos-default.pal` aus `kernel/src/vdi_backend.rs`, gleich EmuTOS. 252 -> 7, 250 -> 17, 93 -> 227; 13 der 92 benutzten Register haben einen Gleichstand zwischen verschiedenen Farben (262 von 9408 Pixeln). Neuer Hosttest `tools/xtask/src/rsc_recolour_tests.rs`, rot mit identischer Abbildung, gruen mit Patch, alles ausser den 8-Plane-Daten bytegleich; `cargo xtask test` 2241/0/1. QEMU: freigegebene Icons pixelgleich zur Umrechnung, icprobe Orakel gegen NewTOS 59/0. Release-Texte 0.7.1: Punkt von den Einschraenkungen nach "Neu". Das Paket muss HypView neu bauen. |
| 2026-11-07 | **[REL071](../TASK/REL071.md): NewTOS 0.7.1 gepackt fuer die Pi-Abnahme, nichts veroeffentlicht (`8d3df21d` Version, `3e825c13` Texte; `measurements/measurement-2026-11-07-rel071-package/RUNINFO.TXT`, Developer).** Warum: Stabilitaets-Release nach FS31 (Datenverlust beim Kopieren und Verschieben zwischen Laufwerken), HW08, A140, A142, DESK70. Version aus der einen Quelle auf `0.7.1`, in QEMU zeigen Bootschirm, About und `ver` "NewTOS 0.7.1"; `cargo xtask test` 2239/0/1. Texte `docs/release/v0.7.1/` als Entwurf fuer den Anwender, FS31 vorn mit Pruefhinweis; FS31 steckt auch in 0.6.1 (`f5efac77` Vorfahre von `v0.6.1`). Paket mit `tools/release/package-release.sh` gegen das `v0.7`-Asset: `NewTOS-0.7.1.zip` sha256 `79b2fa04…`, 70 Eintraege, Namensliste gleich 0.7, alle Pruefungen PASS. Daneben, nicht im ZIP: Testkern `crash-walk,sd-fault` und `WFPROBE.NCP` fuer die WFPROBE-Faelle aus HW08; nur der Testkern antwortet `Ssystem(2091)`. Offen: Durchsicht der Texte, Pi-Abnahme mit Stecker-Test, danach Push/Tag/Release nach Go. |
| 2026-11-07 | **[DESK70](../TASK/DESK70.md) (GitHub #12): CoreMark haelt sein Ergebnis immer bis zur Taste, der Desktop bleibt wie EmuTOS (erster Bau `729bab3b`, zweiter Bau Commit dieser Zeile, Developer; `measurements/measurement-2026-11-07-desk70-build2/RUNINFO.TXT`, Tester des zweiten Baus und Pi offen).** Warum: Am Orakel gemessen wartet auch EmuTOS' Desktop nach einem Textprogramm nicht (`7d700cf3`, `docs/audits/2026-11-07-desk70-tos-exit.md`), also wartet das Programm selbst. `portable_fini` in `sdk/overlays/coremark/newtos/core_portme.c` fragt nach dem letzten Ergebnis, ausserhalb der Zeitmessung, mit `Crawcin` nach einer Taste. Der erste Bau wartete nur ausserhalb der Wurzel (`Dgetpath`), damit CORE/COREVAL aus `C:\AUTO` durchliefen (Tester PASS `measurements/measurement-2026-11-07-desk70-test/RUNINFO.TXT`); Anwenderentscheidung 2026-11-07: immer warten, ohne Test, Schalter oder Variante, denn AUTO ist fuer Treiber und CoreMark ein Programm vom Desktop. Deshalb ist `SD_COREMARK_AUTO` aus `tools/make-sd-image.sh` entfernt (kein lebendes Skript nutzte ihn); ein Messlauf startet CoreMark per `#Z 00 C:\COREMARK\CORE.NCP@` in `NEWDESK.INF`, liest die serielle Leitung und beendet QEMU (QEMU belegt). Nur `core_portme.c` geaendert (CoreMark-Regeln); Texte in `sdk/apps/build-coremark.sh`, `docs/benchmarks/coremark.md`, `docs/sdk/06-image-and-testing.md` angepasst. Kein Rust-Code, Hosttests nicht betroffen. |
| 2026-11-07 | **[A142](../TASK/A142.md) (GitHub #21): QED zeichnet Check-Boxen, Radio-Buttons und Alert-Icons wieder richtig, HypView sein QED.HYP-Logo (`2f6ea7eb`, `56fd81a1`, `6e43e35a`, Developer; Tester 2026-11-07 in QEMU abgenommen, `measurements/measurement-2026-11-07-a142-test/RUNINFO.TXT`, Pi offen).** Warum: cflibs `inline.rsh` haelt seine 44 Bildtabellen als Bytes in m68k-Folge, VDI und AES lesen native Woerter, also tauschten die beiden 8-Pixel-Haelften jedes Worts den Platz. Fix im Overlay: Worttabellen mit denselben Zahlen, Upstream unveraendert. Auf der Karte betrifft das nur `QED.NAP`. Dasselbe Muster aus einer Datei: einfarbige `.HYP`-Bilder gingen ungewandelt an `vr_trnfm`, Fix im HypView-Patch. Danach gefahrene Glyphen und Logo 0 Pixel Abweichung zum Orakel. `sdk/run-host-tests.sh` 36 -> 38 / 0 / 0; `cargo xtask test` nicht betroffen. |

## Notes

The project is intentionally over-documented at the start. The premise:
spec-first development against a fixed, finite, well-documented API
prevents the implementation drift that ruins long-running OS hobby
projects.
