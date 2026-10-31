# SPEC02 — XBIOS-Inventar: Summen und Liste der unbeantworteten Funktionen veraltet

Status: **GEPARKT** 2026-10-31 (Anwender, Vor-Start-Pruefung: reine Nachpflege; offene Arbeit liest die Stellen nicht) · Prioritaet: **P3** · Typ: Researcher (Doku) — Fund aus [XBIOS02](XBIOS02.md) (2026-10-29)

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

Dazu (Fund COOK02, `docs/analysis/cook02-country-facts-2026-10-30.md`
"Out-of-scope observations"): `_AKP` als "low word = language id" bzw.
"keyboard language" ist ungenau — EmuTOS: Bits 15..8 Anzeige (Sprache,
Zeichensatz), Bits 7..0 Tastatur (`references/emutos/bios/country.c:96-113`,
`:162`). Stellen: `tos-api/src/cookie.rs:30@AKP`,
`tos-api/src/i18n.rs:38@current_lang`, `docs/api/bios/_cookie-jar.md`
(Zeile 56), `docs/api/gemdos/154-ssystem.md` (Zeile 647).

Dazu (Lead 2026-10-30, aus COOK02): Kommentar `tos-api/src/cookie.rs:219-220@seed_defaults`
"`_VDO` is open until ARCH06" ist veraltet (ARCH06 entschieden 2026-10-29).

## Arbeit

Jede Zeile gegen `tos-api/src/xbios/mod.rs` (`dispatch`) und die Quittungen
pruefen, Summen neu zaehlen (per Skript, Skript im Ticket nennen).

## Abnahme

- Summen stimmen mit dem Dispatcher ueberein; doc-check 0 Fehler.

[Gemeinsame Regeln](README.md) gelten.
