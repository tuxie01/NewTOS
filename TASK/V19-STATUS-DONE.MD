# V19 — Quittung DONE (2026-11-05)

`vro_cpyfm` Speicher→Speicher mit gleich vielen Ebenen 1..8 kopiert planar wie EmuTOS
`cpy_raster`/`bit_blt` statt 16-Bit-chunky; nichts mehr hinter die Zielform (VDI-DRAW-007,
S1). Fix `506643fb` (Spec `docs/api/vdi/raster/_raster.md`, `planar_mem_to_mem`,
17 Orakel-Hosttests + 3 Unit-Tests, mapping-Zeilen `bit_blt`/`fast_bit_blt`/`setup_pattern`/
`setup_info`). Abnahme `measurements/measurement-2026-11-05-v19-test/RUNINFO.TXT`:
`vdi-v18-raster.tsv` 0/17, `PAT_FLAG`-Probe 0/144, AES/Desktop unveraendert, beide Boards.
Offen ausserhalb: NewTOS beschneidet an `fd_w × fd_h`, EmuTOS liest beide nicht (gleich fuer
jeden Block in der Form); `fd_nplanes <= 0` kopiert nichts (EmuTOS schreibt unbegrenzt).
Nebenbefunde → [V26](V26.md) (Speicherweg greift in ungepruefte Folgezeile),
[V27](V27.md) (`rect_from_ptsin` Ueberlauf), V18 Teil 2 (`vrt_cpyfm` Speicherziel 1..8 Ebenen,
`vro_cpyfm` 9..15 Ebenen).
