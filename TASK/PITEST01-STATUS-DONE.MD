# PITEST01 — Quittung DONE (2026-11-02)

## Getan

- Teil 1: Debug-Feature `crash-nowait`, `P0REP.NAP`, `HZPROBE.NAP`
  (`measurements/measurement-2026-10-31-pitest01-part1/RUNINFO.TXT`).
- Teil 2: Kartenabgleich `tools/sync-card.sh`, Ablaufblatt
  `docs/analysis/tester-run-sheet-2026-10-31.md`
  (`measurements/measurement-2026-10-31-pitest01-part2/RUNINFO.TXT`).
- Teil 3: eine Karte, Sicherung vor dem Schreiben, `cmdline.txt` je Kern
  (`measurements/measurement-2026-10-31-pitest01-part3/RUNINFO.TXT`).
- Pi-Laeufe A, B, C1:
  `measurements/measurement-2026-10-31-pitest01-phase-a/RUNINFO.TXT`,
  `measurements/measurement-2026-10-31-pitest01-phase-b/RUNINFO.TXT`,
  `measurements/measurement-2026-10-31-pitest01-c1/RUNINFO.TXT`.

## Abnahme (Ticket "Abnahme")

- QEMU: P0PAR-Dauerlauf und HZPROBE ohne Eingriff, Protokoll geschrieben;
  Release-Kern byte-gleich (Teil 1).
- Abgleichskript gegen ein Scratch-Abbild, Manifest mit `host_clang` und
  Kern-Commit, verfaelschte Datei erkannt (Teil 2).
- Hosttests gruen (1918/0/1 zum Zeitpunkt von Teil 1).
- Dazu auf der echten Pi: P0REP `PASS 12000/12000`, HZPROBE und AWPROBE PASS.

## Offen

- Funde mit eigenem Ticket: [DOOM01](DOOM01.md) (zurueckgestellt 2026-11-02),
  [PROC08](PROC08.md).
- HypView-Startseite und zView-Fensterschliessen: Anwender 2026-11-02, kein
  Befund bzw. nicht nachvollziehbar (PITEST01 "Abschluss").
