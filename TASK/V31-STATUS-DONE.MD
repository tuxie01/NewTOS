# V31 — Quittung DONE (2026-11-07)

Umgesetzt, gegen das Orakel gemessen und vom Tester abgenommen (PASS, `b1c35c9b`, `measurements/measurement-2026-11-07-v31-test/RUNINFO.TXT`): Hosttests 2258/0/1, beide Boards booten, AES r2/r3/r4 und Desktop-Szenen gleich zur Basis, keine Regression. Keine Pi-Abnahme noetig (reine Rechnung im Textweg); skalierter Text in QED, HighWire, HypView, TeraDesk ist ungesehen.

Skalierter Text laeuft wie EmuTOS `text_blt` (`vdi/vdi_textblit.c:1301-1417`): der Stift ruckt um `char_resize(XDDA, DELX)`,
eine Glyphe ganz ausserhalb des Clip-Fensters wird weder skaliert noch gezeichnet und laesst `XDDA` stehen (007), und
`scale()`s Abstandsschleife bewegt `XDDA` auch bei genauer Verdopplung (008/K14, Anwenderentscheidung: wie EmuTOS).
Fix `17bcaf56` in `tos-api/src/vdi/text.rs` (`glyph_advance`, `wholly_clipped`, `assemble_and_blit`, `scale_glyph`, `resize`), Spec
`docs/api/vdi/output/_text.md` ("Scaled glyphs: the horizontal DDA"), Probe T31A–T31E, 16 Hosttests (10 rot auf dem alten
Code) und ein Einheitstest.

Gemessen (`measurements/measurement-2026-11-07-v31/RUNINFO.TXT`): Probe Kopf→Fix genau 9 Faelle anders, T14E, T14K,
T31A–T31E, ZG012 jetzt orakelgleich, ZG013 485 → 98 px; `vdi-v18-t2` 86/90/94 gleich (nur 50/54, V35); `vdi-all` 0/182.
`cargo xtask test` 2241/0/1 → 2258/0/1; Warnungen unveraendert (rustc 64/93, clippy 73/121, beide Boards).

Nebenbefunde im
Ticket: ZG012 = [V45](V45.md) (jetzt orakelgleich), Rest von ZG013 = Schraeg ([V41](V41.md)), Test-Double-Ueberlauf,
veraltete Phase-12-Notiz in der Spec, Zeilenzahl bei Verkleinerung ungemessen.
