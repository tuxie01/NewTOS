# BIOS08 — Quittung: BIOS-Handles ab 6 laufen ueber die `Bconmap`-Tabelle mit einem Eintrag wie EmuTOS

Datum 2026-10-30 · Beleg
[measurements/measurement-2026-10-30-bios08/](../measurements/measurement-2026-10-30-bios08/RUNINFO.TXT)
· Spec `docs/api/bios/{01,02,03,08}-*.md` (je Abschnitt „[BIOS08]“ in den
NewTOS-Notizen) · Ticket [BIOS08](BIOS08.md), Profil aus
[XBIOS05](XBIOS05.md) Option 1 (Lead 2026-10-30)

- **Weiche wie EmuTOS** (`tos-api/src/bios/chardev.rs:411@device`): EmuTOS
  nimmt in allen vier Aufrufen jeden Handle ab 6 vor der Vektortabelle aus
  `bconmap_root` (`references/emutos/bios/bios.c:923-928`, `:967-972`,
  `:1000-1005`, `:1152-1157`): `map_index = handle - 6` als `WORD`,
  `>= maptabsize` → 0, `>= 0` → Eintrag, sonst Tabellenplatz, negativ → 0.
  `device` macht dasselbe; der Eintrag nennt seinen Port ueber
  `tos-api/src/xbios/bconmap.rs:109@entry_device` (Eintrag 0 = Geraet 1 = AUX,
  EmuTOS `maptable_mfp`, `references/emutos/bios/serport.c:98,972`). Die
  Geraetetabelle endet bei 5 (`NDEV` 6; `install_device(6..)` → `None`).
- **Ergebnis:** Handle 6 antwortet in `Bconstat`/`Bconin`/`Bcostat`/`Bconout`
  wie Handle 1, ueber denselben AUX-Eintrag (raspi4b: 0 / 0 sofort / -1 / 1;
  qemu-virt: die serielle Leitung, `Bconin(6)` wartet ueber den Trap-Eintrag
  und weicht einem `kbdvec`-Haken aus wie `Bconin(1)`); `Bcostat(6)` ohne
  den 3/4-Tausch. Handles 7, 8, 9 und hoeher sowie negative: 0, kein Geraet
  wird aufgerufen.
- **Spec zuerst:** `docs/api/bios/{01,02,03,08}-*.md` — EmuTOS-Tabellen um den
  `BCONMAP_AVAILABLE`-Zweig berichtigt (der Referenzbau hat ihn auf jeder
  Maschine; die Slots 6/7 der Vektortabellen werden dort nie erreicht),
  BIOS08-Notiz und Pi-Tabelle je Funktion, in 08 die Orakeltabelle der
  vier Faelle. `docs/COMPATIBILITY.md` „`Bconmap`: one entry“: drei
  `Bcostat`-Zeilen und `Bconout(7)` als ACCEPTED (Portzahl), der Satz „The
  BIOS handles 6..9 are the device table's“ durch die BIOS08-Regel ersetzt.
  Kopien der alten Aussage („6/7 unused/absent“) nachgezogen:
  `docs/api/xbios/2c-bconmap.md`, `docs/api/bios/_state.md`,
  `docs/api/gemdos/file/3f-fread.md` (Punkt 1, Tabelle jetzt sechs Eintraege),
  `docs/api/bios/_dispatcher.md`, `docs/api/bios/_full-function-inventory.md`,
  `docs/HARDWARE-PROFILE.md`; Moduldoku `tos-api/src/xbios/bconmap.rs`.
- **Tests:** neu `tos-api/tests/bios08_bconmap_handles.rs` (7): Handle 6 = 1
  mit dem eingebauten AUX, Handle 6 erreicht einen Treiber in Platz 1 (alle
  vier Aufrufe, `Bconin` ueber den Trap-Eintrag), `Bconin(6)` weicht aus wie
  `Bconin(1)`, `Bcostat(6)` ungetauscht, 7..11/`0x7FFF`/-1/-2/-32768 → 0 ohne
  Geraeteaufruf, Tabelle endet bei 5, nach `Bconmap(6)`/`Bconmap(7)`
  unveraendert. `tos-api/tests/fs11_device_rw.rs` `bios_absent_profile`:
  Handle 6 aus der Abwesend-Schleife in die AUX-Pruefung. apitest
  (`sdk/apps/apitest/apitest.c`) BC17 `Bconout(6)` → 1, BC18 `Bconout(7)` → 0,
  Kommentar; gebaut (Scratch), nicht gelaufen.
