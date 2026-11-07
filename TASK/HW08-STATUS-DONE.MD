# HW08 — Quittung DONE (2026-11-07)

Diagnose `e421bd5d`, Fix `16bfa004`. Tester in QEMU `measurements/measurement-2026-11-06-hw08-fix-test/RUNINFO.TXT` (PARTIAL nur wegen der fehlenden Pi). Pi: FSWBENCH zweimal ohne Fehler, Fbench vollstaendig, Desktop-Kopie inhaltsgleich, WFPROBE t/d/u/b auf D: bestanden, `verifyVolume` sauber (`measurements/measurement-2026-11-07-hw08-pi5/RUNINFO.TXT`, `measurements/measurement-2026-11-07-hw08-pi6/RUNINFO.TXT`, `measurements/measurement-2026-11-07-rel071-pi/RUNINFO.TXT`). Ein Kartenfehler blockiert spaeteres Speichern nicht mehr; in Release 0.7.1.

Entfallen: Schritt 2 mit der alten Karte (Anwender: Karte defekt, auch im Mac). Geklaert: `nonrecover 1` in u2 ist die erwartete Zaehlung (Ticket, Abschluss). Ausgegliedert: [FS33](FS33.md), [FS34](FS34.md); HW05-Stoppuhrkopie und X-Shot ins Testszenario. Gelesen, nicht gemessen: `ILLEGAL_COMMAND` wird fuer jedes Datenkommando ignoriert (Tester-Abschnitt im Ticket), ohne Ticket.
