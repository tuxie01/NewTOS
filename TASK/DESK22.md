# DESK22 — `.NCP`-Start laesst den Mauszeiger auf dem Textschirm stehen

Status: PARKED (2026-11-11) | Priority: P3 | Reason: legacy, evidence unverified; receipt DESK22-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 19, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by b48703149 (fix(desk): AUTO launches start from the root again), named by body

Line 3 before the migration:
Status: **ERLEDIGT** (2026-09-24) · Prioritaet: **P3** · Typ: Defekt — Nebenbefund DESK06

Abhaengigkeiten: [DESK06](DESK06.md) (erledigt)

## Sachverhalt

DESK06 hat den Konsolenweg ueber EmuTOS' Modusumschaltung gebaut:
`sh_toalpha` (`references/emutos/aes/gemshlib.c:246-260`) nimmt den Zeiger
weg (`gsx_mfset`/`ratexit`) und schliesst die Workstation, bevor der
Textschirm kommt. Der **vorhandene** `.NCP`-Startpfad
(`kernel/src/desk/main.rs` `launch_native`) tut das nicht — er ruft nur
`fbcon::clear_and_home`, und der Zeiger bleibt als schwarzer Fleck im
Textschirm stehen.

Beleg: `measurements/measurement-2026-09-23-desk06/screendumps/redraw-C-program.png`,
Fleck bei rund (100, 197). Der neue EmuCON-Weg hat das nicht
(`cli-C-console.png` ist zeigerfrei).

## Arbeit

`launch_native` auf denselben Weg legen, den DESK06 gebaut hat
(`sh_toalpha`/`sh_tographic` ueber `shell_loop`), statt den Schirm nur zu
loeschen. Screendump vor/nach.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

---

## Abschluss (2026-09-24)

**Gebaut.** `launch_native`s `.NCP`-Arm liegt jetzt auf der Modusumschaltung,
die DESK06 gebaut hat, statt den Schirm nur zu loeschen. Neu sind
`crate::shell_loop::sh_alpha_enter` / `sh_alpha_leave` — ein `sh_main`-Durchlauf
fuer ein zeichenorientiertes `NORMAL_APP`: beide setzen `sh_isgem` so, wie es
`shel_write(SHW_EXEC, isgraf, …)` taete (`aes/gemshlib.c:173`), und rufen dann
das echte `sh_chgrf` (`:513-523`), damit `gl_shgem` der Zustand bleibt, gegen
den der naechste Vergleich laeuft. `sh_toalpha`s `ratexit()` (`:256`) nimmt den
Zeiger weg; der schwarze Fleck ist fort.

Die Reihenfolge des Arms ist die von `sh_main`: Umschaltung, Warnung bei
fehlgeschlagenem Start (`fm_error`, `:722-723`), Desktopaufbau,
`desk_busy_off` (`deskmain.c:2071`), das die Sanduhr von `sh_tographic`
zuruecknimmt.

* Code: `kernel/src/shell_loop.rs` (`sh_alpha_enter`/`sh_alpha_leave`),
  `kernel/src/desk/main.rs` (`launch_native`, `Flavor::Console`).
* Spezifikation: `docs/api/aes/shell-loop.md` §9, Abschnitt *NewTOS
  implementation notes — the mode switch for a character program launched by
  the built-in desktop (DESK22)*.
* Messung: `measurements/measurement-2026-09-24-desk22-desk23-desk28-desk29/` §3,
  `screendumps/d22-before-C-program.png` (Fleck bei (179,187)) gegen
  `d22-after-C-program.png` (zeigerfrei), gleiche Karte, gleiche Klickfolge.
* Pruefung: wie DESK28 (Basiswerte unveraendert), 0 `[EXC]`/`[PANIC]`.
* Offen: Hardwareabnahme, Punkt **56** in
  `docs/api/desktop/_hw-test-2026-09-17.md`.
