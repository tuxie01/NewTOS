# BIOS17 — `swv_vec` mit dem Reset-Einstieg vorbelegen; `RESET_STUB_VA` → `RESET_ENTRY_VA`

Status: DONE (2026-11-11) | Priority: P3 | Commit: 46947fab1 | Receipt: BIOS17-STATUS-DONE.MD

Migrated (owner batch 4, 2026-11-11), evidence: receipt TASK/BIOS17-STATUS-DONE.MD exists, commit 46947fab1, TASK/BIOS17-STATUS-DONE.MD, commit 46947fab1 (fix(bios): swv_vec starts at the reset entry, rename reset entry (BIOS), fix/feat commits naming the id: 46947fab1 (n=1), measurements/measurement-2026-10-30-bios16/, the receipt names no hash for this commit, the Migrated line names it

Line 3 before the migration:
Status: **OFFEN** · Prioritaet: **P3** · Typ: Developer — Anwenderentscheidung 2026-10-30, Nachlauf aus [BIOS16](BIOS16.md)

## Sachverhalt

- EmuTOS setzt `swv_vec` (`$46E`) erst auf `just_rts` und im selben
  `bios_init` auf `os_header.reseth` (`references/emutos/bios/bios.c:363`,
  `:552`); gemessen V01 (tt/falcontc) `swv_vec` = `reseth`
  (`measurements/measurement-2026-10-30-bios16/RUNINFO.TXT`). NewTOS: 0.
- Der NewTOS-Reset-Einstieg (`tos-api/src/bios/reset.rs`, heute
  `RESET_STUB_VA`/`RESET_STUB_CODE`) ist der echte RST01-Neustartpfad, kein
  Platzhalter.

## Entscheidung (Anwender, 2026-10-30)

1. `swv_vec` mit dem NewTOS-Reset-Einstieg vorbelegen (Startwert in
   `PAGE0_HOOKS`, `tos-api/src/bdos/process/hooks.rs`, und wo die Seite 0
   gefuellt wird). Abnahme: `vwprobe` V01 gleich dem Orakel im Wert-Verhaeltnis
   (`swv_vec` = `reseth` des jeweiligen Systems).
2. `RESET_STUB_VA` → `RESET_ENTRY_VA`, `RESET_STUB_CODE` → `RESET_ENTRY_CODE`
   in Code, Doku und `mapping.tsv`; rein mechanisch, kein Verhaltenswechsel,
   Hosttests unveraendert (ausser dem neuen Test fuer `swv_vec`).

## Abnahme

- `vwprobe` beidseitig, V01 gleich; `sxprobe`, `hkprobe`, MEM12 P0PAR
  unveraendert; `cargo xtask test` gruen; doc-check 0.

[Gemeinsame Regeln](README.md) gelten.

## Anwenderentscheidung 2026-10-30

Die Abweichung vom Ticket ("Hosttests unveraendert") ist genehmigt: die Testprozesse fuellen Seite 0 wie der echte Lader (`PAGE0_HOOKS`-Vorgaben), keine Zusicherung geaendert.
