# V21 — Quittung DONE (2026-11-06)

`v_contourfill` liess bei vielen Taschen Flaechen ungefuellt: der eigene Saatstapel (4096)
lief voll und verwarf Saaten still (roter Test, Probe mit 2^20 gruen). Fix `497773f9`: die
Saat-Warteschlange von EmuTOS (`vdi_fill.c:873-1062`, `QSIZE` 1024, Kollision, Wiederverwendung,
Abbruch bei Ueberlauf), eine gemeinsame `pixelread` fuer `v_contourfill` und `v_get_pixel`;
Spec `docs/api/vdi/output/_fill.md`. Abnahme `measurements/measurement-2026-11-06-v21-test/RUNINFO.TXT`:
Orakelfaelle gleich, Ueberlauf-Faelle K14A/B 0 px gegen EmuTOS, keine Regression, AES und
Desktop unveraendert, beide Boards. Offen: Notbremse (Anwender), Pi im Testszenario.
