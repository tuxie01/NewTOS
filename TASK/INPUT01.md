# INPUT01 — the mouse pointer freezes for 2-3 s while zView starts

Status: DONE (2026-11-11) | Priority: P2 | Commit: 22171a83e | Receipt: INPUT01-STATUS-DONE.MD

Migrated (owner batch 4, 2026-11-11), evidence: receipt TASK/INPUT01-STATUS-DONE.MD exists, commit 22171a83e, TASK/INPUT01-STATUS-DONE.MD, commit 22171a83e (feat(input): the timer interrupt moves the mouse while a program compu), fix/feat commits naming the id: 22171a83e (n=1), the receipt names no hash for this commit, the Migrated line names it

Line 3 before the migration:
Status: **OFFEN** · Prioritaet: **P2** · Typ: Abweichung, Ursache unbekannt — Anwender auf der echten Pi 4, 2026-10-07

## Sachverhalt

Nach dem Start von zView bleibt der Mauszeiger einige Sekunden stehen und
faengt sich nach 2-3 Sekunden wieder. zView laedt in dieser Zeit seine Codecs
(`Pexec 5/3/4` je `.NLD`) und liest Verzeichnisse.

Vermutung, NICHT belegt: Mausbewegung wird bei NewTOS im `pump_input`-Pfad
abgeholt (AES-Wartestellen, `docs/STATUS.md` QUIRK-002-Abschnitt), nicht per
Interrupt wie bei EmuTOS (IKBD-ACIA-Interrupt → `mousevec`/Cursor im VBL).
Arbeitet ein Programm ohne AES-Aufruf, bewegt sich der Zeiger nicht.

## Arbeit

Ursache belegen (Zeitstempel der Mauspakete gegen die Aufrufe von zView in
QEMU), mit EmuTOS vergleichen (`bios/ikbd.c`, `aciavecs.S`, VBL-Cursor),
dann Loesungsweg vorlegen.
