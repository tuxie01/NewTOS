# System variables `_timer_ms`, `_v_bas_ad`, `_vblsem`, `_nvbls`, `_vbclock`, `_frclock`, `_hdv_init`, `_hdv_boot`, `_themd`, `_dskbufp`, `dump_vec`, `aux_stat`, `aux_vec`, `_pun_ptr`: contract [BIOS20]

> Researcher output for TASK/BIOS20: first pass (the six cells `dump_vec`,
> `aux_stat`, `aux_vec`, `_frclock`, `_vbclock`, `_timer_ms`, landed as
> `567d3b9b4`), second pass (the eight cells the owner added on 2026-11-20:
> `_v_bas_ad`, `_vblsem`, `_nvbls`, `_hdv_init`, `_hdv_boot`, `_themd`,
> `_dskbufp`, `_pun_ptr`, landed as `a2cd6a7a7`) and **third pass: the spec
> restated as the owner decided it** (questions Q1 to Q8 answered on
> 2026-11-20, ADR-031 amended the same day; TASK/BIOS20.md, sections "Spec gate
> and owner decisions", "Second spec pass and owner answers", "ADR-031
> amended"). Thirteen cells are served; `_pun_ptr` leaves this ticket and stays
> 0 (TASK/BIOS18.md). File beside `docs/api/bios/_sysvars-contract.md` (the
> 84-row table; its rows for these cells were corrected in the second pass,
> section 16, and in the third pass where they contradicted the decision).
> Source: EmuTOS `1a7484ab` (`references/emutos`); oracle values from
> `references/emutos-oracle/1a7484ab`. Nothing here was measured (no QEMU, no
> Hatari, by the handover). "Cited" = read in this run; "inferred" is marked.
> Sections 1-4 are the first pass (EmuTOS facts); sections 5-13 are the contract
> per cell as decided (their former "option" and "question" passages now read as
> the decision); section 14 the tester cases, 15 the localization, 16 the
> corrections; **section 17 is the decided contract in one place: the values, the
> developer checklist and the tester's cases.** The section numbers are kept.

> **Contract vs. reference** (`docs/COMPATIBILITY.md`, "System variables"): the
> value and meaning of a cell a program can read is contract; which EmuTOS
> routine writes it is reference.

## 1. The six first-pass cells at a glance

All six lie at addresses that are 2 mod 4, so none is reachable by
`Setexc(num, -1)` (EmuTOS `bios/bios.c:1070-1080` reads `4 * num`; NewTOS
`tos-api/src/bios/vectors.rs:155@cell`). A probe reads them through a `Supexec`
callback (see section 14).

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
(`deps chardev_init --up`: `bios_init -> chardev_init`, section 15).

## 2. The VBL path: who counts, when, by how much

`_int_vbl` (`bios/vectors.S:329-...`, installed `VEC_VBL = int_vbl` at
`bios/screen.c:500`; that store and the `vblsem = 0` at `:501` lie inside
`#if CONF_WITH_ATARI_VIDEO`, `bios/screen.c:487-501`, a gating the second pass
added; the `vblsem = 1` at `bios/bios.c:553` is unconditional):

1. `addq.l #1, _frclock.w` (`:330`): **every** VBL interrupt, unconditionally.
2. `subq.w #1, _vblsem.w` (`:338`; the ColdFire variant `:333-336`); if the result is negative (`jmi vbl_end`,
   `:339`) the rest is skipped. `vbl_end` restores `_vblsem` with
   `addq.w #1` (`:486-489` region, `:489` the add).
3. Only when `_vblsem` was > 0 on entry: `addq.l #1, _vbclock.w` (`:348`),
   then `detect_monitor_change`, `blink`, palette/screen-base load,
   `flopvbl`, the vblqueue loop (`:440-447`), and the screen-dump check
   (`:467-472`).

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

## 4. NewTOS today (the six first-pass cells)

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

## 5. What a process sees across a process switch: decided P1 with P3 and `Vsync` publishing

Owner decision 2026-11-20 (TASK/BIOS20.md, "Spec gate and owner decisions"):
the counters follow `_hz_200`'s model (P1), a switched-in process is refreshed
at once (P3), and `Vsync` publishes before it returns. Cited facts about
`_hz_200`, the only live counter today:

* The 200 Hz IRQ writes the new value only into the **running** process's page
  0, through its kernel alias (`tos-api/src/bios/tick.rs:108@publish`, called by
  `tos-api/src/bios/tick.rs:78@advance`, called by
  `kernel/src/drivers/timer.rs:171@advance`); with no running process nothing is
  written (`publish`, early `return`).
* `set_current` stores `RUN` and rewrites the `os_run` cell of an accessory; it
  refreshes no counter (`tos-api/src/bdos/process.rs:977@set_current`).
* A process switched out keeps its last value; the period is 5 ms
  (`tos-api/src/bios/tick.rs:61@TICK_HZ`). A new process is seeded from the live
  counter at load (`tos-api/src/bdos/process/loader.rs:1262@HZ_200_ADDR`).

Decided behaviour for `_frclock` and `_vbclock` (the rule; the code is the
developer's):

1. **Running process (P1).** The place that publishes `_hz_200`
   (`tick.rs:108@publish`, through `tos-api/src/bdos/process.rs:1084@current_page0_kva`)
   or a sibling in `tick.rs` also stores the two counters into the running
   process's page 0. Nothing is written when no process runs (EL1 code), as for
   `_hz_200`.
2. **Switch-in (P3).** `set_current`
   (`tos-api/src/bdos/process.rs:977@set_current`) refreshes the counters of the
   process it makes current, for **every** process, not only accessories
   (today it only rewrites `os_run` for an accessory). A resumed process then
   never reads a value up to 5 ms old. The same refresh point serves the cells of
   sections 8 and 9 that follow a global value.
3. **`Vsync` publishes (`Vsync(); x = *_frclock;`).** EmuTOS' `vsync` returns
   when `frclock` has changed (`bios/screen.c:1217-1218`), so the program reads
   the new frame number. NewTOS `vsync` returns when `frame_clock()` changes
   (`tos-api/src/xbios/system.rs:33@vsync`), which can precede the next
   200 Hz publish by up to 5 ms. The rule: `vsync` writes the current counters
   into the calling process's page 0 before it returns. A `Vsync` from code
   without a running process (EL1) publishes nothing. (Inferred: the up-to-5 ms
   lag follows from the 200 Hz publish period and the frame clock being derived
   from `CNTPCT`, not from an interrupt.)

Residual, observable difference (inferred): a program that polls `*_frclock`
without `Vsync` sees a frame boundary up to 5 ms late (the publish granularity),
where EmuTOS' VBL interrupt writes it at the boundary.

## 6. The rate, the start value and `_vbclock`: decided C1 with start 0, V2

Owner decisions 2026-11-20: **C1**, the 60 Hz frame clock `Vsync` waits on
(`kernel/src/host_env.rs:262@frame_clock`, trait
`tos-api/src/host.rs:181@frame_clock`; `CNTPCT * 60 / CNTFRQ`); the cells
**start at 0 at boot**; **V2** for `_vbclock`.

* **Start at 0.** `frame_clock` is absolute (it counts from the generic
  counter's start, not from NewTOS boot). The value a process reads is
  `frame_clock() - frame_base`, with `frame_base` the frame number recorded once
  at boot. No such object exists today (read: `frame_clock` is the only source,
  `host_env.rs:262@frame_clock`); the developer adds one. `Vsync` itself keeps
  comparing raw `frame_clock` values.
* **EmuTOS reference for "0 at boot".** Both cells are 0 at the first-boot clear
  (`bios/memory.S:566-569`); on EmuTOS they are far from 0 when the first
  program runs, because they count from the first VBL (`bios/screen.c:500`
  onward; inferred). "Start at 0" is the owner's choice for NewTOS, not an
  EmuTOS value to compare: a tester compares differences, never absolutes.
* **V2: `_vbclock` advances only while `_vblsem` is greater than 0.** EmuTOS
  skips the body, including `addq.l #1,_vbclock`, when `subq.w #1,_vblsem` goes
  negative, i.e. when `_vblsem <= 0` on entry (`bios/vectors.S:338-339`, count at
  `:348`); `_frclock` counts unconditionally (`:330`). On NewTOS, per frame:
  `_frclock` + 1 always; `_vbclock` + 1 only if the process's own `_vblsem`
  cell is > 0 at that frame (boot value 1, section 9). Consequences:
  1. The kernel **reads** `_vblsem` from the running process's page 0 in the
     timer interrupt (and in `Vsync`, section 5): a signed WORD, compared with
     0 only, "at or below 0" meaning down (`bios/vectors.S:338-339` tests the
     sign after `subq.w`). Rule S10 of `docs/api/_super-state.md` lists the
     cell (`0x452`) as a documented input beside `conterm` and the reset-entry
     magics; the ADR-031 amendment of 2026-11-20 (`docs/MEMORY.md`, item 4) is
     the authority and is an exception to its rule 4 ("for warnings only").
  2. **`_vbclock` accounting, Q1 option (c).** `_vbclock = _frclock - lost`, with
     `lost` the number of frames the process ran with its `_vblsem` at or below
     0. The per-process word `lost` lives in the process record (kernel
     private, not a page-0 cell; the placement is the developer's), one global
     word `acct_fc` (the frame number up to which the running process has been
     accounted) beside it. At every publish (timer interrupt, `Vsync`,
     switch-in), with `fc = frame_clock() - frame_base`: first, if the running
     process's `_vblsem <= 0`, `lost += fc - acct_fc`; then `acct_fc = fc`, and
     the two counters are written (`_frclock = fc`, `_vbclock = fc - lost`).
     At **switch-in** `acct_fc = fc` is set before the counters are written, so
     the frames the process spent away are never added to `lost`: a process
     that was away loses nothing. Frames between two publishes are charged by
     the `_vblsem` seen at the later publish (granularity of the publish
     period, inferred; the same 5 ms residual as section 5). A new process
     starts with `lost = 0`.
  3. **`vb_draw` gate, Q2 option (b).** The frame tick runs `vb_draw` once per
     frame (`kernel/src/vbl.rs:123@run_if_due`; the frame test is the
     `LAST_FRAME` swap, then `vb_draw`). The gate sits between them: when a
     process runs and its `_vblsem` (read through
     `tos-api/src/bdos/process.rs:1084@current_page0_kva`) is at or below 0,
     the frame is skipped, no `vb_draw`. The frame stays consumed (a skipped
     frame is not redone later), as the EmuTOS VBL skips its body for that
     interrupt (`bios/vectors.S:339`). With no running process (EL1) the gate
     does not apply and `vb_draw` runs. Cursor blink, palette and screen-base
     load, `flopvbl` and the vblqueue are not NewTOS frame work and are not
     gated (they do not exist here). These are the two kernel readers of the
     cell; there is no third.

`_timer_ms`: the value is the one `Tickcal` answers, 20
(`tos-api/src/bios/mod.rs:95@TICKCAL_MS`; EmuTOS `bios/mfp.c:196`,
`bios/bios.c:1096-1099`). Static, written at load in `populate_sysvars` next to
the other static cells; every process has its own copy seeded 20 (inferred from
the per-process page 0).

## 7. The three vectors: decided stub, the `DumpVec` default follows

Owner decision 2026-11-20: `_dump_vec`, `_aux_stat`, `_aux_vec` hold the same
stub as `_prt_stat`/`_prt_vec` (`RET_STUB_VA`,
`tos-api/src/bdos/process/loader.rs:1303@RET_STUB_VA`), written in the same
place (`populate_sysvars`, `loader.rs:1149@populate_sysvars`; three more
`wr_long` at `$502`, `$50E`, `$512`). NewTOS still does not **call**
`dump_vec` (`Scrdmp`, `tos-api/src/xbios/system.rs:56@scrdmp`; comment
`tos-api/src/vdi/escape.rs:850@dump_vec`); EmuTOS never calls `aux_*`
(section 2). A program that chains to the old value reaches a `ret`.

Consequences for the developer (cited):

1. **The `DumpVec` watcher compares against the stub.** `check_page0` warns when
   a hook cell differs from its default
   (`tos-api/src/bdos/process/hooks.rs:319@check_page0`), and the default of
   `DumpVec` is 0 (`tos-api/src/bdos/process/hooks.rs:294@DumpVec`); with the
   stub in the cell the default becomes `RET_STUB_VA as u32` (decided: "the
   `DumpVec` watcher compares against the stub"). `aux_*` and `prt_*` have no
   watcher by design (`tos-api/src/bdos/process/hooks.rs:754@silent`; its test
   list names `$50E`/`$512`, unchanged).
2. **ADR-031 amendment (entered 2026-11-20, `docs/MEMORY.md`, ADR-031
   "Amendment (2026-11-20", items 1-2).** `dump_vec`, `aux_stat`, `aux_vec` and
   `hdv_init` hold the `just_rts` stub by default, `hdv_boot` a stub with a
   defined code (section 10); the `DumpVec` warning fires when the cell differs
   from the stub. NewTOS still calls none of them. `aux_*`/`hdv_*` keep no
   watcher (rule 1).
3. `tos-api/src/xbios/system.rs:56@scrdmp` and
   `tos-api/src/vdi/escape.rs:850@dump_vec` say "`dump_vec` not called"; both
   stay true.

## 8. `_v_bas_ad` (`$44E`, LONG): the logical screen base

**Cell.** Address `$44E` (`tosvars.ld:55`, oracle map `:235`), LONG, type
`UBYTE *v_bas_ad` (`include/tosvars.h:42`). 2 mod 4: no `Setexc` read.

**EmuTOS boot value.** Not a constant: `screen_init_address` stores
`screen_start` (`bios/screen.c:636-657`, the store at `:656`); with
`CONF_VRAM_ADDRESS` 0 (`include/config.h:1101-1102`) that is
`balloc_stram(vram_size, TRUE)` (`bios/screen.c:647`), which moves `memtop` down
and returns it (`bios/biosmem.c:112-115`): the top of the ST-RAM, just below the
final `_memtop` (inferred: equality with the page-0 `_memtop` holds only if no
later top allocation follows). Called from `bios_init`
(`bios/bios.c:338`).

**Writers.** `bios/screen.c:656` (init), `:1107` (`setscreen`, only when
`(LONG)logLoc > 0`, `:1106`; stored before the `rez` checks, so it is set even
when `rez == -1`, `:1114-1116`), `:1139` (`setscreen`, Videl `Srealloc` path,
`CONF_WITH_VIDEL`), `bios/vectors.S:396` (VBL, from `_screenpt`, which no code
writes: `_sysvars-contract.md` row `_screenpt`), `bios/nova.c:725` (a Nova card).
**Readers.** `Logbase` returns it (`bios/screen.c:1071-1073`); VT52
(`bios/conout.c:126,507,712,739`, `bios/vt52.c:930`); the VDI draws through it
(`vdi/vdi_raster.c:927,942`, `vdi/vdi_misc.c:172`, `vdi/vdi_control.c:618`,
`vdi/vdi_fill.c:1175,1187`, `vdi/vdi_textblit.c:1033,1226`,
`vdi/vdi_inline.h:16`); boot-sector code (`util/bootsect.S:77`).

**Follows Setscreen / Physbase / Logbase on EmuTOS?** `Logbase`: it **is** the
cell (`:1073`). `Setscreen(logLoc > 0)`: sets it (`:1106-1108`), also with
`rez == -1`. `Physbase`: **no**: it reads the video registers
(`bios/screen.c:917-935`, `:1042`), which `Setscreen` sets through `setphys`
(`:1058`, `:1110-1112`); the cell and the registers are set together only at
init (`:656-659`) and in the Videl realloc (`:1139-1140`).

**What a program does with it.** Reads it (`*(char **)0x44E`, the classic idiom,
named in `docs/COMPATIBILITY.md` "System variables"). Writes it in super state,
which redirects VT52 and VDI drawing on EmuTOS (readers above); on NewTOS the
draw target is the display's draw pointer, not the cell: a program's write
changes only its own page-0 copy (the cell is output-only, S10; a direct
write to `$44E` is outside the `$008-$407` range that
`tos-api/src/bios/vectors.rs:286@adopt_page0_write` adopts). A documented
difference, not a served write.

**NewTOS object it mirrors.** `ScreenInfo.base`, the logical screen
(`tos-api/src/host.rs:559@base`, doc: "the drawing target, EmuTOS `v_bas_ad`"),
which `Logbase` reports (`tos-api/src/xbios/screen.rs:146@logbase`) and
`Setscreen` moves (`tos-api/src/xbios/screen.rs:290@setscreen` ->
`kernel/src/host_env.rs:307@set_screen_logical`). The cell does not exist today
(page 0 zero).

**What NewTOS puts there to answer as EmuTOS.** The address `Logbase` answers
to this process: the **user** address of `ScreenInfo.base`, i.e. its physical
address (VA = PA for the published region, `tos-api/src/xbios/screen.rs:146@logbase`
and the `caller_view` doc above it).
* **Does a 32-bit cell hold it?** Yes. The framebuffer base is a VideoCore bus
  address masked to the low 30 bits
  (`kernel/src/drivers/bcm2711/framebuffer.rs:64@BUS_TO_PHYS`), with the owner
  decision MEM11 (2026-10-31, comment above it) that `Physbase`/`Logbase` keep it
  a positive `LONG` below 1 GiB; the user pointer range ends at `0x8000_0000`
  (`tos-api/src/bdos/process/usermem.rs:57@USER_PTR_END`). The base is **outside**
  the ST/TT RAM pool (project memory; getmpb measured `membot 0x400000`,
  `memtop 0xE00000`, `docs/api/bios/00-getmpb.md` V2).
* **Is it mapped for EL0?** Not at load. `populate_sysvars` publishes nothing
  of the screen; the whole screen memory (`ScreenInfo.mem_base`,
  `tos-api/src/host.rs:564@mem_base`, `mem_bytes`) is published into a process
  only when that process calls `Physbase`/`Logbase`, as `PROT_G`, uncached
  (`tos-api/src/bdos/process/usermem.rs:233@expose_attr`, from
  `tos-api/src/xbios/screen.rs:146@logbase`). A cell holding the address before
  that call would point at memory the process cannot touch (translation
  fault). The first live uncached holder also switches the VDI to
  clean-and-invalidate per primitive
  (`tos-api/src/bdos/process/usermem.rs:136@uncached_shared_active`): publishing
  at load for **every** process would turn that on permanently.
* **Decided, Q4 option (a).** The cell **always** holds the address `Logbase`
  answers (the user address of `ScreenInfo.base`), also before the process has
  called `Logbase`/`Physbase`, and the screen is not mapped for it. A program
  that dereferences `*(char **)0x44E` before calling `Logbase` or `Physbase`
  takes a translation fault and ends with "Bus Error" (S9-like). No publishing
  at load, so the uncached-shared switch above stays off for such a process.
  (Mapping the screen at the first access was named to the owner as a possible
  later step; not examined here.)
* **Refresh points** (inferred, from section 5's model): at load, after the
  running process's own `Setscreen` moved the logical base, and at switch-in
  (another process may have moved the global draw base). When there is no
  screen at load the cell is not written (stays 0; inferred, not in the
  decision, see Open points).

## 9. `_vblsem` (`$452`, WORD) and `_nvbls` (`$454`, WORD)

**Cells.** `$452` "if > 0, the VBL routine is executed" (`tosvars.ld:56`, oracle
map `:236`; `volatile WORD vblsem`, `include/tosvars.h:63`); `$454` "number of VBL
routines" (`tosvars.ld:57`, map `:237`; `WORD nvbls`, `include/tosvars.h:62`,
"number of slots in the array pointed by vblqueue"). Both 2 mod 4.

**EmuTOS boot values and writers.**
* `_vblsem`: 0 at `bios/screen.c:501` (`screen_init_mode`, inside
  `#if CONF_WITH_ATARI_VIDEO`), **1 at `bios/bios.c:553`** after
  `osinit_after_xmaddalt`, unconditional. `Setscreen` writes 0 then 1 around
  `linea_init`/`vt52_init` (`bios/screen.c:1158,1163`). The VBL handler decrements
  it on entry and increments it at `vbl_end` (`bios/vectors.S:338,489`; ColdFire
  `:334-336,484-487`), so outside the interrupt a program reads **1**; inside the
  VBL body the value is 0 (a vblqueue routine reading it sees 0; inferred from
  `:338`). Nothing else writes it (`bios/` word-boundary grep). The boot
  sequence's own gating is in section 2.
* `_nvbls`: `ARRAY_SIZE(vbl_list)` = **8** (`bios/bios.c:366`;
  `extern PFVOID vbl_list[8]`, `bios/bios.c:206`; `tosvars.ld:86`
  "pointers to 8 VBL routines", oracle map `:266` `_vbl_list = 0x4ce`).
  Reader: the VBL loop (`bios/vectors.S:440`).
* Coupling to `_vblqueue` (`$456`): `vblqueue = vbl_list` (`bios/bios.c:367`),
  the eight entries are set NULL (`:370-372`), and the VDI mouse init stores
  `vb_draw` in slot 0 (`vdi/vdi_mouse.c:602,631`). So at the time a user program
  runs after the VDI is up (inferred: before the desktop starts, an AUTO program
  sees slot 0 still NULL): `_nvbls` = 8, `_vblqueue` = `$4CE`, slot 0 non-NULL,
  slots 1-7 NULL. The VBL loop walks `_nvbls` entries from `_vblqueue` and calls
  each non-NULL one (`bios/vectors.S:440-465`).

**What a program does with them.** `_vblsem`: write 0 to stop VBL work (blink,
`vb_draw`, vblqueue), as `Setscreen` does; read to learn the state.
`_nvbls`/`_vblqueue`: the classic VBL installer scans `_nvbls` slots of
`*_vblqueue` for a NULL one and stores its routine. No searched program does
either (`_sysvars-contract.md` section 7: none reads a "missing" cell).

**NewTOS today.** Both read 0 (no writer). There is no VBL interrupt; the frame
work runs from the timer IRQ and the trap-side pump
(`kernel/src/vbl.rs:123@run_if_due`, mapping row
`docs/analysis/deps/mapping.tsv:996`). `_vblqueue` reads 0 and is watched
(`tos-api/src/bdos/process/hooks.rs:285@Vblqueue`, default 0); `_vbl_list`
(`$4CE`) reads 0. The ADR-031 table: `vblqueue` "not served, warning".

**Decided.** `_vblsem` = 1 at load (`populate_sysvars`), WORD, read by the two
kernel readers of section 6 (the `_vbclock` accounting and the `vb_draw` gate)
and by nothing else. `_nvbls` = 8, WORD (static). **`_vblqueue` (`$456`) holds
`$4CE`** and the eight LONG slots `$4CE`-`$4ED` of `_vbl_list` stay 0 (Q3
option (b); ADR-031 amendment item 3): NewTOS's `vb_draw` is kernel code, so
slot 0 stays 0 too. A program may enter a routine into a slot (in super
state, page 0); NewTOS never calls it and warns (watcher, section 17.3). The
classic installer (scan `_nvbls` slots from `*_vblqueue` for a NULL one) finds
slot 0 free and stores there, instead of reading address 0 as in the
state before this work. EmuTOS differs in slot 0, which holds `vb_draw` after
the VDI is up (`vdi/vdi_mouse.c:602,631`).

## 10. `_hdv_init` (`$46A`, LONG) and `_hdv_boot` (`$47A`, LONG)

**Cells.** `tosvars.ld:63,67` ("vector for hard disk initialization" / "boot"),
oracle map `:243,247`; C types `void (*hdv_init)(void)`, `LONG (*hdv_boot)(void)`
(`include/tosvars.h:104-105`). Both 2 mod 4.

**EmuTOS boot values.** Set once in `blkdev_init`, next to the other three:
`hdv_boot = blkdev_hdv_boot` (`bios/blkdev.c:95`), `hdv_init = flop_hdv_init`
(`:98`), then `hdv_bpb`, `hdv_rw`, `hdv_mediach` (`:101-103`); `blkdev_init` runs
from `bios_init` (`bios/bios.c:503-504`). Oracle values: `hdv_init` =
`0x00e09712` (`_flop_hdv_init`, map `:589`); `hdv_boot` is the static function
`blkdev_hdv_boot` (`bios/blkdev.c:50,258`), **not a map symbol**, so its oracle
address is not in the generated files read here (a measurement would give it).

**Callers.** None through the cells. The comment says so for `hdv_init`
("EmuTOS does not use hdv_init, but initializes it for compatibility",
`bios/blkdev.c:96-97`); the whole-tree grep finds no reader of either cell. The
boot path calls `blkdev_hdv_boot` directly (`blkdev_boot`, `bios/blkdev.c:252`;
called from `bios/bios.c:825`). What a call through the cell would do:
`flop_hdv_init` sets `fverify = 0xff`, `seekrate = 3`, `nflops = 0` and
detects floppies (`bios/floppy.c:288-330`, the cells `_fverify`, `_seekrate`,
`_nflops`, all "hardware" rows of the table); `blkdev_hdv_boot` reads the boot
sector into `dskbufp` and executes it, returning 2, 3 or 4 when it cannot
(`bios/blkdev.c:256-284`; the jump `:266-267`; `bootcheck` `:275-296`).

**What a program does with them.** Chain to the old value after installing its
own (an AHDI-era driver patches `hdv_init`/`hdv_boot`); or call them. Neither
occurs in the searched trees (`_sysvars-contract.md` section 7).

**Relation to the `hdv_*` cells NewTOS writes.** NewTOS writes **none** of the
five: `hdv_bpb`, `hdv_rw`, `hdv_mediach` read 0 and are watched against
default 0 (`tos-api/src/bdos/process/hooks.rs:283@PAGE0_HOOKS`, rows `$472`,
`$476`, `$47E`; ADR-031: "not served, warning", floppy and non-floppy use);
`hdv_init`/`hdv_boot` read 0 and have **no** watcher, by rule 1 (EmuTOS never
calls them; `tos-api/src/bdos/process/hooks.rs:754@silent`, test list `$46A`,
`$47A`). So the five are all 0 today; EmuTOS fills all five from one place.

**Decided (Q8; ADR-031 amendment item 1).**
* `_hdv_init` = `RET_STUB_VA` (the shared `ret`,
  `tos-api/src/bdos/process/loader.rs:1303@RET_STUB_VA`): exact for `void`.
* `_hdv_boot` = a new stub that returns the code **4**. EmuTOS's routine
  `blkdev_hdv_boot` returns what `bootcheck` returns when it does not boot
  (`bios/blkdev.c:258-270`): 2 "couldn't load" (boot device out of range,
  `:277-278`), 3 "unreadable" (`:280-282`), 4 "not valid boot sector"
  (checksum not `0x1234`, `:284-285`); 0 = bootable, and then it jumps into the
  sector and does not return (`:262-267`). NewTOS has no boot sector to
  execute: the boot drive exists (so not 2), nothing is read (so not 3), and
  what it holds is not a boot sector, so the stub returns **4**. Choosing 4
  among the three is the researcher's reading of "nothing can be booted"
  (inferred); the value is one constant. The stub is two instructions,
  `d2800080` (`mov x0, #4`) and `d65f03c0` (`ret`), placed in the free part of
  the stub page's `KBDVEC_STUB_AREA` at `RET_STUB_VA + 0x20` (the region is
  0x40 bytes: `ret` at `+0`, `kbdvec_default` at `+0x08..+0x20`,
  `tos-api/src/bdos/process/loader.rs:1056@KBDVEC_STUB_AREA_VA`,
  `tos-api/src/xbios/kbdvecs.rs:92@KBDVEC_DEFAULT_VA`; the new stub ends at
  `+0x28`). The encoding of `mov x0,#4` is `0xD2800000 | 4<<5` (the
  `kbdvecs.rs` stubs use the same MOVZ form, `d2800002`; to be checked with
  clang as there). The cell value is `USER_VA_BASE + KBDVEC_STUB_AREA_VA + 0x20`
  (`USER_VA_BASE = 0`, `tos-api/src/bdos/process/loader.rs:207@USER_VA_BASE`).
* The five `hdv_*` cells: `hdv_bpb`, `hdv_rw`, `hdv_mediach` stay 0 (watched,
  ADR-031 floppy/non-floppy use); `hdv_init`, `hdv_boot` hold stubs, unwatched.
  The test `page0_hooks_are_not_the_cells_emutos_never_calls`
  (`tos-api/src/bdos/process/hooks.rs:754@silent`) stays true.

## 11. `_themd` (`$48E`, 16 bytes): the first memory descriptor

**Cell.** `$48E` "first memory descriptor block" (`tosvars.ld:73`, map `:253`),
`struct _md themd` (`include/tosvars.h:83`); `____md` at `$49E` is 4 bytes of
"additional memory descriptors" space, unused (`tosvars.ld:74`; `_sysvars-contract.md`
row `____md`). The 16-byte m68k `MD`: `m_link`, `m_start`, `m_length`, `m_own`
(`include/bdosdefs.h:108-115`). `$48E` is 2 mod 4.

**EmuTOS value and writers.** Written only by `getmpb`: `m_link = NULL`,
`m_start = membot`, `m_length = memtop - m_start`, `m_own = NULL`, and
`mp_mfl = &themd` (`bios/biosmem.c:144-149`). `getmpb` is called once, by
`umem_init` (`bdos/umem.c:555`). After that `themd` is **live BDOS state**,
not a snapshot: it is the head of `pmd.mp_mfl`, and `ffit` breaks it up
(`bdos/iumem.c:100-135`): the first allocation that is not the whole block makes
`themd` the descriptor of that allocated block (`q->m_length = amount`, linked
into `mp_mal`, `m_own = run`), the free rest is a new `MD` from `xmgetmd`
(`bdos/osmem.c:169`). So what a program reads at `$48E` after boot is the first
allocated block's descriptor, not `[membot, memtop)` (inferred from the cited
code; the oracle value is a measurement). `m_start` stays `membot` as long as
that block is not freed and merged (inferred).

**What a program does with it.** A memory tool reads the MD chain from `$48E`
(only inside `Supexec` on EmuTOS: a user-mode read below `$800` is a bus error,
`docs/api/bios/00-getmpb.md` V2.3); or calls `Getmpb`.

**What the `Getmpb` answer is built from on NewTOS.** One static
`PublicMd` in a `ThemdPage` of its own
(`tos-api/src/bios/getmpb.rs:91@ThemdPage`, `tos-api/src/bios/getmpb.rs:41@PublicMd`),
refilled on every call from `stram_range()` by `describe`
(`tos-api/src/bios/getmpb.rs:128@describe`): link NULL, start `membot`, length
`memtop - membot`, owner NULL, in the LP64 header layout, **32 bytes**
(`docs/api/bios/00-getmpb.md` V1). It is not the BDOS's own pool descriptor
(`pmd`, an internal 24-byte `tos_types::MD`, never published; same spec V1).

**Does a process-visible MD exist without a call?** No. The page is published
read-only into a process only when that process calls `Getmpb`
(`tos-api/src/bios/getmpb.rs:187@expose`); the cell `$48E` is never written
(page 0 zero). And it cannot hold the same bytes: the area `$48E-$4A1` is 20
bytes, the public MD is 32.

**Decided, Q5 option (b): a 16-byte MD in the m68k layout with the extent at
load.** Written at load in `populate_sysvars`, from the same `stram_range()`
as `describe` (`tos-api/src/bdos/mem/init.rs:107@stram_range`, `(membot,
memtop)`), four little-endian LONGs (as every page-0 LONG, e.g. `wr_long`,
`tos-api/src/bdos/process/loader.rs:1123@wr_long`) at `$48E`, `$492`, `$496`,
`$49A`: `m_link = 0`, `m_start = membot`, `m_length = memtop - membot`,
`m_own = 0` (the layout of `include/bdosdefs.h:108-115`; the 16 bytes end at
`$49D`, `____md` at `$49E` stays 0). It is the EmuTOS value at `getmpb` time,
not the live state after boot (above); a program reading `$48E` after the first
allocation sees the initial extent where EmuTOS shows the first allocated
block. The 32-byte `Getmpb` answer keeps its LP64 layout; the two differ in
size by design. No live mirror (Q5 option (c) rejected: no NewTOS object).

## 12. `_dskbufp` (`$4C6`, LONG): the disk buffer pointer

**Cell.** `$4C6` "pointer to disk buffer" (`tosvars.ld:84`, map `:264`),
`UBYTE *dskbufp` (`include/tosvars.h:54`). 2 mod 4.

**EmuTOS value.** `dskbufp = dskbuf` (`bios/blkdev.c:90-92`, in `blkdev_init`,
`bios/bios.c:503-504`). `dskbuf` is `UBYTE dskbuf[DSKBUF_SIZE]` in
`bios/lowstram.c` (aligned 2, "as low as possible in ST-RAM", placed "to put it
out of the way of any old games", `bios/lowstram.c:28-45`);
`DSKBUF_SIZE = max(CHKSUM_SECTORS, 2) * SECTOR_SIZE` = `6 * 512` = **3072 bytes**
(`include/biosext.h:22-29`, `include/biosdefs.h:23`; the comment: "at least two
sectors" per Atari documentation, six because `flop_mediach` checksums
`CHKSUM_SECTORS` sectors). Oracle: `_dskbuf` = **`0x1004`**
(`references/emutos-oracle/1a7484ab/emutos.map:309`; asserted 4-byte aligned,
`:433`), so `$4C6` holds `0x00001004` on the oracle ROM.

**Readers (every one found).** Disk drivers and boot code, not only ACSI and
floppy: `bios/acsi.c:156-670`, `bios/scsi.c:376-387`, `bios/scsidriv.c:314`,
`bios/ide.c:1379`, `bios/floppy.c:352,456,542,1118,1202-1208`,
`bios/disk.c:162-215`, `bios/blkdev.c:266-309,626-638,766` (the boot sector is
read into it and **executed from it**, `:266-267`; the BPB scan parses it),
`bios/clock.c:1087-1125` (the RTC data sector). Writers: only the store at
`bios/blkdev.c:91`; the buffer content is written by those readers' I/O.

**What a program does with it.** Uses it as a scratch sector buffer when it
issues `Rwabs`/disk calls (the Atari documentation says "at least two sectors"
and TOS programs and boot code have used it); reads the boot sector a driver
left there. A program that stores its own data there races with EmuTOS's own
BIOS use of the same buffer (a program-side hazard, inferred).

**NewTOS counterpart.** None found: `grep dskbuf|dskbufp` over `tos-api/src`,
`kernel/src`, `tos-types/src` returns no hit. NewTOS's block path reads into the
caller's buffer or its own block cache (`tos-api/src/bios/bpb.rs:315@scan`
log-in reads the boot sector through its own buffer; no global disk buffer
exists). So `$4C6` can only be served by a buffer that this work **provides**.

**Decided, Q6 option (b): a 3072-byte buffer in the process's private
read-write page.** The page is `tos-api/src/bdos/process/mmu.rs:239@PRIV_PAGE`
(one per process slot, EL0 read-write, 4 KiB, mapped at `PRIV_PAGE_VA =
0x7F000`, `tos-api/src/bdos/process/mmu.rs:259@PRIV_PAGE_VA`). What lives there
today: the `Kbdvbase` table at `+0x00`..`+0x5F`
(`tos-api/src/bdos/process/loader.rs:1048@KBDVECS_AREA_VA`, size `0x60`,
`tos-api/src/bdos/process/loader.rs:1051@KBDVECS_AREA_SIZE`) and the `PARMBLK`
scratch at `+0x100`..`+0x127`
(`tos-api/src/bdos/process/loader.rs:1022@PARMBLK_SCRATCH_VA`, 40 bytes,
asserted `tos-api/src/bdos/process/loader.rs:1078@PARMBLK_SCRATCH_VA`); the
rest is free, no other user in `tos-api/src` or `kernel/src` (grep of
`PRIV_PAGE_VA`, this run). The buffer takes **`+0x400`..`+0xFFF`** (0xC00 =
3072 bytes: `DSKBUF_SIZE = 6 * 512`, above), so `_dskbufp = USER_VA_BASE +
PRIV_PAGE_VA + 0x400 = 0x7F400`, 4-byte aligned (also 8), below 4 GiB. It is
per process, never shared, and no kernel code uses it (no NewTOS counterpart).
The loader zeroes it at load (inferred: a slot's page is reused, and a previous
process's bytes must not show). The `+0x128`..`+0x3FF` gap stays free. The
developer adds an assertion for the new range (`+0x400 + 0xC00 == PAGE_SIZE`)
beside the two at `tos-api/src/bdos/process/loader.rs:1077@PARMBLK_SCRATCH_VA`.

## 13. `_pun_ptr` (`$516`, LONG): the AHDI info table

**Cell.** `$516` "if AHDI, pointer to pun_info" (`tosvars.ld:98`, map `:278`),
`PUN_INFO *pun_ptr` (`include/ahdi.h:33`). 2 mod 4.

**EmuTOS value.** `pun_ptr = &pun_info` at the end of `pun_info_setup`
(`bios/blkdev.c:161`), a `static PUN_INFO pun_info` (`:45`, a BSS object, not a
map symbol); set during `blkdev_init` -> `blkdev_hdv_init` (`bios/blkdev.c:190-195`),
before the BDOS reads it (`bufl_init` `bdos/bdosmain.c:328`, which runs in
`osinit_before_xmaddalt` after `blkdev_init`, `bios/bios.c:504`). Never NULL on
the oracle after boot.

**Layout (`include/ahdi.h:20-31`).** `UWORD puns` (physical device count),
`UBYTE pun[16]` (device number per drive, `0xff` unused), `ULONG
partition_start[16]`, `ULONG cookie` (`'AHDI'` = `0x41484449`, `blkdev.c:156`),
`ULONG *cookie_ptr` (= `&pun_info.cookie`, `:157`), `UWORD version_num`
(`0x300`, `:158`), `UWORD max_sect_siz`, `LONG reserved[16]`. 158 bytes on m68k
if `ULONG` is 2-aligned there (inferred; the `PUN_MAXUNITS` is `16`,
`include/ahdi.h:19`). Filled by `pun_info_setup` (`bios/blkdev.c:112-163`):
`puns` counts physical devices (`:142`), `pun[i] = physdev`, `partition_start[i]
= dev->start` for each valid hard-disk partition, `i` starting at
`NUMFLOPPIES` = 2 (`bios/disk.h:18`; so entries 0 and 1 stay `0xff`),
`max_sect_siz = max(largest recsiz, CONF_LOGSEC_SIZE 512)` (`:148-160`,
`include/config.h:1606`).

**Readers.** `bufl_init` sizes the BDOS buffers by `pun_ptr->max_sect_siz`
(`bdos/fsbuf.c:61`); `XHDI` returns `pun_ptr->version_num` (`bios/xhdi.c:214`,
`CONF_WITH_XHDI`, 1 in the config table; NewTOS provides no XHDI: ADR-032).
Hard-disk drivers and tools (HDDRIVER, partitioners) probe it by the
`'AHDI'` magic and the `cookie_ptr == &cookie` self-check (inferred from the
structure; no searched program reads it).

**What a program does with it.** Reads the partition table (device per drive,
start sector) to address raw sectors; decides "AHDI present" from a non-NULL
pointer; a driver writes it to install itself.

**NewTOS counterpart.** The numbers exist but not the table:
`MAX_SECT_SIZ` (`tos-api/src/bios/bpb.rs:78@MAX_SECT_SIZ`) is set by the boot
scan `scan` (`tos-api/src/bios/bpb.rs:315@scan`), whose doc says: "Not
reproduced (no NewTOS counterpart): the AHDI `PUN_INFO` structure itself and
`pun_ptr`"; the per-drive start sectors are in the partition table
(`tos-api/src/bios/drive.rs:27@Partition`, `start_lba`; `resolve`, `:144`). No
`PUN_INFO` and no pointer exist (`grep pun_ptr|PUN_INFO|AHDI` over `tos-api/src`
and `kernel/src`: only comments in `bpb.rs` and `mod.rs`). Mapping row
`docs/analysis/deps/mapping.tsv:350` records it ("no PUN_INFO/pun_ptr").

**Decided, Q7: `_pun_ptr` stays 0 and leaves this ticket.** It is taken up
with TASK/BIOS18.md (partitions), after it is read whether `Rwabs` honours what
a `PUN_INFO` consumer expects. The row of `_sysvars-contract.md` (`$516`
"missing") stays; a probe reads 0 ("no AHDI") on NewTOS. The EmuTOS facts above
(layout, 158 bytes, `'AHDI'` cookie) are the input for BIOS18, not a BIOS20
work item.

## 14. Measurable cases for the tester (the decided cells are in section 17)

All addresses are 2 mod 4 (`$442`, `$44E`, `$452`, `$454`, `$456`, `$462`, `$466`, `$46A`,
`$47A`, `$48E`, `$4C6`, `$502`, `$50E`, `$512`, `$516`), so none is reachable by
`Setexc(num, -1)`; a new `peek` case type is needed (`_sysvars-contract.md`
section 8 proposes it; not built). `tosdump/probes/hkprobe.c:93@prt_stat_v` already
reads `CELL(0x506)` through `Supexec` and calls through it, the pattern to
copy; or one probe program under `tosdump/probes/`. Expected oracle values are
**source-predicted, not measured**; the oracle frame rate is measured in the
test run (TASK/BIOS20.md).

| Cell / case | Oracle (EmuTOS) | NewTOS today | NewTOS after the work |
|---|---|---|---|
| `$442` WORD | 20 | 0 | 20 |
| `$502`, `$50E`, `$512` LONG | non-zero, equal to each other and to `$506`/`$50A` (`just_rts`, `0x00e20ea4` in the oracle map: compare non-zero/equal, not the number) | 0 | non-zero, equal to `$506` |
| call through each of the three | returns | branch to 0: fault | returns |
| `a = *$466; Vsync(); b = *$466` | `b - a >= 1` (inferred: exactly 1 at a steady VBL) | `b == a`, both 0 | `b - a >= 1` (the section 5 `Vsync` rule) |
| `h0 = $4BA, f0 = $466; wait N Vsync; h1, f1` | `(f1-f0) >= N`; `(h1-h0)/(f1-f0)` = `200 / VBL rate` | 0 | ratio ~ 200/60 (C1) |
| `*$462` against `*$466` after boot, `_vblsem == 1` | `$466 - $462 != 0` (inferred, section 2), constant between reads | both 0 | `$466 - $462 == 0` while `_vblsem` stayed 1 (both start 0): a documented difference |
| V2: in `Supexec` set `$452 = 0`; `Vsync` x3; read `$462`, `$466` deltas; restore `$452 = 1` | `d(466) >= 3`, `d(462) == 0` (source-predicted; also stops blink and the mouse draw on the oracle) | cells 0 | `d(466) >= 3`, `d(462) == 0`; NewTOS also skips its mouse draw while the cell is at or below 0 (Q2 (b); visible on screen only) |
| process switch: parent reads `$466`, child runs `Vsync` x3, parent reads | one global counter, strictly larger | n/a | strictly larger (P3 refresh); oracle cannot show per-process view |
| `$452` WORD | 1 | 0 | 1 |
| `$454` WORD | 8 | 0 | 8 |
| `$456` LONG next to `$454` | `$4CE` (map `:266`) | 0 | `$4CE`; eight slots at `$4CE` read 0 |
| `$44E` LONG | non-zero, `== Logbase()`; (inferred) `== *$436` | 0 | non-zero, `== Logbase()` of that process (a process that never called `Logbase` reads the same address but faults when it dereferences it, Q4 (a)) |
| `$44E` after `Setscreen(L,-1,-1,-1)` (L inside screen memory) | cell `== L`, `Logbase() == L`, `Physbase()` unchanged (`bios/screen.c:1106-1111`) | 0 | cell `== Logbase()` `== L`; `Physbase()` unchanged. NewTOS accepts only a base inside its screen memory on a page boundary (`tos-api/src/xbios/screen.rs:290@setscreen` doc): use `Logbase() + size` |
| `$46A` LONG | `0x00e09712` on the oracle ROM (map `:589`); non-zero | 0 | non-zero, equal to `$506`; call returns |
| `$47A` LONG | non-zero (ROM address of `blkdev_hdv_boot`, not in the map) | 0 | non-zero (own stub); call returns 4 (EmuTOS returns 2, 3, 4 or does not return, `bios/blkdev.c:258-285`) |
| `$48E` 4 LONGs | `m_link` (maybe non-NULL), `m_start`, `m_length`, `m_own = run`: the first allocated block (inferred, section 11); compare field classes, not numbers | 0 | `{0, membot, memtop - membot, 0}` (section 11) |
| `$4C6` LONG; read 1024 bytes through it | `0x00001004` (map `:309`); readable; (optional) write/restore 1024 bytes | 0 | non-zero, 4-aligned, 1024 bytes readable and writable (the buffer is 3072 bytes, section 12) |
| `$516` LONG; `*(ULONG*)(p+82)`, `*(ULONG*)(p+86)`, `*(UWORD*)(p+90)` | non-zero; `'AHDI'`; pointer to that cookie; `0x300` (offsets assume 2-aligned `ULONG`, inferred; `p+86 == p+82` is the check) | 0 | stays 0 (Q7, BIOS18): a documented difference, not a case of this ticket |

**What the pinned oracle cannot show:** (a) the per-process view across a
switch (EmuTOS has one global page 0); (b) the NewTOS rate choice (the oracle
rate is Hatari's video mode; measured in the test run); (c) the stub, ROM-code
and `dskbuf`/`pun_info` *addresses* (compare non-zero/equal/class, not the
number); (d) the 5 ms publish lag (Hatari updates the cells inside the VBL
interrupt); (e) `$48E` after the first free-list change is machine- and
boot-dependent.

## 15. Localization for the developer

`cargo xtask deps <fn> --up/--down` run in this worktree (default kinds
`asm,direct,layer,table,vector`; counts from this run):

| EmuTOS function | `--down` | `--up` | mapping row (`docs/analysis/deps/mapping.tsv`) |
|---|---|---|---|
| `chardev_init` (`bios/chardev.c`) | 0 callees | `bios_init`, `biosmain` (**unmapped**), `memdone` (design) | `:356` -> `loader.rs@populate_sysvars`, `mapped`, note "aux_stat/aux_vec stay 0" (this note and `dump_vec` are what the fix changes) |
| `init_system_timer` (`bios/mfp.c`) | `xbtimer`, `mfpint` (mapped), ... (design) | `bios_init`, `biosmain` (**unmapped**), `memdone` (design) | `:709` `design:hardware:mfp68901`: the `timer_ms = 20` store has **no row**; add one -> `populate_sysvars` |
| `tickcal` | 0 | 10 functions, all mapped | `:314` -> `bios/mod.rs@dispatch`, a constant |
| `vsync` | `stop_until_interrupt` (ext) | 126 functions; `bios_init`, `biosmain` unmapped | `:842` -> `xbios/system.rs@vsync` (add: publishes the counters) |
| `scrdmp` | `just_rts` (ext), `protect_v` (ext) | `escfn17`, `xbiostrap`, ... all mapped | `:1088` -> `xbios/system.rs@scrdmp` |
| `int_vbl` | `blink` (mapped), `detect_monitor_change`, `flopvbl` (design), `just_rts`; **unmapped: `cell_addr`, `invert_cell`** (`bios/conout.c`) | 1, mapped | `:996` -> `vbl.rs@run_if_due`, note "frclock/vbclock ... no counterpart (ADR-031)" (the fix changes it: counters and `_vblsem` read now live in `tick.rs`) |
| `int_timerc` | `kb_timerc_int`, `handle_mouse_mode`, `convert_scancode` (**unmapped**) | 1, mapped | `:994` -> `bios/tick.rs@advance` |
| `flopvbl` | 5 floppy/PSG functions (design) | `int_vbl` | `:559` `design:hardware:floppy` |
| `screen_init_address` | 230 callees; **30 unmapped** in `bios` | `bios_init`, `biosmain` (**unmapped**), `memdone` (design) | `:831` **unmapped** ("ST/TT shifter and VIDEL ... no counterpart"): the `v_bas_ad = screen_start` store has no row; add -> `populate_sysvars` + `xbios/screen.rs@setscreen` |
| `setscreen` | 51 callees; unmapped in `bios`: `cell_addr`, `get_pixel_size`, `invert_cell`, `screen_get_current_mode_info`, `set_screen_shift` | 14, all mapped | `:837` -> `xbios/screen.rs@setscreen`; add: refreshes the running process's `$44E` |
| `logbase` | 0 | 2, all mapped | `:827` -> `xbios/screen.rs@logbase` |
| `bios_init` | n/a | n/a | `:295` unmapped; `:296-297` mapped to `vectors.rs@default_of` and `populate_sysvars`: add `nvbls = 8`, `vblsem = 1` (`bios/bios.c:366,553`) -> `populate_sysvars` |
| `screen_init_mode` | n/a | n/a | `:832` `design:hardware:video-shifter`: `vblsem = 0` at `bios/screen.c:501` has no counterpart by design (no Atari video) |
| `getmpb` | 0 callees | `umem_init` (mapped), `bios_init`, `biosmain` (**unmapped**), `memdone` (design) | `:319-321` mapped; add a row: `$48E` is served at load (Q5 (b)) |
| `blkdev_init` | 254 callees; **34 unmapped** in `bios` | `bios_init`, `biosmain` (**unmapped**) | `:334` **unmapped**: the `dskbufp`, `hdv_boot`, `hdv_init` stores have no row; add -> `populate_sysvars` |
| `pun_info_setup` | 157 callees; 22 unmapped | `bios_init`, `biosmain`, `blkdev_init` (**unmapped**) | `:350` -> `bpb.rs@scan` ("no PUN_INFO/pun_ptr": unchanged, Q7 leaves `$516` 0) |
| `flop_hdv_init` | 15 callees; unmapped: `get_shift` | `bios_init`, `biosmain`, `blkdev_init` (**unmapped**) | `:547` `design:hardware:floppy` |
| `blkdev_hdv_boot` | 109 callees; unmapped: itself, `bootcheck`, `disk_rw`, `flush_data_cache`, `hd_boot_read`, `invalidate_data_cache`, `invalidate_instruction_cache` | not run | `:332` **unmapped** |
| `bufl_init`, `umem_init` | n/a | all mapped | `:82` -> `tos-api/src/bdos/fs/cache.rs`, `:245` -> `tos-api/src/bdos/mem/init.rs` |

**Unmapped entries in the touched layers (all `bios`):** `bios_init`, `biosmain`
(both through `chardev_init`, `init_system_timer`, `screen_init_address`,
`blkdev_init`, `getmpb`; `bios_init` has two *mapped* rows
(`mapping.tsv:296-297`) but `deps` lists it unmapped because its `-` row
(`:295`) comes first: a classification of the tool, not read further),
`cell_addr`, `invert_cell`, `kb_timerc_int`, `handle_mouse_mode`,
`convert_scancode`, `get_pixel_size`, `screen_get_current_mode_info`,
`set_screen_shift`, `get_shift`, `blkdev_init`, `screen_init_address`,
`blkdev_hdv_boot`, `bootcheck`, `disk_rw`, `hd_boot_read`, `flush_data_cache`,
`invalidate_data_cache`, `invalidate_instruction_cache`; plus the long lists of
the two big callee sets (`screen_init_address --down`: 30, `blkdev_init --down`: 34,
`pun_info_setup --down`: 22; `cargo xtask deps <fn> --down` prints them). None
of them is the object of a cell here; they are listed because the ticket asked.

NewTOS places the developer touches: `tos-api/src/bdos/process/loader.rs:1149@populate_sysvars`
(static cells: `_timer_ms`, `_vblsem`, `_nvbls`, `_vblqueue`, the three vectors, and
`_v_bas_ad`, `_hdv_*`, `_themd`, `_dskbufp`; not `_pun_ptr`);
`tos-api/src/bios/tick.rs:108@publish` or a sibling (counters, `_vblsem` read);
`tos-api/src/bdos/process.rs:977@set_current` (switch-in refresh);
`tos-api/src/xbios/system.rs:33@vsync` (publish before return);
`tos-api/src/xbios/screen.rs:290@setscreen` (refresh `$44E`);
`tos-api/src/bdos/process/hooks.rs:294@DumpVec` (default);
`tos-api/src/bios/vectors.rs:185@write_page0_vectors` (reads: it writes only
`$000-$40F`, so none of the fourteen cells is in its range, no change);
tests: `tos-api/src/bdos/process/loader.rs:2030@prt_stat` (the model).

## 16. Corrections

Against the source, in `docs/api/bios/_sysvars-contract.md` (edited in this
commit; rows only, no re-numbering):

1. **Row `_dskbufp`** said "every EmuTOS reader is ACSI/floppy driver code". Wrong:
   readers are also the SCSI, IDE and `scsidriv` code, the BPB scan and the boot
   sector execution in `bios/blkdev.c`, `bios/disk.c` and the RTC code in
   `bios/clock.c` (section 12). Row and the section-5 table row corrected.
2. **Row `_v_bas_ad`** listed readers `bios/conout.c`, `bios/vt52.c` only; the VDI
   reads it in 8 places (section 8). Row and the section-5 table row extended.
3. **Row `_themd`**: added that after the first `ffit` it is live BDOS state, not
   the `getmpb` snapshot (`bdos/iumem.c:100-135`).

Everything else in `_sysvars-contract.md` for the fourteen cells agrees with the
source (checked: writers of `_vblsem`, `_nvbls`, `_hdv_init`, `_hdv_boot`,
`_pun_ptr`, `_vbclock`, `_frclock`, `_timer_ms`, `_dump_vec`, `_aux_*`). One
cite repair for the record: `_boot-init.md` section 7c (`vbclock` "keins
gefunden") stays wrong as noted in BIOS19 section 6.1.

## 17. The decided contract: values, developer checklist, tester cases

Owner decisions of 2026-11-20 (TASK/BIOS20.md, "Spec gate and owner decisions",
"Second spec pass and owner answers", "ADR-031 amended"). This section replaces
the former questions Q1 to Q8; the sections 5 to 13 above carry the reasoning.

### 17.1 The decisions

| Q | Decision | Where |
|---|---|---|
| (P1+P3) | Counters follow `_hz_200`'s model; switch-in refresh for every process; `Vsync` publishes before it returns | section 5 |
| (C1) | `_frclock`/`_vbclock` follow the 60 Hz frame clock, start at 0 at boot | section 6 |
| Q1 (c) | `_vbclock = _frclock - lost`; `lost` = frames the process ran with `_vblsem <= 0`; a process that was away loses nothing | section 6, consequence 2 |
| Q2 (b) | `_vblsem <= 0` in the running process also skips `vb_draw`; S10 lists the cell | section 6, consequence 3; `docs/api/_super-state.md` S10 |
| Q3 (b) | `_nvbls` = 8, `_vblqueue` = `$4CE`, eight empty slots in page 0; a program's routine there is never called, NewTOS warns | section 9 |
| Q4 (a) | `_v_bas_ad` always holds the screen address; use before `Logbase`/`Physbase` ends in a bus error | section 8 |
| Q5 (b) | `_themd` is a 16-byte MD in the m68k layout with the extent at load | section 11 |
| Q6 (b) | `_dskbufp` points at a 3072-byte buffer in the private read-write page | section 12 |
| Q7 | `_pun_ptr` stays 0, out of this ticket (TASK/BIOS18.md) | section 13 |
| Q8 | `_hdv_init` = the `ret` stub; `_hdv_boot` = a stub that returns 4 | section 10 |

### 17.2 The thirteen served cells (and the `$456` partner)

All cells are in each process's own page 0 (2 mod 4 addresses, unaligned
stores as `wr_long`/`wr_word` in `populate_sysvars`,
`tos-api/src/bdos/process/loader.rs:1149@populate_sysvars`). `RET` =
`RET_STUB_VA` (`tos-api/src/xbios/kbdvecs.rs:90@RET_STUB_VA`); `HDVB` =
`USER_VA_BASE + KBDVEC_STUB_AREA_VA + 0x20` (new, section 10).

| # | Cell | Addr | Width | Value | Written by | Refreshed |
|---|---|---|---|---|---|---|
| 1 | `_timer_ms` | `$442` | W | 20 | `populate_sysvars` | never |
| 2 | `_v_bas_ad` | `$44E` | L | user address of `ScreenInfo.base` = what `Logbase` answers (section 8) | `populate_sysvars`; `setscreen` (`tos-api/src/xbios/screen.rs:290@setscreen`) | own `Setscreen`; switch-in |
| 3 | `_vblsem` | `$452` | W | 1 | `populate_sysvars` | never by the kernel (a program writes it in super state; it reads it back) |
| 4 | `_nvbls` | `$454` | W | 8 | `populate_sysvars` | never |
| 5 | `_vbclock` | `$462` | L | `fc - lost` (section 6), 0 at boot | the publish place (`tos-api/src/bios/tick.rs:108@publish` or a sibling) | timer IRQ, `Vsync`, switch-in |
| 6 | `_frclock` | `$466` | L | `fc = frame_clock() - frame_base`, 0 at boot | as #5 | as #5 |
| 7 | `_hdv_init` | `$46A` | L | RET | `populate_sysvars` | never |
| 8 | `_hdv_boot` | `$47A` | L | HDVB (stub: `mov x0,#4; ret`, returns 4) | `populate_sysvars`; the stub bytes by `write_stubs` | never |
| 9 | `_themd` | `$48E` | 16 B | LE LONGs `0`, `membot`, `memtop - membot`, `0` | `populate_sysvars` | never |
| 10 | `_dskbufp` | `$4C6` | L | `USER_VA_BASE + PRIV_PAGE_VA + 0x400` = `0x7F400` | `populate_sysvars`; buffer zeroed at load | never |
| 11 | `_dump_vec` | `$502` | L | RET | `populate_sysvars` | never |
| 12 | `_aux_stat` | `$50E` | L | RET | `populate_sysvars` | never |
| 13 | `_aux_vec` | `$512` | L | RET | `populate_sysvars` | never |
| partner | `_vblqueue` | `$456` | L | `$4CE` | `populate_sysvars` | never |
| partner | `_vbl_list[0..8]` | `$4CE`-`$4ED` | 8 x L | 0 | page 0 zero-fill (`tos-api/src/bdos/process/loader.rs:671@fill`) | never |

Not served: `_pun_ptr` `$516` stays 0 (Q7). Cells #5 and #6: `Vsync` keeps
comparing raw `frame_clock` values; the cells are `frame_clock() - frame_base`
with `frame_base` recorded once at boot (section 6). Cells #5 and #6 are the
only ones with a per-process view beyond the static seed (section 5).

**Where the new stub and buffer live (cited).** The `HDVB` stub: the stub page
(shared, execute-only, `STUB_PAGE_VA = 0x7E000`,
`tos-api/src/bdos/process/mmu.rs:256@STUB_PAGE_VA`), area `KBDVEC_STUB_AREA_VA`
(`STUB_PAGE_VA + 0x40`, 0x40 bytes, `tos-api/src/bdos/process/loader.rs:1056@KBDVEC_STUB_AREA_VA`,
size `tos-api/src/bdos/process/loader.rs:1059@KBDVEC_STUB_AREA_SIZE`), `+0x20..+0x28` of it; the `ret` stub is at `+0`, `kbdvec_default` at
`+0x08..+0x20` (`tos-api/src/xbios/kbdvecs.rs:92@KBDVEC_DEFAULT_VA`, 6 words).
The `_dskbufp` buffer: private read-write page, `+0x400..+0xFFF` (section 12).

### 17.3 The warning watcher (`tos-api/src/bdos/process/hooks.rs`)

* `DumpVec` (`hooks.rs:294@DumpVec`): default becomes `RET_STUB_VA as u32`
  (ADR-031 amendment item 2).
* `Vblqueue`: the row `{addr 0x456, default 0}` is replaced. The watcher
  compares the **eight slots**, `$4CE`, `$4D2`, ..., `$4EA` (LONGs, stride 4),
  each with 0, and warns on a non-zero one (amendment item 3: "the warning
  fires on a non-empty slot instead of on a `vblqueue` other than 0"). `$456`
  itself is therefore no longer compared (literal reading of the amendment;
  inferred). The slots are page-0 reads like the other hook cells
  (`check_page0`, `hooks.rs:319@check_page0`).
* `aux_*`, `hdv_init`, `hdv_boot`: no watcher (rule 1;
  `hooks.rs:754@silent` keeps `$46A`, `$47A`, `$50E`, `$512` in its list).

### 17.4 Rule S10

`docs/api/_super-state.md` S10 now lists `_vblsem` (`0x452`, signed WORD,
compared with 0 only) beside `conterm` and the reset-entry magics, citing the
ADR-031 amendment of 2026-11-20 item 4 (`docs/MEMORY.md`). Its two readers are
the `_vbclock` accounting (section 6, consequence 2, with the per-process word
`lost`) and the `vb_draw` gate (consequence 3). Both read the **running**
process's cell through `current_page0_kva`; no other cell of page 0 gains a
kernel reader.

### 17.5 Developer checklist

Host-test model: `tos-api/src/bdos/process/loader.rs:2030@prt_stat` (calls
`populate_sysvars` on a page buffer, reads LONGs back). A probe case is a case
id of 17.6.

| Cell | Value | Writer place | Host test | Probe case |
|---|---|---|---|---|
| `$442` | 20 | `populate_sysvars` | `rd16(0x442) == 20` and `== TICKCAL_MS` (`tos-api/src/bios/mod.rs:95@TICKCAL_MS`) | C01 |
| `$44E` | `Logbase` address | `populate_sysvars`, `setscreen`, `set_current` refresh | with a host double: cell equals `logbase()`; after `setscreen` equals the new base | C02, C03 |
| `$452` | 1 | `populate_sysvars` | `rd16(0x452) == 1` | C04 |
| `$454` | 8 | `populate_sysvars` | `rd16(0x454) == 8` | C05 |
| `$456`, slots | `$4CE`; 8 x 0 | `populate_sysvars` | `rd32(0x456) == 0x4CE`; slots zero; a non-zero slot makes the watcher warn, a zero one does not | C06 |
| `$462` | `fc - lost` | publish place, `vsync`, `set_current` | feed `frame_clock`; `_vblsem = 1`: both equal; `_vblsem = 0` for n frames: `$466 - $462 == n`; a switch-away of k frames adds nothing to `lost` | C07, C08, C09 |
| `$466` | `fc` | as `$462` | starts 0; advances with the host frame clock; `vsync` publishes before it returns; switch-in refresh | C07, C09 |
| `$46A` | RET | `populate_sysvars` | equals `RET_STUB_VA`, equals `$506` | C10 |
| `$47A` | HDVB | `populate_sysvars`, `write_stubs` | stub bytes `d2800080 d65f03c0` at HDVB; cell equals HDVB, differs from RET | C11 |
| `$48E` | 16 bytes | `populate_sysvars` | the four LE LONGs equal `0, membot, memtop - membot, 0` from `stram_range()`; `$49E` still 0 | C12 |
| `$4C6` | `0x7F400` | `populate_sysvars` | cell equals `PRIV_PAGE_VA + 0x400`; 4-aligned; `+0xC00` ends at the page end; buffer zero at load | C13 |
| `$502` | RET | `populate_sysvars`; `DumpVec` default | equals `$506`; `check_page0` quiet at default, warns on another value | C10 |
| `$50E`, `$512` | RET | `populate_sysvars` | equal `$506`; still not in `PAGE0_HOOKS` | C10 |

Also: the `mapping.tsv` rows of section 15 (ratchet); the existing
`silent` test and the `bios-r1`, `bios-core-r1`, `xbios-chardev-r1`,
`xbios-system-r1` tables stay as the ticket's acceptance says.

### 17.6 Tester's cases

Read through a `Supexec` callback (the cells are 2 mod 4: `Setexc` cannot read
them); pattern `tosdump/probes/hkprobe.c:93@prt_stat_v` (reads `CELL(0x506)`
and calls through it). "Oracle" values are source-predicted, not measured; the
oracle rate is measured in the run. Compare address cells as non-zero/equal/
class, not as numbers (`0x1004`, `0x00e20ea4`, `0x00e09712` are the oracle's
own).

| Id | Case | Oracle (EmuTOS) | NewTOS |
|---|---|---|---|
| C01 | read `$442` WORD | 20 | 20 |
| C02 | read `$44E`; compare with `Logbase()` | non-zero, equal | non-zero, equal (read after `Logbase`: before it a dereference faults, Q4) |
| C03 | `Setscreen(L,-1,-1,-1)`, then `$44E`, `Logbase()`, `Physbase()` | cell `== L`, `Logbase == L`, `Physbase` unchanged | same (`L` inside the screen memory on a page boundary: `Logbase() + size`) |
| C04 | read `$452` WORD | 1 | 1 |
| C05 | read `$454` WORD | 8 | 8 |
| C06 | read `$456`, then the 8 slots | `$4CE` (map `:266`); slot 0 non-NULL after the VDI is up, 1-7 NULL | `$4CE`; all 8 slots 0 (slot 0 differs by design) |
| C07 | `a = *$466; Vsync(); b = *$466` and the same for `$462` | `b - a >= 1` | `b - a >= 1` (the `Vsync` publish) |
| C08 | in `Supexec` set `$452 = 0`; `Vsync` x3; deltas of `$462` and `$466`; restore `$452 = 1` | `d466 >= 3`, `d462 == 0` | `d466 >= 3`, `d462 == 0` (Q1 (c)); the mouse draw stops while it is 0 (on screen only) |
| C09 | `$466` in parent; child runs `Vsync` x3; parent reads again | one global counter, strictly larger | strictly larger (switch-in refresh); the oracle cannot show the per-process view |
| C09b | `h0 = $4BA, f0 = $466`; N x `Vsync`; `h1, f1` | `(h1-h0)/(f1-f0)` = 200 / VBL rate | about 200/60 |
| C10 | read `$46A`, `$502`, `$50E`, `$512`, `$506`, `$50A`; call each through its old value | `$46A` `0x00e09712` (`flop_hdv_init`, not `just_rts`: non-zero, calling it re-detects floppies: do not call it on the oracle); `$502`, `$50E`, `$512` equal to `$506`; calls return | all six non-zero, `$46A` `$502` `$50E` `$512` equal `$506`; calls return, no fault |
| C11 | read `$47A`; call it | non-zero ROM address; the call would try to boot: **do not call on the oracle**; compare the cell as non-zero only | non-zero; the call returns 4 |
| C12 | read 4 LONGs at `$48E` | first allocated block (field classes) | `{0, membot, memtop - membot, 0}`; field classes compared, no number |
| C13 | read `$4C6`; read 1024 bytes through it; write and restore 1024 | `0x1004`; readable | non-zero, 4-aligned, 1024 bytes readable and writable (3072 available) |
| C14 | read `$516` | non-zero | 0 (Q7: documented difference, not a failure of BIOS20) |

Cannot be shown by the oracle: the per-process view (EmuTOS has one global page
0); the NewTOS rate choice (oracle rate = Hatari's video mode); the stub, ROM
and buffer addresses; the 5 ms publish lag (Hatari writes inside the VBL
interrupt); `$48E` after the first free-list change; the effect of `hdv_boot`
(calling it on the oracle starts a boot attempt).

## Open points

- Inferred: the value 4 for `_hdv_boot` is the researcher's choice among the
  three codes `bootcheck` returns (`bios/blkdev.c:277-285`); one constant if the
  owner prefers 2 or 3. The encoding `d2800080` of `mov x0,#4` is derived from
  the MOVZ form of the stubs in `kbdvecs.rs` and not run through clang here.
- Inferred: the `Vblqueue` watcher no longer compares `$456` (literal reading of
  ADR-031 amendment item 3); a program that points `_vblqueue` at its own table
  is then not warned. The owner can add `$456 == $4CE` to the watcher by one
  row.
- Inferred: `lost` is charged by the `_vblsem` seen at the later publish
  (period 5 ms); `acct_fc` is one global word; the per-process word lives in the
  process record. The developer may choose another placement, the rule (frames
  with the semaphore down, away time not charged) is the contract.
- Inferred: the `_dskbufp` buffer is zeroed at load; `_v_bas_ad` stays 0 when
  no screen exists at load (not in the decision).
- Inferred, not read: `_frclock - _vbclock` constant > 0 on the oracle; the 5 ms
  `Vsync` publish lag; `_v_bas_ad == _memtop` on the oracle; `vblqueue[0] ==
  NULL` for an AUTO program; `$48E` after boot is the first allocated block; no
  other user of the private page (grep this run).
- Needs deeper analysis (not blocking): the Hatari VBL rate for the oracle's TT
  profile and the size of `_frclock - _vbclock` after boot; the oracle contents
  of `$46A`/`$47A`/`$48E`/`$4C6` are source-predicted (`$4C6` `0x1004` and
  `_flop_hdv_init` come from the generated map).
- Needs deeper analysis (BIOS18): whether `Rwabs` honours raw device numbers as a
  `PUN_INFO` consumer expects; `_pun_ptr`.
- Not searched: the gitignored upstream-source `Applications` tree for programs
  that read these cells (as in BIOS19). The "no local source" gap for the
  standard image is as in `_sysvars-contract.md` section 7.
- Out of scope: the four `bcon*_vec` tables (BIOS22); `_fverify`, `_seekrate`,
  `_nflops`; ADR-032 (XHDI); COMPATIBILITY/DEVIATIONS lines for the cells that
  stay different (`_v_bas_ad` write not honoured, `_pun_ptr` 0, per-process
  `_themd`/`_dskbufp`, `_vbclock` offset), files the lead owns.
