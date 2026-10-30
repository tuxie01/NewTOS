# COOK02 — `_AKP` folgt dem aktiven Tastaturlayout

Status: **GEPARKT** 2026-10-30 (Anwender; wieder aufnehmen, wenn ein Port es braucht) · Prioritaet: **P3** · Typ: Developer — Anwenderentscheidung 2026-10-29, ersetzt 2026-10-30

## Sachverhalt

- EmuTOS baut `_AKP` aus Land und Tastatur (`references/emutos/bios/country.c:133-162`,
  `detect_akp`; Tastatur im niedrigen, Sprache im hohen Byte, `:98`, `:113`).
- NewTOS setzt `_AKP` und `_IDT` fest (`tos-api/src/cookie.rs:222@AKP`, „bring-up
  policy“); das Layout waehlt `kernel/src/keymap.rs:367@init_from_disk`
  unabhaengig davon (`KEYBOARD.KBD` oder `#K` in NEWDESK.INF). Programme, die
  `_AKP` lesen, sehen ein anderes Layout als das aktive.

## Entscheidung (2026-10-29, ersetzt 2026-10-30, siehe unten)

- `_AKP`-Tastaturbyte = Kennung des aktiven Layouts (Zuordnung nach
  EmuTOS' Laendertabelle).
- Sprachbyte und `_IDT` bleiben fest, bis es eine Einstellung dafuer gibt
  (Einstellungen menschenlesbar, kein rohes `cmdline.txt`).

## Abnahme

- `jar:_AKP` je Layout gleich dem Orakel mit demselben Land; Hosttest;
  STATUS im selben Commit.

[Gemeinsame Regeln](README.md) gelten.

## Anwenderentscheidung 2026-10-30 (aus `docs/analysis/cook02-country-facts-2026-10-30.md`)

1. **Geparkt.** Richtung fuer spaeter: das Land kommt aus `cmdline.txt`
   (menschenlesbar `country=xx`, neuer Parser fuer Textwerte), maschinenweit;
   daraus folgen beide `_AKP`-Bytes, `_IDT` und das Standard-Tastaturlayout.
   `C:\KEYBOARD.KBD` und `#K` in `NEWDESK.INF` ueberschreiben nur das Layout.
   Wieder aufnehmen, wenn ein Port es braucht.
2. **`_AKP`:** Orakel `0x0100` = Hatari-NVRAM-Vorgabe (Byte 7), keine
   Eigenschaft des `de`-ROMs (gemessen, vier Varianten,
   `measurements/measurement-2026-10-30-cook02/RUNINFO.TXT`). In
   `docs/COMPATIBILITY.md` (Fall 52) ACCEPTED mit dieser Begruendung; NewTOS
   bleibt `0`.
3. **`_IDT`:** NEWDESK nimmt bei der Einstellung "Standard" das Trennzeichen
   aus `_IDT` (`programs/desktop/src/model.rs:194-201@rendered`, wie EmuTOS
   `references/emutos/desk/deskinf.c:140-145`). Deshalb fester Wert
   `0x112E` (gleich dem Orakel, deutsches `.`); Hosttest und `jar:_IDT` gleich.
   Umsetzung als eigener Developer-Schritt (Lead 2026-10-30).
4. Nebenbefunde (ungenaue Texte "low word = language") nach [SPEC02](SPEC02.md);
   Warnung zu `references/emutos` in `CLAUDE.md`.

## Stand 2026-10-30 (Punkt 3 umgesetzt)

`_IDT` = `0x0000112E` (`tos-api/src/cookie.rs:226@IDT`); `jar-r1` Fall 53 gleich
allen drei Orakel-Referenzen, sonst keine Aenderung in `jar-r1`, `bios-r1`,
`xbios-system-r1`. Beleg `measurements/measurement-2026-10-30-cook02-idt/RUNINFO.TXT`.
Hosttests 1895/0/1. Das sichtbare Datum im Desktop ist nicht gemessen
(abgeleitet aus `programs/desktop/src/model.rs:194@rendered`).

