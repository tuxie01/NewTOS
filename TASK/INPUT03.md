# INPUT03 — Tastatur-Mausemulation (Alt+Pfeile, Alt+Clr/Home, Alt+Insert) fehlt

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner pre-start check 2026-10-31: no program needs the mouse-keyboard emulation | Reopen: a program needs the mouse-keyboard emulation

Migrated (owner batch 10, 2026-11-11), evidence: TASK/INPUT03.md status line: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: kein Programm braucht die Maus-Tastatur-Emulation)

Line 3 before the migration:
Status: **GEPARKT** 2026-10-31 (Anwender, Vor-Start-Pruefung: kein Programm braucht die Maus-Tastatur-Emulation) · Prioritaet: **P3** · Typ: Developer — Fund O1 aus [BIOS10](BIOS10.md) (2026-10-29)

## Sachverhalt

EmuTOS emuliert die Maus mit der Tastatur (`references/emutos/bios/ikbd.c:93-99`,
`:295-387`, `:904-908`, `handle_mouse_mode`): Alt+Clr/Home und Alt+Insert sind
Maustasten, nicht Tasten in der Schlange. NewTOS reiht sie als Tasten ein
(`ksprobe` Q-Zeilen: NewTOS 8 Eintraege, Orakel 2;
`measurements/measurement-2026-10-29-bios10/RUNINFO.TXT`). `handle_mouse_mode`
ist `unmapped`.

## Arbeit

Nach EmuTOS nachbauen; Sonde `ksprobe` Q-Zeilen gleich dem Orakel.

## Abnahme

- Q-Zeilen gleich; Hosttests gruen.

[Gemeinsame Regeln](README.md) gelten.
