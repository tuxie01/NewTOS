# XBIOS11 — `Random` holt den ersten Seed aus der 60-Hz-Bildfrequenzuhr statt aus `_hz_200`

Status: **GEPARKT** 2026-10-31 (Anwender, Vor-Start-Pruefung: kein Kartenprogramm haengt am Seed-Ursprung (einziger Nutzer TeraDesk-Formatieren)) · Prioritaet: **P3** · Typ: Developer (Seed aus `_hz_200`) oder Researcher (COMPATIBILITY-Eintrag) — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-xbios-system.md`

## Sachverhalt

**XBIOS-SYS-004 — S4 — MEASURED.** Beleg: Bericht Abschnitt 2, Lauf `measurements/measurement-2026-10-28-xbios-system-audit/RUNINFO.TXT`.

- Sonde `xsseed.c` X01 (erster `Random()` nach dem Start, `_hz_200` davor und danach): EmuTOS `yes` (tt, tt-repeat, falcontc), NewTOS `no` (beide Laeufe, `hz_200` 3024, Wert `91cc67` = Seed aus `frame_clock` 908) (`measurements/measurement-2026-10-28-xbios-system-audit/runs/SEED-EMUTOS-tt`, `measurements/measurement-2026-10-28-xbios-system-audit/runs/SEED-NEWTOS-raspi4b`). Der Generator selbst ist gleich (R01–R03, P08).
- Lokalisiert: `tos-api/src/xbios/system.rs:14-28@random` (`h.frame_clock()`; `s = 1` bei Seed 0, wo EmuTOS mit 0 weiterrechnet — in der Praxis unerreichbar).
- EmuTOS-Stelle: `references/emutos/bios/xbios.c:421-430`.
- Spec: `docs/api/xbios/11-random.md` fuehrt den Auto-Seed als „Reference“, nicht als Vertrag; kein COMPATIBILITY-Eintrag.
- Fehlerbild: nur ein Programm, das `_hz_200` liest und den ersten `Random`-Wert vorhersagt.

## Abnahme

- Bestaetigender Fall: Sonde X01 (`yes`) gleich dem Orakel; R01–R03 und P08 bleiben gleich.
- Kein vorher gleicher Fall in `bios-r1.tsv`, `bios-core-r1.tsv`, `xbios-chardev-r1.tsv`, `xbios-system-r1.tsv` weicht neu ab.
- `cargo xtask test` gruen.
- `docs/STATUS.md` und Spec im selben Commit; `cargo xtask doc-check` der geaenderten Doku 0 Fehler.
