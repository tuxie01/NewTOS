# V26 — Quittung DONE (2026-11-05)

Der Chunky-Speicherweg (und `mono_to_mem`) griff bei `fd_w > fd_wdwidth*16` in Zeilen, die der
Kernel nicht gegen die Rechte des Programms gebunden hatte (fuenf rote Tests, `1c84f4d2`).
Anwenderentscheidung 2026-11-05 "ja a": Speicherformen wie EmuTOS adressiert (Zeile nur aus
`fd_wdwidth`, `fd_w` keine Grenze, Umlauf in die naechste Zeile, Fuellspalten), Schnitt nur
hinter `fd_h` mit Eintrag in `docs/COMPATIBILITY.md`; der Kernel bindet genau die beruehrte Huelle
(`87f1684e`, auch der planare Weg aus V19). Abnahme
`measurements/measurement-2026-11-05-v26-test/RUNINFO.TXT`: Umbruch-Probe 40/40 gleich EmuTOS,
alte Orakelfaelle und AES/Desktop unveraendert, beide Boards. Offen (in V18 Teil 2): Richtung bei
selbstueberdeckendem Block, `vrt_cpyfm` Speicherziel 1..8 Ebenen, 9..15 Ebenen.
