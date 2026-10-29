# BIOS04 — Quittung: MIDI (3) und IKBD (4) antworten wie das Orakel

Datum 2026-10-29 · Beleg
[measurements/measurement-2026-10-29-bios04/](../measurements/measurement-2026-10-29-bios04/RUNINFO.TXT)
· Ticket [BIOS04](BIOS04.md) (Entscheidung des Anwenders 2026-10-29) · Spec
`docs/api/bios/{01,02,03,08}-*.md` (BIOS04-Notizen), `docs/api/gemdos/file/3f-fread.md`
Punkt 1, `docs/api/xbios/0c-midiws.md`

## Fix

- Geraetetabelle `tos-api/src/bios/chardev.rs:348@DEVICES`: Eintraege 3 und 4 sind
  nicht mehr `Absent`, sondern
  - `tos-api/src/bios/chardev.rs:163@MidiPort` — EmuTOS `bconstat3`/`bconin3`/`bcostat3`/
    `bconout3` mit MIDI-ACIA (Referenzbau, `references/emutos/include/config.h:994-996`):
    `Bconout(3)` 1 (Byte verworfen), Ausgabestatus -1; `Bconstat(3)` -1 solange der MIDI-Ring
    von `Iorec(2)` ein Byte haelt, sonst 0; `Bconin(3)` **wartet** auf ein Byte im Ring und
    liefert es null-erweitert (`references/emutos/bios/midi.c:22-95`).
  - `tos-api/src/bios/chardev.rs:236@IkbdPort` — `bcostat4`/`bconout4`
    (`references/emutos/bios/ikbd.c:916-937`): Ausgabestatus -1, `Bconout(4)` 1 (verworfen);
    Eingabe `char_dummy`, `Bconstat(4)`/`Bconin(4)` 0.
- Das Warten `tos-api/src/bios/chardev.rs:209@midi_read` hat den AUX-Stil (XBIOS15/XBIOS18):
  Eingabe gepumpt (`Host::pump_input`, Unterbrechungen kurz offen), kein Abgeben an andere
  Prozesse, auf dem Trap-Weg weicht es einem `kbdvec`-Haken aus (Neustart, nichts verbraucht).
  Ohne Host 0 (Bootsperre `CHARDEV_AVAILABLE`). Der Ring wird ueber
  `tos-api/src/xbios/iorec.rs:497@pop_midi` und `midi_pending` gelesen; kein Kern-Erzeuger
  fuellt ihn (E5), ein Byte darin hat ein Programm ueber den `Iorec(2)`-Record gelegt.
- Die `Bcostat`-Vertauschung bleibt in `tos-api/src/bios/chardev.rs:403@bcostat_handle`.
- `Midiws`/`Ikbdws`: beobachtbar unveraendert (`void`, kehren sofort zurueck); nur
  Kommentare in `tos-api/src/xbios/system.rs` und `tos-api/src/xbios/mod.rs`.
- GEMDOS: unveraendert, `Fread`/`Fwrite` erreichen nur die Geraete 0..2
  (`references/emutos/bdos/bdosmain.c:589-639`); in `3f-fread.md` Punkt 1 klargestellt.

Abweichung von der Wortlaut-Tabelle der Entscheidung, zur Kenntnis: „`Bconstat` 0 (kein
Eingang)“ fuer MIDI ist mit dem Ring umgesetzt, wie `bconstat3` es tut — 0 ohne Eingang,
-1 wenn ein Programm ein Byte in den `Iorec(2)`-Ring gelegt hat. Das Orakel antwortet genauso
(Sonde R02, beidseitig gleich); ohne den Ring haette das Warten von `Bconin(3)` nichts, worauf
es warten kann.

## Abnahme

- **Tabellen:** `bios-r1` 66/67 (`Bcostat(3)`/`(4)`) jetzt EQUAL gegen tt und falcontc; 63–65,
  68, 71 EQUAL wie vorher. **69/70 (`Bcostat(6)`/`(7)`) bleiben DIFF** — das ist BIOS-CORE-007
  (Bconmap, [BIOS08](BIOS08.md)), nicht dieses Ticket. Keine neue Abweichung in `bios-r1`,
  `bios-core-r1`, `xbios-chardev-r1`, `xbios-system-r1` (acht Vergleiche,
  `measurements/measurement-2026-10-29-bios04/runs/before-after.txt`).
