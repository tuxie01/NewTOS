# XBIOS01 — `Iorec` ist nicht verdrahtet: Funktionsnummer als Ergebnis, Tastaturpuffer-Leeren endet mit Bus Error

Status: **ERLEDIGT** 2026-10-29 ([Quittung](XBIOS01-STATUS-DONE.MD)), vom Anwender abgenommen (`a32921d5`) · Prioritaet: **P1** · Typ: Developer (Vertrag des IOREC-Abbilds: Researcher) — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-xbios-chardev.md`; kommt nach BIOS01 Befund 001 sofort (Reihenfolge laut [BIOS01](BIOS01.md))

## Sachverhalt

**XBIOS-CHARDEV-001 — S1 — CONFIRMED (Rueckgabe) + MEASURED (Struktur, Absturz).**
Beleg: Bericht Abschnitt 2, Lauf `measurements/measurement-2026-10-28-xbios-chardev-audit/RUNINFO.TXT`.

- EmuTOS liefert fuer `Iorec(0/1/2)` einen Zeiger auf ein IOREC (`0x4A54`/`0x30D4`/`0x30E2`; `ibufsiz` 256/256/128, Wassermarken 64/192 bzw. 32/96; Geraet 0 mit Ausgabe-IOREC und Baud/Flow/UCR). Ungueltiges Geraet: `-1`. NewTOS liefert fuer jedes Geraet `0x0E` (die Funktionsnummer); die „Struktur“ dahinter sind Nullbytes aus Seite 0.
- Faelle: `bios-r1` 101/102, `xbios-chardev-r1` 24–27 — EXACT-DIFF, EmuTOS `-1`, NewTOS `0x0E`, TT und Falcon (`measurements/measurement-2026-10-28-xbios-chardev-audit/runs/NEWTOS-xbios-chardev-r1/tosdiff-newtos-vs-emutos.txt`). `bios-r1` 98–100 und `xbios-chardev-r1` 21–23 bestehen PTR_RAM nur, weil `0x0E` gerade und ungleich 0 ist. Sonde I01–I08 (`measurements/measurement-2026-10-28-xbios-chardev-audit/runs/PROBE-EMUTOS-tt`, `measurements/measurement-2026-10-28-xbios-chardev-audit/runs/PROBE-NEWTOS-raspi4b`).
- Absturz: Sonde XCIOFL (`io = Iorec(1); io->ibufhd = io->ibuftl;` im Usermodus): EmuTOS laeuft weiter (TT und Falcon), NewTOS „Bus Error … page-0 system-variable memory, write“, `far` `0x18`, Programm beendet (`measurements/measurement-2026-10-28-xbios-chardev-audit/runs/XCIOFL-NEWTOS-raspi4b`, `measurements/measurement-2026-10-28-xbios-chardev-audit/runs/XCIOFL-EMUTOS-tt`).
- Lokalisiert: `tos-api/src/xbios/mod.rs:65-138@dispatch` (Sammelzweig `other => other as i64`, kein `Iorec`-Arm; Absicht im Kommentar `tos-api/src/xbios/mod.rs:26-31@PHYSBASE`).
- EmuTOS-Stelle: `references/emutos/bios/xbios.c:336-348`.
- Fehlerbild: „Tastaturpuffer leeren“ (Kopf = Schwanz) ist die verbreitete Nutzung; auf NewTOS endet jedes solche Programm sofort. Lesende Nutzer sehen Nullen.
- Spec: `docs/api/xbios/0e-iorec.md` „decision deferred“ und behauptet `EINVFN` — falsch (Spec-Ticket XBIOS07).

## Arbeit

1. Researcher: Vertrag des IOREC-Abbilds festlegen (Struktur, Groesse, Schreibbarkeit im Usermodus, welche Geraete — Bericht „Route“); Spec `docs/api/xbios/0e-iorec.md` zuerst.
2. Developer: Fix gegen `references/emutos/bios/xbios.c:336-348`; Hosttests fuer Rueckgabewert und `-1` bei ungueltigem Geraet.
3. Kommentar `tos-api/src/xbios/mod.rs:26-31@PHYSBASE` mit dem Fix richtigstellen.
4. Offen im Bericht: der Fall „durch den Rueckgabezeiger“ (`14:0x0E w:1 ret:14`, Zeile I02b) braucht eine Capture-Form, die es heute nicht gibt (Abschnitt 5, Punkt 11.2) — ohne sie ist der Inhalt nur per Sonde pruefbar.

## Abnahme

- Bestaetigende Faelle: `bios-r1` 101/102 und `xbios-chardev-r1` 24–27 EQUAL gegen den Orakel (TT und Falcon); Sonde XCIOFL laeuft auf NewTOS ohne Bus Error durch; Sonde I01–I08 (Struktur) gleich.
- Kein vorher gleicher Fall in `bios-r1.tsv`, `bios-core-r1.tsv`, `xbios-chardev-r1.tsv`, `xbios-system-r1.tsv` weicht neu ab.
- `cargo xtask test` gruen.
- `docs/STATUS.md` und Spec im selben Commit; `cargo xtask doc-check` der geaenderten Doku 0 Fehler.

## Anwenderentscheidung 2026-10-28

- Der Inhalt hinter dem Rueckgabezeiger wird per tosdiff abgenommen, sobald
  [TOOL16](TOOL16.md) (Dumper liest durch Rueckgabezeiger, Layout aus
  `docs/api/`) steht; TOOL16 kommt in der Warteschlange davor.
- Bis dahin gilt fuer den MEASURED-Teil die Sondenregel aus CLAUDE.md
  „Testing“ (Sonde als dauerhafter Test, beidseitig gegen das gepinnte
  Orakel, mit Eingabe-Trace; Folgeticket nach tosdiff).
## Stand 2026-10-28 (Punkt 1, Entwurf)

- Vertrag des IOREC-Abbilds als Entwurf in `docs/api/xbios/0e-iorec.md`
  (`[XBIOS01]`): LP64-Layout des SDK-Headers, Ort ausserhalb Seite 0, Ring =
  die Schlange von `Bconstat(2)`/`Bconin(2)`, `head`/`tail` als
  unvertrauenswuerdige Eingabe. Entscheidungen E1–E6 beim Anwender.

## Entscheidungen des Anwenders 2026-10-28 (E1-E6)

- E1 gemeinsamer Ring ersetzt `RX_RING`, Kernel validiert `head`/`tail`
  (und `buf`/`size`) bei jeder Nutzung; E2 32-Bit-Eintraege (schliesst
  [BIOS11](BIOS11.md)); E3 (a) vorerst; E4 nach Erhebung; E5 leere Records
  fuer 0 und 2; E6 ja (in [BIOS03](BIOS03.md)). Volltext:
  `docs/api/xbios/0e-iorec.md`, Abschnitt „Entscheidungen“.
- **Vor dem Developer (Researcher):** Erhebung in den portierten Programmen
  (`sdk/apps/`, `sdk/overlays/`, Portliste): (E3) pollt eines den Ring ohne
  Trap? (E4) ersetzt eines `buf`/`size`? Ergebnis mit `file:line`; danach
  steht E3/E4 fest.
- **Abnahme zusaetzlich:** `offsetof`/`sizeof`-Pruefung zur
  Uebersetzungszeit des SDK-`_IOREC` (und der Geraet-0-Struktur) gegen die
  Spec; Flush (`ibufhd = ibuftl` → `Bconstat(2) == 0`) als gemessener Fall
  auf beiden Seiten — braucht „durch den Rueckgabezeiger schreiben“ in
  [TOOL16](TOOL16.md).
- **Lokalisierung vor dem Fix (Lead):** NewTOS-Funktion per
  `cargo xtask deps <fn> --down` + `docs/analysis/deps/mapping.tsv` und die
  `--up`-Retest-Menge hier eintragen, bevor der Developer beginnt.
- **Erhebung E3/E4 erledigt 2026-10-28**
  (`docs/analysis/iorec-survey-2026-10-28.md`): kein Nutzer von `Iorec` in den
  portierten Quellen → E3 (a), E4 (a) kernel-eigener Puffer mit einmaliger
  Warnung und Wiederherstellung (`docs/api/xbios/0e-iorec.md`, „Entscheidungen“).

## Stand 2026-10-29 (TOOL16)

- Messfaelle stehen: `tosdump/cases/xbios-ptr-r1.tsv` (Faelle 24–31 Iorec
  samt Layout aus `docs/api/structures/iorec.md`, Flush-Fall `+put
  ibufhd=ibuftl` → `Bconstat(2)`), Referenzen tt/falcontc. NewTOS heute DIFF
  (`0x0E`, nichts gelesen/geschrieben). Beleg
  `measurements/measurement-2026-10-29-tool16/RUNINFO.TXT`.
- **Grenze fuer die Abnahme:** am Orakel war der Ring leer; der Flush-Fall
  zeigt nur, dass das Schreiben angenommen wird und `Bconstat(2)` danach 0
  ist — nicht das Leeren eines vollen Rings. Fuer die Abnahme „Flush gemessen“
  braucht der Fall Tasten im Ring vor dem Flush (Eingabe am Orakel und in
  QEMU). Ausserdem kennt `tools/biostrace-check.py` die Zeilen `+put`/`+set`
  nicht (kein NewTOS-Eingabe-Trace fuer diese Tabelle).

## Lokalisierung (Lead, 2026-10-29, Graph nach TOOL12)

- EmuTOS → NewTOS laut `cargo xtask deps <fn> --down` + `mapping.tsv`:
  `iorec` (XBIOS 0x0E) → **ohne Gegenstueck** (Sammelzweig
  `tos-api/src/xbios/mod.rs@dispatch`); `push_ikbdiorec` → **ohne
  Gegenstueck** (heute `kernel/src/ikbd.rs@push_rx` in `RX_RING`);
  `do_key_repeat` → `kernel/src/ikbd.rs@poll_repeat`; `kbd_int` →
  `kernel/src/ikbd.rs@kbd_int`; `bconstat2` → `tos-api/src/bios/chardev.rs@bconstat`;
  `bconin2` → `chardev.rs@bconin`, `@pack_console_key`, `@shift_fold`.
- `--up`-Retest-Mengen (gesamt / mit Gegenstueck): `iorec` 3/2,
  `push_ikbdiorec` 10/5, `kbd_int` 6/3, `bconstat2` 366/37, `bconin2` 359/32.
  Ohne Gegenstueck in `bios`/`bdos` (heutiges `mapping.tsv`): `iorec`,
  `push_ikbdiorec`, `kb_timerc_int`, `bconout0` und die Start-/
  Erkennungsfunktionen wie in BIOS01 (`biosmain`, `bios_init`, `detect_*`, …);
  nach TOOL17 neu klassifizieren. Der Fix ergaenzt die Zeilen fuer `iorec`
  und `push_ikbdiorec` (Ratsche).
- Retest praktisch: `xbios-ptr-r1.tsv`, `bios-r1.tsv`, `xbios-chardev-r1.tsv`
  (tt/falcontc), Tastatur-Hosttests, `Bconin(2)`/`Cconin`-Pfade (BIOS11 geht
  mit), AES-Eingabe (Desktop bis zur Menueleiste, Taste im Dialog), `Fread`
  auf `CON:`.


## Stand 2026-10-29 (Umsetzung)

- Umgesetzt nach E1-E6, Quittung [XBIOS01-STATUS-DONE.MD](XBIOS01-STATUS-DONE.MD),
  Beleg `measurements/measurement-2026-10-29-xbios01/RUNINFO.TXT`. Flush mit
  Tasten im Ring: neue Tabelle `tosdump/cases/xbios-flush-r1.tsv`, beidseitig
  gleich. Schliesst [BIOS11](BIOS11.md).
