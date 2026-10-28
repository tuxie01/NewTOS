# BIOS06 — `Getmpb`: Kern und SDK-Header sind sich ueber das `MD`-Layout nicht einig

Status: **OFFEN** · Prioritaet: **P1** · Typ: Researcher (Vertrag festlegen), danach Developer oder SDK — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-bios-core.md`

## Sachverhalt

**BIOS-CORE-003 — S2 — MEASURED — NOT LOCALIZED.** Beleg: Bericht Abschnitt 2, Lauf `measurements/measurement-2026-10-28-bios-core-audit/RUNINFO.TXT`.

- Kern: `tos-types/src/memory.rs:61@MD` (`m_start`/`m_length` je 4 Byte an Offset 8/12, 24 Byte). SDK: `sdk/libc/overlay/include/mint/ostruct.h:293@_md` (`long md_start`/`md_length`, unter LP64 je 8 Byte an 8/16, 32 Byte). Welche Seite den Vertrag bricht, ist eine Vertragsfrage (COMPATIBILITY „Native vs m68k struct variants“; EmuTOS' `m_start` ist ein Zeiger `UBYTE *`). Beobachtet ueber `Getmpb` (`tos-api/src/bios/getmpb.rs:84@getmpb`).
- Sonde G00–G06 (`measurements/measurement-2026-10-28-bios-core-audit/runs/PROBE-EMUTOS-tt`, `measurements/measurement-2026-10-28-bios-core-audit/runs/PROBE-NEWTOS-raspi4b`): EmuTOS `md_start = 0x00023BA0 = _membot`, `md_length = 0x003B6B60`, `start+length = _memtop`; NewTOS durch `_MD`: `md_start = 0x00A00000_00400000`, `md_length = 0`; Rohbytes G05 zeigen das Kern-Layout mit richtigen Werten (0x00400000, 0x00A00000). Gleich: `mp_mfl` eine TOS-Adresse, `mp_mal = NULL`, `mp_rover` unberuehrt, `m_link`/`m_own` NULL, zweiter Aufruf gleicher Zeiger.
- Nebenbefund: EmuTOS' `themd` liegt bei `$48E` in geschuetztem Speicher (Usermodus-Lesen = Busfehler, Sonde liest per `Supexec`); NewTOS erlaubt den Usermodus-Lesezugriff — harmlose Richtung.
- `sdk/apps/apitest/apitest.c:493@MDX` umgeht die Diskrepanz mit eigenem Kern-Layout.
- EmuTOS-Stelle: `references/emutos/bios/biosmem.c:134-151`.
- Fehlerbild: jedes SDK-Programm, das `Getmpb` ruft und `mp_free->md_length` liest (Speicher-Infoprogramme), liest 0 und eine unsinnige Startadresse.

## Arbeit

1. Researcher: Vertrag aus COMPATIBILITY und `references/emutos/bios/biosmem.c:134-151` festlegen (Zeigerbreite folgt der ABI?), Spec dazu.
2. Developer oder SDK (Overlay `ostruct.h` — wie [SDK02](SDK02.md)) setzt den Vertrag um; Hosttest oder `_Static_assert` gegen die Spec.

## Abnahme

- Bestaetigender Fall: Sonde G00–G06 mit dem Vertragslayout gleich dem Orakel (`md_length` = Restlaenge, `start+length` = Speicherende); vorgeschlagene Zeile `13:0 b:0 0:12.L` (Bericht Abschnitt 5) braucht eine Super-Zustands-Vorstufe ([BIOS13](BIOS13.md)).
- Kein vorher gleicher Fall in `bios-r1.tsv`, `bios-core-r1.tsv`, `xbios-chardev-r1.tsv`, `xbios-system-r1.tsv` weicht neu ab.
- `cargo xtask test` gruen.
- `docs/STATUS.md` und Spec im selben Commit; `cargo xtask doc-check` der geaenderten Doku 0 Fehler.

## Anwenderentscheidung 2026-10-28

- Der Inhalt hinter dem Rueckgabezeiger wird per tosdiff abgenommen, sobald
  [TOOL16](TOOL16.md) (Dumper liest durch Rueckgabezeiger, Layout aus
  `docs/api/`) steht; TOOL16 kommt in der Warteschlange davor.
- Bis dahin gilt fuer den MEASURED-Teil die Sondenregel aus CLAUDE.md
  „Testing“ (Sonde als dauerhafter Test, beidseitig gegen das gepinnte
  Orakel, mit Eingabe-Trace; Folgeticket nach tosdiff).

## Stand 2026-10-28 (Vertrag, Entwurf)

- Vertrag als Vorschlag in `docs/api/bios/00-getmpb.md` („NewTOS-Vertrag
  [BIOS06]“, V1–V5) und `docs/api/structures/md.md`: LP64-Header-Layout
  (`_MD` 32 Byte, `_MPB` 24 Byte); falsch ist abgeleitet der Kern, der sein
  internes 24-Byte-BDOS-`MD` veroeffentlicht; Inhalt ein statischer
  Schnappschuss `[membot, memtop)`, nur `mp_mfl`/`mp_mal` werden geschrieben.
  Entscheidungen O1–O4 beim Anwender.
- Programmerhebung: einziger Leser `sdk/apps/apitest/apitest.c` (eigenes
  `MDX`). Kartenluecke: per Konstruktion leer fuer das Standardabbild (alle
  ausfuehrbaren Dateien aus lokalen Rezepten,
  `docs/analysis/iorec-survey-2026-10-28.md`, Nachtrag).
- Randbefund: `docs/COMPATIBILITY.md` (TSR-Bloecke ueber `Getmpb` auffindbar;
  „`Getmpb` unchanged“) widerspricht dem Code (`mp_mal` immer NULL, ein MD).

