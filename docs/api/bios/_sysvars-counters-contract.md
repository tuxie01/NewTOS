# System variables `dump_vec`, `aux_stat`, `aux_vec`, `_frclock`, `_vbclock`, `_timer_ms`: contract [BIOS20]

> Researcher output for TASK/BIOS20 work item 1 (spec gate). File beside
> `docs/api/bios/_sysvars-contract.md` (which stays the 84-row table; its rows
> for these six cells say "missing"/"hardware" and are right for main
> `bfe644183`; the developer moves them to "served"). Source: EmuTOS `1a7484ab`
> (`references/emutos`); oracle values from `references/emutos-oracle/1a7484ab`.
> Nothing here was measured (no QEMU, no Hatari, by the handover). "Cited" =
> read in this run; "inferred" is marked.

> **Contract vs. reference** (`docs/COMPATIBILITY.md`, "System variables"): the
> value and meaning of a cell a program can read is contract; which EmuTOS
> routine writes it is reference.

## 1. The six cells at a glance

All six lie at addresses that are 2 mod 4, so none is reachable by
`Setexc(num, -1)` (EmuTOS `bios/bios.c:1070-1080` reads `4 * num`; NewTOS
`tos-api/src/bios/vectors.rs:155@cell`). A probe reads them through a `Supexec`
callback (see section 8).

| Cell | Address | Width | EmuTOS boot value | EmuTOS writer | Changes |
|---|---|---|---|---|---|
| `_timer_ms` | `$442` | WORD | 20 | `bios/mfp.c:196` (`init_system_timer`) | never after boot (no other store in the tree) |
| `_vbclock` | `$462` | LONG | 0 (first-boot clear), then counts | `bios/vectors.S:348` (`addq.l #1,_vbclock.w`, inside `_int_vbl`) | +1 per VBL **that runs the VBL body** |
| `_frclock` | `$466` | LONG | 0, then counts | `bios/vectors.S:330` (`addq.l #1,_frclock.w`, first insn of `_int_vbl`) | +1 per VBL interrupt, always |
| `_dump_vec` | `$502` | LONG | `just_rts` | `bios/chardev.c:84` (`chardev_init`) | only a program's own write |
| `_aux_stat` | `$50E` | LONG | `just_rts` | `bios/chardev.c:78` | as `dump_vec` |
| `_aux_vec` | `$512` | LONG | `just_rts` | `bios/chardev.c:79` | as `dump_vec` |

Addresses and names: `references/emutos/tosvars.ld:49,61,62,93,96,97`; oracle
map `references/emutos-oracle/1a7484ab/emutos.map:229,241,242,273,276,277`
(`_timer_ms`, `_vbclock`, `_frclock`, `_dump_vec`, `_aux_stat`, `_aux_vec`). C types: `WORD timer_ms` `include/tosvars.h:51`,
`volatile LONG frclock` `:65`, the three vectors `void (*)(void)` `:90-92`;
`vbclock` has no C declaration (assembly-only, `bios/vectors.S:348`).
Widths follow the `tosvars.ld` comments and the `addq.l`/`move.w` operand sizes
(`bios/vectors.S:330,348,541`).

Boot clear: the whole `sysvars_start..sysvars_end` range is zeroed on first
boot (`bios/memory.S:566-569`), so `_frclock`, `_vbclock` and `_timer_ms` are 0
until their writers run (cited). `chardev_init` runs from `bios_init`
(`deps chardev_init --up`: `bios_init -> chardev_init`, section 9).

## 2. The VBL path: who counts, when, by how much

`_int_vbl` (`bios/vectors.S:329-...`, installed `VEC_VBL = int_vbl` at
`bios/screen.c:500`):

1. `addq.l #1, _frclock.w` (`:330`): **every** VBL interrupt, unconditionally.
2. `subq.w #1, _vblsem.w` (`:338`; the ColdFire variant `:333-336`); if the result is negative (`jmi vbl_end`,
   `:339`) the rest is skipped. `vbl_end` restores `_vblsem` with
   `addq.w #1` (`:486-489` region, `:489` the add; cited in the contract table
   for `_vblsem`).
3. Only when `_vblsem` was > 0 on entry: `addq.l #1, _vbclock.w` (`:348`),
   then `detect_monitor_change`, `blink`, palette/screen-base load,
   `flopvbl`, the vblqueue loop, and the screen-dump check (`:467-472`).

