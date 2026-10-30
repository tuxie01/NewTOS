# SPEC02 — XBIOS-Inventar: Summen und Liste der unbeantworteten Funktionen veraltet

Status: **OFFEN** · Prioritaet: **P3** · Typ: Researcher (Doku) — Fund aus [XBIOS02](XBIOS02.md) (2026-10-29)

## Sachverhalt

`docs/api/xbios/_full-function-inventory.md`: Summen und die Liste der
unbeantworteten Funktionen sind seit XBIOS01 und BIOS15 nicht nachgezogen;
XBIOS02 hat nur die Zeile 0x18 berichtigt.

Dazu veraltete Aussagen (Funde aus BIOS04/BIOS10): `docs/HARDWARE-PROFILE.md`
(IKBD/AUX/MIDI-Zustand, „`Kbshift` trap not exposed“), `docs/api/bios/_state.md`
(„`Kbshift` trap not exposed“).

Dazu (Fund BIOS17): der Ort `0xA40` des Reset-Einstiegs in
`docs/api/structures/osheader.md`, `docs/api/bios/_reset-warm-cold.md` §5 und
`docs/api/gemdos/154-ssystem.md` ist seit MEM12 veraltet (Stub-Seite
`0x7E000` + `0x80`); Kommentar `kernel/src/reset.rs` „page-0 reset entry“.

## Arbeit

Jede Zeile gegen `tos-api/src/xbios/mod.rs` (`dispatch`) und die Quittungen
pruefen, Summen neu zaehlen (per Skript, Skript im Ticket nennen).

## Abnahme

- Summen stimmen mit dem Dispatcher ueberein; doc-check 0 Fehler.

[Gemeinsame Regeln](README.md) gelten.
