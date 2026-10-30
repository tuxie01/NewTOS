# BIOS17 — `swv_vec` mit dem Reset-Einstieg vorbelegen; `RESET_STUB_VA` → `RESET_ENTRY_VA`

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
