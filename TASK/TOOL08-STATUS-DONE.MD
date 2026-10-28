# TOOL08 — Quittung: BIOS/XBIOS-Argument-Tracer

Datum 2026-10-28 · Beleg
[measurements/measurement-2026-10-28-tool08/](../measurements/measurement-2026-10-28-tool08/RUNINFO.TXT)
· Ticket [TOOL08](TOOL08.md)

- Kernel-Feature `bios-arg-trace` (`kernel/src/dbg_biostrace.rs`): eine Zeile
  je abgeschlossenem Trap-#13/#14-Aufruf auf der seriellen Konsole —
  Aufrufer, `trap:fn`, Name, Argumente in der Breite der EmuTOS-Aufrufform
  (aus den Debug-Wrappern `bios_N`/`xbios_N` des Pins, je Platz zitiert),
  64-Bit-Ergebnis. Schreibt kein Laufwerk, also auch fuer die Plattenrunde.
  EL0-`Supexec` wird vor dem Upcall aufgezeichnet, ein per KEYS02
  zurueckgespulter Aufruf einmal.
- `tools/biostrace-check.py`: prueft Trace gegen Falltabelle (Breite,
  Vorzeichenerweiterung, Puffer, Poison auf unbelegten Plaetzen) und BIN.
  QEMU raspi4b: `bios-r1` 147 Aufrufe, `bios-core-r1` 44, `bios-r2` 51 + 3
  Vorspann — alle PASS, 0 Abweichungen.
- Standardbau (Feature aus): gleiche Groesse; abweichend nur die
  Zeilennummern in `core::panic::Location` (lib.rs +4, exceptions.rs +12),
  in allen vier Bauten erklaert. Kontrolllauf ohne Feature: tosdiff-Ausgaben
  bytegleich.
- Orakel: Hatari `--trace bios` zeigt `Rwabs` ohne `lrecno`; ein
  Debugger-Breakpoint auf `_lrwabs` (Adresse aus der map des Pins) zeigt alle
  sechs Argumente, 34 von 34 gleich der Tabelle. Vorschlag (nicht umgesetzt):
  Trace-Betriebsart in `tools/hatari-oracle.sh`.
- `cargo xtask test` 1726/0/1 (Lead-Gegenlauf gleich); doc-check der
  geaenderten Doku 0 Fehler, voller Lauf 752 (Basislinie).
