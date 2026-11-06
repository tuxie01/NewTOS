# V25 — Quittung DONE (2026-11-06)

`curve` steigt wie EmuTOS `gdp_curve` (`vdi/vdi_gdp.c:395-402`) aus, wenn der Clip die Kurve nach
`xc ± xrad`, `yc ± yrad` ganz ausschliesst — ohne Linienbreite und Pfeilenden, strenge Vergleiche,
16-Bit-Summen, nur bei Clip an. VDI-DRAW-011 war schon durch V20 erledigt. Fix `294272e2` (Spec
`docs/api/vdi/output/_gdp.md`, 7 Tests; TOOL32-Kandidat 11 "wohl nicht beobachtbar" widerlegt).
Abnahme `measurements/measurement-2026-11-06-v25-test/RUNINFO.TXT`: `vdi-v18.tsv` 0/19, `vdprobe`
281 Faelle nur K5D (Entscheidung C) abweichend, AES/Desktop unveraendert, beide Boards.
