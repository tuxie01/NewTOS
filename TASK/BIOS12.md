# BIOS12 — Spec `docs/api/bios/` (`Setexc`, `Kbshift`) widerspricht Code und EmuTOS-Quelle

Status: **ERLEDIGT** 2026-10-31 ([Quittung](BIOS12-STATUS-DONE.MD)) · Prioritaet: **P3** · Typ: Researcher — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-bios-core.md`

## Sachverhalt

**BIOS-CORE-011 (Punkte 2 und 3) — S4 — MEASURED (Doku widerspricht Messung oder Code am auditierten Commit).** Beleg: Bericht Abschnitt 2, Lauf `measurements/measurement-2026-10-28-bios-core-audit/RUNINFO.TXT`. Dateigruppe `docs/api/bios/`:

1. `docs/api/bios/05-setexc.md` ist veraltet: „Out-of-range `num` (`>= NVEC`) returns 0“ — der Code hat seit B1 den Seite-0-Zweig `0x120 … 0x1FF`; „Pterm does not run etv_term“ — der Code tut es (H1-Upcall); der Verweis auf `vector` als `#[allow(dead_code)]` stimmt nicht mehr; die `0x120`-Begruendung („GEM hooks“) ist falsch (siehe [BIOS07](BIOS07.md), BIOS-CORE-006).
2. `docs/api/bios/0b-kbshift.md`: „`MODE_HOME`/`MODE_INSERT` … TT/Falcon-era; on a plain ST never set“ — widerlegt durch `references/emutos/bios/ikbd.c:876-883` (siehe [BIOS10](BIOS10.md)).

## Arbeit

1. Beide Dateien aus dem Code und der EmuTOS-Quelle korrigieren, jede Korrektur mit `file:line`; Korrekturen als solche vermerken.
2. Aussagen, die von [BIOS07](BIOS07.md)/[BIOS10](BIOS10.md) abhaengen, nach deren Fix nachziehen.

## Abnahme

- Jede korrigierte Aussage ist belegt; kein Eintrag widerspricht mehr `bios-core-r1.tsv` 46–62 oder Sonde K01–K13.
- `cargo xtask test` gruen (unveraendert); `cargo xtask doc-check` der geaenderten Doku 0 Fehler.

## Stand 2026-10-28

- `docs/api/bios/05-setexc.md` und `docs/api/bios/0b-kbshift.md` berichtigt
  (`[BIOS12]`): drei Bereiche im Pi-Binding von `Setexc` (Schatten < 0x120,
  Seite 0 bis 0x1FF vorzeichenerweitert, ab 0x200 Rueckgabe 0); `etv_term` wird
  gerufen; ADR-031-Tabelle; `etv_timer` steht bei EmuTOS nach dem Start auf
  `tikfrk` (`references/emutos/bdos/time.c:167`), nicht auf `just_rts`;
  `Kbshift`-Bits 5/6 auf jeder Maschine, Bit 7 wird behalten.
- Offene Entscheidungen (in den Spec-Dateien): `Setexc` D1 Rueckgabebreite,
  D2 `num >= 0x200`, D3 Schatten gegen Seite 0 (speist [BIOS07](BIOS07.md));
  `Kbshift` D1 Speicherung von Bit 5–7 (speist [BIOS10](BIOS10.md)).
