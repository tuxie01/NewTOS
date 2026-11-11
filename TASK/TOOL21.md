# TOOL21 — `mapping.tsv`: ADR-Beleg mit Fundstelle, Gruppe A als Hardware, AUX-Pfad nach XBIOS15

Status: DONE (2026-11-11) | Priority: P2 | Commit: daa2973ea | Receipt: TOOL21-STATUS-DONE.MD

Migrated (owner batch 5, 2026-11-11), evidence: receipt TASK/TOOL21-STATUS-DONE.MD exists, commit daa2973ea, TASK/TOOL21-STATUS-DONE.MD, commit daa2973ea (feat(deps): ADR evidence with location, hardware slots, AUX path (TOOL), fix/feat commits naming the id: daa2973ea (n=1), the receipt names no hash for this commit, the Migrated line names it

Line 3 before the migration:
Status: **ERLEDIGT** 2026-10-29 ([Quittung](TOOL21-STATUS-DONE.MD)) · Prioritaet: **P2** · Typ: Developer (Werkzeug + Zuordnung) — Anwenderentscheidung 2026-10-29, aus [TOOL17](TOOL17.md)

## Sachverhalt

- TOOL17 hat Ermessensfaelle als `design:ADR-NNN` eingetragen (NatFeats als
  `hardware:natfeats`; Aufloesungswechsel ADR-017; `jdisint`/`jenabint`,
  `etv_critic`, `Kbdvbase`-Haken ADR-031; `setvalue_snd` ADR-012). Anwender:
  bestaetigt, **jede Zeile nennt das ADR mit Datei und Zeile** im Beleg.
- Gruppe A des TOOL17-Berichts (DSP XBIOS 0x60–0x7F samt LOD-Lader,
  DMA-Ton 0x80–0x8D samt `devconnect_*`/`sndcmd_*`, TT-Shifter 0x50–0x57,
  VIDEL 0x58–0x5F samt `get_videl_mode`/`lookup_videl_mode`): Klasse „mit
  Absicht“, Beleg `hardware:<name>`.
- AUX-Pfad (Gruppe D: `bconout1`, `bcostat1`, `rsconf1`,
  `mfp_rs232_tx_interrupt(_handler)`, `bconinB`/`bconoutB`/`bconstatB`/
  `bcostatB`/`rsconfB`, `scc_tx_interrupt_handler`, `sccb_tx_interrupt`,
  `put_iorecbuf`, `init_serport`, `init_bconmap`, `rsconf_dummy`): gegen das
  XBIOS15-Ergebnis neu einordnen, `mapped`, wo es ein Gegenstueck gibt
  (`kernel/src/console.rs@AuxSerial`, `tos-api/src/bios/chardev.rs@AuxNoCable`,
  `tos-api/src/xbios/iorec.rs`).
- Gruppe E (Maschinenerkennung) gehoert zu [ARCH06](ARCH06.md); Gruppen B
  (XHDI/SCSIDRV), C (OS-Pool/DND-Baum) und F (Startfunktionen) warten auf die
  Erhebung des Researchers.

## Arbeit

1. Belegform `design:ADR-NNN@<datei>:<zeile>` (oder gleichwertig), der
   Hosttest prueft, dass die Zeile in der genannten Datei die Ueberschrift
   bzw. den Abschnitt des ADR trifft (z. B. ADR-017 `docs/MEMORY.md:824`).
2. Gruppe A und den AUX-Pfad umstellen.
3. `cargo xtask deps --coverage` vorher/nachher im Bericht.

## Abnahme

- Hosttests gruen (`cargo xtask test`), `--check-mapping` gruen; doc-check 0
  Fehler.
