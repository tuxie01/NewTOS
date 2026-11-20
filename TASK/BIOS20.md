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

## Spec gate and owner decisions (2026-11-20)

Spec gate: `docs/api/bios/_sysvars-counters-contract.md`, landed as `567d3b9b4` (researcher `c473ec81d`, verdict
GAPS with three decisions for the owner). Owner 2026-11-20, reminded first that EmuTOS is the yardstick ("ich
wünsche emutos als masstab") and asked with the options of sections 5 to 7 of that file, answered "ja" to the lead's
recommendations:

- **Wider scope.** Not only the six cells above: every unserved system variable that needs only a value or a
  counter is served as on EmuTOS. Added: `_v_bas_ad`, `_vblsem`, `_nvbls`, `_hdv_init`, `_hdv_boot`, `_themd`,
  `_dskbufp`, `_pun_ptr`. The section "Not in this ticket" above no longer holds for them. The four `bcon*_vec`
  tables are TASK/BIOS22.md.
- **Rate (option C1).** `_frclock` and `_vbclock` follow the 60 Hz frame clock `Vsync` waits on, and start at 0 at
  boot.
- **`_vbclock` (option V2).** It advances only while `_vblsem` is greater than 0, as on EmuTOS; `_vblsem` is served
  with EmuTOS's boot value.
- **Per-process view (P1 with P3).** The running process gets the value in the timer interrupt, a process that is
  switched in gets it refreshed at once, and `Vsync` publishes the value before it returns, so
  `Vsync(); x = *_frclock;` reads the new frame number as on EmuTOS.
- **ADR-031.** `dump_vec`, `aux_stat` and `aux_vec` hold the return stub as on EmuTOS; NewTOS still does not call
  `dump_vec`; the `DumpVec` watcher compares against the stub. The ADR text is shown to the owner before it is
  entered; the developer does not start before the amendment is in `docs/MEMORY.md`.

Before the developer starts: a second spec pass for the eight added cells (same file), and the ADR amendment.
For the tester: the six cells and the added ones lie at addresses `Setexc` cannot read; the probe reads through
`Supexec`. The frame rate of the pinned oracle is measured in the test run.

## Second spec pass and owner answers (2026-11-20)

Second pass landed as `a2cd6a7a7` (researcher `0d1deb151`, GAPS with questions Q1 to Q8, section 17 of
`docs/api/bios/_sysvars-counters-contract.md`). Owner 2026-11-20, asked with the options of each question,
followed the lead's recommendations ("ich gehe mit deinen empfehlungen"):

- **Q1, option (c).** `_vbclock` is `_frclock` minus the frames the process ran with `_vblsem` at or below 0; a
  process that was away loses nothing.
- **Q2, option (b).** `_vblsem` at or below 0 in the running process also stops the kernel's frame work for the
  mouse (`vb_draw`), as on EmuTOS. Rule S10 of `docs/api/_super-state.md` gets `_vblsem` as a documented input.
- **Q3, option (b).** `_nvbls` is 8 and `_vblqueue` points at eight empty slots in page 0; a program may enter a
  routine, NewTOS never calls it and warns. The `vblqueue` row of ADR-031 changes.
- **Q4, option (a).** `_v_bas_ad` always holds the screen address. A program that uses it before calling `Logbase`
  or `Physbase` ends with a bus error. Mapping the screen at the first access instead was named to the owner as a
  possible later step; the researcher did not examine it.
- **Q5, option (b).** `_themd` is a 16-byte MD in the m68k layout with the extent at load.
- **Q6, option (b).** `_dskbufp` points at a 3072-byte buffer in the process's private read-write page.
- **Q7.** `_pun_ptr` leaves this ticket and stays 0: it is taken up with TASK/BIOS18.md (partitions), after it is
  read whether `Rwabs` honours what a `PUN_INFO` consumer expects.
- **Q8.** `_hdv_init` holds the return stub; `_hdv_boot` holds a stub that returns a defined error code (the value
  from the EmuTOS routine, fixed in the spec); ADR-031 gets a line for it.

So thirteen cells are served by this ticket. Before the developer starts: the amendment of ADR-031 (stub row,
`vblqueue` row, the `_hdv_boot` stub) and of rule S10, in a wording the owner sees first; then the spec file's
section 17 is restated as decided.

## ADR-031 amended (2026-11-20)

The owner approved the amendment's wording ("ok"); it is in `docs/MEMORY.md` (ADR-031, "Amendment (2026-11-20")
with the commit of this note. Left before the developer: the researcher adds `_vblsem` to rule S10 in
`docs/api/_super-state.md` and restates section 17 of the spec file as decided.
