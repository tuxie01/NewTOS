# V24 — Quittung DONE (2026-11-06)

`v_get_pixel` ausserhalb des Schirms liefert wie EmuTOS `pixelread` (`vdi/vdi_fill.c:671-681`)
das umgelaufene Pixel am Index `y*stride+x`; jenseits des logischen Schirms 0
(Anwenderentscheidung B, `docs/COMPATIBILITY.md`). Gleiche Adressregel wie die Linien aus V20.
`v_contourfill` sucht wie EmuTOS nur im Clip und sieht den Umlauf nicht. Fix `caf77fab` (Spec
`docs/api/vdi/inquiry/_color-pixel-input.md`, 6 Tests). Abnahme
`measurements/measurement-2026-11-06-v24-test/RUNINFO.TXT`: 15/18/21/24 und neue Umlaufpunkte K16A
orakelgleich, alles andere unveraendert, beide Boards. Offen: Pi (Zeilenlaenge = Breite?).
