# DESK55 — `win_sname` zeigt nur das Verzeichnis statt der ganzen `p_spec`

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner decision 2026-11-11 at the legacy migration: not on the work queue | Reopen: a card program or port is affected, or the owner schedules it

Migrated (owner batch 9, 2026-11-11), evidence: TASK/DESK55.md status line: OFFEN, no DONE receipt, no fix/feat commit names the id

Line 3 before the migration:
Status: **OFFEN** · Prioritaet: **P3** · Typ: Abweichung von EmuTOS — Nebenfund aus [DESK50](DESK50.md)

Abhaengigkeiten: [DESK50](DESK50.md) (`do_wopen`/`do_diropen`, `w_obid`)

## Sachverhalt

EmuTOS setzt den Fenstertitel aus der ganzen `p_spec`, z.B. `" C:\SUB\*.* "` (`references/emutos/desk/deskwin.c:846-858`). Bei uns titelt `set_window_mask`/`spec_and_title` mit dem Verzeichnis (plus Maske nur, wenn sie nicht `*.*` ist). `docs/api/desktop/desksupp-wopen.md` fuehrt das als *Unterschied 1* mit der Begruendung „vorbestehende Darstellungsentscheidung" — das ist keine Quellenbegruendung; Standard ist angleichen.

Der Befund stammt aus der DESK50-Arbeit ([Statusdatei](DESK50-STATUS-DONE.MD),
Abschnitt *Offen*) und ist **nicht nachgeprueft**. Erster Schritt: die
EmuTOS-Stelle lesen und unseren Code dagegenhalten, bevor etwas geaendert wird.

## Arbeit

1. EmuTOS-Stelle lesen, Spec in `docs/api/desktop/` pruefen bzw. ergaenzen.
2. Abweichung belegen (Quelle + QEMU-Lauf) oder als konform schliessen.
3. 1:1 angleichen (ADR-022), ohne Umgehung.

Abnahme: Verhalten wie EmuTOS, Beleg unter `measurements/`, Basislinien gehalten.
