# V34 — Quittung DONE (2026-11-07)

Umgesetzt, gegen das Orakel gemessen und vom Tester abgenommen (PASS, `038463fe`, `measurements/measurement-2026-11-07-v34-test/RUNINFO.TXT`): Hosttests 2261/0/1, beide Boards booten, AES r2/r3/r4 und Desktop-Szenen bytegleich zur Basis, Spec-Zitate geprueft. Keine Pi-Abnahme noetig (ein Wort in Seite 0).

`Scrdmp` setzt `dumpflg` (`$4EE`) wie EmuTOS `scrdmp` (`bios/xbios.c:490-494`) auf -1, in der Seite 0 des aufrufenden
Prozesses; `dump_vec` wird nach ADR-031 nicht gerufen (011). `v_hardcopy` (Escape 17) ruft `Scrdmp` wie EmuTOS `escfn17`
(`vdi/vdi_esc.c:260-263`) (012, nach Probe T18A auf dem 011-Stand entschieden: "after Scrdmp" gleich, "after v_hardcopy"
noch verschieden). Fix `5f299bf8`. Code `tos-api/src/xbios/system.rs` (`scrdmp`, `DUMPFLG_ADDR`), `tos-api/src/vdi/escape.rs` (`v_escape`
Arm 17); Spec-Korrektur zu EmuTOS in `docs/api/xbios/14-scrdmp.md`, `docs/api/vdi/_escape.md`, `docs/api/vdi/_discrepancies.md`
(D-VDI-4), `docs/api/bios/_state.md`; `docs/analysis/deps/mapping.tsv` Notizen; Test `tos-api/tests/xbios_scrdmp_dumpflg.rs`
(3, vor dem Fix 2 rot).

Gemessen (`measurements/measurement-2026-11-07-v34/RUNINFO.TXT`): T18A EmuTOS -1/-1, NewTOS Kopf 5/5, 011 5/-1, Fix -1/-1;
Kopf→Fix nur die zwei T18A-Logzeilen anders, 0 von 909 Grabs; Eingangsspur `bios-arg-trace` passt; `vdi-v18-t2` 2/32 (V35),
`vdi-all` 0/182, Basis = Fix. `cargo xtask test` 2258/0/1 → 2261/0/1; Warnungen unveraendert (rustc 64/93, clippy 73/121,
beide Boards).

Folgetickets: tosdiff-Fall zu `$4EE` [TOOL35](TOOL35.md) (CLAUDE.md "Testing", geparkt); Alt+Help zaehlt `dumpflg` nicht, `dumpflg` je Prozess: [KEYS05](KEYS05.md).
