# EMU01 — m68k-Emulator (FR-4): Sammelnotiz fuer den spaeteren Entwurf

Status: PARKED (2026-11-11) | Priority: P4 | Reason: idea (owner 2026-11-07): Line-A thoughts recorded, no assignment, after 1.0 | Reopen: after 1.0, the owner takes up the m68k emulator direction

Migrated (owner batch 10, 2026-11-11), evidence: TASK/EMU01.md status line: IDEE (2026-11-07, Anwender: Line-A-Ueberlegungen festhalten)

Line 3 before the migration:
Status: **IDEE** (2026-11-07, Anwender: Line-A-Ueberlegungen festhalten) · Prioritaet: **P4** · Typ: Richtungsnotiz — kein Auftrag, nach 1.0

## Zweck

`docs/REQUIREMENTS.md` FR-4: originale Atari-`.PRG` laufen ueber einen m68k-Emulator im Benutzerraum, der die native
TOS-API ueber ABI-Marshaling teilt. Dieses Ticket sammelt, was bei dessen Entwurf nicht neu erarbeitet werden soll.
Neue Punkte kommen als Abschnitt dazu, mit Datum und Quelle.

## Line-A (2026-11-07)

Stand: ADR-030 — kein Line-A-Trap fuer native Programme, kein exportierter LINEA-Variablenblock; der Trap gehoert in
den Emulator (Punkt 2), dessen Entwurf offen ist. Kein Kartenprogramm und kein Port liest Line-A
(`docs/analysis/line-a-readers-2026-10-26.md`); Teilbestandsaufnahme der Funktionen `docs/api/_line-a.md`.

Fuer den Emulator gilt (Anwender 2026-11-07, im Gespraech zu V18 Teil 3 K26):

- **Line-A ist dort Pflicht.** Alte Binaerprogramme (Spiele, Demos, fruehe GEM-Programme) nutzen `$A000` (Zeiger auf
  Variablenblock, Fonts, Funktionen), `$A009`/`$A00A` (Maus zeigen/verbergen), `$A005` (Rechteck fuellen), `$A007`
  (BitBlt) und lesen den Block direkt (`V_REZ_HZ`, `MOUSE_BT`, `CUR_WORK` …).
- **Trap im Emulator:** `$A000`–`$A00F` als nicht implementierte Opcodes abfangen (EmuTOS `int_linea`,
  `references/emutos/bios/bios.c:176`, Tabelle `references/emutos/bios/linea.S:230-249`), nicht im Kern.
- **Variablenblock m68k-seitig:** im Adressraum des m68k-Programms, aus dem VDI-Zustand gespeist (Geometrie aus
  `vq_extnd`, Maus aus `tos-api/src/vdi/mouse.rs`), Rueckschreiben nur, wo EmuTOS den Wert wirken laesst.
  Offsets aus `references/emutos/bios/lineavars.S`.
- **Primitive auf die Raster-Kerne:** Fuellen, BitBlt und Expansion laufen auf denselben Kernen wie die VDI
  (`kernel/src/raster_kernel.rs`, V49; Expansion V50; Kopieren V51, hinter `RasterBackend`, ADR-036). Die
  Rechteck-Schnittstelle der Kerne ist dafuer der richtige Schnitt — beim Ausbau nicht wieder pixelweise machen.
- **Messbarkeit:** Mit dem Emulator wird K26 aus [V18](V18.md) Teil 3 (Workstation-Kette ueber `CUR_WORK`) messbar.

## Weitere offene Punkte, die erst mit dem Emulator zaehlen

- [V52](V52.md) — Handler schreiben in die lokale `PTSIN`-Kopie (nur Aufrufe mit zu kleinem `CONTRL[1]`, also
  m68k-Programme mit eigenen Bindungen).
- [BIOS04](BIOS04.md) — MIDI/IKBD-Ausgabe; Programme dafuer kommen erst als m68k-Binaries.
- [ACC05](ACC05.md) — `.PRG`/`.TOS`/`.TTP` gehoeren dem Emulator, native Programme eigene Endungen.
- [A53](A53.md) — Restwerte im oberen Wort von `d0` aus m68k-Programmen.

[Gemeinsame Regeln](README.md) gelten.
