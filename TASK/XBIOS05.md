# XBIOS05 — `Bconmap` meldet „nicht vorhanden“ (`0x2C`) fuer jedes Argument

Status: **ERLEDIGT** 2026-10-29 ([Quittung](XBIOS05-STATUS-DONE.MD)) · Prioritaet: **P2** · Typ: Researcher (Profil „ein Port, `maptabsize` 1“ oder Abweichung in COMPATIBILITY), danach Developer — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-xbios-chardev.md` · Voraussetzung fuer [BIOS08](BIOS08.md)

## Sachverhalt

**XBIOS-CHARDEV-005 — S3 — CONFIRMED.** Beleg: Bericht Abschnitt 2, Lauf `measurements/measurement-2026-10-28-xbios-chardev-audit/RUNINFO.TXT`.

- Faelle: `bios-r1` 183, 185, 186; `xbios-chardev-r1` 43, 45–53 — EXACT-DIFF: EmuTOS `6` (TT) / `7` (Falcon) fuer `-1`, `0` fuer ungueltige Geraete; NewTOS `44` (`measurements/measurement-2026-10-28-xbios-chardev-audit/runs/NEWTOS-xbios-chardev-r1/tosdiff-newtos-vs-emutos.txt`). `bios-r1` 184 / `xbios-chardev-r1` 44 (`Bconmap(-2)`) bestehen PTR_RAM mit `0x2C`.
- Sonde B01–B24: EmuTOS `BCONMAP` mit gueltigem `maptab`, `maptabsize` 4 (TT) / 3 (Falcon), Umschalten auf 7, 8, 9 und zurueck aendert `Iorec(0)`; NewTOS dereferenziert an `0x2C`: `maptab` NULL, Groesse 0 (`measurements/measurement-2026-10-28-xbios-chardev-audit/runs/PROBE-EMUTOS-tt`, `measurements/measurement-2026-10-28-xbios-chardev-audit/runs/PROBE-NEWTOS-raspi4b`).
- Lokalisiert: `tos-api/src/xbios/system.rs` (`bconmap`, bis XBIOS05; seitdem `tos-api/src/xbios/bconmap.rs`) (konstant `0x2c`).
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

## Entscheidung des Anwenders 2026-10-28

- Option 1: Tabelle mit einem Eintrag wie EmuTOS auf dem ST
  (`Bconmap(-1)` = 6, `-2` = Zeiger auf `{maptab, 1, 6}`, andere Geraete 0;
  `references/emutos/bios/serport.c:962-965`, `:1096-1097`). Die Falcon-Faelle
  (7) werden als Geraeteparameter je Profil ACCEPTED.
- Die MAPTAB-Funktionszeiger: kein portierter Nutzer → Anforderung zum
  Wiedervorlegen; bis dahin aus EL0 nicht aufrufbar, laut fehlschlagend,
  dokumentiert in `docs/COMPATIBILITY.md`.
- Neue Orakelzeile `Bconmap(-2)` mit Mitschnitt 8 Byte (`BCONMAP`) + 24 Byte
  (erstes `MAPTAB`), in einer neuen Tabelle (bestehende sind eingefroren).
- Lokalisierung (Lead) vor Beginn nach CLAUDE.md „Testing“.

## Lokalisierung (Lead, 2026-10-29)

- `bconmap` → `tos-api/src/xbios/system.rs@bconmap` (`mapped`); `--down`
  vollstaendig. `--up`: `xbiostrap` → `tos-api/src/xbios/mod.rs@dispatch`,
  `biostrap` → `tos-api/src/bios/mod.rs@dispatch`; vollstaendig.
- Mitbetroffen (Wiedertest): `Iorec(0)` (`tos-api/src/xbios/iorec.rs`, haengt in
  EmuTOS am gemappten Port), `Rsconf` (`tos-api/src/xbios/system.rs`), die
  BIOS-Geraetetabelle `tos-api/src/bios/chardev.rs@DEVICES` (Handles 6–9 —
  deren Profil ist [BIOS08](BIOS08.md), nicht dieses Ticket).
- Spec zuerst: `docs/api/xbios/2c-bconmap.md` auf die Entscheidung (Option 1)
  umstellen; Falcon-Faelle (7) und die nicht aufrufbaren `MAPTAB`-Zeiger in
  `docs/COMPATIBILITY.md`.

## Stand 2026-10-29 (Umsetzung)

- Umgesetzt nach Option 1, Quittung [XBIOS05-STATUS-DONE.MD](XBIOS05-STATUS-DONE.MD),
  Beleg `measurements/measurement-2026-10-29-xbios05/RUNINFO.TXT`. Neue Tabelle
  `tosdump/cases/xbios-bconmap-r1.tsv` (CRC `DD429BF8`) mit Referenzen tt/falcontc.
- Fuer [BIOS08](BIOS08.md): mit `maptabsize` 1 ist nach EmuTOS
  (`references/emutos/bios/bios.c:923-928,967-972,1000-1005,1152-1157`) Handle 6
  = Eintrag 0 = AUX (wie Handle 1), Handles 7..9 (und jedes hoehere) liefern 0 —
  Entscheidung dort.


## Entscheidung (Anwender, 2026-10-30)

Bleibt wie gebaut: der Kern liest die veroeffentlichte `Bconmap`-Tabelle nicht
zurueck; die Abweichung bleibt dokumentiert (`docs/api/xbios/2c-bconmap.md`,
`docs/COMPATIBILITY.md`).
