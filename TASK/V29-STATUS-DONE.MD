# V29 — Quittung DONE (2026-11-06)

`v_justified` zeichnet bei `chup` 900/1800/2700 gedreht wie EmuTOS `gdp_justified` → `output_text`
(`vdi/vdi_text.c:1177-1289`, Deltas je Zeichen `:476-492`). Fix `b4e419e5` (Spec
`docs/api/vdi/output/_gdp.md` "Justified-text rendering", 20 Tests; Test-Double `draw_glyph_sized`
zeichnet jetzt wie der Kernel). Abnahme `measurements/measurement-2026-11-06-v29-test/RUNINFO.TXT`:
99/102/105 und T1610–32 orakelgleich, alles andere bytegleich zur Basis, beide Boards.
Nebenbefunde [V37](V37.md), [V27](V27.md).
