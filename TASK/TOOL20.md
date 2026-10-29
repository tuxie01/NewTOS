# TOOL20 — Sondenfaelle `pxprobe`/`gcprobe` nach tosdiff

Status: **OFFEN** · Prioritaet: **P2** · Typ: Developer (Werkzeug) — Anwenderentscheidung 2026-10-29, Folgeticket aus [FS22](FS22.md) (Sondenregel, CLAUDE.md „Testing“)

## Sachverhalt

- FS20 und FS22 sind nach der Sondenregel abgenommen: `tosdump/probes/gcprobe.c`
  und `tosdump/probes/pxprobe.c`, beidseitig mit Eingabe-Trace. Die Regel
  verlangt je Fix ein Folgeticket, das die Faelle nach tosdiff bringt.
- Ausserdem fehlen Faelle fuer die zusaetzlichen FS22-Pfade nach einem
  Medienwechsel (Dateiauswahl, Accessory-Lader, AUTO, Shell-Aufrufe;
  [FS22](FS22.md), Nachtrag).

## Arbeit

1. Was die Sonden pruefen, als Falltabellen ausdruecken (zerstoerende
   Plattentabellen mit `+set`, `!disk 2 destructive`; Pexec und Fsfirst/Fsnext
   als Faelle; DTA per Layout aus `docs/api/`), soweit der Waechter es zulaesst;
   was nicht geht, im Bericht.
2. Referenzen auf dem Orakel (Profil `disk`), zweimal bitgleich; NewTOS-Lauf
   per `cargo xtask tosdump --disk`.
3. Die Sonden bleiben, bis die Faelle in DUMPGDOS aufgehen.

## Abnahme

- Neue Tabellen EQUAL auf NewTOS; `tosdiff --selfcheck`; Hosttests gruen;
  doc-check 0 Fehler.
