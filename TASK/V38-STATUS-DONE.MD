# V38 — Quittung DONE (2026-11-08)

Umgesetzt und vom Tester abgenommen (PASS, `f6ae5160`, `measurements/measurement-2026-11-07-v38-test/RUNINFO.TXT`). Am Orakel gemessen: EmuTOS kehrt im 5x3-Fall nicht zurueck (30000 VBL), NewTOS mit Bremse nach ~3 s (QEMU, 640x480); alle echten Fuellungen bytegleich. Offen: Dauer einer gebremsten Fuellung bei 1920x1080 auf der Pi (QEMU-Hochrechnung 20–25 s, keine Messung) ins Testszenario; Testluecke M3 (Fuellung, die genau am Budget endet) nur gelesen; Anwenderfrage Grenzhoehe.

`v_contourfill` hat eine Notbremse: die Fuellung endet nach 16 entnommenen Spannen je Schirmpixel
(`tos-api/src/vdi/contour.rs:118@BRAKE_SPANS_PER_PIXEL`, Pruefung vor jeder Entnahme in `Flood::run`) und
kehrt zurueck wie eine fertige; Gezeichnetes bleibt. Grenze aus der Geometrie: echte Fuellungen brauchen
hoechstens eine Spanne je Pixel (gemessen bis 1920x1080, Labyrinth 0,5/px). Sanktioniert in
`docs/COMPATIBILITY.md`, Zeile in `docs/DEVIATIONS.md`.

Befund dabei: die EmuTOS-Fuellung haengt auch bei echten Eingaben (5x3-Clip mit zwei Punkten, zwei
unabhaengige Modelle), und zwar mit gesperrten Interrupts (gelesen, mit Zitaten im Ticket).

Gemessen (`measurements/measurement-2026-11-07-v38/RUNINFO.TXT`): Hosttest 3 rot ohne Bremse, 5 gruen mit;
`cargo xtask test` 2297/0/6 → 2302/0/6; Probe 954 Faelle und `vdi-all`/`vdi-v18`/`vdi-v18-t2` Basis = Fix
bytegleich, K8*/K14* orakelgleich; Warnungen beide Boards unveraendert (64/93, clippy 73/121).

Offen: Tester-Abnahme; am Orakel ungemessen, dass EmuTOS beim 5x3-Fall nicht zurueckkehrt; Dauer einer
gebremsten Fuellung auf der Pi; Anwenderfrage, ob die Grenze hoeher soll (lange, von selbst endende
Fuellungen mit Mustern gibt es, siehe Ticket).
