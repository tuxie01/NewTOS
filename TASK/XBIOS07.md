# XBIOS07 — Spec und Doku der Zeichengeraete/Tastatur widersprechen Messung und Code

Status: **OFFEN** · Prioritaet: **P3** · Typ: Researcher — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-xbios-chardev.md`

## Sachverhalt

**XBIOS-CHARDEV-007 — S4 — MEASURED (Doku widerspricht Messung oder Code).** Beleg: Bericht Abschnitt 2, Lauf `measurements/measurement-2026-10-28-xbios-chardev-audit/RUNINFO.TXT`. Dateigruppe `docs/api/xbios/` plus `docs/COMPATIBILITY.md`:

1. `docs/api/xbios/0e-iorec.md` und `docs/api/xbios/18-bioskeys.md` (Pi binding): „returns `EINVFN`“ — gemessen die Funktionsnummer (`0x0E`, `0x18`).
2. `tos-api/src/xbios/mod.rs:26@PHYSBASE`: `Iorec`/`Bioskeys` „need NewTOS keyboard/mouse infrastructure that does not exist yet“ — fuer `Bioskeys` nennt die eigene Spec `keymap::select_builtin` als vorhanden (Code-Kommentar: Developer, siehe [XBIOS01](XBIOS01.md)/[XBIOS02](XBIOS02.md)).
3. `docs/COMPATIBILITY.md`, Abschnitt „The IKBD vector table is per process“: „NewTOS keeps the table in each process's own page 0 (`0x9A0` …)“ — gemessen `0x7F010` (private RW-Seite seit MEM12, wie der MEM12-Absatz weiter unten richtig sagt).
4. `docs/api/xbios/2c-bconmap.md`: „single-serial build“ — der gepinnte Referenzbau hat `BCONMAP_AVAILABLE` ([XBIOS05](XBIOS05.md)).
5. `docs/api/xbios/0f-rsconf.md` „zero-extended“ gegen das Trap-ABI ([XBIOS06](XBIOS06.md)).

(Punkt 6 des Berichts, das Ledger, steht in [BIOS13](BIOS13.md).)

## Arbeit

1. Die Punkte 1, 3, 4, 5 aus der EmuTOS-Quelle und den Messlaeufen korrigieren, jede Korrektur mit `file:line` belegen.
2. Wo ein Punkt von einer offenen Entscheidung abhaengt (4, 5): erst nach der Entscheidung in XBIOS05/XBIOS06.

## Abnahme

- Jede korrigierte Aussage hat einen Beleg (EmuTOS `file:line` oder Laufdatei); kein Eintrag widerspricht mehr einem Fall der vier Tabellen.
- `cargo xtask test` gruen (unveraendert); kein vorher gleicher tosdump-Fall weicht neu ab.
- `cargo xtask doc-check` der geaenderten Doku 0 Fehler.

## Nachtrag 2026-10-28 (aus XBIOS01 Punkt 1, Researcher)

- `docs/api/structures/iorec.md` nennt die IKBD-Eintraege „WORD-sized“; sie
  sind 4 Byte (`references/emutos/bios/ikbd.c:163,269`).
- `docs/api/xbios/_state.md` §3: `EXT_IOREC` „24 bytes“ — es sind 33 Byte
  Felder, auf m68k 34.
- `docs/api/structures/iorec.md:24` „total 16 bytes“ widerspricht EmuTOS
  (14 Byte, `references/emutos/bios/iorec.h:20-27`) und dem Vertrag in
  `docs/api/xbios/0e-iorec.md` (aus der SDK04-Erhebung).

## Stand 2026-10-28

- Berichtigt (`[XBIOS07]`): `docs/api/xbios/18-bioskeys.md`,
  `docs/api/xbios/2c-bconmap.md` (pending XBIOS05), `docs/api/xbios/0f-rsconf.md`
  (pending XBIOS06), `docs/api/structures/iorec.md` (14 Byte, 4-Byte-IKBD-
  Eintraege, Geraetezuordnung 0 = RS-232, 1 = IKBD, 2 = MIDI),
  `docs/api/xbios/_state.md` (`EXT_IOREC` 33/34 Byte, abgeleitet);
  `docs/COMPATIBILITY.md` (Lage der IKBD-Vektortabelle: private Seite
  `0x7F000`, nicht Seite 0) vom Lead nach Vorschlag eingetragen.
- Offen: `docs/api/xbios/22-kbdvbase.md` nennt noch `0x9A0` als Tabellenort;
  `docs/api/xbios/_state.md` beschreibt im IOREC-Block `head`/`tail` vertauscht
  (`references/emutos/bios/ikbd.c:163-170`); in `2c-bconmap.md` ist offen, ob
  `init_bconmap` allein durch `BCONMAP_AVAILABLE` ausgeblendet wird.
- Entscheidung fuer [XBIOS05](XBIOS05.md) (`Bconmap`-Profil): Optionen in
  `2c-bconmap.md`, „NewTOS implementation notes“.

