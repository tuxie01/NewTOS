# PROC06 — `Pexec` liefert nach Ctrl-C `-32` vorzeichenerweitert statt 65504

Status: IMPLEMENTED (2026-11-11) | Priority: P3 | Commit: 689d88b0a | Missing: check that the change is committed (legacy line says uncommitted)

Migrated (owner batch 15, 2026-11-11, to IMPLEMENTED), evidence: receipt TASK/PROC06-STATUS-DONE.MD exists (legacy DONE receipt, IMPLEMENTED accepted by TOOL55), commit 689d88b0a (fix(gemdos): Pexec caller gets the child's exit code zero-extended (PR), VERIFIED: diff of 689d88b0a (fix(gemdos): Pexec caller gets the child's exit code zero-ex) touches kernel/src/arch/aarch64/exceptions.rs, tos-api/src/bdos/process/pterm.rs, tos-api/src/gemdos/console.rs (+6), names the id by subject, 148 files, legacy line: ERLEDIGT (2026-10-16, uncommitted)

Line 3 before the migration:
Status: **ERLEDIGT (2026-10-16, uncommitted)** · Prioritaet: **P3** · Typ: Konformitaet — Nebenfund DESK43 U9-0 (2026-10-14) · [Quittung](PROC06-STATUS-DONE.MD)

EmuTOS legt den Exitcode in `xterm(UWORD rc)` zero-extended ab
(`bdos/proc.c:601,616`): ein per Ctrl-C beendetes Kind (`Pterm(-32)`) kommt
beim `Pexec`-Aufrufer als 65504 an. NewTOS liefert dem EL0-Aufrufer -32
(vorzeichenerweitert). Der Shell-Start (PROC05, U9-0b) rechnet schon mit
`pterm::parent_exit_code` richtig; der `Pexec`-Rueckweg zum Programm noch
nicht. Belegen, ob das fuer alle `Pterm`-Werte gilt, dann angleichen
(Spec `docs/api/gemdos/process/`, Hosttest, APITEST-Fall).

[Gemeinsame Regeln](README.md) gelten.

## Abschluss 2026-10-16

Belegt (Spec `docs/api/gemdos/process/4c-pterm.md`, „Exit-code matrix“): bei
EmuTOS bekommt der `Pexec`-Aufrufer fuer **jeden** `Pterm`-Wert `rc & 0xFFFF`
zero-extended. Angeglichen im gemeinsamen Terminate-Ende `finish_terminate`
(`pterm::exit_code_for_caller`); Ctrl-C liefert jetzt 65504 statt −32, der
Upcall-Ueberlauf 64538 statt −998. Hosttest
`tos-api/tests/proc06_pexec_exit_code.rs` (rot → gruen), APITEST-Fall T33
(109/0/5), Ctrl-C-Kind auf dem Ziel rot (−32) → gruen (65504):
`measurements/measurement-2026-10-16-proc06/RUNINFO.TXT`. Offen: echter Pi 4.

