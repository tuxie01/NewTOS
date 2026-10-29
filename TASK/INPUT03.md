# INPUT03 — Tastatur-Mausemulation (Alt+Pfeile, Alt+Clr/Home, Alt+Insert) fehlt

Status: **OFFEN** · Prioritaet: **P3** · Typ: Developer — Fund O1 aus [BIOS10](BIOS10.md) (2026-10-29)

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
