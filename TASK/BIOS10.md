# BIOS10 — `Kbshift(flag)` verwirft die Bits 5–7

Status: **ERLEDIGT** 2026-10-29 ([Quittung](BIOS10-STATUS-DONE.MD)) · Prioritaet: **P2** · Typ: Developer — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-bios-core.md`

## Sachverhalt

**BIOS-CORE-009 — S3 — MEASURED.** Beleg: Bericht Abschnitt 2, Lauf `measurements/measurement-2026-10-28-bios-core-audit/RUNINFO.TXT`.

- Sonde K04/K05 `Kbshift(0x1FF)` → danach `Kbshift(-1)`: EmuTOS `0xFF`, NewTOS `0x1F`; K08/K09 `0x7F81` → `0x81` / `0x01` (`measurements/measurement-2026-10-28-bios-core-audit/runs/PROBE-EMUTOS-tt`, `measurements/measurement-2026-10-28-bios-core-audit/runs/PROBE-NEWTOS-raspi4b`). Gleich: Rueckgabe = alter Wert, Abschneiden auf ein Byte, negative Flags fragen nur ab (K06, K10), Wiederherstellung (K12/K13). `bios-r1` 77 (`-1`) EQUAL.
- Lokalisiert: `kernel/src/ikbd.rs:366@set_shift_state` (bis BIOS10 nur Bits 0–4; vorher Zeile 389), `kernel/src/ikbd.rs:358@shift_state` (vorher Zeile 362).
- EmuTOS-Stellen: `references/emutos/bios/ikbd.c:145-153` (`shifty` ist `UBYTE`); `MODE_HOME`/`MODE_INSERT` setzt der Tastaturtreiber auf jeder Maschine (`references/emutos/bios/ikbd.c:876-883`, ohne Maschinenschalter).
- Folgerung (PLAUSIBLE, nicht gemessen): auch der Tastaturpfad setzt Bit 5/6 auf NewTOS nie — Fall: Tastensonde mit Alt+Clr/Home, danach `Kbshift(-1)` (Real-Pi4-Liste Punkt 2 des Berichts: Home/Insert).
- Spec: `docs/api/bios/0b-kbshift.md` begruendet das Nicht-Zurueckgeben mit „TT/Falcon-era“ — durch die EmuTOS-Quelle widerlegt ([BIOS12](BIOS12.md)).

## Lokalisierung (Lead, 2026-10-29)

- `kbshift` → `tos-api/src/bios/mod.rs@kbshift` (`mapped`); Zustand in
  `kernel/src/ikbd.rs@set_shift_state` / `@shift_state`; Tastaturpfad
  (`kbd_int` → `kernel/src/ikbd.rs@kbd_int`) setzt die Bits. `--down`:
  vollstaendig.
- `--up --layers bios,bdos`: `biostrap` → `tos-api/src/bios/mod.rs@dispatch`,
  `initinfo` → `kernel/src/initinfo.rs@run`; `unmapped` nur `bios_init`,
  `biosmain` (Start). Ausserhalb der Schichten: VDI `vq_key_s`
  (`gshift_s`) und AES `chkkbd`/`gsx_kstate` lesen den Zustand — NewTOS-
  Gegenstuecke im Desktop/AES pruefen (Wiedertest: Shift/Alt-Klicks im Desktop).
  Vollstaendig: nein, diese Liste.

## Arbeit

1. Fix gegen `references/emutos/bios/ikbd.c:145-153` und `:876-883`; Hosttests fuer Setzen/Abfragen.
2. Den PLAUSIBLE-Teil (Tastaturpfad Bit 5/6) mit einem Fall messen und im Ticket vermerken.

## Abnahme

- Bestaetigende Faelle: Sonde K01–K13 gleich dem Orakel; vorgeschlagene Zeilen `13:11 w:0x1FF` und `13:11 w:0x7F81` (Bericht Abschnitt 5, EXACT) sind erst nach Lockerung des Waechters tabellenfaehig ([BIOS13](BIOS13.md)); `bios-r1` 77 bleibt EQUAL.
- Kein vorher gleicher Fall in `bios-r1.tsv`, `bios-core-r1.tsv`, `xbios-chardev-r1.tsv`, `xbios-system-r1.tsv` weicht neu ab.
- `cargo xtask test` gruen.
- `docs/STATUS.md` und Spec im selben Commit; `cargo xtask doc-check` der geaenderten Doku 0 Fehler.

## Entscheidung des Anwenders 2026-10-28 (BIOS12)

- D1 = 1: ein echtes Byte im Tastaturdekoder wie EmuTOS' `shifty`
  (`docs/api/bios/0b-kbshift.md`, „Decision“).

## Messung Tastenpfad (Arbeit 2, 2026-10-29)

Gemessen mit der dauerhaften Sonde `tosdump/probes/ksprobe.c` (Tasten aus
`tosdump/probes/ksprobe-keys.txt`; NewTOS ueber den IKBD-Injektor, Orakel ueber
Hatari `--cmd-fifo`), Beleg `measurements/measurement-2026-10-29-bios10/RUNINFO.TXT`.
Wechsel von `Kbshift(-1)` nach jeder Taste:

| Folge | Orakel tt = falcontc | NewTOS vorher | NewTOS nachher |
|---|---|---|---|
| Alt, Home, Home los, Alt los | `08 28 08 00` | `08 00` | `08 28 08 00` |
| Alt, Insert, Insert los, Alt los | `08 48 08 00` | `08 00` | `08 48 08 00` |
| Home ohne Alt | kein Wechsel | kein Wechsel | kein Wechsel |
| Alt, Home, Alt los, Home los | `08 28 20 00` | `08 00` | `08 28 20 00` |
| Alt, Insert, Home, Home los, Insert los, Alt los | `08 48 68 48 08 00` | `08 00` | `08 48 68 48 08 00` |
| nach `Kbshift(0x80)`: LShift, LShift los, Alt, Home, Home los, Alt los | `82 80 88 a8 88 80` | `02 00 08 00` (Bit 7 nicht gespeichert) | `82 80 88 a8 88 80` |

Die PLAUSIBLE-Folgerung ist damit gemessen: vorher setzte der Tastaturpfad
Bit 5/6 nie. Nebenbefund (nicht Teil des Tickets): EmuTOS stellt
Alt+Clr/Home und Alt+Insert als Maustasten der Tastatur-Mausemulation dar
und reiht sie nicht als Taste ein (`references/emutos/bios/ikbd.c:93-99,904-908`);
NewTOS hat keine Mausemulation und reiht sie ein (Sonde: 8 statt 2 Tasten).

