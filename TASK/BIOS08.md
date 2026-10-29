# BIOS08 — Bconmap-Handles 6–9 sind auf NewTOS abwesende Geraete (`Bcostat`/`Bconout`/`Bconin`)

Status: **ERLEDIGT** 2026-10-30 ([Quittung](BIOS08-STATUS-DONE.MD)) · Prioritaet: **P2** · Typ: Developer (Profil folgt aus XBIOS05 Option 1) — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-bios-core.md` · haengt an [XBIOS05](XBIOS05.md) (Wurzel: NewTOS hat kein `Bconmap`)

## Sachverhalt

**BIOS-CORE-007 — S3 — CONFIRMED (`Bcostat`) + MEASURED (`Bconout`, `Bconin`).** Beleg: Bericht Abschnitt 2, Lauf `measurements/measurement-2026-10-28-bios-core-audit/RUNINFO.TXT`.

- `bios-r1` 69/70 `Bcostat(6)/(7)` EmuTOS `-1`, NewTOS `0`; `bios-core-r1` 65 `Bcostat(8)` `-1`/`0` (beide Profile), 66 `Bcostat(9)` nur TT (TT 4 Ports, Falcon 3: `references/emutos/bios/serport.c:981,993`). Sonde B04/B05 `Bconout(6/7,'A')` `1`/`0`; HANG-6 `Bconin(6)` blockiert auf EmuTOS, NewTOS `0` sofort (`measurements/measurement-2026-10-28-bios-core-audit/runs/NEWTOS-bios-core-r1`, `measurements/measurement-2026-10-28-bios-core-audit/runs/HANG-EMUTOS-tt-6`, `measurements/measurement-2026-10-28-bios-core-audit/runs/HANG-NEWTOS-raspi4b`).
- Lokalisiert: `tos-api/src/bios/chardev.rs:360@DEVICES` (6/7 `Absent`, 8/9 jenseits `NDEV = 8`).
- EmuTOS-Stelle: `references/emutos/bios/bios.c:923-927,1152-1156` (Bconmap-Zweig).
- Spec/Entscheidung: FS11 („6/7 unused“), nicht in COMPATIBILITY.

## Arbeit

1. Nach der Entscheidung in [XBIOS05](XBIOS05.md): Profil der Handles 6–9 festlegen (Spec, COMPATIBILITY); Developer-Ticket erst danach.

## Abnahme

- Bestaetigende Faelle: `bios-r1` 69/70 und `bios-core-r1` 65/66 gleich dem Orakel (bei Profil „wie EmuTOS“), sonst als ACCEPTED mit COMPATIBILITY-Eintrag; Sonde B04/B05.
- Kein vorher gleicher Fall in `bios-r1.tsv`, `bios-core-r1.tsv`, `xbios-chardev-r1.tsv`, `xbios-system-r1.tsv` weicht neu ab.
- `cargo xtask test` gruen.
- `docs/STATUS.md` und Spec im selben Commit; `cargo xtask doc-check` der geaenderten Doku 0 Fehler.

## Profil und Lokalisierung (Lead, 2026-10-30)

Profil folgt aus der XBIOS05-Entscheidung (Option 1, Tabelle mit einem Eintrag
wie EmuTOS auf dem ST, `bios/bios.c:923-928`, `:967-972`, `:1000-1005`,
`:1152-1157`): Handle 6 = `maptab[0]` = AUX, dieselben Antworten wie Handle 1;
Handles 7, 8, 9 und hoeher liefern 0. Gegen das TT-Orakel bleiben `bios-r1` 70
und `bios-core-r1` 65/66 verschieden (Portzahl = Geraeteparameter, ACCEPTED wie
in XBIOS05); `bios-r1` 69 wird EQUAL.

- `bconstat`/`bconin`/`bcostat`/`bconout` → `tos-api/src/bios/chardev.rs`
  (`@bconstat`, `@bconin`, `@bcostat`, `@bconout`, Tabelle `@DEVICES`);
  AUX-Eintraege `kernel/src/console.rs` (`@bconstat`, `@bconin`, `@bcostat`,
  `@bconout`); Tabelle `tos-api/src/xbios/bconmap.rs`. `--down` `unmapped`:
  nur `protect_v`/`protect_ww` (ext, m68k-Registerschutz, kein Gegenstueck).
- `Bconin(6)` wartet wie `Bconin(1)` (XBIOS15: AUX ohne Kabel wartet; auf
  raspi4b ohne `serial=on`?) — Verhalten gleich Handle 1, gemessen.

## Stand 2026-10-30 (Umsetzung)

- Umgesetzt nach dem Profil oben, Quittung [BIOS08-STATUS-DONE.MD](BIOS08-STATUS-DONE.MD),
  Beleg `measurements/measurement-2026-10-30-bios08/RUNINFO.TXT`.
  `tos-api/src/bios/chardev.rs:411@device` leitet Handles ab 6 wie EmuTOS ueber
  die Tabelle (`tos-api/src/xbios/bconmap.rs:109@entry_device`); die
  Geraetetabelle endet bei 5.
- `bios-r1` 69 EQUAL; 70 und `bios-core-r1` 65/66 (tt) ACCEPTED als Portzahl
  (`docs/COMPATIBILITY.md` „`Bconmap`: one entry“); Sonde B04 1 = EmuTOS, B05 0
  (ACCEPTED). Kein neu abweichender Fall in den fuenf Tabellen.
- `Bconin(6)`: auf raspi4b gemessen gleich `Bconin(1)` (0 sofort); auf qemu-virt
  nur per Hosttest (Trap-Eintrag des AUX-Treibers), Boot beim Tester.
