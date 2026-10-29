# SPEC02 — XBIOS-Inventar: Summen und Liste der unbeantworteten Funktionen veraltet

Status: **OFFEN** · Prioritaet: **P3** · Typ: Researcher (Doku) — Fund aus [XBIOS02](XBIOS02.md) (2026-10-29)

## Sachverhalt

`docs/api/xbios/_full-function-inventory.md`: Summen und die Liste der
unbeantworteten Funktionen sind seit XBIOS01 und BIOS15 nicht nachgezogen;
XBIOS02 hat nur die Zeile 0x18 berichtigt.

Dazu veraltete Aussagen (Funde aus BIOS04/BIOS10): `docs/HARDWARE-PROFILE.md`
(IKBD/AUX/MIDI-Zustand, „`Kbshift` trap not exposed“), `docs/api/bios/_state.md`
(„`Kbshift` trap not exposed“).

## Arbeit

Jede Zeile gegen `tos-api/src/xbios/mod.rs` (`dispatch`) und die Quittungen
pruefen, Summen neu zaehlen (per Skript, Skript im Ticket nennen).

## Abnahme

- Summen stimmen mit dem Dispatcher ueberein; doc-check 0 Fehler.

[Gemeinsame Regeln](README.md) gelten.