- **Ratsche** `docs/analysis/deps/mapping.tsv`: je `bconstat`, `bconin`,
  `bconout`, `bcostat` (`bios/bios.c`) zwei Zeilen `device` und
  `entry_device` (+8), Zeilennummern der Trap-Eintraege in den Notizen
  aktualisiert. `deps --check-mapping` 1063 Zeilen gruen, `--up` der vier
  vollstaendig.
- **Verschobene Anker** (nur Zeilennummern) in Spec-, Audit- und TASK-Dateien
  nachgezogen (53 Stellen in `TASK/BIOS04*.MD`, `TASK/BIOS08.md`,
  `TASK/BIOS09.md`, `TASK/BIOS11.md`, `TASK/MEM12.md`,
  `TASK/XBIOS05-STATUS-DONE.MD`, `TASK/XBIOS17.md`, `docs/COMPATIBILITY.md`,
  `docs/api/bios/0{1,2,3,8}-*.md`, `docs/api/gemdos/file/3f-fread.md`,
  `docs/api/xbios/0c-midiws.md`, `docs/api/xbios/0e-iorec.md`,
  `docs/api/xbios/2c-bconmap.md`, `docs/audits/2026-10-28-bios-core.md`,
  `docs/audits/2026-10-29-xbios15-serial-rx.md`).

## Abnahme

- `bios-r1` 69 `Bcostat(6)`: EQUAL gegen tt und falcontc (vorher 0 gegen -1).
- `bios-r1` 70 `Bcostat(7)`, `bios-core-r1` 65 `Bcostat(8)`, 66 `Bcostat(9)`
  (nur tt): NewTOS 0, Orakel -1 — die Ports SCC B / TT-MFP / SCC A, die die
  Orakelprofile haben (TT 4 Eintraege, `references/emutos/bios/serport.c:993`;
  Falcon 3, `:981`). Derselbe ROM liefert auf einer Maschine mit einem Eintrag
  0: `map_index >= maptabsize` (`references/emutos/bios/bios.c:1153-1154`),
  `maptabsize = 1` ausser Falcon/MSTe/ARAnyM/TT (`serport.c:965`). ACCEPTED
  als Geraeteparameter in `docs/COMPATIBILITY.md`. `bios-core-r1` 66 gegen
  falcontc EQUAL (beide 0).
- Sonde der Audit-Messung (`bcprobe.c`, unveraendert kopiert): B04
  `Bconout(6,'A')` 0 → 1 = EmuTOS tt/falcontc; B05 `Bconout(7,'A')` 0 → 0
  (EmuTOS 1, Port SCC B, ACCEPTED). Sonst gleich vorher bis auf die
  T01-Schleifenzahl. Haengevariante (Handles 0, 1, 6, 7) auf raspi4b: alle
  kehren mit 0 zurueck, vorher und nachher — `Bconin(6)` = `Bconin(1)` auf
  diesem Board (AUX ohne Kabel).
- Kein vorher gleicher Fall neu verschieden in `bios-r1`, `bios-core-r1`,
  `xbios-chardev-r1`, `xbios-system-r1`, `xbios-bconmap-r1` (zehn Vergleiche,
  vorher = `git archive 60824ec8`): bios-r1 tt 33 → 32, falcontc 38 → 37
  (69 weg), sonst gleich
  (`measurements/measurement-2026-10-30-bios08/runs/before-after.txt`).
- `cargo xtask test` 1888/0/1 (Basis 1881/0/1: +7
  `tos-api/tests/bios08_bconmap_handles.rs`). Ein erster Volllauf scheiterte
  an `fs11_device_rw::bios_absent_profile` (pinnte `Bconout(6)` = 0, das
  alte Profil); nachgezogen, zweiter Volllauf gruen.
- Bau beider Boards (release) 159 Warnzeilen wie Basis (kernel 93 /
  tos-api 64); Clippy kernel 121 / tos-api 74 auf beiden Boards wie Basis;
  keine Warnung in `chardev.rs`/`bconmap.rs`; Clippy `tos-api --tests` ohne
  Warnung in den beruehrten Dateien.
- `cargo xtask doc-check` der geaenderten Doku: keine Fehler auf geaenderten
  Zeilen (Altbestand der Dateien unveraendert, siehe Bericht).

## Offen / nicht geprueft

- qemu-virt nur gebaut und gelintet, nicht gebootet (Tester): `Bconin(6)`
  wartet dort auf ein serielles Byte wie `Bconin(1)` (`tosdump/probes/axprobe.c`
  mit Handle 6 waere die Messung); `Bconout(6)` 1.
- Echte Pi 4: keine hardwarespezifische Aenderung; nicht gelaufen.
- apitest BC17/BC18 nur gebaut, nicht auf NewTOS gelaufen.
