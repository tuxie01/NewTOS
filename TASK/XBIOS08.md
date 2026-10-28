# XBIOS08 — `Settime` normalisiert bzw. klemmt unmoegliche Felder statt sie roh zu uebernehmen

Status: **OFFEN** · Prioritaet: **P2** · Typ: Developer (rohe Felder halten wie EmuTOS) oder Researcher (Abweichung in COMPATIBILITY, Spec korrigieren) — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-xbios-system.md`

## Sachverhalt

**XBIOS-SYS-001 — S3 — MEASURED.** Beleg: Bericht Abschnitt 2, Lauf `measurements/measurement-2026-10-28-xbios-system-audit/RUNINFO.TXT`; Gegenueberstellung `measurements/measurement-2026-10-28-xbios-system-audit/probe-diff-emutos-tt-vs-newtos.txt`; Laeufe `measurements/measurement-2026-10-28-xbios-system-audit/runs/PROBE-EMUTOS-tt`, `measurements/measurement-2026-10-28-xbios-system-audit/runs/PROBE-NEWTOS-raspi4b`.

Sonde T10–T22 (sofort zuruecklesen), T23–T28 (nach dem naechsten Uhrtakt). Gesetzt → gelesen, EmuTOS / NewTOS:

- `2001-02-03 25:00:00` → `25:00:00` / `2001-02-04 01:00:00`
- `2001-02-03 04:05:62` → `04:05:62` / `04:06:02`; nach einem Takt `04:06:00` / `04:06:04`
- `2001-13-03` → `2001-13-03` / `2001-12-03`; `2001-02-30` → `2001-02-30` / `2001-02-28`
- `0x00000000` → `1980-00-00 00:00:00` / `1980-01-01 00:00:00`
- `2001-02-03 23:59:62` nach einem Takt → `23:60:00` / `2001-02-04 00:00:02`
- `2001-02-30 23:59:58` nach einem Takt → `2001-03-01 00:00:00` / `2001-02-28 23:59:58`

Gueltige Werte (T01–T08: Tag-, Monats-, Jahreswechsel, Schaltjahr 2000, 2100, Ueberlauf 2107→1980) sind gleich.

- Lokalisiert: `tos-api/src/gemdos/time.rs:205-216@xbios_settime` (Monat/Tag geklemmt, `compose` rechnet Stunde/Minute/Sekunde in Sekunden um); beobachtet ueber `Tgetdate`/`Tgettime` und `Gettime`.
- Eingabepruefung: EmuTOS — die 27 `Settime`-Argumente im Hatari-Trace (`measurements/measurement-2026-10-28-xbios-system-audit/runs/PROBE-EMUTOS-tt/trace-program.txt`) sind die `set=`-Werte des Protokolls; NewTOS indirekt (T01 liest exakt zurueck).
- EmuTOS-Stellen: `references/emutos/bios/clock.c:1218-1222` (`current_time`/`current_date` roh gesetzt), `references/emutos/bdos/time.c:175-230` (`tikfrk` zaehlt die rohen Felder weiter), `references/emutos/bdos/time.c:98-135` (`Tgetdate`/`Tgettime` geben den Cache zurueck).
- Spec: `docs/api/xbios/16-settime.md` vermerkt das Klemmen als „documented deviation“, behauptet aber Stunde/Minute/Sekunde „pass through unvalidated“ (siehe [XBIOS13](XBIOS13.md)). Kein COMPATIBILITY-Eintrag.
- Fehlerbild: Uhr-Stellprogramme/Testprogramme, die Rohwerte schreiben und zuruecklesen; normale Programme setzen gueltige Zeiten.

## Arbeit

1. Entscheidung Researcher/Anwender: Rohfelder halten oder Abweichung eintragen.
2. Fix gegen die EmuTOS-Stellen oben; Hosttests fuer die Beispielfaelle.

## Abnahme

- Bestaetigender Fall: Sonde T10–T28 (Tgetdate/Tgettime nach `Settime`) gleich dem Orakel; `Settime` ist am Orakel nur ueber den GEMDOS-Cache vergleichbar ([XBIOS14](XBIOS14.md)).
- Kein vorher gleicher Fall in `bios-r1.tsv`, `bios-core-r1.tsv`, `xbios-chardev-r1.tsv`, `xbios-system-r1.tsv` weicht neu ab.
- `cargo xtask test` gruen.
- `docs/STATUS.md` und Spec im selben Commit; `cargo xtask doc-check` der geaenderten Doku 0 Fehler.
