# BIOS11 — `Bconin(2)` mit `conterm` Bit 3 liefert den Shift-Zustand beim Lesen, EmuTOS den beim Tastendruck

Status: **ERLEDIGT** 2026-10-29 mit [XBIOS01](XBIOS01.md) (`a32921d5`), vom Anwender abgenommen · Prioritaet: **P2** · Typ: Developer — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-bios-core.md`

## Sachverhalt

**BIOS-CORE-010 — S3 — MEASURED.** Beleg: Bericht Abschnitt 2, Lauf `measurements/measurement-2026-10-28-bios-core-audit/RUNINFO.TXT`.

- Tastensonde K16 (`measurements/measurement-2026-10-28-bios-core-audit/runs/KEYS-EMUTOS-tt`, `measurements/measurement-2026-10-28-bios-core-audit/runs/KEYS-NEWTOS-raspi4b-injector`): Shift+b, Shift losgelassen, dann `conterm |= 8` und `Bconin(2)`: EmuTOS `0x02300042`, NewTOS `0x00300042`. Die fuenf Tasten davor (a, Shift+a, F1, Ctrl+c, Return) und alle `Bconstat(2)`-Werte sind gleich (`0x001E0061`, `0x001E0041`, `0x003B0000`, `0x002E0003`, `0x001C000D`).
- Lokalisiert: `shift_fold` in `tos-api/src/bios/chardev.rs` (seit XBIOS01 entfernt; liest den aktuellen `Kbshift`-Zustand), aufgerufen aus `tos-api/src/bios/chardev.rs:471@pack_console_key`; der Kommentar dort behauptet „stored `shifty` byte“ — es ist der aktuelle.
- EmuTOS-Stellen: `references/emutos/bios/ikbd.c:900` (Shift-Zustand wird mit dem Tastenwert ins Iorec geschrieben), `:279-280` (beim Lesen nur ausmaskiert).
- Fehlerbild: Editoren (Shift+Cursor) sehen die Umschalttaste nicht, wenn sie vor dem Lesen losgelassen wurde — bei getippten Zeichen im Puffer der Normalfall. Betrifft auch `Cconin`/`Cnecin`/`Crawcin`/`Crawio` (gleicher Packer).
- NewTOS-Eingabepruefung: eingespeiste IKBD-Bytes im seriellen Protokoll, BIOS-Argumente nicht (TOOL08 war zur Messzeit offen).

## Arbeit

1. Shift-Zustand beim Tastendruck mit dem Tastenwert puffern (Fix gegen `references/emutos/bios/ikbd.c:900` und `:279-280`); Hosttest fuer Shift+Taste, Shift loslassen, dann lesen.
2. Real-Pi4-Pruefung (Bericht Abschnitt 7, Punkt 1): `0x02300042` mit echter USB-Tastatur.

## Abnahme

- Bestaetigender Fall: Tastensonde K11–K16 gleich dem Orakel (`0x02300042`); Tasten-Eingabe ist kein tosdump-Tabellenfall (braucht Eingabekanal im Dumperlauf, Bericht Abschnitt 5).
- Kein vorher gleicher Fall in `bios-r1.tsv`, `bios-core-r1.tsv`, `xbios-chardev-r1.tsv`, `xbios-system-r1.tsv` weicht neu ab.
- `cargo xtask test` gruen.
- `docs/STATUS.md` und Spec im selben Commit; `cargo xtask doc-check` der geaenderten Doku 0 Fehler.

## Anwenderentscheidung 2026-10-28

- Wird mit [XBIOS01](XBIOS01.md) geschlossen: E2 = 32-Bit-Ringeintraege mit
  dem Shift-Zustand zum Push-Zeitpunkt (`docs/api/xbios/0e-iorec.md`,
  Abschnitt „Entscheidungen“). Kein eigener Fix; die Abnahmefaelle dieses
  Tickets gehen in die Abnahme von XBIOS01 ein.

## Stand 2026-10-29

- Geschlossen mit [XBIOS01](XBIOS01.md) (E2): K16 auf NewTOS jetzt
  `0x02300042` wie EmuTOS, K11-K15 und K20 gleich
  (`measurements/measurement-2026-10-29-xbios01/RUNINFO.TXT`, Ergebnis 3).
  Real-Pi4-Pruefung (Arbeit Punkt 2) offen.
