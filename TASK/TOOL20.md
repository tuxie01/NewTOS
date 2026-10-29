# TOOL20 — Sondenfaelle `pxprobe`/`gcprobe` nach tosdiff

Status: **ERLEDIGT** 2026-10-29 ([Quittung](TOOL20-STATUS-DONE.MD)); drei Funde zur Triage beim Anwender · Prioritaet: **P2** · Typ: Developer (Werkzeug) — Anwenderentscheidung 2026-10-29, Folgeticket aus [FS22](FS22.md) (Sondenregel, CLAUDE.md „Testing“)

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

## Stand 2026-10-29

- Drei Tabellen (`tosdump/cases/gemdos-mc-fs20.tsv`, `-fs22.tsv`,
  `-paths.tsv`) mit Pin-Referenzen (Profil `disk`, je zweimal bitgleich);
  Waechter-Abweisungen fuer GEMDOS-Dateiaufrufe nur in zerstoerenden
  Plattentabellen aufgehoben (Regeln im Ledger). Accessory-Lader und AUTO:
  kein Fall moeglich (nur beim Start). Beleg
  `measurements/measurement-2026-10-29-tool20/RUNINFO.TXT`.
- NewTOS: fs20 0/17 (Endzustand: nur `FA_ARCHIVE`, FS21), fs22 6/13,
  paths 12/49. **Funde (Triage beim Anwender):**
  - F1: NewTOS fuellt `d_fname` nach dem NUL mit Nullen
    (`tos-api/src/bdos/fs/find.rs:462@makbuf`), EmuTOS schreibt nur Name und
    NUL (`references/emutos/bdos/fsdir.c:1297-1324`, `packit`) — 6 Faelle in
    fs22, 11 in paths.
  - F2: `Dsetpath` direkt nach dem Wechsel: EmuTOS trifft den Wechsel nicht
    (`Mediach` bleibt 2), NewTOS meldet neu an (`Mediach` 0).
  - F3: Endzustand paths: beim Orakel steht der Eintrag von `DELME.TXT` noch
    unbeloescht (Cluster frei, `fsck_msdos` 206), NewTOS schrieb 0xE5 —
    Schreibpolitik von EmuTOS oder Zeitpunkt-Artefakt (Orakel 2 s nach dem LOG
    angehalten)? Nicht bewertet.

## Anwenderentscheidung 2026-10-29

- F1 angleichen: [FS23](FS23.md). F2 angleichen, Vorschlag zuerst:
  [FS24](FS24.md). F3 erst messen (Zeitpunkt?): [FS25](FS25.md).
