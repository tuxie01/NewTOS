# XBIOS12 — Startuhr ohne RTC: 1980-01-01 bzw. FAT-Datum von `KERNEL8.IMG`, EmuTOS Baudatum des ROMs

Status: **OFFEN** · Prioritaet: **P3** · Typ: Researcher — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-xbios-system.md` · Bezug: [RTC01](RTC01.md)

## Sachverhalt

**XBIOS-SYS-005 — S4 — PLAUSIBLE.** NOT LOCALIZED (nicht gegen das Orakel gemessen). Beleg: Bericht Abschnitt 2, Lauf `measurements/measurement-2026-10-28-xbios-system-audit/RUNINFO.TXT`.

- NewTOS gemessen: `[clock] no kernel8.img FAT date on C: — clock stays at 1980-01-01` und T00 `1980-01-01 00:00:14` (`measurements/measurement-2026-10-28-xbios-system-audit/runs/PROBE-NEWTOS-raspi4b`); Quelle `kernel/src/lib.rs:1315@seed_clock_from_kernel_file`.
- EmuTOS: `DEFAULT_DATETIME` = `MAKE_ULONG(os_header.os_dosdate, 0)` (`references/emutos/bios/ikbd.h:95`), Rueckfall `references/emutos/bios/clock.c:1319`, Initialisierung der IKBD-Uhr `references/emutos/bios/clock.c:1211`.
- Entscheidender Fall: ein Orakelprofil ohne RTC — gibt es nicht (Hatari liefert die Wirtsuhr). Ersatz: echte TT/ST-Hardware ohne Batterie, `Gettime` direkt nach dem Start `== os_dosdate << 16`.
- Spec: `docs/api/xbios/17-gettime.md` nennt den Unterschied, verschweigt die Saat aus `KERNEL8.IMG`; kein COMPATIBILITY-Eintrag. `TASK/RTC01.md` ersetzt die Quelle spaeter.

## Arbeit

1. Verhalten festlegen (Baudatum des ROMs wie EmuTOS oder dokumentierte Abweichung), Spec und COMPATIBILITY entsprechend; Hinweis auf [RTC01](RTC01.md).

## Abnahme

- Entscheidung dokumentiert; ohne Orakelprofil ist die Abnahme eine Pruefung der Doku gegen `kernel/src/lib.rs:1315@seed_clock_from_kernel_file` und `references/emutos/bios/clock.c:1319`.
- Kein vorher gleicher tosdump-Fall weicht neu ab; `cargo xtask test` gruen; `cargo xtask doc-check` der geaenderten Doku 0 Fehler.
