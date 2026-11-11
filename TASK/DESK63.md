# DESK63 — Desktop-Verknuepfung auf Programm-Verknuepfung: „illegale Operation“ statt Start

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner decision 2026-11-11 at the legacy migration: not on the work queue | Reopen: a card program or port is affected, or the owner schedules it

Migrated (owner batch 9, 2026-11-11), evidence: TASK/DESK63.md status line: OFFEN, no DONE receipt, no fix/feat commit names the id

Line 3 before the migration:
Status: **OFFEN** · Prioritaet: **P3** · Typ: fehlender Zweig — Nebenfund DESK62 (2026-10-16)

`fun_desk2desk` (`programs/desktop/src/dir.rs`, um Zeile 2171) fehlt der Weg
ueber `fun_file2any` → `fun_file2desk`, den EmuTOS geht
(`desk/deskfun.c:1297`, `:1180`). Zieht man ein Datei-Icon vom Desktop auf
eine Programm-Verknuepfung auf dem Desktop, zeigt NewTOS `illegal_op_msg`;
EmuTOS startet das Programm mit der Datei.

Quelle: `docs/analysis/desk62-hourglass-after-drop-on-shortcut-2026-10-16.md`
(Abschnitt „Ausserhalb des Auftrags“). Erst in QEMU reproduzieren und gegen
EmuTOS belegen, dann angleichen; der Merker `exit_desktop` aus DESK62 muss
auch diesen Zweig tragen.

[Gemeinsame Regeln](README.md) gelten.
