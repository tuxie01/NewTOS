# GUI03 — `CONF_WITH_LOADABLE_CURSORS`: Mauszeiger aus einer Ressourcendatei

Status: PARKED (2026-11-11) | Priority: P4 | Reason: owner decision 2026-11-11 at the legacy migration: not on the work queue | Reopen: a card program or port is affected, or the owner schedules it

Migrated (owner batch 10, 2026-11-11), evidence: TASK/GUI03.md status line: OFFEN, no DONE receipt, no fix/feat commit names the id

Line 3 before the migration:
Status: **OFFEN** · Prioritaet: **P4** · Typ: Funktionsluecke AES — aus der Schaltertabelle (REF01)

Abhaengigkeiten: —

## Sachverhalt

`CONF_WITH_LOADABLE_CURSORS` ist im Referenzbau **1**
(`references/emutos/include/config.h:1286-1295`): die AES laedt beim Start
eigene Mauszeigerformen aus `A:\EMUCURS.RSC`, statt nur die eingebauten zu
kennen.

Bei uns gibt es davon nichts; die einzige Erwaehnung ist ein Kommentar, der
das festhaelt (`kernel/src/launch.rs:174`).

## Arbeit

Quelle lesen (wo die AES die Datei sucht, was sie enthaelt, was bei Fehlen
passiert), dann nachbauen. Der Pfad ist bei uns nicht `A:` — das Laufwerk ist
Teil der Entscheidung, zusammen mit der Frage, ob die Datei wie bei den
Accessories an zwei Orten gesucht wird (ADR-018s Muster).

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.
