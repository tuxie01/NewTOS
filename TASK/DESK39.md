# DESK39 — `inf_file_folder` benennt nicht selbst um und zaehlt Ordner nicht

Status: IMPLEMENTED (2026-11-11) | Priority: P3 | Commit: f7c61f863 | Missing: tester acceptance through cargo xtask accept

Migrated (owner batch 17, 2026-11-11, to IMPLEMENTED), evidence: commit f7c61f863 (feat(desk): rebuild the Show-info dialogs as EmuTOS's resource trees), VERIFIED: f7c61f863 (feat(desk): rebuild the Show-info dialogs as EmuTOS's resour) names the id by status-line-named, its message names the ticket-title identifier inf_file_folder, touches code (98 files), legacy line: ERLEDIGT 2026-09-25 (`f7c61f86`, ADFFINFO-Neubau: `STRENAME`-Retry-Schleife und

Line 3 before the migration:
Status: **ERLEDIGT 2026-09-25** (`f7c61f86`, ADFFINFO-Neubau: `STRENAME`-Retry-Schleife und `dos_chmod`-Rueckschreiben in `inf_file_folder`, `count_ffs` fuer Ordner — `programs/desktop/src/inf.rs`; nachgetragen 2026-10-22) · Prioritaet: **P3** · Typ: Funktionsluecken — Nebenbefunde DESK38

Abhaengigkeiten: [DESK38](DESK38.md) (erledigt)

## Die Stellen

1. **Umbenennen gehoert in die Funktion.** EmuTOS' `inf_file_folder` benennt
   in einer `while(1)`-Schleife selbst um, mit `STRENAME`-*Retry* bei
   Misserfolg (`references/emutos/desk/deskinf.c:551,629-634`). Bei uns
   benennt der **Aufrufer** einmal um; misslingt es, gibt es keine zweite
   Chance im selben Kasten.
2. **Das Schreibschutz-Attribut wird nicht zurueckgeschrieben**
   (`deskinf.c:591-609`) — die Zeile im Kasten ist reine Anzeige.
3. **Fuer einen Ordner fehlt die Zaehlung** (`count_ffs`/`inf_fifosz`,
   `deskinf.c:525-531`): der Ordnerkasten zeigt weder Datei- noch Ordnerzahl
   noch Groesse.

Punkte 1 und 2 sind **aelter als DESK38** und wurden dort nicht angefasst;
sie sind der Grund, warum `do_info` nicht 1:1 ist. Punkt 3 faellt erst mit
DESK38 auf, weil es den Kasten vorher gar nicht gab.

## Arbeit

Der Reihe nach, jede Stelle mit ihrer Fundstelle. 1 und 2 zusammen (beide in
derselben Schleife), 3 getrennt — `count_ffs` gibt es seit DESK38 bereits.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.