So `_vbclock` counts the VBLs whose body ran; `_frclock` counts all VBLs.
`_frclock - _vbclock` = the number of VBL interrupts that found `_vblsem <= 0`
(cited from the code; the arithmetic is mine). The boot sequence holds
`_vblsem` at 0 from `bios/screen.c:501` (before interrupts are enabled,
`bios/bios.c:396` `set_sr(0x2300)`) until `bios/bios.c:553` (`vblsem = 1`), and
`Setscreen`'s mode change holds it 0 around `linea_init`/`vt52_init`
(`bios/screen.c:1158-1163`). Consequence (inferred): on the oracle after boot
`_frclock` exceeds `_vbclock` by a constant that is not 0 and grows with each
`Setscreen` mode switch and with any program that writes `_vblsem = 0`.
The comment at `bios/bios.c:389` says the same of VBL work ("will not be
enabled until later, when the vblsem semaphore is set").

`_timer_ms` is not a counter: Timer C's handler pushes it as the WORD argument
to the `etv_timer` chain (`bios/vectors.S:541-544`), every 4th Timer C
interrupt (`timer_c_sieve`, `:521`; enabled `bios/bios.c:463`). `Tickcal`
answers the constant 20 (`bios/bios.c:1096-1099`, comment "system timer is 50 Hz
so 20 ms is the period").

The three vectors are called by EmuTOS only for `dump_vec`:
`scrdmp` (`bios/xbios.c:490-494`, `protect_v((PFLONG)dump_vec)` then
`dumpflg = -1`) and the VBL (`bios/vectors.S:467-472`: `tst.w _dumpflg`, `jsr
(a0)` of `_dump_vec`, then `dumpflg = -1`). `aux_stat`/`aux_vec` have **no**
reader in the tree (word-boundary grep over `bios/`, `bdos/`, `vdi/`, `aes/`,
`util/` found only the stores at `chardev.c:78-79`); `just_rts` is a bare `rts`
(`bios/aciavecs.S:377`, in the oracle at `0x00e20ea4`, map `:1004`, `_just_rts`).

## 3. Readers in EmuTOS

| Cell | Reader | What it does |
|---|---|---|
| `_frclock` | `vsync()` `bios/screen.c:1205-1228` (lines `:1217-1218`) | snapshot `a = frclock`; loop until `frclock != a` (with `set_sr(0x2300)` first, restored after). Called by XBIOS `Vsync` (`bios/xbios.c:753-760`, `VEC(xbios_25, vsync)` `:1340`) and internally (`screen.c:192,572,577,1149,1322`) |
| `_frclock` | `flopvbl` `bios/floppy.c:1351,1359` | `frclock & 7` gates the 8th-VBL floppy work; `& 8` picks the drive. Skipped when `nflops == 0` (`:1343`); no floppy on NewTOS (`docs/COMPATIBILITY.md` floppy group) |
| `_vbclock` | none | write-only in the tree |
| `_timer_ms` | `bios/vectors.S:541` | argument to `etv_timer` |
| `_dump_vec` | `bios/xbios.c:492`, `bios/vectors.S:469` | called, section 2 |
| `_aux_stat`, `_aux_vec` | none | never called |

Programs: none of the searched trees reads any of the six
(`docs/api/bios/_sysvars-contract.md` section 7; `timer_ms` in
`programs/newtos-gem/src/aes.rs:252@timer_ms` is the AES `MU_TIMER` field). The
gitignored upstream-source `Applications` tree was not searched (as in BIOS19).

## 4. NewTOS today

| Cell | Value a process reads | Writer / reader on NewTOS |
|---|---|---|
| `_timer_ms` | 0 | none. `Tickcal` answers 20 from a constant, `tos-api/src/bios/mod.rs:95@TICKCAL_MS` |
| `_vbclock`, `_frclock` | 0, never change | none. No VBL interrupt; `kernel/src/vbl.rs:123@run_if_due` runs only `vb_draw` and touches no page-0 cell (`docs/analysis/deps/mapping.tsv:996`). `Vsync` spins on the host frame clock, `tos-api/src/xbios/system.rs:33@vsync` -> `tos-api/src/host.rs:181@frame_clock` -> `kernel/src/host_env.rs:262@frame_clock` (`CNTPCT * 60 / CNTFRQ`, absolute since the generic counter started, not since boot of NewTOS) |
| `_dump_vec` | 0 | none; `Scrdmp` does not call it, `tos-api/src/xbios/system.rs:56@scrdmp`. Watched: `tos-api/src/bdos/process/hooks.rs:294@DumpVec` compares the cell with the **default 0** and warns on any other value (ADR-031 row `dump_vec`) |
| `_aux_stat`, `_aux_vec` | 0 | none; deliberately not in `PAGE0_HOOKS` (`tos-api/src/bdos/process/hooks.rs:754@silent` test list) |
| `_prt_stat`, `_prt_vec` (the model) | `RET_STUB_VA` | `tos-api/src/bdos/process/loader.rs:1303@RET_STUB_VA` and `:1304@RET_STUB_VA`, inside `populate_sysvars` (`loader.rs:1149@populate_sysvars`), the one place where they get the stub. `RET_STUB_VA = USER_VA_BASE + KBDVEC_STUB_AREA_VA` (`tos-api/src/xbios/kbdvecs.rs:90@RET_STUB_VA`): a single `ret` in the stub page every process maps execute-only. The same stub is `JUST_RTS` of the shared vector cells (`tos-api/src/bios/vectors.rs:71@JUST_RTS`) |

Page 0 of each process is its own copy (`tos-api/src/bdos/process/loader.rs:671@fill`
zeroes it, `populate_sysvars` seeds it, once per process). The writers of a
page-0 cell are the list of `_sysvars-contract.md` section 3; none of them
touches the six cells.

## 5. What a process sees across a process switch

Cited facts about `_hz_200`, the only live counter today:

* The 200 Hz IRQ writes the new value only into the **running** process's page
  0, through its kernel alias (`tos-api/src/bios/tick.rs:108@publish`, called by
  `tos-api/src/bios/tick.rs:78@advance`, called by
  `kernel/src/drivers/timer.rs:171@advance`); with no running process nothing is
  written (`publish`, early `return`).
* `set_current` only stores `RUN` (and rewrites the `os_run` cell of an
  accessory): it does not refresh any counter (`tos-api/src/bdos/process.rs:977@set_current`).
* So a process switched out keeps its last value, and after it runs again it
  sees that old value until the next 200 Hz publish (at most one 5 ms period
  later; the period is 5 ms by `tos-api/src/bios/tick.rs:61@TICK_HZ`).
  Per-process value never decreases (inferred: the source is monotonic and each
  page is only written forward).
* A new process is seeded from the live counter at load
  (`tos-api/src/bdos/process/loader.rs:1262@HZ_200_ADDR`).

For `_frclock`/`_vbclock` the source **leaves a choice** (NewTOS has no
EmuTOS-side behaviour to copy: EmuTOS has one global page 0). The options, not
chosen:

* **(P1) Running process only**, as `_hz_200`: simplest, no new writer
  beyond the one in the IRQ. A resumed process may read a value up to one
  publish period old.
* **(P2) Every loaded slot**: the loop of `store_shared`
  (`tos-api/src/bios/vectors.rs:170@store_shared`, `page_bytes_mut` over
  `MAX_PROCESSES`) applied to a counter. A resumed process sees the current
  value at once; cost: a loop in an interrupt path (`tick::publish` is
  documented "must stay interrupt-context clean (no locks, no allocation, no
  I/O)", `tos-api/src/bios/tick.rs:78@advance`).
* **(P3) Refresh at switch-in**: in `set_current`
  (`tos-api/src/bdos/process.rs:977@set_current`), as it already rewrites one cell
  for accessories.

A second, independent gap that applies under every option (inferred): the IRQ
publish lags the true frame boundary by up to one 5 ms period. `EmuTOS`
`Vsync` returns exactly after `frclock` changed, so a program that does
`Vsync(); x = *_frclock;` sees the new value (`bios/screen.c:1217-1218`). On
NewTOS `Vsync` returns when `frame_clock()` changes
(`tos-api/src/xbios/system.rs:33@vsync`), possibly before the next publish, so
the program could read the previous frame number. Satisfying the EmuTOS reading
needs `vsync` itself to publish before it returns (or the cell to be computed
there). To be decided with the developer; flagged here because it is observable.

## 6. The rate on NewTOS and the clock that gives it: decision for the owner

EmuTOS' `_frclock` rate is the display's vertical refresh, a device parameter
(`docs/COMPATIBILITY.md` Q5 text: "depending on the monitor's frame rate",
`:1408-1411`; the Hatari oracle rate for the TT profile is **not measured here**,
needs a measurement). NewTOS has no VBL
(`docs/api/bios/_sysvars-contract.md` row `_vblsem`). What exists:

| Candidate | Rate | Where | Fit |
|---|---|---|---|
| C1 frame clock | ~60 Hz (`CNTPCT * 60 / CNTFRQ`) | `kernel/src/host_env.rs:262@frame_clock`, trait `tos-api/src/host.rs:181@frame_clock` | the clock `Vsync` already waits on (`tos-api/src/xbios/system.rs:33@vsync`) and `vb_draw` is paced by (`kernel/src/vbl.rs:123@run_if_due`); counter and `Vsync` agree. It is absolute, not zero at NewTOS boot (an offset would be a further choice) |
| C2 `_hz_200 / 4` | 50 Hz | `tos-api/src/bios/tick.rs:61@TICK_HZ`, `advance` `:78@advance` | agrees with `Tickcal` 20 ms and `_timer_ms`, disagrees with the 60 Hz `Vsync` |
| C3 `monotonic_millis` scaled | any | `kernel/src/host_env.rs:239@monotonic_millis` | no existing consumer; a third clock |

**Decision for the owner (not made here):** which rate `_frclock`/`_vbclock` have
(the researcher's reading: C1 is the one under which a program that polls the
cell and a program that calls `Vsync` agree; C2 is the one under which the
cells agree with `Tickcal`; both are inferences), whether they start at 0 at NewTOS
boot or carry the absolute frame number, and whether `_vbclock` follows `_vblsem`
(next point).

`_vblsem` and `_vbclock`: EmuTOS' only difference between the two counters is
the semaphore (section 2). On NewTOS `_vblsem` is a "missing" cell reading 0
(`_sysvars-contract.md` row `_vblsem`; **not in this ticket**). Options:
(V1) `_vbclock == _frclock` always, `_vblsem` ignored; (V2) `_vbclock` advances
only while the running process's `_vblsem` cell is > 0, which needs `_vblsem`
served first (EmuTOS boot value 1, `bios/bios.c:553`) — a different ticket.
The oracle shows `_frclock != _vbclock` after boot (inferred, section 2); under
V1 NewTOS shows equality: a measured difference to list in
`docs/DEVIATIONS.md` (owner's file; not edited).

`_timer_ms`: the value is the one `Tickcal` answers, 20
(`tos-api/src/bios/mod.rs:95@TICKCAL_MS`; EmuTOS `bios/mfp.c:196`,
`bios/bios.c:1096-1099`). Static, written at load next to the other static
cells in `populate_sysvars`; it does not change over a process switch (every
process has its own copy, seeded 20; a program's own write is not seen by
another process: inferred from the per-process page 0).

## 7. The three vectors: value, the stub, the ADR

Contract: `_dump_vec`, `_aux_stat`, `_aux_vec` read as the same stub value
`_prt_stat`/`_prt_vec` read (`RET_STUB_VA`,
`tos-api/src/bdos/process/loader.rs:1303@RET_STUB_VA`), written in the same place
(`populate_sysvars`). Nothing in EmuTOS ever calls `aux_*`; `dump_vec` is
called by `Scrdmp` and the VBL, and NewTOS calls it from neither (ADR-031 row
`dump_vec`, read only here). A program that chains to the old value reaches a
`ret`.

Consequences for the developer that the handover did not state (cited):

1. **The `DumpVec` watcher must follow.** `check_page0` warns when a hook cell
   differs from its default (`tos-api/src/bdos/process/hooks.rs:319@check_page0`,
   `:330-331`) and the `DumpVec` default is 0
   (`tos-api/src/bdos/process/hooks.rs:294@DumpVec`). With the stub in the cell,
   every process would warn at its first super-state exit unless the default
   becomes the stub (`prt_*` and `aux_*` have no watcher by design,
   `hooks.rs:754@silent`).
2. **ADR-031 has to change** (owner): its table row `dump_vec, swv_vec,
   exec_os, resvector/resvalid, vblqueue` says "not served, warning" and the row
   `prt_stat, prt_vec` is the only stub row; rule 1 lists `aux_stat`/`aux_vec`
   among the hooks EmuTOS never calls (read `docs/MEMORY.md:3127` onwards, not
   edited). A stub for `dump_vec` and `aux_*` is the amendment; `dump_vec` is still
   not *called* by NewTOS, so "served as a cell" in the sense of
   `_sysvars-contract.md` section 2, not "served as a hook".
3. `tos-api/src/xbios/system.rs:56@scrdmp` and the comment at
   `tos-api/src/vdi/escape.rs:850@dump_vec` say "`dump_vec` not called"; both stay true.

## 8. Measurable cases for the tester

A new `peek` case type is needed for every row below (all six addresses are 2 mod
4, section 1); `_sysvars-contract.md` section 8 proposes it and it is not built.
`tosdump/probes/hkprobe.c:93@prt_stat_v` already reads `CELL(0x506)` through `Supexec`
and calls through it, the pattern to copy. Alternatively one probe program under
`tosdump/probes/` doing the same, as `hkprobe` does for `prt_*`.

| Case | Oracle (EmuTOS, source-predicted) | NewTOS today | NewTOS after the fix |
|---|---|---|---|
| read `$442` WORD | 20 | 0 | 20 |
| read `$502`, `$50E`, `$512` LONG | all non-zero and equal to each other and to `$506`/`$50A` (`just_rts`, `0x00e20ea4` in the oracle map: the value differs by machine, so compare nonzero/equal, not the number) | 0 | non-zero, equal to `$506` |
| call through each of the three | returns (`rts`) | branch to 0: fault | returns |
| `a = *$466; Vsync(); b = *$466` | `b - a >= 1` (inferred: exactly 1 at a steady VBL) | `b == a` and every read 0 | `b - a >= 1`, which is the §5 lag case |
| `h0 = $4BA, f0 = $466; wait N Vsync; h1, f1` | `(f1-f0) >= N`; the ratio `(h1-h0)/(f1-f0)` is `200 / VBL rate` | 0 | ratio equals the owner's rate (section 6) |
| `*$462` against `*$466` after boot | `$466 - $462 != 0` (inferred, section 2) and constant between reads when `_vblsem == 1` | both 0 | equal under V1: a documented difference |
| a process switch: parent reads `$466`, a child runs `Vsync` three times, parent reads again | one global counter: strictly larger | n/a | per option P1/P2/P3 (section 5) |

**What the pinned oracle cannot show:** (a) the per-process view across a switch
(EmuTOS has one global page 0 and no preemption of a process by another reading
its own copy); (b) the NewTOS rate choice (the oracle's rate is Hatari's video
mode; not measured); (c) the stub's *address* (ROM address vs NewTOS stub-page
address: only "non-zero, equal, callable" compares); (d) the 5 ms publish lag
(Hatari updates the cell inside the VBL interrupt). The oracle can show all six
cells at a peek, subject to the Hatari-side measurement of the rate. The
expected values above are source-predicted, not measured.

## 9. Localization for the developer

`cargo xtask deps <fn> --up/--down` run in this worktree (default kinds
`asm,direct,layer,table,vector`):

| EmuTOS function | `--down` | `--up` | mapping row (`docs/analysis/deps/mapping.tsv`) |
|---|---|---|---|
| `chardev_init` (`bios/chardev.c`) | 0 callees | `bios_init`, `biosmain` (**unmapped**), `memdone` (design) | `:356` -> `loader.rs@populate_sysvars`, `mapped`, note: "aux_stat/aux_vec stay 0" (this note is what the fix changes, with `dump_vec`) |
| `init_system_timer` (`bios/mfp.c`) | `xbtimer`, `mfpint` (mapped, `xbios/mfp.rs`), `setup_timer`, `jdisint`, `jenabint`, ... (design) | `bios_init`, `biosmain` (**unmapped**), `memdone` (design) | `:709` `design:hardware:mfp68901`: the `timer_ms = 20` store has **no row**; a developer adds one pointing at `populate_sysvars` |
| `tickcal` | 0 | 10 functions, all mapped (`biostrap`, VDI/AES callers) | `:314` -> `bios/mod.rs@dispatch`, a constant |
| `vsync` | `stop_until_interrupt` (ext, unmapped, not a touched layer) | 126 functions, `bios_init`, `biosmain` unmapped in bios | `:842` -> `xbios/system.rs@vsync` |
| `scrdmp` | `just_rts` (`via dump_vec`, ext), `protect_v` (ext) | `escfn17`, `xbiostrap`, ... all mapped | `:1088` -> `xbios/system.rs@scrdmp`, "dump_vec not called" |
| `int_vbl` (`bios/vectors.S`) | `blink` (mapped), `detect_monitor_change`, `flopvbl` (design), `just_rts` (ext), `invert_cell`, `cell_addr` (**unmapped**, `bios/conout.c`) | 1, mapped | `:996` -> `vbl.rs@run_if_due`, note: "frclock/vbclock ... no counterpart (ADR-031)" (the fix changes this note) |
| `int_timerc` | `kb_timerc_int`, `handle_mouse_mode`, `convert_scancode` (**unmapped**, `bios/ikbd.c`) and more | 1, mapped | `:994` -> `bios/tick.rs@advance`, "the 200 Hz counter" |
| `flopvbl` | 5 floppy/PSG functions (design) | `int_vbl` | `:559` `design:hardware:floppy` |

**Unmapped entries in the touched layers (all `bios`):** `bios_init`, `biosmain`
(both via `chardev_init`/`init_system_timer`; `bios_init` has two *mapped*
rows already, `mapping.tsv:296-297`, but `deps` lists it unmapped because its
`-` row comes first: a classification of the tool, not read further), `cell_addr`,
`invert_cell`, `kb_timerc_int`, `handle_mouse_mode`, `convert_scancode`. None of
them is touched by the six cells; they are listed because the ticket asked
for every unmapped entry.

Rows the developer adds or amends (ratchet): `chardev_init` (`:356`, note),
`int_vbl` (`:996`, note), `init_system_timer` (`timer_ms = 20` -> `populate_sysvars`),
and one for the VBL counters' NewTOS writer. NewTOS places that the developer
touches: `tos-api/src/bdos/process/loader.rs:1303@RET_STUB_VA` (the three
vectors and `_timer_ms`), `tos-api/src/bios/tick.rs:108@publish` or a sibling
(the counters), `tos-api/src/bdos/process/hooks.rs:294@DumpVec` (default),
`tos-api/src/xbios/system.rs:33@vsync` (publish before return, section 5), and
the tests `tos-api/src/bdos/process/loader.rs:2030@prt_stat` (`prt_stat`/`prt_vec` assertions, the model).

## 10. Corrections

None to existing spec entries: `_sysvars-contract.md` rows for these cells are
correct against the source (`_aux_*`: stub vs 0, `_dump_vec`: value 0 versus the
EmuTOS `just_rts`, `_timer_ms` 20, `_frclock`/`_vbclock` writers). One cite
repair for the record: the row `_vbclock` of that table names `bios/vectors.S:348` as
the writer and `no C reader`, which holds; `_boot-init.md` section 7c
(`vbclock` "keins gefunden") stays wrong as noted in BIOS19 section 6.1.

## Open points

- STOP (owner decision): the rate and base of `_frclock`/`_vbclock` (C1/C2/C3,
  absolute or 0 at boot) and whether `_vbclock` follows `_vblsem` (V1/V2).
- STOP (owner decision): the per-process view of the counters (P1/P2/P3). The
  source (one global page 0 on EmuTOS) leaves the choice.
- STOP (owner): ADR-031 row `dump_vec` and the stub row must be amended before
  the stub for `dump_vec` and `aux_*` is built; `hooks.rs:294@DumpVec` default
  follows.
- Needs deeper analysis: the Hatari VBL rate for the oracle's TT profile and
  the size of `_frclock - _vbclock` after boot; not read from source, to be
  measured (the oracle column of section 8 is a prediction).
- Inferred, not read: `_frclock - _vbclock` constant > 0 on the oracle; the
  5 ms `Vsync` publish lag; per-process monotonicity; that no process sees
  another's write to `_timer_ms`; `bios_init` listed unmapped by `deps` despite
  two rows in `mapping.tsv:296-297`.
- Not searched: the gitignored upstream-source `Applications` tree, for programs that read the
  six cells (same gap as BIOS19).
- Out of scope: `_vblsem`, `_nvbls`, `_vblqueue` (named in the ticket as not
  in BIOS20); the `bcon*_vec` tables (BIOS19 section 6.2).
