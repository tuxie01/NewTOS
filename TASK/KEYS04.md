# KEYS04 — `Kbshift(-1)` sieht ein veraltetes Byte, solange niemand pumpt

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner pre-start check 2026-10-31: no card program polls Kbshift (only apitest) | Reopen: a program polls Kbshift

Migrated (owner batch 10, 2026-11-11), evidence: TASK/KEYS04.md status line: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: kein Kartenprogramm pollt Kbshift (nur apitest))

Line 3 before the migration:
Status: **GEPARKT** 2026-10-31 (Anwender, Vor-Start-Pruefung: kein Kartenprogramm pollt Kbshift (nur apitest)) · Prioritaet: **P3** · Typ: Developer (erst messen) — Fund O3 aus [BIOS10](BIOS10.md) (2026-10-29)

## Sachverhalt

EmuTOS aktualisiert `shifty` im Tastatur-Interrupt. NewTOS dekodiert
Tastenbytes nur trap-seitig: `Kbshift` pumpt die Eingabe nicht, `Bconstat(2)`
nur bei leerer Schlange (`kernel/src/host_env.rs:56@fill_from_device`). Ein
Programm, das `Kbshift(-1)` pollt (Spiele, Shift-Abfrage im Hauptloop), sieht
den Zustand verspaetet oder gar nicht. Im BIOS10-Entwicklungslauf gesehen.

## Arbeit

1. Fall: Shift halten, Schleife nur aus `Kbshift(-1)`, beidseitig.
2. Vorschlag: `Kbshift` pumpt wie `read_key_loop` (ohne Schlangeneintrag zu
   verbrauchen) oder Modifikatoren schon im IRQ-Pfad nachfuehren.

## Abnahme

- Fall gleich dem Orakel; keine neue Abweichung.

[Gemeinsame Regeln](README.md) gelten.
