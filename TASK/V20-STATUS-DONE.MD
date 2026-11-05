# V20 — Quittung DONE (2026-11-05)

`abline` zeichnet mit dem Rechenweg von EmuTOS (`vdi_line.c`): senkrechte Segmente ohne
XOR-Kuerzung, Clip ueber die Endpunkte (`clip_line`, `mul_div_round`), bei `vs_clip(0)` Umlauf in
die Nachbarzeile, Schnitt erst hinter dem logischen Schirm (Anwenderentscheidung A,
`docs/COMPATIBILITY.md`). Fix `bec04bab` (neu `tos-api/src/vdi/line.rs`, Spec
`docs/api/vdi/output/_lines-markers.md`, 20 neue Tests). Kein Ausgleich der alten Abweichung in
AES/Desktop gefunden. Abnahme `measurements/measurement-2026-11-05-v20-test/RUNINFO.TXT`:
Orakelfaelle gleich, AES-Tabellen und Desktop-Szenen mit Injektor pixelgleich, Rahmen ueber
den Schirmrand per `dbprobe` EmuTOS-gleich; beide Boards. Offen: Pi im Testszenario;
Text-Unterstreichung bei `vs_clip(0)` (V18 Teil 2); `clip_line`-Rundengrenze 64 und
16-Bit-Ueberlauf bei Abstaenden > 16383 px ungemessen.
