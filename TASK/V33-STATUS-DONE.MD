# V33 — Quittung DONE (2026-11-06)

`vst_load_fonts`, `vst_unload_fonts` und `vst_font` arbeiten wie das EmuTOS-ROM (`vdi/vdi_text.c:1292-1362`), der Stub ist weg. Fix
`877a14b9` samt gemlib- und mintlib-Overlay (D1, D3), Spec `docs/api/vdi/attributes/_text.md`, `docs/COMPATIBILITY.md` ("GDOS font chain").
Abnahme `measurements/measurement-2026-11-06-v33-test/RUNINFO.TXT`: T13A, T13B orakelgleich, kein Probefall gekippt, `vdi-all` 0/182, AES-Tabellen
ohne Fallunterschied zur Basis (nur Adresswort), Desktop bytegleich. Offen: Text wird langsamer ([V44](V44.md)), Schraeg im Replace-Modus sichtbar
beschaedigt, schon vor V33 ([V41](V41.md)), alte Binaries neu linken, echte Pi im Testszenario.