- **Sonde (dauerhaft):** `tosdump/probes/mdprobe.c` (Bau `make -C tosdump mdprobe-m68k
  mdprobe-newtos`, Lauf `tosdump/probes/run-mdprobe.sh`). Das NewTOS-Log ist byte-gleich mit
  dem Orakel tt und falcontc: D06/D07 = Sonde B02/B03 des Audits (`Bconout` 1), D08/D09 =
  B11/B12 (`Bcostat` -1), R02–R05 Ringlesen, W01 `Bconin(3)` auf leerem Ring wartet nach
  10 s noch. Vorher NewTOS: 0/0, Ring nie gelesen, W01 kehrt sofort mit 0 zurueck.
  Eingaben per `bios-arg-trace` geprueft (alle Argumente wie im Quelltext).
- **`Bconin(3)`-Warten ist kein tosdump-Fall:** DUMPBIOS weist es ab (`tosdump/core/cases.c`
  `CR_BCONIN`, „waits for input“), ebenso `Bconout` an 3/4 (`CR_BCONOUT`). Deshalb Sonde plus
  Hosttest (Regel in CLAUDE.md „Testing“); das Folgeticket fuer die Tabellenzeilen ist
  [BIOS13](BIOS13.md) Punkt 7 (Round-1-Waechter).
- **Hosttests:** `cargo xtask test` 1847/0/1 (Basis 1843/0/1, +4): neu
  `tos-api/tests/bios04_midi_ikbd.rs` (Profil beider Geraete, Ringlesen in Reihenfolge und ueber
  das Ringende, Null-Erweiterung, Bootsperre, `Midiws`) und
  `tos-api/tests/xbios_kbdvecs.rs` `midi_wait_gives_way_without_side_effects` (Ausweichen bei
  wartendem und waehrend des Pumpens eintreffendem Hakenbyte, danach liefert der neu
  ausgefuehrte Aufruf das MIDI-Byte). Angepasst: `tos-api/tests/fs11_device_rw.rs`
  (`bios_absent_profile` ohne 3/4, mit den neuen Werten; der Tausch-Test setzt den anderen
  Platz auf `Absent`, sonst waere der Tausch mit -1/-1 unsichtbar).
- **SDK:** `sdk/run-host-tests.sh` 34/0/0. apitest: BC15/BC16 → 1, BC23/BC24 → -1; BC08 legt
  ein Byte in den `Iorec(2)`-Ring und liest es mit `Bconin(3)` (BC08a/BC08b Status davor/danach) —
  das alte BC08 (`Bconin(3)` → 0) haette jetzt fuer immer gewartet. Rauchlauf auf QEMU raspi4b:
  BIOS-CHR 27/0/5, alle BC-Pruefungen OK.
- **Bau:** beide Boards bauen; Warnungen unveraendert (rustc 93 kernel / 64 tos-api, clippy
  121 / 74), keine in den beruehrten Dateien.
- **doc-check:** keine Ankerzitat-Fehler in die beruehrten Quelldateien mehr; die durch die
  Zeilenverschiebung gebrochenen Anker in Spec, Audits und Tickets nachgezogen (siehe
  Bericht). Verbleibende Fehler in den bearbeiteten Dateien sind vorher schon da gewesen.

## Nachgezogen

- Spec: `docs/api/bios/01-bconstat.md`, `02-bconin.md`, `03-bconout.md`, `08-bcostat.md`
  (BIOS04-Notizen, Geraetetabellen), `docs/api/gemdos/file/3f-fread.md` Punkt 1 (GEMDOS
  erreicht nur 0..2; MIDI/IKBD nicht mehr „absent“), `docs/api/gemdos/file/40-fwrite.md`,
  `docs/api/xbios/0c-midiws.md` (die drei Stellen „Behaviour on the Pi“, Edge case 2,
  Kriterium „No-MIDI profile“, dazu Konfigurationsnotiz und FS11-Absatz),
  `docs/api/xbios/0e-iorec.md` (E5).
- **Ratsche (`docs/analysis/deps/mapping.tsv`):** `bconin3` (`midi_read`,
  `bconin_restartable`), `bconstat3`, `bcostat3`, `bcostat4`, `bconout4` von
  `design:hardware:*` auf `mapped` (`tos-api/src/bios/chardev.rs`); `bconout3` zusaetzlich
  auf `bconout` (MidiPort), Notiz der `midiws`-Zeile berichtigt. `cargo xtask deps
  --check-mapping`: 1048 Zeilen, alle aufgeloest.

Nicht gemessen: echte Pi 4; qemu-virt (nur gebaut); das Ausweichen des MIDI-Wartens vor einem
`kbdvec`-Haken Ende-zu-Ende (nur Hosttest; QEMU raspi4b hat keine USB-Tastatur).
