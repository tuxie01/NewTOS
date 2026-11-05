# A130 — Quittung DONE (2026-11-05)

GitHub #2 behoben: Die Fenster-Rollbalken verhalten sich wie EmuTOS `hctl_window`.
Schieberzug mit gedruecktem Schieber und XOR-Umriss statt Dauer-Neuzeichnen
(die Musterbaender aus #2), `WM_VSLID`/`WM_HSLID`-Werte wie das Orakel,
Seitenklick und Pfeile wiederholen bei gehaltener Taste, Treffertest mit
3D-Rand. Fix `d9e44143`; Messung `measurements/measurement-2026-11-05-a130/RUNINFO.TXT`,
Abnahme am Orakel `measurements/measurement-2026-11-05-a130-test/RUNINFO.TXT`,
Pi des Anwenders `measurements/measurement-2026-11-05-a130-card/RUNINFO.TXT`
("funktioniert wie erwartet"). Rest ausserhalb: Fasen-Grauton (Klasse A131),
`msg[1]` der wiederholten `WM_ARROWED` (fehlender `p_msg`-Weg, kein Ticket
ohne Anwenderentscheidung).
