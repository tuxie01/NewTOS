# INPUT02 — Ctrl+Alt+Del erreicht ein Programm in einer Endlosschleife nicht

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner decision 2026-11-11 at the legacy migration: not on the work queue | Reopen: a card program or port is affected, or the owner schedules it

Migrated (owner batch 10, 2026-11-11), evidence: TASK/INPUT02.md status line: OFFEN, no DONE receipt, no fix/feat commit names the id

Line 3 before the migration:
Status: **OFFEN** · Prioritaet: **P3** · Typ: Abweichung von EmuTOS — Nebenfund RST01 (2026-10-17)

EmuTOS prueft die Neustart-Kombination in `kbd_int`, also in der
Tastatur-Unterbrechung (`bios/ikbd.c:734-747`) — ein Programm, das ohne
Systemaufruf in einer Schleife haengt, laesst sich so neu starten. Seit
[INPUT01](INPUT01-STATUS-DONE.MD) dekodiert NewTOS Tasten nicht mehr im
Timer-Interrupt; `decode_key` laeuft erst beim naechsten Trap/Eingabe-Pump.
Folge: ein haengendes EL0-Programm ist nur per Strom zu beenden.

Auftrag: INPUT01-Gruende lesen, Weg finden, die Reset-Pruefung (nur sie) im
Interrupt-Kontext auszuwerten, ohne INPUT01 zurueckzudrehen; Test mit einer
Sonde in Endlosschleife.

[Gemeinsame Regeln](README.md) gelten.
