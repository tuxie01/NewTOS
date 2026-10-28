# XBIOS09 — `Gettime`: gepackter LONG nullerweitert; ab Jahr 2044 positiv statt negativ

Status: **OFFEN** · Prioritaet: **P2** · Typ: Developer — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-xbios-system.md`

## Sachverhalt

**XBIOS-SYS-002 — S3 — MEASURED.** Beleg: Bericht Abschnitt 2, Lauf `measurements/measurement-2026-10-28-xbios-system-audit/RUNINFO.TXT`.

- Sonde `xshi.c` H01, `Settime(0x8CCF6000)` (2050-06-15 12:00:00): `ret=8ccf6000 hi=00000000 negative-as-long=no` (`measurements/measurement-2026-10-28-xbios-system-audit/runs/HIGH-NEWTOS-raspi4b`, `measurements/measurement-2026-10-28-xbios-system-audit/runs/HIGH-NEWTOS-raspi4b-repeat`).
- Lokalisiert: `tos-api/src/xbios/mod.rs:108@GETTIME` (`xbios_gettime() as i64` auf einem `u32`).
- EmuTOS-Seite: `LONG gettime(void)` (`references/emutos/bios/clock.c:1266`); die obere Haelfte ist die Vorzeichenerweiterung des LONG (`docs/TOS-REFERENCE-DUMPER.md` §3, `ret_hi`). **Am Orakel nicht messbar:** Hatari liefert die Wirtsuhr (2026), nie Bit 31 (`measurements/measurement-2026-10-28-xbios-system-audit/runs/HIGH-EMUTOS-tt`, `measurements/measurement-2026-10-28-xbios-system-audit/runs/PROBE-EMUTOS-tt-nvtrace`, siehe [XBIOS14](XBIOS14.md)) — der Befund stuetzt sich dort auf die ABI-Regel (gelesen, nicht gemessen).
- Spec: `docs/api/xbios/17-gettime.md` „LONG result **sign-extended** into `x0`“, ebenso `sdk/libc/mintlib/include/mint/osbind.h`; der Kommentar `tos-api/src/xbios/mod.rs:101-103@SETTIME` sagt „zero-extended“. Gleiche Klasse wie [XBIOS06](XBIOS06.md).
- Fehlerbild: `long t = Gettime(); if (t < 0) …` faellt ab 2044 anders aus.

## Arbeit

1. Vorzeichenerweiterung des LONG im Dispatcher; Kommentar `tos-api/src/xbios/mod.rs:101-103@SETTIME` richtigstellen; Hosttest mit `0x8CCF6000`.

## Abnahme

- Bestaetigender Fall: Sonde H01 (`hi=ffffffff`); am Orakel nicht messbar (XBIOS14), daher gilt der Hosttest als Abnahmefall; vorgeschlagene Zeile `14:0x17 -` (Bericht Abschnitt 5) braucht ein Orakel mit schreibbarer RTC.
- Kein vorher gleicher Fall in `bios-r1.tsv`, `bios-core-r1.tsv`, `xbios-chardev-r1.tsv`, `xbios-system-r1.tsv` weicht neu ab.
- `cargo xtask test` gruen.
- `docs/STATUS.md` und Spec im selben Commit; `cargo xtask doc-check` der geaenderten Doku 0 Fehler.

## Anwender 2026-10-28

- Vorzeichenerweiterung gemaess Regel (`docs/COMPATIBILITY.md`, „LONG
  results in `x0`“): `Gettime` erweitert heute mit Nullen
  (`tos-api/src/xbios/mod.rs:108@GETTIME`).
