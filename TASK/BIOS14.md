# BIOS14 — `cargo xtask deps`: fehlende Kanten BIOS-Eintritt → Geraetehandler, `rsconf`, `bconmap`, `kbdvbase`

Status: **OFFEN** · Prioritaet: **P3** · Typ: Researcher (Track B / Werkzeugbefund) — Anwenderentscheidung 2026-10-28, aus den Audits `docs/audits/2026-10-28-bios-core.md` und `docs/audits/2026-10-28-xbios-chardev.md`

## Sachverhalt

**BIOS-CORE-012 — S4 — PLAUSIBLE (Werkzeugbefund)** und **XBIOS-CHARDEV-008 — S4 — PLAUSIBLE (Werkzeugbefund)** (Berichte Abschnitt 2; Laeufe `measurements/measurement-2026-10-28-bios-core-audit/RUNINFO.TXT`, `measurements/measurement-2026-10-28-xbios-chardev-audit/RUNINFO.TXT`). Wiederholung von BIOS-STOR-009 (`docs/audits/2026-10-28-bios-storage.md`, siehe [BIOS01](BIOS01.md)); noch nicht gemessen, nur beobachtet:

1. `cargo xtask deps` hat keine Kanten vom BIOS-Eintritt zu den Geraetehandlern (`bconstat → protect_v` endet); alle `--down`-Mengen sind leer, die Lokalisierung nur leer erfuellt.
2. `rsconf` hat keine Kanten ueber `rsconfptr` zu `rsconf1`/`rsconfB`/`rsconfTT`/`rsconf_dummy`, `bconmap` keine zu den `MAPTAB`-Zielen; `kbdvbase` keine Kante zu den Rufern der Haken (`aciavecs.S` `kbd_jump_vec`).
3. `docs/analysis/deps/mapping.tsv` bildet `bconout3` auf `tos-api/src/xbios/system.rs@midiws` ab (`deps midiws --down`); richtig waere der MIDI-Eintrag der Geraetetabelle `tos-api/src/bios/chardev.rs`.
4. `couplings.md` unter `docs/analysis/deps/`, auf die die Rollendatei verweist, existiert weiterhin nicht.

## Arbeit

1. Prueffall: `deps bcostat --down` listet `bcostat0..4`, `char_dummy`, Bconmap-`maptab`; `deps rsconf --down` listet die vier `rsconf*`-Ziele.
2. Aenderung am Graph-Werkzeug ist Code: Developer-Ticket nach Entscheidung des Anwenders; dieses Ticket legt Befund und Prueffall fest.

## Abnahme

- Die beiden Prueffaelle (`deps bcostat --down`, `deps rsconf --down`) liefern die genannten Ziele; Mapping-Zeile fuer `bconout3` korrigiert.
- Kein vorher gleicher tosdump-Fall weicht neu ab; `cargo xtask test` gruen; `cargo xtask doc-check` der geaenderten Doku 0 Fehler.
