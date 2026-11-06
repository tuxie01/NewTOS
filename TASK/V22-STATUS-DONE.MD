# V22 — Quittung DONE (2026-11-06)

`v_fillarea` (und `fill_polygon`) zeichnet mit 1 und 2 Ecken wie EmuTOS `polygon` (keine
Mindestzahl, `vdi/vdi_fill.c:560-610`); 0 Ecken zeichnet nichts. >= 1024 Ecken: sauber
geschlossen, Anwenderentscheidung C mit Eintrag in `docs/COMPATIBILITY.md`. Fix `7108bdff`
(Spec `docs/api/vdi/output/_fill.md` ergaenzt, 12 Tests). Abnahme
`measurements/measurement-2026-11-06-v22-test/RUNINFO.TXT`: 51/53 und K4A–H orakelgleich,
K5D = Entscheidung C, winzige GDP-Kurven unveraendert gleich, AES/Desktop unveraendert, beide
Boards. Nebenbefund negatives `CONTRL[1]` → [V23](V23.md).
