# DESK56 — Zwei Wurfpfade in `dir.rs` verwerfen die Rueckgabe von `do_aopen_drop`

Status: PARKED (2026-11-11) | Priority: P3 | Reason: legacy, evidence unverified; receipt DESK56-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 20, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: 526ad8ac0 (docs(task): close six open tickets per the open-list review) added the receipt TASK/DESK56-STATUS-DONE.MD itself, the report of this research or measurement ticket (16 files), bookkeeping or bulk commit, not accepted as evidence

Line 3 before the migration:
Status: **ERLEDIGT** 2026-10-17 ([Quittung](DESK56-STATUS-DONE.MD)) · Prioritaet: **P3** · Typ: Abweichung von EmuTOS — Nebenfund aus [DESK50](DESK50.md)

Abhaengigkeiten: [DESK50](DESK50.md) (`do_wopen`/`do_diropen`, `w_obid`)

## Sachverhalt

In EmuTOS setzt der Rueckgabewert von `do_aopen` beim Ablegen auf ein Programmsymbol `exit_desktop` (`references/emutos/desk/deskfun.c:1034`, `:1249`). Zwei Pfade in `kernel/src/desk/dir.rs` verwerfen ihn. Wirkung pruefen: verlaesst der Desktop nach einem Drop-Start korrekt seine Schleife?

Der Befund stammt aus der DESK50-Arbeit ([Statusdatei](DESK50-STATUS-DONE.MD),
Abschnitt *Offen*) und ist **nicht nachgeprueft**. Erster Schritt: die
EmuTOS-Stelle lesen und unseren Code dagegenhalten, bevor etwas geaendert wird.

## Arbeit

1. EmuTOS-Stelle lesen, Spec in `docs/api/desktop/` pruefen bzw. ergaenzen.
2. Abweichung belegen (Quelle + QEMU-Lauf) oder als konform schliessen.
3. 1:1 angleichen (ADR-022), ohne Umgehung.

Abnahme: Verhalten wie EmuTOS, Beleg unter `measurements/`, Basislinien gehalten.
