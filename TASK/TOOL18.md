# TOOL18 — Cookie-Falltabelle, gemessen auf dem gepinnten ROM (alle drei Profile)

Status: IMPLEMENTED (2026-11-11) | Priority: P2 | Commit: 9b882e8d4 | Missing: seven cookies open with the owner

Migrated (owner batch 15, 2026-11-11, to IMPLEMENTED), evidence: receipt TASK/TOOL18-STATUS-DONE.MD exists (legacy DONE receipt, IMPLEMENTED accepted by TOOL55), commit 9b882e8d4 (feat(tosdump): cookie-jar cases measured on the pin (TOOL18)), VERIFIED: diff of 9b882e8d4 (feat(tosdump): cookie-jar cases measured on the pin (TOOL18)) touches tosdump/tests/test_jar.c, tosdump/cases/jar-r1.tsv, names the id by subject, 77 files, legacy line: ERLEDIGT 2026-10-29 ([Quittung](TOOL18-STATUS-DONE.MD)), 7 Cookies OPEN beim Anw

Line 3 before the migration:
Status: **ERLEDIGT** 2026-10-29 ([Quittung](TOOL18-STATUS-DONE.MD)); 7 Cookies OPEN beim Anwender · Prioritaet: **P2** · Typ: Developer (Werkzeug) — Anwenderentscheidung 2026-10-28, aus [ARCH05](ARCH05.md)

## Sachverhalt

- Es gibt keine Cookie-Falltabelle; die vorhandenen Jar-Werte stammen aus der
  `SCRNINFO`-Sonde vom 2026-10-02 auf **EmuTOS 1.4**, nicht vom Pin
  (`docs/analysis/machine-identity-adr-proposal-2026-10-28.md`, Abschnitt 1).
  Vorschlag dort: Pseudozeile `jar:<TAG>`, Faelle je Cookie plus Anzahl und
  Reihenfolge; `td_jar_read` existiert (`tosdump/core/scrninfo.h:39@td_jar_read`).
- Anwender 2026-10-28: messen auf dem gepinnten ROM, Profile `tt`,
  `falcontc`, `disk`; die ACCEPTED-Abweichungen in `docs/COMPATIBILITY.md`
  nutzen die gemessenen Werte.

## Arbeit

1. Zeilentyp fuer Cookie-Faelle (Wert je Tag, „nicht vorhanden“, Anzahl,
   Reihenfolge), Spec in `docs/TOS-REFERENCE-DUMPER.md`, Tests in
   `tosdump/tests`.
2. Neue Tabelle, Referenzen fuer `tt`, `falcontc`, `disk` je zweimal bitgleich.
3. NewTOS-Lauf per `cargo xtask tosdump`, Ergebnis je Fall im RUNINFO.
4. Vorschlag fuer die ACCEPTED-Tabellen je Profil in `docs/COMPATIBILITY.md`
   mit den gemessenen Werten (der Lead traegt ein).

## Abnahme

- `tosdiff --selfcheck` besteht; Hosttests gruen (`cargo xtask test`,
  `tosdump/tests`); doc-check 0 Fehler.
