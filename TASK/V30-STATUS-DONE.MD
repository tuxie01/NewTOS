# V30 — Quittung DONE (2026-11-06)

Die Unterstreichung laeuft wie EmuTOS `output_text` (`vdi/vdi_text.c:500-542`) von `startx` bis `DESTX`
einschliesslich, die Maske wandert von Linie zu Linie. Bei `vs_clip(0)` laufen Zeichen und Unterstreichung in die
Nachbarzeile, Schnitt erst hinter dem logischen Schirm (Anwenderentscheidung, Regel von Entscheidung A). Fix
`858e5f68` (Spec `docs/api/vdi/output/_text.md`, `docs/COMPATIBILITY.md`, 40 Tests, davon 31 rot auf der Basis).
Abnahme `measurements/measurement-2026-11-06-v30-test/RUNINFO.TXT`: `vdi-v18-t2` 61/64/67/70/74/76/79 orakelgleich,
`vdi-all` 0/182, AES-Tabellen und Desktop bytegleich zur Basis. Offen: echte Pi im Testszenario. Folgen [V39](V39.md) bis [V43](V43.md), [A137](A137.md) bis
[A139](A139.md).
