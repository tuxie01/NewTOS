# FS21 — Quittung

Erledigt 2026-10-31. `Fclose` setzt `FA_ARCHIVE` wie EmuTOS `ixclose`
(`references/emutos/bdos/fsopnclo.c:530-549`): nur wenn die Datei schmutzig
ist (`Fcreate`, Verlaengern, erster Cluster, `Fdatime` setzen); das Bit wird in
das aktuelle Attribut geODERt. Endzustaende `gemdos-mc-fs20` und `-fs24` jetzt
identisch zum Orakel, FS20-Sonde identisch; keine neue Abweichung in den
`gemdos-mc-*`-Tabellen und `bios-r1`. Hosttests 1918/0/1 (+8,
`tos-api/tests/fs21_close_archive.rs`). Beleg
`measurements/measurement-2026-10-31-fs21/RUNINFO.TXT`.
