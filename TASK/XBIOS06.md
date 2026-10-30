# XBIOS06 — `Rsconf(-1 x6)`: TSR-Byte ohne Bit 7, obere 32 Bit null statt vorzeichenerweitert

Status: **OFFEN** (entschieden 2026-10-30: Option a, Messung zuerst) · Prioritaet: **P2** · Typ: Developer (Erweiterung) + Researcher (Spec gegen ABI, Geraeteprofil der Registerbytes) — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-xbios-chardev.md`

## Sachverhalt

**XBIOS-CHARDEV-006 — S3 — CONFIRMED.** Beleg: Bericht Abschnitt 2, Lauf `measurements/measurement-2026-10-28-xbios-chardev-audit/RUNINFO.TXT`.

- Faelle: `bios-r1` 105, `xbios-chardev-r1` 31 — EXACT-DIFF: TT `0xFFFFFFFF_88018100`, Falcon `0xFFFFFFFF_88000000` (SCC), NewTOS `0x00000000_88010100` (`measurements/measurement-2026-10-28-xbios-chardev-audit/runs/NEWTOS-xbios-chardev-r1/tosdiff-newtos-vs-emutos.txt`).
- Sonde S01–S18: Baud setzen/abfragen, `-1` behaelt, `16`/`-3` ausserhalb, UCR/RSR setzen und zuruecklesen, Maskierung, Wiederherstellung sind gleich; nur das TSR-Byte (`81` gegen `01`) und die obere Haelfte weichen ab (`measurements/measurement-2026-10-28-xbios-chardev-audit/runs/PROBE-EMUTOS-tt`, `measurements/measurement-2026-10-28-xbios-chardev-audit/runs/PROBE-NEWTOS-raspi4b`).
- Lokalisiert: `tos-api/src/xbios/system.rs:239@RS_TSR` (Schatten `0x01`) und `tos-api/src/xbios/system.rs:254-286@rsconf` (`old as i64`, nullerweitert).
- EmuTOS-Stelle: `references/emutos/bios/serport.c:217-249` (`rsconf_mfp` liest die MFP-Register; TSR Bit 7 = „Puffer leer“).
- Spec: `docs/api/xbios/0f-rsconf.md` sagt „zero-extended into `x0`“ — widerspricht `tos-api/src/trap.rs` („sign-extended `i64`“) und `docs/TOS-REFERENCE-DUMPER.md` §3 (`ret_hi`). Die Registerbytes sind Geraeteparameter (TT ≠ Falcon); ohne COMPATIBILITY-Eintrag DIFF.
- Fehlerbild: LP64-Programm, das `long r = Rsconf(-1,…)` als negativ erwartet, oder das TSR-Bit „Sender leer“ abfragt.

## Arbeit

1. Researcher: Spec `docs/api/xbios/0f-rsconf.md` gegen das Trap-ABI korrigieren; Geraeteprofil der Registerbytes (TT/Falcon) entscheiden.
2. Developer: Vorzeichenerweiterung und TSR-Bit 7 gemaess Entscheidung; Hosttest je Rueckgabe.

## Abnahme

- Bestaetigende Faelle: `bios-r1` 105 und `xbios-chardev-r1` 31 EQUAL gegen das Orakel (bei Geraeteprofil-Entscheidung: DIFF nur in den Registerbytes, mit COMPATIBILITY-Eintrag ACCEPTED); Sonde S01–S18 gleich.
- Kein vorher gleicher Fall in `bios-r1.tsv`, `bios-core-r1.tsv`, `xbios-chardev-r1.tsv`, `xbios-system-r1.tsv` weicht neu ab.
- `cargo xtask test` gruen.
- `docs/STATUS.md` und Spec im selben Commit; `cargo xtask doc-check` der geaenderten Doku 0 Fehler.

## Anwender 2026-10-28 und Pruefung durch den Lead

- Vorzeichenerweiterung gemaess Regel (`tos-api/src/trap.rs:6@TrapArgs`;
  `docs/COMPATIBILITY.md`, „LONG results in `x0`“). Die Kommentare in
  `tos-api/src/xbios/system.rs` (heute „zero-extended“, passend zum heutigen
  Code) werden **mit diesem Fix** geaendert, nicht vorher.
- TSR-Byte, Schluss des Anwenders geprueft: EmuTOS nutzt TSR Bit 7 selbst als
  „Sender leer“ — gesendet wird bei leerem Ring und gesetztem Bit (`:314`,
  TT-MFP `:433`). **Berichtigt 2026-10-30:** `Bcostat` antwortet im Referenzbau
  aus dem Ausgabering (`references/emutos/bios/serport.c:278-281`); die Abfrage
  `tsr & 0x80` gibt es nur im Debug-Bau mit `RS232_DEBUG_PRINT` (`:275-276`); `Rsconf` liefert das alte TSR im Byte
  15–8 (`:229`). Gemessen: TT `0x81`, NewTOS `0x01`; Falcon (SCC) `0x00`.
  Abgeleitet: ein Programm, das auf Bit 7 wartet, wartet auf NewTOS ewig. Kein
  portierter Nutzer von `Rsconf` gefunden
  (`docs/analysis/abi-assumptions-check-2026-10-28.md` §5). Vorschlag fuer die
  Entscheidung: ruhender Port meldet „Sender leer“ (Bit 7 gesetzt).

## Teilpunkt 2026-10-29 (Anwender, aus XBIOS01)

- `Rsconf` schreibt Baudrate und Flusssteuerung nicht in den Record von
  Geraet 0 (`Iorec(0)`); EmuTOS tut das (`references/emutos/bios/serport.c:222-240`).
  Gefunden beim XBIOS01-Lauf, nicht gemessen
  (`measurements/measurement-2026-10-29-xbios01/RUNINFO.TXT`).
- **Abnahme:** MEASURED-Sonde beidseitig gegen das Orakel (Sondenregel,
  CLAUDE.md „Testing“): `Rsconf(speed, flow, …)`, danach Feld-Dump von
  `Iorec(0)` (Layout aus `docs/api/structures/iorec.md`, TOOL16-Capture),
  `baudrate`/`flowctrl` gleich dem Orakel.

## Researcher 2026-10-30

`docs/analysis/xbios06-rsconf-profile-2026-10-30.md`; Spec
`docs/api/xbios/0f-rsconf.md` gegen das Trap-ABI korrigiert (Vertrag
vorzeichenerweitert, heute nullerweitert, getrennt markiert).

- **Korrektur oben (Lead geprueft):** `Bcostat(1)` = `-1` bei `tsr & 0x80`
  gilt nur im Bau mit `RS232_DEBUG_PRINT`
  (`references/emutos/bios/serport.c:275-276`); im Referenzbau antwortet
  `bcostat1` aus dem Ausgabering (`:278-281`). TSR Bit 7 steht nur in
  `bconout1`/`bconoutTT` (`:314`, `:433`).
- Gegen Falcon weichen zwei Bytes ab (RSR und TSR), nicht nur TSR.
- Optionen (a) TT/MFP mit TSR `0x81`, (b) Falcon/SCC, (c) eigene Bytes +
  COMPATIBILITY; Empfehlung des Researchers (a), passend zur Bconmap-Tabelle
  (Geraet 6, MFP; XBIOS05). Offene Fragen in der Analyse Abschnitt "Offene
  Fragen". Entscheidung des Anwenders siehe unten.

## Entscheidung (Anwender, 2026-10-30)

1. **Option (a) TT/MFP**, passend zu XBIOS05 (Geraet 6). Rueckgabe
   vorzeichenerweitert auf 64 Bit nach dem ABI (`tos-api/src/trap.rs:6@TrapArgs`).
   Abweichung gegen Falcon **ACCEPTED** mit Eintrag in `docs/COMPATIBILITY.md`.
2. **Erst messen, dann fixen:** neue dauerhafte Sonde `rsprobe.c` (neu, unter `tosdump/probes/`)
   (Muster `mdprobe`, beide Seiten gegen das Orakel, Eingabespur
   `bios-arg-trace`): TSR `0x00`/`0x08`/`0xFF` und P16 (woher antwortet
   `Rsconf(-2)`: Record oder Schatten). **Bericht vor dem Fix.**
3. `Rsconf` schreibt nur `baudrate` und `flowctrl` in `Iorec(0)` (MFP-Weg),
   nicht `ucr`/`datamask`.
4. Folgeticket [TOOL27](TOOL27.md): `rsprobe`-Faelle nach tosdiff, sobald die
   Rsconf-Sperre in `tosdump/core/cases.c:4050-4055@CR_RSCONF` es zulaesst.
5. Ticket-Text zu `Bcostat` berichtigt (oben).

## Arbeit, Schritt 1 (Developer, nur Sonde und Messung)

- `rsprobe.c` + Bau-/Laufskripte nach `mdprobe` (`tosdump/Makefile`,
  `tosdump/build-mdprobe-newtos.sh`, `tosdump/probes/run-mdprobe.sh`);
  Fallentwurf in `docs/analysis/xbios06-rsconf-profile-2026-10-30.md`
  (P00–P17, S19–S21).
- Laeufe: Orakel tt und falcontc, NewTOS raspi4b (mit `bios-arg-trace`).
- Kein Produktcode. Ergebnis als Messlauf; danach Entscheidungsvorlage.

