# BIOS22 — BIOS character calls dispatch through the `bcon*_vec` tables as on EmuTOS

Status: OPEN (2026-11-20) | Priority: P2

Type: researcher (spec gate), developer, tester. Owner decision 2026-11-20 (EmuTOS is the yardstick), from finding
6.2 of `docs/api/bios/_sysvars-contract.md` (TASK/BIOS19.md).

## Facts

Read from source, not measured:

- EmuTOS dispatches `Bconstat`, `Bconin`, `Bcostat` and `Bconout` through four tables of eight longs each in the
  system variables, `_bconstat_vec`, `_bconin_vec`, `_bcostat_vec`, `_bconout_vec` (`$51E-$59D`;
  `references/emutos/bios/bios.c:932,976,1009,1161`). A program redirects a device by patching a slot.
- NewTOS does not write the tables and does not call through them; `PAGE0_HOOKS`
  (`tos-api/src/bdos/process/hooks.rs:283@PAGE0_HOOKS`) does not watch them, so a patched slot is neither served
  nor warned: a gap of ADR-031 rule 1 ("never silent").
- No program in the searched trees patches them; the gitignored `Applications/` tree was not searched.

## Work

1. Spec gate: the tables' layout, boot contents and the call convention of a slot from the EmuTOS source; what a
   slot holds on NewTOS (the m68k routine address has no counterpart), how a process's patched slot is called
   (upcall, as `kbdvec`) and what the per-process page 0 means for it. Choices go to the owner, an ADR-031
   amendment included.
2. Implement as decided.

## Acceptance

- A probe patches a `Bconout` slot and sees its routine called; the unpatched tables read as specified; the same
  probe on the pinned oracle.
- No case that was equal before in `bios-r1.tsv`, `bios-core-r1.tsv`, `xbios-chardev-r1.tsv`,
  `xbios-system-r1.tsv` differs newly; `cargo xtask test` green; both boards build; doc-check 0 failures.
