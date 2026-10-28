# XBIOS14 — Orakelgrenze: Hatari nimmt keinen Schreibzugriff auf die RTC an, `Gettime` nach `Settime` ist nicht vergleichbar

Status: **OFFEN** · Prioritaet: **P3** · Typ: Researcher — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-xbios-system.md`

## Sachverhalt

**XBIOS-SYS-007 — S4 — MEASURED (Orakelgrenze).** Beleg: Bericht Abschnitt 2, Lauf `measurements/measurement-2026-10-28-xbios-system-audit/RUNINFO.TXT`.

- `measurements/measurement-2026-10-28-xbios-system-audit/runs/PROBE-EMUTOS-tt-nvtrace/trace.txt`: nach `Settime(0x2A4320A3)` schreibt EmuTOS (`nvram_setregs`, `references/emutos/bios/clock.c:893-907`) Jahr 33, Monat 2, Tag 3, 4:05:06 in die Register 9…0; das folgende `Gettime` liest 58/10/28 12:20:12 (Wirtsuhr). In allen EmuTOS-Laeufen bleibt `Gettime` die Wirtsuhr.
- Folge: `Gettime` nach `Settime` und `Gettime` mit Bit 31 ([XBIOS09](XBIOS09.md)) sind am Orakel nicht vergleichbar; der Audit vergleicht `Settime` ueber den GEMDOS-Cache (`Tgetdate`/`Tgettime`), den `Settime` auf EmuTOS immer schreibt (`references/emutos/bios/clock.c:1221-1222`).

## Arbeit

1. Grenze in `docs/analysis/ORACLE.md` vermerken; pruefen, ob ein Profil oder eine Hatari-Option eine schreibbare RTC liefert (nicht aus dem Gedaechtnis — Quelle/Messung angeben).

## Abnahme

- Die Grenze steht in `docs/analysis/ORACLE.md` mit Beleg; die zwei Zeilen `+14:0x16` / `14:0x17` aus Bericht Abschnitt 5 sind als „wartet auf RTC-faehiges Orakel“ vermerkt.
- Kein vorher gleicher tosdump-Fall weicht neu ab; `cargo xtask test` gruen; `cargo xtask doc-check` der geaenderten Doku 0 Fehler.
