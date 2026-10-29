# TOOL17 — `mapping.tsv`: „kein Gegenstueck mit Absicht“ von „noch nicht abgebildet“ trennen; Abdeckung je Schicht

Status: **ERLEDIGT** 2026-10-29 ([Quittung](TOOL17-STATUS-DONE.MD)); Ermessensfaelle (Gruppe G) zur Bestaetigung beim Anwender · Prioritaet: **P1** · Typ: Developer (Werkzeug + Zuordnung) — Anwenderentscheidung 2026-10-28

## Sachverhalt

- `docs/analysis/deps/mapping.tsv` (nach TOOL12: 350 Zeilen mit Gegenstueck,
  671 ohne) kennt fuer „kein Gegenstueck“ nur `-` plus freie Notiz. Darin
  stehen zwei verschiedene Dinge in derselben Form:
  - **mit Absicht**, z. B. Hardware, die die Pi nicht hat (SCSI, Floppy WD1772,
    Shifter/VIDEL, DSP56001), oder per ADR ausgeschlossen (ADR-022
    EmuDESK-Nachbau, ADR-030 kein Line-A);
  - **noch nicht abgebildet bzw. nicht gefunden**, z. B. `mark_bcbs_invalid`
    „no NewTOS function found“, `offree` „no NewTOS counterpart found“.
- Zeilen gibt es heute nur fuer EmuTOS `bios/` und `bdos/`; `vdi`, `aes`,
  `desk` (in `edges.tsv` vorhanden) haben keine.
- Folge: eine `--up`-Retest-Menge sieht vollstaendig aus, obwohl Funktionen
  darin nur nicht abgebildet sind.

## Arbeit

1. **Format:** eine Klassenspalte (oder ein festes Praefix der Notiz, Wahl des
   Developers, im Kopf der Datei beschrieben): `mapped` / `design:<grund>` /
   `unmapped`. `design` braucht einen Beleg: `ADR-NNN` (z. B. ADR-022,
   ADR-030) oder `hardware:<geraet>` (Atari-Hardware ohne Gegenstueck auf der
   Pi) oder einen anderen benannten Grund mit Quelle. Alles ohne Beleg ist
   `unmapped`.
2. **Bestand klassifizieren:** jede der 671 `-`-Zeilen nach Regel 1; Faelle,
   die nicht eindeutig sind, als `unmapped` und im Bericht gelistet (der Lead
   entscheidet mit dem Anwender).
3. **Schichten ohne Zeilen** (`vdi`, `aes`, `desk` laut `edges.tsv`): gelten als
   `unmapped`, ohne dass jede Funktion jetzt eine Zeile braucht; die Abdeckung
   zaehlt sie so.
4. **`cargo xtask deps`:**
   - Abdeckung je Schicht: `mapped` / `design` / `unmapped` (Zahl und Anteil),
     eigener Unterbefehl oder Schalter.
   - Bei `--up`/`--down`: je Schicht der Ergebnismenge dieselben drei Zahlen
     und die Liste der `unmapped`-Funktionen; die Ergebnismenge heisst
     **vollstaendig** nur, wenn in den beruehrten Schichten keine
     `unmapped`-Funktion vorkommt.
5. **Pruefung:** der Hosttest prueft die Klassen (jede `-`-Zeile hat eine,
   `design` hat einen Beleg, `ADR-NNN` existiert in `docs/MEMORY.md`).
6. `tools/deps/README.md` und der Kopf von `mapping.tsv` beschreiben Klassen,
   Abdeckung und die Ratschenregel (CLAUDE.md „Testing“).

## Abnahme

- Abdeckung je Schicht im Bericht (vorher nicht messbar, nachher Zahlen);
  Liste der unklaren Faelle.
- `cargo xtask deps blkdev_rwabs --up` zeigt die `unmapped`-Liste der
  beruehrten Schichten (Beispiel fuer BIOS01).
- `cargo xtask test` gruen (Basislinie 1748/0/1 plus neue); doc-check 0
  Fehler.

## Nachtrag Anwender 2026-10-28

- Floppy-Funktionen (`flop*`, FDC/WD1772 und Verwandte) werden
  `design:hardware:floppy` (Nicht-Ziel in CLAUDE.md, FAT12/Floppy ausser
  Reichweite; ADR-031 trennt `hdv_*` ebenso).

## Nachtrag 2026-10-29 (aus XBIOS15, SERRX-004)

- Zeile fuer `scc_rx_interrupt_handler` (`references/emutos/bios/serport.c:647-683`):
  Gegenstueck des Push-Teils ist `kernel/src/console.rs@serial_rx_byte` wie bei
  `push_serial_iorec` (Falcon-Zielprofil).
