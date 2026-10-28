# BIOS03 — Wirken eingehaengte `hdv_*`-Vektoren? Zielfrage, ADR zuerst

Status: **OFFEN** · Prioritaet: **P2** · Typ: Researcher (ADR-Vorschlag), Entscheidung Anwender — Anwenderentscheidung 2026-10-28

## Sachverhalt

- BIOS-STOR-010 (S3, nicht gemessen, NOT LOCALIZED;
  `docs/audits/2026-10-28-bios-storage.md`): ein Programm, das `hdv_rw`,
  `hdv_bpb` oder `hdv_mediach` umhaengt, wird auf NewTOS nicht gerufen. Bei
  EmuTOS laufen `Rwabs`, `Getbpb` und `Mediach` ueber diese Vektoren.
- RAM-Disks und Plattencaches haengen sich genau dort ein. Ob NewTOS das
  unterstuetzen soll, ist eine Zielfrage, keine Fehlerbehebung.
  Anwenderentscheidung 2026-10-28: **erst ein ADR**, wie bei Line-A
  (ADR-030 in `docs/MEMORY.md`).
- Bezug: seit MEM12 ist Seite 0 fuer EL0 nur im Super-Zustand beschreibbar
  (`docs/api/_super-state.md`); TeraDesk liest `hdv_*` als 32-Bit-Zellen.

## Arbeit

1. Bestand: wo EmuTOS die Vektoren setzt und ruft (`references/emutos/bios/`),
   was NewTOS heute an den Adressen hat und wer sie liest oder schreibt
   (Kern, SDK, portierte Programme).
2. Wer sie umhaengt: bekannte Atari-Programme (RAM-Disks, Caches,
   Treiber) mit Quelle; welche davon in der Portliste stehen oder fuer 1.0
   bzw. 2.0 in Frage kommen.
3. ADR-Vorschlag mit Optionen (z. B. Vektoren wirken wie bei EmuTOS — native
   EL0-Routine ueber den Vektor in Seite 0 gerufen, Rueckkehr in den Kern;
   Vektoren nur lesbar, Schreiben ohne Wirkung und dokumentiert; spaeter mit
   dem m68k-Thunk), je mit Aufwand, Sicherheitsfolgen (EL0-Code im
   BIOS-Pfad), Folgen fuer GEMDOS und Messfall.
4. Entscheidung beim Anwender; danach Messfall auf dem Orakel und ggf.
   Developer-Ticket.

## Abnahme

- ADR-Vorschlag mit Quellen; vom Anwender entschieden und als ADR in
  `docs/MEMORY.md` eingetragen (Lead).
- doc-check der geaenderten Doku 0 Fehler.
