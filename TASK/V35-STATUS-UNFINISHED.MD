# V35 — Quittung UNFINISHED (2026-11-07)

Umgesetzt und gegen das Orakel gemessen; **offen ist die Abnahme des Testers** (und eine Entscheidung, unten).
UNFINISHED heisst hier: Tester offen, nicht Implementierung offen.

`GSX_ENTRY`s lokale `PTSIN`-Kopie wie EmuTOS (`vdi/vdi_asm.S:21,79-81,93,99-108`, `include/vdiext.h:48-51`): `dispatch_in`
kopiert nach der Kappung und vor `screen()` die `CONTRL[1]` Paare (0 → keine, hoechstens 1024, negativ → 1024) in einen
statischen Puffer; jeder Handler liest `PTSIN` daraus (`tos-api/src/vdi/mod.rs` `LCL_PTSIN`, `lcl_ptsin_copy`,
`lcl_ptsin_word`, `Vdipb::ptsin`). Die Ueberlagerung mit Text-Scratch und Konturfuell-Warteschlange ist nach der
Anwenderentscheidung 2026-11-06 nicht nachgebaut (`docs/COMPATIBILITY.md`, neuer Eintrag). Aufrufer mit zu kleinem
`CONTRL[1]` offen korrigiert: `kernel/src/vdi_client.rs` `gtext` (Bootschirm), `kernel/src/selftest_vdi.rs` (nicht verdrahtet),
Testhilfe `tos-api/src/vdi/raster.rs`. Kein Kartenprogramm betroffen (nur gemlib, volle Zaehler).

Artefakte: Fix `788a8aa9`; Code oben; Test `tos-api/tests/vdi_reference/cases_v35.rs` (13, davon TPH ignoriert); Probe `tosdump/probes/vdprobe.c`
`t35` (TPF–TPH); Spec `docs/api/vdi/_dispatcher.md` (Korrektur zu EmuTOS), `docs/COMPATIBILITY.md`; Lauf
`measurements/measurement-2026-11-07-v35/RUNINFO.TXT`. Zahlen und Uebergabeliste: [V35](V35.md).

Offen:
- Tester-Abnahme ([V35](V35.md) "Abnahme (Tester) — Uebergabe").
- Entscheidung: die Schreibzugriffe der EmuTOS-Handler in die Kopie (Probe TPH, Test ignoriert) — nicht im entschiedenen Umfang.
- `docs/analysis/deps/mapping.tsv` Zeile `GSX_ENTRY`: nicht geaendert (Datei waehrend V49 gesperrt); der neue Notiztext steht in [V35](V35.md).
- Folgeticket tosdiff-Fall fuer TPF–TPH (CLAUDE.md "Testing"), vom Lead.
