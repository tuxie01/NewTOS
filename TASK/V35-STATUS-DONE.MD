# V35 — Quittung DONE (2026-11-07)

Umgesetzt, gegen das Orakel gemessen und vom Tester abgenommen (PASS, `44558a7a`, `measurements/measurement-2026-11-07-v35-test/RUNINFO.TXT`): `vdi-v18-t2` 0/32, `vdi-all` 0/182, AES r2/r3/r4 und Desktop bytegleich zur Basis, Spec-Zitate geprueft. Nebenbefund des Testers: Konsolenphase nach `appl_init` in QEMU etwa 7 % langsamer, nicht durch die Kopie (Zaehler- und Fuellkern-Gegenprobe), Ursache offen, nicht blockierend; im naechsten Pi-Szenario mit ansehen.

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

Folgen: Schreibzugriffe der Handler in die Kopie [V52](V52.md) (geparkt, Anwender); `mapping.tsv`-Zeile `GSX_ENTRY` vom Lead nachgetragen (`a862f764`); tosdiff-Fall [TOOL36](TOOL36.md) (geparkt).
