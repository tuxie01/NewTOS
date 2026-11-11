# DESK60 — Shell meldet „starting DESKTOP.NAP“ nach einem `#Z`-gestarteten Programm

Status: DONE (2026-11-11) | Priority: P3 | Commit: a217b1aa0 | Receipt: DESK60-STATUS-DONE.MD

Migrated (owner batch 4, 2026-11-11), evidence: receipt TASK/DESK60-STATUS-DONE.MD exists, commit a217b1aa0, TASK/DESK60-STATUS-DONE.MD, commit a217b1aa0 (fix(tools,shell): injector arms on launch, honest desktop log line (T1), fix/feat commits naming the id: a217b1aa0 (n=1), measurements/measurement-2026-10-15-desk43-acceptance/, the receipt names no hash for this commit, the Migrated line names it

Line 3 before the migration:
Status: **ERLEDIGT** 2026-10-16 ([Quittung](DESK60-STATUS-DONE.MD)) · Prioritaet: **P3** · Typ: Diagnose/Log — Nebenfund DESK43-Abnahme (2026-10-15)

Endet ein per `#Z` gestartetes TeraDesk, protokolliert die Shell in beiden
Desktop-Pfaden „starting DESKTOP.NAP“, obwohl NEWDESK bzw. der eingebaute
Desktop hochkommt (`measurements/measurement-2026-10-15-desk43-acceptance/`).
Vermutlich ein Rest des vorigen Kommandos in `D.s_cmd`. Gegen EmuTOS
`sh_main`/`sh_ldapp` pruefen, ob `s_cmd` dort beim Wechsel auf
`DESKTOP_APP` neu gesetzt wird; nur Log oder auch Verhalten (`appl_find`,
`p_nameit`) betroffen?

[Gemeinsame Regeln](README.md) gelten.

## Abschluss — 2026-10-16

Status: **ERLEDIGT (QEMU), nicht committet.** Nur die Logzeile war irrefuehrend; das Verhalten ist EmuTOS-gleich: TeraDesk ruft
`shel_write(SHW_NOEXEC)` (`startprg.c:386`), damit ist `sh_nextapp = DESKTOP_APP`, der AUTORUN-Ruecksprung
(`gemshlib.c:662-666`) ueberspringt `set_default_desktop`, und `sh_chdef` (`:551-565`) baut `D.s_cmd` aus dem `#Z`-`sh_desk` =
`DESKTOP.NAP` — in EmuTOS genauso (`p_nameit` → `DESKTOP`, `shel_read` → `DESKTOP.NAP`). `kernel/src/shell_loop.rs` nennt jetzt den
gestarteten Desktop und `D.s_cmd` getrennt; Hosttest `tos-api/tests/aes_shell_desk60_autorun_noexec.rs` pinnt die EmuTOS-Folge.
Quittung: [DESK60-STATUS-DONE.MD](DESK60-STATUS-DONE.MD); Beleg `measurements/measurement-2026-10-16-t16-desk60/RUNINFO.TXT`.
