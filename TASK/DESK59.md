# DESK59 — `act_chg` prueft `ob_flags & DISABLED` statt `ob_state`

Status: DONE (2026-11-11) | Priority: P3 | Commit: d890e322f | Receipt: DESK59-STATUS-DONE.MD

Migrated (owner batch 4, 2026-11-11), evidence: receipt TASK/DESK59-STATUS-DONE.MD exists, commit d890e322f, TASK/DESK59-STATUS-DONE.MD, commit d890e322f (fix(desk): act_chg tests DISABLED in ob_state, not ob_flags (DESK59)), fix/feat commits naming the id: d890e322f (n=1), measurements/measurement-2026-10-16-desk59/, the receipt names no hash for this commit, the Migrated line names it

Line 3 before the migration:
Status: **ERLEDIGT** 2026-10-16 ([Quittung](DESK59-STATUS-DONE.MD)) · Prioritaet: **P3** · Typ: Konformitaet — Nebenfund DESK43 U5b (2026-10-14)

`kernel/src/desk/act.rs:75` bricht ab, wenn `ob_flags & DISABLED` gesetzt ist.
EmuTOS `act_chg` (`desk/deskact.c:428-430`) prueft `ob_state & DISABLED`.
`DISABLED` (0x0008) ist ein State-Bit; im Flag-Wort ist 0x0008 `EDITABLE`.
Folge: ein Objekt mit `EDITABLE` wird faelschlich uebersprungen, ein wirklich
deaktiviertes nicht. Red-Test, dann Fix; Driftprobe/Orakel vorher-nachher.

[Gemeinsame Regeln](README.md) gelten.

## Abschluss — 2026-10-16

Status: **ERLEDIGT (QEMU), nicht committet.** `act_chg` prueft jetzt `ob_state & DISABLED`. Test: Desktop-Selbsttest
`[desk-act]` 0/3 → 3/3. Keine sichtbare Aenderung (45/45 Dumps byte-gleich zur Abnahme 2026-10-15). Quittung:
[DESK59-STATUS-DONE.MD](DESK59-STATUS-DONE.MD); Beleg `measurements/measurement-2026-10-16-desk59/RUNINFO.TXT`.
