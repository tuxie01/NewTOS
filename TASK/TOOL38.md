# TOOL38 — `wkprobe`: Wartegrenze vor dem ersten Mauspaket zu knapp

Status: PARKED (2026-11-11) | Priority: P3 | Reason: runs are accepted by repeating them, blocks no fix (2026-11-08) | Reopen: a fix acceptance is blocked by it

Migrated (owner batch 12, 2026-11-11), evidence: TASK/TOOL38.md status line: GEPARKT (2026-11-08, Laeufe sind durch Wiederholen abnehmbar, blockiert keinen Fix)

Line 3 before the migration:
Status: **GEPARKT** (2026-11-08; Laeufe sind durch Wiederholen abnehmbar, blockiert keinen Fix) · Prioritaet: **P3** · Typ: Werkzeug — Nebenfund aus [V54](V54.md)

`tosdump/probes/wkprobe.c` wartet hoechstens 2 000 000 Abfragen auf das erste Mauspaket; der Kommentar nennt "30 s". Bei
heutiger Abfragerate liegt das knapp unter den ~1980 Ticks bis zum ersten Paket: einer von sechs Laeufen in V54 stieg 46 Ticks
zu frueh aus, danach war jeder Paketfall um eins verschoben (`measurements/measurement-2026-11-08-v54/RUNINFO.TXT`, Punkt 5).

Auftrag: Wartegrenze an der Zeit (`_hz_200`) statt an der Zahl der Abfragen festmachen, oder ein Abbruch faellt im LOG als
ungueltiger Lauf auf. Neue Probe-Version, Orakel und NewTOS je zweimal bytegleich.

[Gemeinsame Regeln](README.md) gelten.
