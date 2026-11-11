# PROC05 — Shell-Start liefert nie den Exitcode des Programms

Status: PARKED (2026-11-11) | Priority: P3 | Reason: legacy, evidence unverified; receipt PROC05-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 15, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by acb0ff45c (feat(shell): exit codes, exact registry names, pub), named by body

Line 3 before the migration:
Status: **ERLEDIGT (2026-10-14, DESK43 U9-0b, uncommitted)** · Prioritaet: **P3** · Typ: Konformitaet — Nebenfund DESK43 U6a (2026-10-14) · [Quittung](PROC05-STATUS-DONE.MD)

`kernel/src/launch.rs` `run_program_with_tail_rc` (`:62-65,172`) gibt fuer
jedes Programm, das `Pterm` erreicht, `E_OK` zurueck. EmuTOS' `sh_ldapp`
bekommt von `dos_exec(PE_LOADGO, …)` (`aes/gemshlib.c:645`) den Rueckgabewert
des Kindes und macht aus einem negativen Wert ein `fm_error`-Alert
(`:722-723`). Bei NewTOS bleibt z. B. `Pterm(-1)` (Rust-Panic in HELLOGEM,
`measurements/measurement-2026-10-14-desk43-u6a/`) still `rc=0`.

Erst belegen (EmuTOS-Pfad `sh_ldapp` → `sh_main`, welche Werte ein Alert
ausloesen), dann angleichen. Beruehrt DESK43 U9 / D6 (Absturzpolitik des
Desktops).

[Gemeinsame Regeln](README.md) gelten.

## Korrektur 2026-10-14 (U9-Vorbereitung)

`docs/analysis/desk43-u9-prep-2026-10-14.md` §2: EmuTOS' `xterm(UWORD rc)`
legt den Code zero-extended ab (`bdos/proc.c:601,616`) — `Pterm(-1)` kommt
beim Elternprozess als 65535 an, `sh_ldapp` gibt dann 0 zurueck, **kein**
Alert. `fm_error` entsteht nur aus Ladefehlern (EFILNF … EPLFMT), und die
gibt NewTOS schon weiter (`launch.rs:86-89`). Fehlend ist allein die
Weitergabe des Kind-Exitcodes (`pterm::terminate` verwirft ihn,
`launch.rs:172` liefert immer `E_OK`); gebraucht fuer D6 (`rc != 0 →
wm_new()`). Eingeplant als DESK43 U9-0b.
