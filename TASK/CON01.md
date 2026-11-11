# CON01 — Spaltenzaehler (`glbcolumn`) und BDOS-Typeahead fuer die Konsole

Status: IMPLEMENTED (2026-11-11) | Priority: P3 | Commit: 04862df4b | Missing: real-Pi test

Migrated (owner batch 14, 2026-11-11, to IMPLEMENTED), evidence: commit 04862df4b (feat(gemdos): EmuTOS console layer - typeahead, Ctrl-S/Q/X, tabs (CON0), VERIFIED: diff of 04862df4b (feat(gemdos): EmuTOS console layer - typeahead, Ctrl-S/Q/X, ) touches tos-api/src/gemdos/console.rs, names the id by subject, 57 files, legacy line: ERLEDIGT 2026-10-18 (QEMU, `5d2a3812`, Pi offen)

Line 3 before the migration:
Status: **ERLEDIGT** 2026-10-18 (QEMU, `5d2a3812`; Pi offen) · Prioritaet: **P3** · Typ: Konformitaet — Anwenderentscheidung 2026-10-13 bei der FS11-Spec

Abhaengigkeiten: [FS11](FS11.md) (Geraeteweiche)

## Sachverhalt

EmuTOS fuehrt je Geraet einen Spaltenzaehler (`glbcolumn[]`) und einen
64-Tasten-Typeahead (`bdos/console.c`). Darauf beruhen Tab-Expansion in
`tabout`/`cookdout`, das Neueinruecken in `cgets` (`newline(h, stcol)`),
Ctrl-S/Ctrl-Q-Flusskontrolle und die Typeahead-Uebernahme in `conbrk`.
NewTOS hat beides nicht (freigegebene Abweichung, `output_break` in
`tos-api/src/gemdos/console.rs`; `docs/api/gemdos/console/02-cconout.md`,
`09-cconws.md`). Die Abweichung betrifft gemeinsam `Cconout`, `Cconws`,
`Cconrs` und `Fwrite`/`Fread` auf CON und wird deshalb hier fuer alle
zusammen behandelt, nicht einzeln.

Nebenbei: `docs/api/gemdos/console/0a-cconrs.md` ist veraltet („Blocked on
Pterm“), obwohl `edit_line` existiert.

## Arbeit

1. Researcher: Zustand und Ablaeufe aus EmuTOS belegen; Konsequenz fuer den
   SPSC-Eingabering in NewTOS; Spec-Entwurf, Freigabe durch den Anwender.
2. Developer: umsetzen; Hosttests.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Anwenderentscheidung (2026-10-18)

Die am 2026-10-13 genehmigte Abweichung (nur Ctrl-C, keine Tab-Expansion, kein
Typeahead) wird **zurueckgenommen**: wie EmuTOS angleichen — `conbrk` mit
Ctrl-S/Ctrl-Q/Ctrl-X und Typeahead, `glbcolumn` je Geraet, Tab-Expansion in
`tabout` (`references/emutos/bdos/console.c:202-300`). Reihenfolge: direkt nach
PROC04 Punkt 1 (Umlenkung, selber Code). Spec-Stellen, die die Abweichung als
genehmigt fuehren (`docs/api/gemdos/file/40-fwrite.md`, `console.rs:30-60`
Kommentar), mitziehen.

## Erledigt (2026-10-18, QEMU, `5d2a3812`)

Typeahead, Ctrl-S/Q/X, Tab-Expansion, `cgets` wie EmuTOS; Belege
`measurements/measurement-2026-10-18-con01/` (rot 20 → gruen) und
`measurements/measurement-2026-10-18-con01-test/` (Tester PASS, `27b36347`).
Offen: Pi (Kosten des USB-Polls je Ausgabebyte); Anwenderentscheidung zu
`newline` (Ctrl-U nach Backspace in Spalte 0: EmuTOS zaehlt ohne Grenze
herunter, NewTOS stoppt bei 0 — `docs/api/gemdos/console/0a-cconrs.md`).
