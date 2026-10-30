# TOOL14 — Trace-Betriebsart fuer `tools/hatari-oracle.sh` (Eingabepruefung EmuTOS-Seite)

Status: **GEPARKT** 2026-10-31 (Anwender, Vor-Start-Pruefung: Sonden laufen ohne Trace-Betriebsart) · Prioritaet: **P2** · Typ: Developer (Werkzeug) — Anwenderentscheidung 2026-10-28, Vorschlag aus [TOOL08](TOOL08.md)

## Sachverhalt

- Die Eingabepruefung („Inputs before outputs“, `.claude/agents/auditor.md`)
  hat auf NewTOS den Tracer `bios-arg-trace` (TOOL08). Auf dem Orakel zeigt
  Hatari `--trace bios` `Rwabs` ohne `lrecno`; vollstaendig nur per
  Debugger-Breakpoint auf `_lrwabs`. TOOL08 hat dafuer eine Kopie des
  Skripts benutzt (`measurements/measurement-2026-10-28-tool08/method/oracle-traced.sh`).

## Arbeit

1. `tools/hatari-oracle.sh` erhaelt eine Trace-Betriebsart (Schalter, Profil
   bleibt gleich): Hatari-Trace und Breakpoints auf die BIOS/XBIOS-Eintritte
   mit Adressen aus der map des gepinnten ROM-Baus (keine festen Adressen im
   Skript), Ausgabe aller Argumente je Aufruf in einem Format, das
   `tools/biostrace-check.py` gegen die Falltabelle pruefen kann.
2. Ohne Schalter: Kommandozeile und Ausgaben unveraendert; Kanarienvogel aller
   Profile (ORACLE §6) OK.
3. Mit Schalter: `bios-r1.tsv` und `bios-r2.tsv` je PASS im Pruefskript.
4. `docs/analysis/ORACLE.md` und `.claude/agents/auditor.md` (Wortlaut im
   Bericht vorschlagen).

## Abnahme

- Punkte 2 und 3 belegt im RUNINFO; doc-check 0 Fehler; `cargo xtask test`
  gruen.
