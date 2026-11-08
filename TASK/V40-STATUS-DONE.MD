# V40 — Quittung DONE (2026-11-08)

Umgesetzt, gegen das Orakel gemessen und vom Tester abgenommen (PASS, `da2aad51`, `measurements/measurement-2026-11-08-v40-test/RUNINFO.TXT`): 34/34 Faelle orakelgleich, Basis gegen Fix genau 16 Faelle (Endpixel der Unterstreichung), AES/Desktop bytegleich. Testluecke: der Schreiber der Menue-Trennlinie hat keinen Test (Mutant ueberlebt).

`LSTLIN` wird wie in EmuTOS als ein globales Flag gefuehrt (`bios/lineavars.S:288`): `polyline` schreibt es fuer jede
Punktzahl (FALSE beim Start, TRUE vor dem letzten Segment, `vdi/vdi_line.c:1106-1108`), die Unterstreichung liest es
(`vdi/vdi_text.c:527,534` → `abline` `:2319`). Dazu laufen die Polylines, die EmuTOS faehrt, auch dort, wo sie nichts
zeichnen (`v_pline` Breite 1 mit 0/1 Punkt, `v_fillarea` mit 0 Punkten), und die Umrisse entfallen, wo `polygon` vorher
zurueckkehrt (Clip an, y ausserhalb des Bands, `vdi/vdi_fill.c:584-588`). Die AES, die am VDI-Trap vorbei zeichnet, setzt
es an den Stellen, die in EmuTOS ein `v_pline` sind (`draw_line`, `draw_border`, `rect_outline`, Menue-Trennlinie).

Artefakte: `tos-api/src/vdi/line.rs` (`lstlin`, `set_lstlin`), `primitives.rs` (`polyline`, `v_pline`, `v_fillarea`,
`polygon`, `fill_polygon`), `gdp.rs` (`curve`, `rbox`), `text.rs` (`underline_lines`), `backend.rs` (`draw_line`),
`tos-api/src/aes/object.rs` (`draw_border`), `window.rs` (`rect_outline`), `menu.rs` (`menu_bar`); Tests
`tos-api/tests/vdi_reference/cases_v40.rs` (12), Unit-Tests in `menu.rs` und `window.rs`; Probe
`tosdump/probes/vdprobe.c` `t40` (L40A–L40Zb, L41A–L41D); Spec `docs/api/vdi/output/_lines-markers.md` § `LSTLIN`,
`_text.md` § Underline (Korrektur hin zu EmuTOS); `docs/DEVIATIONS.md` Zeile V40 entfernt; `docs/analysis/deps/mapping.tsv`;
Lauf `measurements/measurement-2026-11-08-v40/RUNINFO.TXT`. Ursache, Zahlen, AES-Gegenprobe, Uebergabe: [V40](V40.md).

Ergebnis: TVJ, TVK und alle 32 neuen Faelle orakelgleich; gegen die Basis genau 16 Probefaelle anders (TVK und 15 L40,
je das Endpixel); `vdi-v18-t2` 0/32, `vdi-all` 0/182; AES-Tabellen und Desktop/Bootschirm siehe RUNINFO.

Offen: Tester-Abnahme. Nicht gemessen: Menue-Trennlinie und Fensterrahmen als Schreiber gegen das Orakel (gelesen,
Fensterrahmen mit Unit-Test).
