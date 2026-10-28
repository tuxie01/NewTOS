# TOOL08 — BIOS/XBIOS-Argumente im NewTOS-Tracer

Status: **ERLEDIGT** 2026-10-28 ([Quittung](TOOL08-STATUS-DONE.MD)) · Prioritaet: **P1** · Typ: Werkzeug (Eingabepruefung) — Anwenderentscheidung 2026-10-28

## Sachverhalt

- Die Auditor-Rolle verlangt fuer jeden Ergebnisvergleich eine
  Eingabepruefung auf beiden Seiten („Inputs before outputs“,
  `.claude/agents/auditor.md`). Fuer BIOS/XBIOS ist sie auf NewTOS nicht
  moeglich: `svc-seq-trace` schreibt nur Funktion und Ergebnis
  (`kernel/src/dbg_qedtrace.rs:397@Bios`), `gemdos-trace` nur GEMDOS.
- Weder der DUMPBIOS-Lauf vom 2026-10-27 noch das Audit „BIOS-Speicher“ hat
  deshalb eine NewTOS-seitige Eingabepruefung
  (`docs/audits/2026-10-28-bios-storage.md`, Abschnitt 11, Punkt 4).
- Hatari `--trace bios` zeigt `Rwabs` ohne das 6. Argument (`lrecno`); die
  Orakel-Seite ist damit ebenfalls lueckenhaft.

## Arbeit

1. Der `svc-seq-trace`-Satz (oder ein eigenes Feature) traegt fuer Trap #13
   und #14 die Argumente, wie sie am Dispatcher ankamen, in Wire-Breite
   (Wort/Langwort wie die Aufrufform), plus Ergebnis. Nur mit dem Feature
   gebaut; der Standardbau bleibt unveraendert.
2. Ausgabeformat so, dass ein Skript es gegen die Falltabelle pruefen kann
   (Fall-ID ueber die Aufrufreihenfolge oder Marker des Dumpers);
   beschrieben in `docs/TOS-REFERENCE-DUMPER.md` oder dem Kopfkommentar.
3. Orakel-Seite: wie das 6. `Rwabs`-Argument dort belegt wird (Hatari-Trace,
   Debugger-Breakpoint, Dumper-Log), im Bericht messen und benennen; wenn es
   ein Werkzeug braucht, Vorschlag statt Umbau.
4. Ein Pruefskript vergleicht Tracer-Ausgabe und Falltabelle fuer
   `bios-r1.tsv` (und `bios-r2.tsv`, sobald TOOL07 sie hat).

## Abnahme

- `bios-r1.tsv` auf QEMU `raspi4b` mit dem Feature: jede Argumentzeile
  stimmt mit der Falltabelle ueberein (Pruefskript, Ausgabe im RUNINFO).
- Ohne Feature: Kernel bytegleich zum Bau ohne die Aenderung bzw. Begruendung
  jeder Abweichung.
- Hosttests gruen (`cargo xtask test`); doc-check der geaenderten Doku 0
  Fehler.
