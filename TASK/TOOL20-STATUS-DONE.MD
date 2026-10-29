# TOOL20 — Quittung: Sondenfaelle `gcprobe`/`pxprobe` und die FS22-Zusatzpfade als tosdiff-Tabellen

Datum 2026-10-29 · Beleg
[measurements/measurement-2026-10-29-tool20/](../measurements/measurement-2026-10-29-tool20/RUNINFO.TXT)
· Spec `docs/TOS-REFERENCE-DUMPER.md` Abschnitt 6, "GEMDOS after a media change"
· Ledger `tosdump/cases/COVERAGE-BIOS.md`, gleicher Titel
· Ticket [TOOL20](TOOL20.md), [FS22](FS22.md) Nachtrag Lead 2026-10-29

- **Drei neue Tabellen** (DUMPBIOS, `!disk 2 destructive`, ein gemeinsames
  Testabbild mit `C:\PXPROBE.PRG`, `C:\PXPROBE.NAP`, `C:\DELME.TXT`,
  `C:\SUB\INNER.TXT`):
  `tosdump/cases/gemdos-mc-fs20.tsv` (17 Faelle, CRC `55B17174`, gcprobe
  A01-B08), `gemdos-mc-fs22.tsv` (13, `26353FB1`, pxprobe P00-F07),
  `gemdos-mc-paths.tsv` (49, `39FDDD95`: Dateiauswahl-Listing Wurzel und
  Ordner, `Dsetpath`, `Dgetpath`, `Dsetdrv`/`Dgetdrv`, `Fdelete` nach einem
  Medienwechsel).
- **Waechter:** GEMDOS-Aufrufe nur in destruktiven Plattentabellen; jede
  aufgehobene Abweisung im Ledger mit Begruendung: `Dsetdrv`, `Dgetdrv`,
  `Fsetdta`, `Fgetdta`, `Dfree`, `Dsetpath`, `Fcreate`, `Fclose`, `Fwrite`,
  `Fdelete`, `Dgetpath`, `Pexec`, `Fsfirst`, `Fsnext`. Regeln
  (`gdos_refusal`, `tosdump/core/cases.c`): Pfade nur auf dem Testlaufwerk;
  Datei-Klammer `Fcreate ... Fclose(t:1)`; Laufwerks-Klammer
  `Dsetdrv(Test) ... Dsetdrv(t:2)` mit dem Wert eines `Dgetdrv` davor; DTA =
  ein Dumper-Puffer, den keine andere Zeile nennt; `Pexec` nur Modus 0;
  nicht in einer `Super`-Klammer, nie als Vorstufe.
- **Neu im Format:** Argumente `s:` (Zeichenkette), `c:` (Kommandozeile mit
  Laengenbyte), `x:` (Programmpfad; die Endung nennt das Profil in
  `PROGEXT.TXT`: Orakel `PRG` ueber `tools/hatari-oracle.sh`, NewTOS `NAP`
  ueber `cargo xtask tosdump`), Token `t:1`/`t:2`, Capture `d:<layout>`
  (die DTA). Layout-Grammatik: `:packed`, Feld `x<n>` (opak, kein Slot),
  Felder `b<n>`/`w<n>`/`L<n>`. DTA-Layout als `tosdump-layout`-Block in
  `docs/api/structures/dta.md`.
- **Orakel:** je Tabelle zwei Laeufe, BIN und LOG bytegleich; Selfcheck
  besteht; Werte = Sondenlogs von FS20/FS22. Referenzen
  `tosdump/reference/EMUTOS-1a7484ab-BIOS-mc-<fs20|fs22|paths>-disk`.
- **NewTOS** (`cargo xtask tosdump --testdisk`, QEMU raspi4b): fs20 0 von
  17 verschieden, fs22 6 von 13, paths 12 von 49. Erwartet war EQUAL; die
  Abweichungen sind Funde, nicht behoben:
  - F1 `d_fname` hinter dem NUL: NewTOS fuellt die 14 Bytes mit Nullen
    (`tos-api/src/bdos/fs/find.rs:462@makbuf`), EmuTOS schreibt nur Name +
    NUL (`packit`, `bdos/fsdir.c:1297-1324`) -- alle Abweichungen von fs22
    und 11 von paths.
  - F2 `Dsetpath` nach einem Medienwechsel: EmuTOS trifft den Wechsel nicht
    (`Mediach` danach 2), NewTOS meldet C: neu an (0) -- paths Fall 67.
  - Endzustand fs20: 1 Byte, `FA_ARCHIVE` von `GCPROBE.DAT` (offener Fund
    FS21). Endzustand fs22: identisch.
  - F3 Endzustand paths: beim Orakel steht der Verzeichniseintrag von
    `C:\DELME.TXT` nach dem Lauf noch unbeloescht auf der Platte (Cluster
    frei, `fsck_msdos` 206), NewTOS hat ihn geloescht. Offen, ob das die
    Schreibpolitik von EmuTOS ist oder nur der Zeitpunkt, zu dem das Orakel
    angehalten wird -- Frage an den Lead.
- **Bleibt Sonde:** die 21 reservierten DTA-Bytes ueber `Pexec` und
  `Fgetdta() == &dta` (systemeigen bzw. Adresse), gcprobes Textmarke, die
  Eingabe-Traces der Sonden (`tools/biostrace-check.py` liest weder `+set`
  noch Zeichenketten; GEMDOS wird nicht getract). Accessory-Lader und
  AUTO-Ordner laufen nur beim Start: kein Fall moeglich.
- **Kein Rueckschritt:** Kanarien `canary-disk` (bios-r2) und `canary-ptr`
  (xbios-ptr-r1) aus diesem Baum: BIN+LOG gleich den gepinnten Referenzen.
- **Tests:** `make -C tosdump host-test` gruen (neu `test_cases_gdos`,
  `test_runner_gdos`; Selfcheck 37 Referenzen); `cargo xtask test`
  1821/0/1 (= Ausgangsstand). Kein NewTOS-Produktcode geaendert.
