# BIOS04 — MIDI und IKBD melden „nie bereit“ und verweigern Ausgabe (`Bcostat`/`Bconout` 3, 4)

Status: **OFFEN** · Prioritaet: **P1** · Typ: Researcher (FS11-Entscheidung neu treffen), danach Developer — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-bios-core.md`

## Sachverhalt

**BIOS-CORE-001 — S2 — CONFIRMED (`Bcostat`) + MEASURED (`Bconout`).** Beleg: Bericht Abschnitt 2, Lauf `measurements/measurement-2026-10-28-bios-core-audit/RUNINFO.TXT`.

- `bios-r1` 66 `Bcostat(3)`, 67 `Bcostat(4)` — DIFF, EmuTOS `-1`, NewTOS `0`, beide Profile (`measurements/measurement-2026-10-28-bios-core-audit/runs/NEWTOS-bios-r1/tosdiff-newtos-vs-emutos.txt`).
- Sonde: B02 `Bconout(3,0xFE)` EmuTOS `1`, NewTOS `0`; B03 `Bconout(4,0x11)` `1` / `0`; B11/B12 `Bcostat(3)`/`(4)` nach der Ausgabe `-1` / `0` (`measurements/measurement-2026-10-28-bios-core-audit/runs/PROBE-EMUTOS-tt`, `measurements/measurement-2026-10-28-bios-core-audit/runs/PROBE-NEWTOS-raspi4b`).
- Lokalisiert: Geraetetabelle `tos-api/src/bios/chardev.rs:229@DEVICES` (Eintraege 3 MIDI und 4 IKBD = `Absent`, `tos-api/src/bios/chardev.rs:85@Absent`).
- EmuTOS-Stellen: `references/emutos/bios/chardev.c:38-42` (Slots 3/4 vertauscht), `references/emutos/bios/ikbd.c:916` `bcostat4`, `:933` `bconout4` (gibt `1`); `references/emutos/bios/midi.c:66` `bcostat3`, `:84` `bconout3` (gibt `1`). Auch EmuTOS ohne IKBD-ACIA antwortet `bcostat4` mit `-1` „OK (but output will be ignored)“ (`ikbd.c:916`ff., `#else`-Zweig).
- Entscheidung/Spec: FS11-Profil „absent device“ (Anwender 2026-10-13) steht in `docs/api/gemdos/file/3f-fread.md` (NewTOS-Notizen, Punkt 1), nicht in `docs/COMPATIBILITY.md`; NewTOS verhaelt sich wie dort beschrieben, die Abweichung ist beobachtbar und ohne COMPATIBILITY-Eintrag nicht ACCEPTED.
- Fehlerbild: ein MIDI-Programm mit `while (!Bcostat(4)) ;` haengt auf NewTOS **fuer immer** (neue Information fuer den Anwender; bei AUX hat FS11 den Fall erkannt, `tos-api/src/bios/chardev.rs:115@AuxNoCable`). Auch `Midiws` (XBIOS) geht ueber denselben Eintrag.

## Arbeit

1. Researcher: Entscheidung mit dem Haenger als neuer Information vorlegen — FS11 fuer MIDI/IKBD auf „bereit, Ausgabe verwerfen“ (EmuTOS-Muster) aendern oder die Abweichung in `docs/COMPATIBILITY.md` aufnehmen.
2. Developer: Fix gemaess Entscheidung; Hosttests fuer `Bcostat` und `Bconout` an 3/4.

## Abnahme

- Bestaetigende Faelle: `bios-r1` 66/67 (und 63–71) sowie Sonde B02/B03/B11/B12 EQUAL gegen das Orakel; vorgeschlagene Zeilen `13:3 w:3,w:0xFE` und `13:3 w:4,w:0x11` (Bericht Abschnitt 5) sind erst nach Lockerung des Round-1-Waechters in `bios-core-r1.tsv` aufnehmbar ([BIOS13](BIOS13.md)).
- Kein vorher gleicher Fall in `bios-r1.tsv`, `bios-core-r1.tsv`, `xbios-chardev-r1.tsv`, `xbios-system-r1.tsv` weicht neu ab.
- `cargo xtask test` gruen.
- `docs/STATUS.md` und Spec im selben Commit; `cargo xtask doc-check` der geaenderten Doku 0 Fehler.
