# TOOL16 — Dumper liest durch zurueckgegebene Zeiger (Strukturlayout aus `docs/api/`)

Status: **OFFEN** · Prioritaet: **P1** · Typ: Developer (Werkzeug) — Anwenderentscheidung 2026-10-28

## Sachverhalt

- Mehrere Aufrufe liefern einen Zeiger auf eine Struktur, deren Inhalt das
  eigentliche Ergebnis ist: `Iorec` (IOREC, [XBIOS01](XBIOS01.md)),
  `Getmpb` (MPB/MD, [BIOS06](BIOS06.md)), weitere in den Audit-Berichten
  (Abschnitt 5 bzw. 11 von `docs/audits/2026-10-28-xbios-chardev.md`,
  `docs/audits/2026-10-28-bios-core.md`).
- Heute kann eine Falltabelle nur `r:<len>` (rohe Bytes hinter dem Zeiger,
  TOOL07) und `PTR_RAM` (TOOL11); Zeiger in der Struktur, Feldbreiten und
  das unterschiedliche Layout (m68k 32-Bit-Zeiger, NewTOS LP64) kann sie
  nicht ausdruecken. Die Abnahme von XBIOS01 und BIOS06 haengt daran
  (BIOS13 Punkt 7).

## Arbeit

1. Capture-Form in der Falltabelle: „durch den Rueckgabezeiger mit Layout
   `<name>`“; das Layout (Felder, Breite, Zeiger-Felder, denen weiter gefolgt
   wird, Tiefe begrenzt) kommt aus `docs/api/` (Struktur-Spec), nicht aus
   dem Dumper-Code. Vorschlag zur Quelle des Layouts (z. B. eine Tabelle, die
   ein xtask-Test gegen die Spec prueft) im Bericht.
2. Runner: folgt nur Zeigern innerhalb der `PTR_RAM`-Grenzen
   (`tosdump/core/record.h`, TOOL11); ausserhalb = eigener Fehlerwert, kein
   Lesen.
3. tosdiff vergleicht feldweise, Zeigerfelder nach `PTR_RAM`, alles andere
   EXACT; Vergleichsregel steht in der Falltabelle.
4. Erste Nutzer: `Iorec(0/1/2)` und `Getmpb` als Faelle in einer neuen
   Tabelle (bestehende Tabellen sind eingefroren); Referenzen auf dem Orakel
   (`tt`, `falcontc`) zweimal bitgleich.
5. `docs/TOS-REFERENCE-DUMPER.md` und `tosdump/tests` nachziehen.

## Abnahme

- `tosdiff --selfcheck` besteht fuer alle Referenzen; Orakel zweimal
  bitgleich; NewTOS-Lauf per `cargo xtask tosdump`, Ergebnis je Fall im
  RUNINFO (DIFF bei `Iorec`/`Getmpb` erwartet bis XBIOS01/BIOS06).
- Hosttests gruen (`cargo xtask test`, `tosdump/tests`); doc-check 0 Fehler.

## Nachtrag Anwender 2026-10-28

- Zusaetzlich **durch den Rueckgabezeiger schreiben**, minimal wie `+set`
  (TOOL07): ein Feld der Struktur hinter dem Rueckgabezeiger eines
  vorangehenden Falls setzen, bevor der naechste Fall laeuft — Feld per Name
  aus dem Layout, nur innerhalb der `PTR_RAM`-Grenzen, nur in Tabellen, die
  das ausdruecklich erlauben. Erster Nutzer: der Flush-Fall von
  [XBIOS01](XBIOS01.md) (`ibufhd = ibuftl`, danach `Bconstat(2)`).
