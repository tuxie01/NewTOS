# XBIOS02 — `Bioskeys` ist nicht verdrahtet: per `Keytbl` installierte Tabelle bleibt aktiv

Status: **OFFEN** · Prioritaet: **P1** · Typ: Developer — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-xbios-chardev.md`

## Sachverhalt

**XBIOS-CHARDEV-002 — S2 — MEASURED.** Beleg: Bericht Abschnitt 2, Lauf `measurements/measurement-2026-10-28-xbios-chardev-audit/RUNINFO.TXT`.

- EmuTOS stellt nach `Bioskeys()` alle drei Zeiger auf die Ausgangstabellen zurueck (`unshift_is_n0=1 shift_is_s0=1 caps_is_c0=1`); NewTOS behaelt die Programm-Tabellen (`unshift_is_my=1`). Sonde K04–K10 (`measurements/measurement-2026-10-28-xbios-chardev-audit/runs/PROBE-EMUTOS-tt`, `measurements/measurement-2026-10-28-xbios-chardev-audit/runs/PROBE-NEWTOS-raspi4b`).
- Eingabesonde (`measurements/measurement-2026-10-28-xbios-chardev-audit/runs/KEYS-EMUTOS-tt`, `measurements/measurement-2026-10-28-xbios-chardev-audit/runs/KEYS-NEWTOS-raspi4b-injector`): Taste `a` nach `Keytbl(my_n)` auf beiden `0x001E004B`; Taste `a` nach `Bioskeys()` EmuTOS `0x001E0061`, NewTOS `0x001E004B`.
- XCKTNL (`measurements/measurement-2026-10-28-xbios-chardev-audit/runs/XCKTNL-NEWTOS-raspi4b`, `measurements/measurement-2026-10-28-xbios-chardev-audit/runs/XCKTNL-EMUTOS-tt`): nach `Keytbl(NULL,-1,-1)` und `Bioskeys()` zeigt `norm` auf EmuTOS wieder auf eine Tabelle, auf NewTOS ist er NULL (128 Byte aus Seite 0 als Tabelle uebernommen, `tos-api/src/xbios/keytbl.rs:210@set_key_table`).
- Lokalisiert: `tos-api/src/xbios/mod.rs:65-138@dispatch` (kein `Bioskeys`-Arm, Rueckgabe `0x18`).
- EmuTOS-Stelle: `references/emutos/bios/ikbd.c:136-140`.
- Fehlerbild: ein Programm installiert eigene Tabellen und stellt mit `Bioskeys()` zurueck. Auf NewTOS bleibt die Tastatur verstellt; `set_key_table` aendert den globalen Uebersetzer (`kernel/src/host_env.rs:330@set_key_table`) — Folge fuer Desktop und Folgeprogramme bis zum Neustart: PLAUSIBLE, nicht gemessen (Fall: XCKTNL, danach im Desktop eine Taste pruefen).
- Spec: `docs/api/xbios/18-bioskeys.md` „Step-2 — not wired“; die Spec nennt `keymap::select_builtin` als vorhanden.

## Lokalisierung (Lead, 2026-10-29)

- `bioskeys` (`references/emutos/bios/ikbd.c:136-140`) → noch kein Gegenstueck
  (`mapping.tsv` `unmapped`): Arm fehlt in `tos-api/src/xbios/mod.rs@dispatch`;
  der Zustand, den es zuruecksetzt, liegt in `tos-api/src/xbios/keytbl.rs@keytbl`
  (EmuTOS `current_keytbl`, `keytbl` `mapped`).
- `--down`: `get_keytbl`, `get_kbd_index`, `get_country_record`,
  `get_current_country_input`, `get_default_country` (`bios/country.c`), alle
  `unmapped` — Gegenstueck der Ausgangstabelle ist das aktive Layout in
  `kernel/src/keymap.rs` (`select_builtin` und Umgebung); der Fix traegt die
  Zeilen ein, die er beruehrt (Ratsche).
- `--up` (Wiedertestmenge): `xbiostrap` → `tos-api/src/xbios/mod.rs@dispatch`,
  `biostrap` → `tos-api/src/bios/mod.rs@dispatch`, `kbd_init` →
  `kernel/src/key_repeat.rs@new` (EmuTOS ruft `bioskeys` beim Start; NewTOS'
  Startwert der Tabellen pruefen). `unmapped` darin nur `bios_init`, `biosmain`
  (Start, Kerninit) und `bioskeys` selbst. Vollstaendig: nein, die drei sind hier
  gelistet.

## Arbeit

1. Fix gegen `references/emutos/bios/ikbd.c:136-140`; Hosttest: nach `Keytbl(my)`+`Bioskeys()` zeigen alle drei Zeiger wieder auf die Ausgangstabellen.
2. Kommentar `tos-api/src/xbios/mod.rs:26-31@PHYSBASE` richtigstellen (gemeinsam mit [XBIOS01](XBIOS01.md)).
3. Den PLAUSIBLE-Teil (Zustand nach Programmende) mit XCKTNL messen und im Ticket vermerken.

## Abnahme

- Bestaetigender Fall: Sonde K07–K11 und Eingabesonde X01/X03 (`xkprobe`) gleich dem Orakel (`0x001E0061` nach `Bioskeys()`); der vorgeschlagene Fall K08 (`14:0x18` nach `Keytbl(b0,-1,-1)`) braucht eine Zeigervergleichs-Klasse und ist bis dahin Sonde (Bericht Abschnitt 5).
- Kein vorher gleicher Fall in `bios-r1.tsv`, `bios-core-r1.tsv`, `xbios-chardev-r1.tsv`, `xbios-system-r1.tsv` weicht neu ab.
- `cargo xtask test` gruen.
- `docs/STATUS.md` und Spec im selben Commit; `cargo xtask doc-check` der geaenderten Doku 0 Fehler.
