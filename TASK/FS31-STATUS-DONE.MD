# FS31 — Quittung DONE (2026-11-06)

`ixdel` vergleicht das Laufwerk, ein Loeschen trifft nur noch offene Dateien des eigenen Laufwerks, wie EmuTOS. Fix `20f87d1d`, 17 Tests (8 rot auf der Basis), Sonde `tosdump/probes/xdprobe.c`. Abnahme `measurements/measurement-2026-11-06-fs31-test/RUNINFO.TXT`: Orakel-Log bytegleich, NEWDESK Kopieren und Verschieben gefahren, vorher Datenverlust, jetzt richtig, keine Regression in den gefahrenen Szenen. Offen: FSWBENCH-Protokoll auf der Pi. Folge [FS32](FS32.md).
