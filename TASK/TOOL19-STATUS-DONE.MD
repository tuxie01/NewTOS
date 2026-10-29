# TOOL19 — Quittung: `DISKUNIT.TXT` und `PROGEXT.TXT` aus einer Profildefinition, aufgezeichnet, Abweisung ohne Standardwert

Datum 2026-10-29 · Beleg
[measurements/measurement-2026-10-29-tool19/](../measurements/measurement-2026-10-29-tool19/RUNINFO.TXT)
· Spec `docs/TOS-REFERENCE-DUMPER.md` Abschnitt 6, "The profile values"
· Ticket [TOOL19](TOOL19.md); `PROGEXT.TXT` (TOOL20) nach denselben drei Bedingungen

- **Eine Definition:** `tosdump/profiles.tsv` -- je Runner und Profil
  Zeigerbreite, Wortreihenfolge, Einheit, Programmendung
  (`hatari-oracle disk`: 4, 1, 18, `PRG`; `xtask-tosdump disk`: 8, 2, 26,
  `NAP`). Die beiden Konstanten in `tools/xtask/src/tosdump.rs` und die
  `printf '18'`/`'PRG'` in `tools/hatari-oracle.sh` sind entfernt; beide
  Runner lesen die Zeile zur Laufzeit, tosdiff uebersetzt sie
  (`tosdump/host/mkprofiles.awk` erzeugt `profiles.h`; eine fehlerhafte
  Datei bricht den Bau ab). Ein Profil ohne Zeile nennt keinen Wert.
- **(a)** Keine handgeschriebene Kopie: `measurements/*/method` (TOOL07b,
  TOOL20) nennen die Dateien nur in Kommentaren, keine Tabelle traegt eine
  Einheit oder Endung (nur die Token `u:0`/`x:`; `Pexec` nur mit `x:`).
- **(b)** Aufgezeichnet: Orakel in `<out-dir>/profile.txt` (Zeile, sha256
  der Definition, was die Tabelle braucht, beide Werte nach dem
  Zuruecklesen, LOG-Pruefung); `cargo xtask tosdump` in `tosdump-run.txt`
  (dasselbe, Zuruecklesen von der Karte vor dem Start). Das Dumper-LOG
  schrieb die Werte schon (`unit token u:0 = ...`, `program extension
  x:<path>.<EXT> ...`) -- Format unveraendert.
- **(c)** Abweisungen, gemessen:
  - Orakel: `u:0`-Tabelle im Profil `tt`, `x:`-Tabelle in `falcontc` --
    Exit 2 vor Hatari; Wert liest nicht zurueck -- Exit 2; LOG ohne den
    Profilwert -- BIN wird nicht kopiert, Exit 1.
  - `cargo xtask tosdump`: `u:0`- oder `x:`-Tabelle ohne `--disk` --
    nichts gebaut; Datei fehlt auf der Karte oder falsche Bytes -- vor QEMU;
    LOG ohne den Profilwert (`tosdiff --check-profile`) -- Lauf FAILED.
  - tosdiff: neuer Modus `--check-profile CASES.TSV DUMP.BIN`, dieselbe
    Pruefung vor jedem Vergleich und jedem Selfcheck einer Tabelle mit
    `u:0`/`x:`: LOG fehlt, ist nicht der Lauf des Dumps (Zeiger,
    Wortreihenfolge, Tabellen-CRC), Wert fehlt, doppelt oder nicht der der
    Profilzeile der Maschine des Dumps (Kopf, Bytes 20-23), Format 2 oder
    Maschine ohne Zeile -- Exit 2.
  - Positiv: Orakel-Lauf und NewTOS-Lauf von `bios-r2-phys.tsv` nehmen
    18 bzw. 26 auf und bestehen; Orakel-BIN gleich der gepinnten Referenz bis
    auf die Build-ID; NewTOS 11 von 12 verschieden wie in jedem frueheren
    Lauf (offener Fund, nicht diese Aenderung).
- **Referenzen:** keine Auswirkung. LOG-Format unveraendert; die zwei
  gepinnten Referenzen mit `u:0`/`x:` bestehen die Pruefung wie committet
  (Selfcheck 40 Referenzen gruen). `cargo xtask tosdump` kopiert das LOG
  jeder Referenz neben sie in `cmp`.
- **Tests:** `make -C tosdump host-test` gruen (neu in
  `tosdump/tests/test_tosdiff.sh`: 18 Profilfaelle auf Kopien der gepinnten
  Referenzen, 8 zur Definition); `cargo xtask test` 1833/0/1 (1828 + 5 neue
  in `tools/xtask/src/tosdump.rs`). Kein NewTOS-Produktcode geaendert.
- Nebenbei: Zeilenanker in `TASK/TOOL11.md`, `TOOL15.md`, `XBIOS17.md`
  nachgezogen (durch die Einfuegungen verschoben).
