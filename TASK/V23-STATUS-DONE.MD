# V23 — Quittung DONE (2026-11-06)

`CONTRL[1]` wird fuer jeden VDI-Aufruf zentral in `dispatch_in` auf `MAX_VERTICES` (1024)
gekappt und danach bedingungslos zurueckgeschrieben, wie EmuTOS `GSX_ENTRY`
(`vdi/vdi_asm.S:88-116`); private Kappungen in `v_pline`/`v_fillarea` entfernt. Negativer Zaehler
wie EmuTOS ungekappt an den Handler (zeichnet nichts); das Ueberschreiben fremden Speichers durch
EmuTOS ist nicht nachgebaut (`docs/COMPATIBILITY.md`). Fix `ee67903f` (Spec
`docs/api/vdi/_dispatcher.md`, 5 Tests). Abnahme `measurements/measurement-2026-11-06-v23-test/RUNINFO.TXT`:
K5A/K5B orakelgleich, alles andere bytegleich zur Basis, beide Boards. Folgeticket
[TOOL34](TOOL34.md) (Orakelfall > 1024 Punkte, geparkt).
