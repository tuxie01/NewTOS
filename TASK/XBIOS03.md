# XBIOS03 — Haken in `mousevec`/`statvec`/`clockvec`/`joyvec` der `Kbdvbase`-Tabelle werden nie gerufen

Status: **OFFEN** · Prioritaet: **P1** · Typ: Developer (oder Researcher, falls die Luecke als Abweichung sanktioniert werden soll) — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-xbios-chardev.md` · verwandt mit [BIOS03](BIOS03.md) und [BIOS05](BIOS05.md) (Haken, die NewTOS nie ruft)

## Sachverhalt

**XBIOS-CHARDEV-003 — S2 — MEASURED.** Beleg: Bericht Abschnitt 2, Lauf `measurements/measurement-2026-10-28-xbios-chardev-audit/RUNINFO.TXT`.

- Eingabesonde X04, zwei echte Mauspakete (rechte Taste ab/auf): EmuTOS `mousevec calls=2`, letztes Paket `F8 00 00` (TT, Falcon, Wiederholung); NewTOS `calls=0` (beide Laeufe), obwohl der Injektor die Bytes nachweislich eingespeist hat (`measurements/measurement-2026-10-28-xbios-chardev-audit/runs/KEYS-NEWTOS-raspi4b-injector`, `measurements/measurement-2026-10-28-xbios-chardev-audit/runs/KEYS-EMUTOS-tt`).
- Sonde W04: `clockvec/statvec/joyvec` EmuTOS `1/1/1` (Statuspaket `08 00…` = relativer Mausmodus), NewTOS `0/0/0`; die Wurzel ist von [XBIOS04](XBIOS04.md) (auf NewTOS entsteht kein Paket) nicht trennbar.
- Gleich auf beiden Seiten: Zeiger gerade und < 2 GiB, zweiter Aufruf gleicher Zeiger, neun Slots gueltig, `kbstate` 0, `kbdvec` gueltig, Slots im Usermodus beschreibbar und zuruecksetzbar (V01–V03). Unterschied ohne Befund: NewTOS belegt alle neun Slots mit demselben `ret`-Stub (EmuTOS 7 verschiedene Werte; `tos-api/src/xbios/kbdvecs.rs:147@write_table`).
- Lokalisiert: `kernel/src/ikbd.rs:187@MS_BUTTONS` (Mauspaket-Kopf geht in den VDI-Mauszustand, `kernel/src/ikbd.rs:219@mouse_event`), `kernel/src/ikbd.rs:194@PKT_REMAINING` (Status-/Uhr-/Joystickpakete verschluckt); nur `kbdvec` wird ausgewertet (`tos-api/src/xbios/kbdvecs.rs:352@route_key`).
- EmuTOS-Stelle: `references/emutos/bios/aciavecs.S:470-530` (Paket → Vektor, `kbd_jump_vec`).
- Fehlerbild: Programme, die Maus-Rohdaten ueber `mousevec` lesen (Spiele, Maus-Beschleuniger, Malprogramme), bekommen still nichts.
- Spec: `docs/api/xbios/22-kbdvbase.md` C6 „D7-Luecke“, `docs/STATUS.md` „Blocked“ (KEYS02/D7); nicht in `docs/COMPATIBILITY.md`.

## Arbeit

1. Fix gegen `references/emutos/bios/aciavecs.S:470-530`, oder (Researcher) die Luecke mit Begruendung in `docs/COMPATIBILITY.md` eintragen.
2. Ein Haken-Aufruf braucht EL0-Code im Eingabepfad: gleiche Zielfrage wie bei den `hdv_*`-Vektoren ([BIOS03](BIOS03.md)). **Offener Punkt — Entscheidung beim Anwender: gemeinsames ADR mit BIOS03?** Bis dahin kein Fix.
3. Hosttest: Paket → Haken-Aufruf je Vektor; Eingabesonde X04 als Messfall.

## Abnahme

- Bestaetigender Fall: Eingabesonde X04 (`mousevec calls=2`) und Sonde W04 (`clockvec/statvec/joyvec 1/1/1`) gleich dem Orakel; `Kbdvbase`-Faelle `bios-r1` 136 und `xbios-chardev-r1` 37 bleiben EQUAL. Haken-Aufrufe sind kein tosdump-Tabellenfall (brauchen Rueckruf-Code, Bericht Abschnitt 5).
- Kein vorher gleicher Fall in `bios-r1.tsv`, `bios-core-r1.tsv`, `xbios-chardev-r1.tsv`, `xbios-system-r1.tsv` weicht neu ab.
- `cargo xtask test` gruen.
- `docs/STATUS.md` und Spec im selben Commit; `cargo xtask doc-check` der geaenderten Doku 0 Fehler.
