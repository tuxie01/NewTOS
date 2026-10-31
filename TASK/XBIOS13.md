# XBIOS13 — Spec `docs/api/xbios/` (`Settime`, `Gettime`, NVRAM) widerspricht Messung und Code

Status: **GEPARKT** 2026-10-31 (Anwender, Vor-Start-Pruefung: reine Spec-Pflege) · Prioritaet: **P3** · Typ: Researcher — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-xbios-system.md`

## Sachverhalt

**XBIOS-SYS-006 (Punkte 1, 2, 3, 5) — S4 — MEASURED (Doku widerspricht Messung oder Code).** Beleg: Bericht Abschnitt 2, Lauf `measurements/measurement-2026-10-28-xbios-system-audit/RUNINFO.TXT`. Dateigruppe `docs/api/xbios/`:

1. `docs/api/xbios/16-settime.md` (Pi binding): „Hour/minute/second pass through unvalidated“ — gemessen normalisiert (T10–T14, [XBIOS08](XBIOS08.md)); ebenso der Code-Kommentar `tos-api/src/gemdos/time.rs:211@hour` „(EmuTOS parity)“ (Code-Kommentar: Developer).
2. `tos-api/src/xbios/mod.rs:109-111@SETTIME` „Gettime returns the packed LONG (zero-extended)“ gegen `docs/api/xbios/17-gettime.md` „sign-extended“ ([XBIOS09](XBIOS09.md)).
3. `docs/api/xbios/_triage.md:80` „`CONF_WITH_NVRAM` off“ und `docs/api/xbios/_dispatcher.md:171` („NVRAM (0x2E) entries are `xbios_unimpl` in the profile“) gegen `docs/api/xbios/_full-function-inventory.md` (=1 im Referenzbau) ([XBIOS10](XBIOS10.md)).
4. `docs/api/xbios/17-gettime.md` „fallback = 1980-01-01 if the clock was never set“ — ohne die Saat aus `KERNEL8.IMG` ([XBIOS12](XBIOS12.md)).

(Punkte 4 und 6 des Berichts — ERRATA und Ledger — stehen in [BIOS13](BIOS13.md).)

## Arbeit

1. Punkte 1, 3, 4 aus Code und EmuTOS-Quelle korrigieren, jede Korrektur mit `file:line`; Punkte, die von XBIOS08–XBIOS12 abhaengen, nach deren Entscheidung.

## Abnahme

- Jede korrigierte Aussage ist belegt; kein Eintrag widerspricht mehr Sonde T01–T33 oder `xbios-system-r1.tsv`.
- `cargo xtask test` gruen (unveraendert); `cargo xtask doc-check` der geaenderten Doku 0 Fehler.
