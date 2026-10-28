# TOOL06 — Quittung: Orakel-Plattenprofil `disk`

Datum 2026-10-28 · Beleg
[measurements/measurement-2026-10-28-tool06/](../measurements/measurement-2026-10-28-tool06/RUNINFO.TXT)
· Ticket [TOOL06](TOOL06.md)

- `tools/hatari-oracle.sh disk` (mit `HATARI_ORACLE_DISK=<Abbild>`): Falcon030,
  VGA, Floppys aus, das Testabbild als Klon im Ausgabeordner als IDE-Master
  = BIOS-Laufwerk C:, `--ide-swap 0=off`; der Dumper auf Hataris
  GEMDOS-Laufwerk D:. Gemessen `Drvmap` = `0x0000000C`. `tt` und `falcontc`
  unveraendert: Kommandozeile gleich, §6-Kanarienvogel dreimal OK mit den
  gepinnten sha256; `tools/oracle/*` nicht angefasst.
- `tools/mktestdisk.sh`: FAT16-Testabbild (64 MiB, MBR Typ 0x06 ab LBA 63,
  Geometrie und BPB wie die `mkcard`-Standardkarte, feste Seriennummer,
  Label und Zeitstempel) bytegleich reproduzierbar, mit Manifest ueber
  `tools/sd-manifest.sh`; `fsck_msdos -n` sauber.
- EmuTOS liest das Abbild selbst (`--trace ide,bios,gemdos`): IDE-Lesen auf
  MBR, Bootsektor und Wurzelverzeichnis nach `Getbpb(2)`/`Rwabs`, kein
  IDE-Schreibbefehl, Abbild-sha256 vor = nach. Hatari beantwortet auf C:
  nur das erste `Fopen("C:\EMUDESK.INF")` aus seinem `--auto`-INF-Override.
- Kontrollen: ohne IDE-Master nur D:; Floppys an → `Drvmap` 0xF;
  `--ide-swap` auto/on ohne Wirkung auf die 147 Faelle; Programm auf einem
  zweiten IDE-Abbild startet nicht (`--auto` braucht Hataris
  GEMDOS-Emulation).
- Kanarienvogel des Profils (ORACLE §6): `bios-r1.tsv` ohne die Zeilen 112
  (`Random`, Startwert aus `hz_200`) und 125 (`Gettime`), die von Lauf zu
  Lauf schwanken; 145 Faelle, fuenf Laeufe bytegleich, Lead-Gegenlauf OK.
- `cargo xtask test` 1726/0/1; doc-check ORACLE.md 0 Fehler.
- Offen beim Anwender: mtools kommt aus PATH, nicht aus `oracle.env` (die
  sha256-Pruefung des Abbilds faengt eine andere Version ab); Videomodus des
  Profils ist der Falcon-Standard 0x001A, nicht Truecolor.
