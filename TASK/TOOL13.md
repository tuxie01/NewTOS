# TOOL13 — xtask-Pruefung: Debug-Feature aus = Kernel bytegleich bis auf `panic::Location`

Status: **GEPARKT** 2026-10-31 (Anwender, Vor-Start-Pruefung: kein anstehender Fix bringt ein Debug-Feature) · Prioritaet: **P2** · Typ: Developer (Werkzeug) — Anwenderentscheidung 2026-10-28, aus [TOOL08](TOOL08.md)

## Sachverhalt

- Stehende Regel (Anwender 2026-10-28, CLAUDE.md „Testing“): mit
  abgeschaltetem Debug-Feature sind Code- und Datenbytes des Kernels gleich
  dem Bau ohne die Aenderung, ausgenommen die Zeilen-/Spaltenfelder von
  `core::panic::Location`.
- TOOL08 hat das von Hand belegt (4 Bauten, `locdiff-*.txt` in
  `measurements/measurement-2026-10-28-tool08/RUNINFO.TXT`, Zeilen 120-126).
  Die Ausnahme soll gepruefte Regel sein, nicht erklaerte.

## Arbeit

1. `cargo xtask` erhaelt einen Befehl (Name waehlt der Developer, z. B.
   `cargo xtask feature-identity --base <rev>`): baut die Boards `rpi4` und
   `qemu-virt` je release und dev auf `<rev>` und auf dem Arbeitsstand
   (Feature aus), in getrennten Target-Verzeichnissen im Scratch.
2. Vergleich der ELF-Abschnitte: Code byteweise gleich; Daten gleich bis auf
   Felder, die nachweislich `Location`-Datensaetze sind (Zuordnung ueber die
   Symbole/Relokationen, nicht ueber Musterraten); jede andere Abweichung =
   Fehler mit Adresse und Abschnitt.
3. Ausgabe: je Bau Zahl der ausgenommenen Felder, 0 unerklaerte.
4. Gegenprobe: das TOOL08-Paar (`3d74514b` gegen `61f39de2`) besteht; eine
   absichtlich eingefuegte Code-Aenderung faellt durch.
5. CLAUDE.md-Wortlaut fuer den Befehl im Bericht vorschlagen (der Lead traegt
   ein).

## Abnahme

- Gegenprobe aus Punkt 4 belegt im RUNINFO.
- `cargo xtask test` gruen; doc-check der geaenderten Doku 0 Fehler.
