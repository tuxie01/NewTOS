# TOOL07 — Quittung: DUMPBIOS-Plattenrunde, Abschlusstabellen

Datum 2026-10-28 · Belege
[measurements/measurement-2026-10-28-tool07/](../measurements/measurement-2026-10-28-tool07/RUNINFO.TXT)
(Etappe 1) und
[measurements/measurement-2026-10-28-tool07b/](../measurements/measurement-2026-10-28-tool07b/RUNINFO.TXT)
(Rest) · Ticket [TOOL07](TOOL07.md)

- Etappe 1 (`bios-r2.tsv`, 51 Faelle, `Drvmap` EXACT, Karte mit
  Testpartition): siehe Beleg tool07; BIOS-STOR-001, -003, -007 DIFF.
- Format nach den Anwenderentscheidungen 2026-10-28 (Variante B, Lead-Wahl
  „Byte n setzen“), `docs/TOS-REFERENCE-DUMPER.md` Abschnitt 6 „The closing
  tables“: Kopf `!disk<TAB>2<TAB>destructive`; Vorspann
  `+set<TAB><offset>=<byte>[,…]` nur in zerstoerenden Tabellen, direkt nach
  dem Lese-Vorspann, danach nur der Write-back von „gelesen + gesetzt“;
  Einheiten-Token `u:0` (einzige Zulassung von `RW_NOTRANSLATE`, nur lesend),
  im Datensatz `0xB0F40000`.
- Einheit aus der Profildefinition, nicht einkompiliert (Abschnitt 2.1 laesst
  nur die Bytereihenfolge als Unterschied der Builds zu): das Profil legt
  `D:\DISKUNIT.TXT` hin — `tools/hatari-oracle.sh disk` 18,
  `cargo xtask tosdump --disk` 26 (`NEWTOS_DISK_UNIT`); DUMPBIOS liest die
  Datei nur fuer eine Tabelle mit `u:0` und verweigert ohne sie den Lauf.
- Runner-Regel: zerstoerende Tabelle nur auf frischem Klon. `hatari-oracle.sh`
  verweigert sie ausserhalb von Profil `disk` und bei einem Abbild, dessen
  Partition 1 `fsck_msdos -n` nicht besteht; `cargo xtask tosdump` verweigert
  sie ohne `--disk`/`--testdisk` und bei einem nicht sauberen `--testdisk`,
  behaelt `testpart-after.img` fuer den Endzustandsvergleich. Der Dumper selbst
  kann frisch/benutzt nicht erkennen, er protokolliert die Markierung.
  Verweigerungen gemessen
  (`measurements/measurement-2026-10-28-tool07b/runs/refusals.txt`).
- Neue Tabellen und Referenzen `EMUTOS-1a7484ab-BIOS-r2-{phys,x004,x005,x006}-disk`
  (Orakel je zwei Laeufe bytegleich, Selbstpruefung bestanden). NewTOS:
  phys 11/12 DIFF (BIOS-STOR-002, Einheit 26 → EUNDEV, erwartet),
  x004 7/10 DIFF (BIOS-STOR-004: `Getbpb` liefert BPB statt 0, Lesen Daten
  statt ESECNF), x005 3/13 DIFF (BIOS-STOR-005: numcl 32672/B_16 statt 0/0),
  x006 6/11 DIFF (BIOS-STOR-006: logisches `Rwabs` Daten statt ESECNF);
  dazu BIOS-STOR-001 (`Mediach` nach Schreiben von Sektor 0) in jeder Tabelle.
  Funde, kein Fix.
- Endzustand: fuer x004, x005, x006 enden Orakel und NewTOS bytegleich in der
  Testpartition (x004 nur Byte 0x0C von Sektor 0, x006 nur Byte 0x16, x005
  wiederhergestellt); phys laesst das Abbild auf beiden Seiten unveraendert.
- Kanarienvogel des Plattenprofils (ORACLE §6) auf `bios-r2.tsv` umgestellt
  (`measurements/measurement-2026-10-28-tool07b/method/canary-disk.sh`):
  Referenz mit dem geaenderten Dumper zweimal bitgleich, VDI-Kanarien und
  alter bios-r1-Kanarienvogel weiter OK.
- Tests: `make -C tosdump host-test` gruen (29 Referenzen in der
  Selbstpruefung), `cargo xtask test` 1751/0/1, doc-check 0 Fehler.
- Offen bzw. nicht Teil: letzter physischer Geraetesektor (Geraetegroessen
  verschieden); `tools/biostrace-check.py` kennt `+set` und `u:0` nicht.
