# XBIOS01 — `Iorec` ist nicht verdrahtet: Funktionsnummer als Ergebnis, Tastaturpuffer-Leeren endet mit Bus Error

Status: **OFFEN** · Prioritaet: **P1** · Typ: Developer (Vertrag des IOREC-Abbilds: Researcher) — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-xbios-chardev.md`; kommt nach BIOS01 Befund 001 sofort (Reihenfolge laut [BIOS01](BIOS01.md))

## Sachverhalt

**XBIOS-CHARDEV-001 — S1 — CONFIRMED (Rueckgabe) + MEASURED (Struktur, Absturz).**
Beleg: Bericht Abschnitt 2, Lauf `measurements/measurement-2026-10-28-xbios-chardev-audit/RUNINFO.TXT`.

- EmuTOS liefert fuer `Iorec(0/1/2)` einen Zeiger auf ein IOREC (`0x4A54`/`0x30D4`/`0x30E2`; `ibufsiz` 256/256/128, Wassermarken 64/192 bzw. 32/96; Geraet 0 mit Ausgabe-IOREC und Baud/Flow/UCR). Ungueltiges Geraet: `-1`. NewTOS liefert fuer jedes Geraet `0x0E` (die Funktionsnummer); die „Struktur“ dahinter sind Nullbytes aus Seite 0.
- Faelle: `bios-r1` 101/102, `xbios-chardev-r1` 24–27 — EXACT-DIFF, EmuTOS `-1`, NewTOS `0x0E`, TT und Falcon (`measurements/measurement-2026-10-28-xbios-chardev-audit/runs/NEWTOS-xbios-chardev-r1/tosdiff-newtos-vs-emutos.txt`). `bios-r1` 98–100 und `xbios-chardev-r1` 21–23 bestehen PTR_RAM nur, weil `0x0E` gerade und ungleich 0 ist. Sonde I01–I08 (`measurements/measurement-2026-10-28-xbios-chardev-audit/runs/PROBE-EMUTOS-tt`, `measurements/measurement-2026-10-28-xbios-chardev-audit/runs/PROBE-NEWTOS-raspi4b`).
- Absturz: Sonde XCIOFL (`io = Iorec(1); io->ibufhd = io->ibuftl;` im Usermodus): EmuTOS laeuft weiter (TT und Falcon), NewTOS „Bus Error … page-0 system-variable memory, write“, `far` `0x18`, Programm beendet (`measurements/measurement-2026-10-28-xbios-chardev-audit/runs/XCIOFL-NEWTOS-raspi4b`, `measurements/measurement-2026-10-28-xbios-chardev-audit/runs/XCIOFL-EMUTOS-tt`).
- Lokalisiert: `tos-api/src/xbios/mod.rs:65-138@dispatch` (Sammelzweig `other => other as i64`, kein `Iorec`-Arm; Absicht im Kommentar `tos-api/src/xbios/mod.rs:26@PHYSBASE`).
- EmuTOS-Stelle: `references/emutos/bios/xbios.c:336-348`.
- Fehlerbild: „Tastaturpuffer leeren“ (Kopf = Schwanz) ist die verbreitete Nutzung; auf NewTOS endet jedes solche Programm sofort. Lesende Nutzer sehen Nullen.
- Spec: `docs/api/xbios/0e-iorec.md` „decision deferred“ und behauptet `EINVFN` — falsch (Spec-Ticket XBIOS07).

## Arbeit

1. Researcher: Vertrag des IOREC-Abbilds festlegen (Struktur, Groesse, Schreibbarkeit im Usermodus, welche Geraete — Bericht „Route“); Spec `docs/api/xbios/0e-iorec.md` zuerst.
2. Developer: Fix gegen `references/emutos/bios/xbios.c:336-348`; Hosttests fuer Rueckgabewert und `-1` bei ungueltigem Geraet.
3. Kommentar `tos-api/src/xbios/mod.rs:26@PHYSBASE` mit dem Fix richtigstellen.
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
