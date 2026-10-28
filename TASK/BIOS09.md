# BIOS09 — `Bconin` auf AUX, MIDI und Bconmap-Ports kehrt sofort mit 0 zurueck

Status: **OFFEN** · Prioritaet: **P2** · Typ: Researcher — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-bios-core.md`

## Sachverhalt

**BIOS-CORE-008 — S3 — MEASURED.** Beleg: Bericht Abschnitt 2, Lauf `measurements/measurement-2026-10-28-bios-core-audit/RUNINFO.TXT`.

- `measurements/measurement-2026-10-28-bios-core-audit/runs/HANG-EMUTOS-tt-1`, `-3`, `-6` (`BCHANG.LOG`): Kontrollhandle 0 kehrt zurueck (`H0 after ret=0`, `references/emutos/bios/parport.c:123` `bconin0`), `Bconin(1)`, `(3)`, `(6)` nicht — 20 s Wanduhr bei Fast-Forward, Trace endet im Aufruf. `measurements/measurement-2026-10-28-bios-core-audit/runs/HANG-NEWTOS-raspi4b`: alle vier `after ret=00000000`.
- Lokalisiert: `tos-api/src/bios/chardev.rs:115@AuxNoCable` (`bconin` → 0), `tos-api/src/bios/chardev.rs:85@Absent`.
- EmuTOS-Stellen: `references/emutos/bios/serport.c:260` `bconin1` (Iorec-Warteschleife), `references/emutos/bios/midi.c:35` `bconin3` (`while(!bconstat3())`).
- Spec/Entscheidung: `docs/api/gemdos/file/3f-fread.md` FS11 Punkt 1a nennt das Nichtblockieren fuer AUX ausdruecklich „documented divergence“ (auch im Code, `tos-api/src/bios/chardev.rs:110-114@AuxNoCable`); kein COMPATIBILITY-Eintrag.
- Fehlerbild: Terminal- oder MIDI-Eingabeprogramme lesen einen endlosen Strom von NUL-Bytes statt zu warten.

## Arbeit

1. Entscheidung: FS11 Punkt 1a in `docs/COMPATIBILITY.md` aufnehmen oder auf Blockieren aendern (mit Folgen fuer Programme ohne Eingang: Haenger).
2. Developer-Ticket erst nach der Entscheidung. Kein tosdump-Fall (EmuTOS blockiert, unbeaufsichtigte Tabelle ausgeschlossen).

## Abnahme

- Bestaetigender Fall: Lauf-Familie HANG (`BCHANG.LOG`) mit Handle 1/3/6 gleich dem Orakel (bei „Blockieren“), sonst COMPATIBILITY-Eintrag und ACCEPTED; kein Tabellenfall moeglich (Bericht Abschnitt 5).
- Kein vorher gleicher Fall in `bios-r1.tsv`, `bios-core-r1.tsv`, `xbios-chardev-r1.tsv`, `xbios-system-r1.tsv` weicht neu ab.
- `cargo xtask test` gruen.
- `docs/STATUS.md` und Spec im selben Commit; `cargo xtask doc-check` der geaenderten Doku 0 Fehler.
