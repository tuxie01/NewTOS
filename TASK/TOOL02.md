# TOOL02 — Jedes Abbild eines Messlaufs bekommt ein Manifest, auch die `mkcard.sh`-Karten

Status: **ERLEDIGT** 2026-10-27 — [Quittung](TOOL02-STATUS-DONE.MD) · Prioritaet: **P2** · Typ: Werkzeug (Reproduzierbarkeit) — Anwenderentscheidung 2026-10-27

## Sachverhalt

- `tools/make-sd-image.sh` schreibt neben jedes Abbild `<image>.manifest`
  (Format v1: sha256 des Abbilds, `SD_*`-Schalter, `optional_missing`, sha256
  jeder Datei auf der Karte); Messlaeufe kopieren es, RUNINFO zitiert
  `optional_missing` woertlich (CLAUDE.md, Platzierungsregel Messlaeufe).
- Injektor-Laeufe auf raspi4b (`tools/inject-arm.py`) bauen ihre Karten mit
  einem `mkcard.sh`, das von Lauf zu Lauf in das method-Verzeichnis des Laufs kopiert wird
  (z. B. `measurements/measurement-2026-10-27-mem12-td-lp64/method/mkcard.sh`:
  64 MiB MBR + FAT16, `NEWDESK.INF` plus `ziel=quelle`-Paare). Diese Karten
  haben kein Manifest; der Lauf vermerkt es nur
  (`measurements/measurement-2026-10-27-mem12-td-lp64/RUNINFO.TXT`).
- Anwenderentscheidung 2026-10-27: „mkcard.sh: must write the same
  `<image>.manifest` as make-sd-image.sh (or call it), so injector runs on
  raspi4b have a manifest too.“

## Arbeit

1. **Ein Manifest-Schreiber, zwei Aufrufer.** Den Manifest-Teil aus
   `tools/make-sd-image.sh` in ein eigenes Werkzeug unter `tools/` ziehen
   (Format v1 unveraendert, byte-gleiche Ausgabe fuer ein Abbild aus
   `make-sd-image.sh` — Vorher/Nachher-Vergleich im Bericht).
   `make-sd-image.sh` ruft es; Verhalten und Ausgabe sonst unveraendert.
2. **Ein gemeinsames `mkcard`** unter `tools/` statt der Kopien je Lauf:
   gleiche Schnittstelle wie die heutigen Kopien (Abbild, `NEWDESK.INF`,
   `ziel=quelle`-Paare), schreibt das Manifest ueber das Werkzeug aus 1.
   Laufspezifisches (welche Dateien) bleibt im Aufruf des Laufs, nicht im
   Werkzeug. Was `mkcard` nicht kennt (z. B. `SD_*`-Schalter), steht im
   Manifest ausdruecklich als `n/a`, nicht weggelassen.
3. Bestehende Laeufe bleiben unveraendert (Laeufe sind unveraenderlich); neue
   Laeufe kopieren statt `mkcard.sh` nur noch ihren Aufruf ins method-Verzeichnis.
4. Doku: `docs/sdk/06-image-and-testing.md` und die CLAUDE.md-Regel auf
   „jedes Abbild, das ein Messlauf bootet“ (nicht nur `make-sd-image.sh`)
   pruefen; `.claude/agents/tester.md` und `auditor.md` verweisen schon auf
   die Regel.

## Reihenfolge

Vor dem Abnahmelauf von [MEM12](MEM12.md) (Pruefpunkt 3, Injektor auf
raspi4b), damit dessen Karten ein Manifest haben.

## Abnahme

- `make-sd-image.sh`: Manifest byte-gleich zu vorher (ausser Zeitstempeln,
  falls das Format welche hat — dann benennen).
- Eine `mkcard`-Karte hat `<image>.manifest` mit `optional_missing`-Zeile und
  sha256 jeder Datei; Probe-Lauf auf raspi4b bootet sie.
- doc-check der geaenderten Doku 0 Fehler.
