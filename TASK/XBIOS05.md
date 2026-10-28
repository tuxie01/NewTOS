# XBIOS05 — `Bconmap` meldet „nicht vorhanden“ (`0x2C`) fuer jedes Argument

Status: **OFFEN** · Prioritaet: **P2** · Typ: Researcher (Profil „ein Port, `maptabsize` 1“ oder Abweichung in COMPATIBILITY), danach Developer — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-xbios-chardev.md` · Voraussetzung fuer [BIOS08](BIOS08.md)

## Sachverhalt

**XBIOS-CHARDEV-005 — S3 — CONFIRMED.** Beleg: Bericht Abschnitt 2, Lauf `measurements/measurement-2026-10-28-xbios-chardev-audit/RUNINFO.TXT`.

- Faelle: `bios-r1` 183, 185, 186; `xbios-chardev-r1` 43, 45–53 — EXACT-DIFF: EmuTOS `6` (TT) / `7` (Falcon) fuer `-1`, `0` fuer ungueltige Geraete; NewTOS `44` (`measurements/measurement-2026-10-28-xbios-chardev-audit/runs/NEWTOS-xbios-chardev-r1/tosdiff-newtos-vs-emutos.txt`). `bios-r1` 184 / `xbios-chardev-r1` 44 (`Bconmap(-2)`) bestehen PTR_RAM mit `0x2C`.
- Sonde B01–B24: EmuTOS `BCONMAP` mit gueltigem `maptab`, `maptabsize` 4 (TT) / 3 (Falcon), Umschalten auf 7, 8, 9 und zurueck aendert `Iorec(0)`; NewTOS dereferenziert an `0x2C`: `maptab` NULL, Groesse 0 (`measurements/measurement-2026-10-28-xbios-chardev-audit/runs/PROBE-EMUTOS-tt`, `measurements/measurement-2026-10-28-xbios-chardev-audit/runs/PROBE-NEWTOS-raspi4b`).
- Lokalisiert: `tos-api/src/xbios/system.rs:106@bconmap` (konstant `0x2c`).
- EmuTOS-Stellen: `references/emutos/bios/serport.c:1082-1124`; Vorbelegung `references/emutos/bios/serport.c:955-1003` — auch eine ST-Maschine unter diesem ROM hat `maptabsize = 1`, `mapped_device = 6` (`:962-965`).
- Spec: `docs/api/xbios/2c-bconmap.md` waehlt `0x2c` „as on a single-serial build“; der Referenzbau (make 512) ist kein solcher Bau. Kein COMPATIBILITY-Eintrag. Wurzel von BIOS-CORE-007 ([BIOS08](BIOS08.md)).
- Fehlerbild: Programme mit Pruefung `Bconmap(0) == 0x2C` fallen auf Handle 1 zurueck (funktioniert); wer `Bconmap(-2)` ohne Pruefung liest, liest Seite-0-Bytes als Tabelle.

## Arbeit

1. Researcher: Profil festlegen (Bericht „Route“) und `docs/api/xbios/2c-bconmap.md` korrigieren; bei Abweichung Eintrag in `docs/COMPATIBILITY.md`.
2. Developer erst nach freigegebener Spec; Fix gegen `references/emutos/bios/serport.c:1082-1124`.

## Abnahme

- Bestaetigende Faelle: `bios-r1` 183/185/186 und `xbios-chardev-r1` 43, 45–53 EQUAL gegen das Orakel (bei Profil „wie EmuTOS“); `Bconmap(-2)` (`xbios-chardev-r1` 44) liefert einen echten Zeiger (Sonde B02–B04).
- Kein vorher gleicher Fall in `bios-r1.tsv`, `bios-core-r1.tsv`, `xbios-chardev-r1.tsv`, `xbios-system-r1.tsv` weicht neu ab.
- `cargo xtask test` gruen.
- `docs/STATUS.md` und Spec im selben Commit; `cargo xtask doc-check` der geaenderten Doku 0 Fehler.
