# BIOS20 — System variables: return stubs for `dump_vec` and `aux_*`, the frame counters, `_timer_ms`

Status: OPEN (2026-11-20) | Priority: P2

Type: researcher (spec gate), developer, tester. Owner decision 2026-11-19, from the findings of TASK/BIOS19.md
(`docs/api/bios/_sysvars-contract.md`, sections 5 and 6, landed as `71d0d925e`).

## Facts

All read from source by the researcher of BIOS19, none measured:

- `dump_vec`, `aux_stat` and `aux_vec` hold 0 on NewTOS; EmuTOS boots them with its empty return routine
  (`references/emutos/bios/chardev.c:78-84`). `prt_stat` and `prt_vec` already hold a return stub on NewTOS
  (ADR-031). A program that hooks one of them and chains to the old value branches to address 0.
- `_frclock` (`$466`) and `_vbclock` (`$462`) read 0 and never count; on EmuTOS the VBL counts them
  (`references/emutos/bios/vectors.S:330-348`). A program that waits on `*_frclock` never leaves its loop. NewTOS
  has a frame clock (`kernel/src/host_env.rs:262@frame_clock`) that is not mirrored into the cells.
- `_timer_ms` (`$442`) reads 0; `Tickcal` answers 20 (`references/emutos/bios/vectors.S:541`).
- No program in the searched trees reads any of them; the gitignored `Applications/` tree was not searched.

## Work

1. Spec gate: the per-variable contract for these six cells (boot value, who counts, when, what a process sees
   across a process switch, as `_hz_200` is per process), from the EmuTOS source, with the measurable cases named.
2. Serve them: the three vectors hold the same stub as `prt_*`; the two counters count as on EmuTOS; `_timer_ms`
   holds the value `Tickcal` answers.
3. `docs/api/bios/_sysvars-contract.md`: the rows move from "missing" to "served"; the ADR-031 table is not changed
   by the developer (an ADR change is the owner's).

## Acceptance

- Host tests: boot values of the six cells; the counters advance; the stub address equals the `prt_*` one.
- On the booted kernel (raspi4b): a probe reads the six cells, calls the three vectors through their old value
  without a crash, and sees `_frclock` and `_vbclock` advance.
- The same reads on the pinned oracle where the oracle can show them; a cell it cannot show is named.
- No case that was equal before in `bios-r1.tsv`, `bios-core-r1.tsv`, `xbios-chardev-r1.tsv`,
  `xbios-system-r1.tsv` differs newly; `cargo xtask test` green; both boards build; warnings delta 0; doc-check of the
  edited docs 0 failures.

## Not in this ticket

The four `bcon*_vec` tables, `_v_bas_ad`, `_vblsem`, `_nvbls`, `_hdv_init`, `_hdv_boot`, `_themd`, `_dskbufp`,
`_pun_ptr`: listed in `docs/DEVIATIONS.md` by TASK/SPEC08.md until a program needs them.
