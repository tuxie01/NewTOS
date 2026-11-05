# A131 — Quittung DONE (2026-11-05)

GitHub #7 behoben: Der Fensterschatten hat die Randfarbe des `W_BOX`-Farbworts
wie EmuTOS `just_draw` (`SHADOWED`-Zweig), Standard Stift 1 (schwarz), folgt
`WF_COLOR`/`WF_DCOLOR`. Die graue Linie war die Anwenderentscheidung vom
2026-09-10 (Stift 8, ohne COMPATIBILITY-Eintrag); aufgehoben vom Anwender
2026-11-05 ("wie emutos"). Messung `measurements/measurement-2026-11-05-a131/RUNINFO.TXT`
(`docs/audits/2026-11-05-a131-shadow.md`), Fix `837e1001`, Abnahme am Orakel
`measurements/measurement-2026-11-05-a131-test/RUNINFO.TXT` (0 Schattenpixel
Differenz, `WF_COLOR`-Fall rot wie EmuTOS), Pi des Anwenders
`measurements/measurement-2026-11-05-a131-card/RUNINFO.TXT` ("es entspricht
EmuTOS 1:1"; die Optik gefaellt dem Anwender persoenlich nicht, kein Auftrag).
`WF_DCOLOR`-Fall nur per Hosttest, am Orakel ungemessen.
