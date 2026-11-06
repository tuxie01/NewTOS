# V28 — Quittung DONE (2026-11-06), schliesst auch V36

`vrt_cpyfm` in Speicherformen mit 1..8 Ebenen planar wie EmuTOS (`planar_blit`, Op je Ebene,
`MAP_COL`), Zeilenlaenge aller Speicherformen `fd_wdwidth * fd_nplanes` (`setup_info`); die
V26-Bindung nutzt sie. `mem_to_mem` kopiert nach der Startzeiger-Regel von `vro_cpyfm16`
(`vdi/vdi_raster.c:989-1002`) — damit ist [V36](V36.md) erledigt. Fix `f1006588` (Spec
`docs/api/vdi/raster/_raster.md`, 30 Tests). Abnahme
`measurements/measurement-2026-11-06-v28-test/RUNINFO.TXT`: 20/26 orakelgleich, 6 nur hinter dem
Formularende (V26-Grenze), Ueberlapp-Probe TRA–TRM gleich, AES/Desktop unveraendert, beide Boards.
