# HW08 — Quittung UNFINISHED (2026-11-06)

Auftragspunkte 1 und 2 erledigt, Commit `e421bd5d`; Punkte 3–5 offen (Pi-Lauf, Ursache, Fix, Tempo). Kein Fix am Fehlerpfad, keine Limits geaendert.

Gebaut: Debug-Features `sd-diag` (Bericht auf dem Schirm, Strg+Alt+F9, ohne Karte und ohne serielle Leitung) und `sd-fault` (Fehler im Treiber einspeisen), Sonde `tosdump/probes/wfprobe.c`, Schreibmessung `sdk/examples/fswbench.c`, 5 Host-Tests. Ohne Feature bytegleich bis auf `panic::Location`.

Gemessen, QEMU raspi4b und Orakel: `measurements/measurement-2026-11-06-hw08/RUNINFO.TXT`. Haengen bis zum Neustart hat zwei Mechanismen, Glied Cache reproduziert das Ticket, Glied Treiber ist neu; EmuTOS haengt nie. Stand im Ticket.

Offen: Pi-Lauf `measurements/measurement-2026-11-06-hw08/PI-RUN.TXT` (Lead und Anwender), danach Anwenderentscheidung A/B zum Cache und der Fix. Hardwareabnahme fehlt ganz.
