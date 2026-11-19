# System variables of `tosvars.ld`: per-variable contract table [BIOS19]

> Researcher output for TASK/BIOS19 (work item 1). Source: EmuTOS @ `1a7484ab`
> (`references/emutos`; generated values from the oracle build tree
> `references/emutos-oracle/1a7484ab`). NewTOS side: `tos-api/` at main
> `37c6d8e31`. This file is the contract table; the older table in
> `docs/api/bios/_boot-init.md` section 7 is a reading aid and is wrong in
> places (corrections below).

> **Contract vs. reference** — `docs/COMPATIBILITY.md` ("System variables —
> compatible how", `:2148`): the *values and semantics* of the documented
> variables are contract; which EmuTOS routine writes a cell is reference.

## 1. Count

`references/emutos/tosvars.ld` has **84 numeric assignments** (`name = 0x…;`):
the 83 of the ticket plus `_warm_magic` (`tosvars.ld:119`), which is compiled
only with `CONF_DETECT_FIRST_BOOT_WITHOUT_MEMCONF`. That option is 0 in
`make 512` (`docs/api/_emutos-config.md:278`, `references/emutos/include/config.h:1414-1418`)
and the oracle map has no `_warm_magic` symbol: the oracle build has **83**
(`references/emutos-oracle/1a7484ab/emutos.map`, section `.first_stram`,
`:206-291`). One of the 84 is `sysvars_start` (`tosvars.ld:31`), an address
marker for the first-boot clear (`bios/memory.S:566-569`), not a variable. So
there are **82 variables** in the oracle build, 83 with `_warm_magic`.
`_pmmutree` (`tosvars.ld:128`) is compiled with `CONF_WITH_68030_PMMU`, 1 in
`make 512` (`docs/api/_emutos-config.md:167`).

## 2. Classes

| Class | Meaning |
|---|---|
| **served** | NewTOS gives the cell a value with the EmuTOS meaning: written at load (static) or kept current (live), or EmuTOS itself never writes it beyond the boot clear and the zeroed NewTOS page equals it. A hook cell that holds the `ret` stub is served *as a cell*; whether the hook is called is ADR-031 |
| **hardware** | NewTOS does not write it and the reason is a citable ADR, a documented non-goal or the absence of the hardware or CPU. The reason is in the last column |
| **missing** | EmuTOS code or the classic contract uses it, NewTOS does not write it, and no ADR/COMPATIBILITY text covers it. A finding for the lead, not a decision |
| **free** | EmuTOS-private state with no meaning outside EmuTOS' own code (`docs/COMPATIBILITY.md` audit rubric item 2: "mark it free"). Not a class the ticket named; it holds the three rows that would otherwise be false "missing" entries |
| n/a | not a variable (`sysvars_start`) or not in the oracle build (`_warm_magic`) |

Counts: **served 37, hardware 25, missing 17, free 3, n/a 2** (= 84).

## 3. NewTOS writers (all that exist)

Page 0 of every process is the NewTOS stand-in for the `$000-$7FF` area
(`tos-api/src/bdos/process/loader.rs:671@fill` zeroes it, then
`populate_sysvars` fills it). The writers, from reading `tos-api/` and
`kernel/` for page-0 stores:

1. `populate_sysvars` (`tos-api/src/bdos/process/loader.rs:1149@populate_sysvars`,
   called `:672@populate_sysvars`): the static values, once per process, at load.
2. `write_page0_vectors` (`tos-api/src/bios/vectors.rs:185@write_page0_vectors`,
   called `loader.rs:1312@write_page0_vectors`) and `store_shared`
   (`vectors.rs:170@store_shared`): the 260 shared cells `$000-$40F` (vectors
   0-0xFF, `etv_*`), every process's copy, rewritten whenever the cell changes.
   This also covers `$380-$3CF` (the EmuTOS crash-dump area), see the table.
3. `Setexc` for `0x104 <= num < 0x200` (`vectors.rs:155@cell`, `:159@write_unaligned`):
   a program's own write into one page-0 cell, any variable `$410-$7FC`.
4. `tick::publish` (`tos-api/src/bios/tick.rs:108@publish`, called from
   `advance` `:78@advance` by `kernel/src/drivers/timer.rs:171@advance`): `_hz_200`
   only, only into the **running** process's page.
5. `scrdmp` (`tos-api/src/xbios/system.rs:61@scrdmp`): `_dumpflg` = -1.
6. `adopt_page0_write` (`vectors.rs:286@adopt_page0_write`): takes a super-state
   program's direct write to `$008-$407` over as the shared cell (BIOS16).
7. `write_cookie_jar` / `refresh_cookie_mirrors` (`loader.rs:1347@write_cookie_jar`,
   `:1409@refresh_cookie_mirrors`): the jar array behind `_p_cookies`, not a
   `tosvars.ld` variable.

**Readers/watchers, not writers:** `PAGE0_HOOKS`
(`tos-api/src/bdos/process/hooks.rs:283@PAGE0_HOOKS`, 10 cells) only compares
and warns (ADR-031 rule 4); `current_conterm`
(`tos-api/src/bios/chardev.rs:484@current_conterm`) reads `_conterm`.

**Searched for timer, VBL and console writers:** there is none beyond the list.
The timer writes only `_hz_200` (4). There is no VBL interrupt; the frame
work (`kernel/src/vbl.rs`) runs `vb_draw` on a derived ~60 Hz frame clock and
touches no page-0 cell. The console code reads `conterm`, writes nothing.
A program may also write a cell itself in the super state
(`docs/api/_super-state.md` S2); that is not a kernel writer.

## 4. The table

`Size`: B byte, W word, L long, `nL` n longs. `EmuTOS writer(s)`: the code that
stores the cell (cites `references/emutos/`), then the readers that matter;
"none" = no store in the whole tree beyond the first-boot clear
(`bios/memory.S:566-569`). Every EmuTOS row was found by a word-boundary
`grep` of the symbol over `*.c`, `*.S`, `*.h` of the pinned tree, not taken
from the older table.

| Address | Name | Size | EmuTOS writer(s) / readers | NewTOS writer | Class | Reason / note |
|---|---|---|---|---|---|---|
| `0x380` | `_proc_lives` | L | `bios/vectors.S:274`, `bios/panicasm.S:70` (crash only); read `bios/kprint.c:315` | `tos-api/src/bios/vectors.rs:185@write_page0_vectors` writes the shared vector cell there (vectors `0xE0`-`0xFA`, `JUST_RTS`), no crash dump | **hardware** | m68k exception state; AArch64 has no m68k trap/exception frame (ADR-031, row "trap and CPU vectors"); a crash is reported by the kernel, not in page 0 (`docs/api/bios/_exception-crash.md`). EmuTOS cold boot holds `user_vec` addresses here (`bios/vectors.S:125-136`), as vector cells, until a crash |
| `0x384` | `_proc_dregs` | 8L | `bios/vectors.S:229-234`, `bios/panicasm.S:48-53` (crash only); read `bios/kprint.c:529-531` | `tos-api/src/bios/vectors.rs:185@write_page0_vectors` writes the shared vector cell there (vectors `0xE0`-`0xFA`, `JUST_RTS`), no crash dump | **hardware** | as `_proc_lives` |
| `0x3A4` | `_proc_aregs` | 8L | `bios/vectors.S:241` (a7; a0-a6 are stored by the d0-d7/a0-a6 `movem` at `:234`); read `bios/kprint.c:533-535` | `tos-api/src/bios/vectors.rs:185@write_page0_vectors` writes the shared vector cell there (vectors `0xE0`-`0xFA`, `JUST_RTS`), no crash dump | **hardware** | as `_proc_lives` |
| `0x3C4` | `_proc_enum` | L | `bios/vectors.S:246,259`; read `bios/kprint.c:319-431` | `tos-api/src/bios/vectors.rs:185@write_page0_vectors` writes the shared vector cell there (vectors `0xE0`-`0xFA`, `JUST_RTS`), no crash dump | **hardware** | as `_proc_lives` |
| `0x3C8` | `_proc_usp` | L | `bios/vectors.S:237`, `bios/panicasm.S:56`; read `bios/kprint.c:537-550` | `tos-api/src/bios/vectors.rs:185@write_page0_vectors` writes the shared vector cell there (vectors `0xE0`-`0xFA`, `JUST_RTS`), no crash dump | **hardware** | as `_proc_lives` |
| `0x3CC` | `_proc_stk` | 16W | `bios/vectors.S:266-272`; read `bios/kprint.c:322-541` | `tos-api/src/bios/vectors.rs:185@write_page0_vectors` writes the shared vector cell there (vectors `0xE0`-`0xFA`, `JUST_RTS`), no crash dump | **hardware** | as `_proc_lives` |
| `0x400` | `sysvars_start` | - | none (linker symbol; start of the first-boot clear `bios/memory.S:566-569`) | none | **n/a** | Address marker, not a variable. |
| `0x400` | `_etv_timer` | L | `bios/bios.c:360`, `bdos/time.c:167`, `vdi/vdi_misc.c:132,149` (`Setexc(0x100)`); called `bios/vectors.S:542` | `tos-api/src/bios/vectors.rs:185@write_page0_vectors`; `Setexc` stores `tos-api/src/bios/vectors.rs:159@write_unaligned` | **served** | Cell only, holds `JUST_RTS`; NewTOS never calls it (ADR-031, ACCEPTED BIOS-CORE-002); EmuTOS value is its own chain head. |
| `0x404` | `_etv_critic` | L | `bios/bios.c:361`; called `bios/vectors.S:813` | `tos-api/src/bios/vectors.rs:185@write_page0_vectors` | **served** | Cell only (`JUST_RTS`), never called (ADR-031). |
| `0x408` | `_etv_term` | L | `bios/bios.c:362`; read via `Setexc(0x102,-1)` `bdos/proc.c:606` | `tos-api/src/bios/vectors.rs:185@write_page0_vectors` | **served** | Cell, and called by `Pterm` (`docs/api/bios/05-setexc.md`). |
| `0x40C` | `_etv_xtra` | L | none; never read | `tos-api/src/bios/vectors.rs:83@default_of` (0) | **served** | 0 on both sides (`bios-core-r1` case 58). |
| `0x420` | `_memvalid` | L | `bios/memory.S:584` (first boot; `:578` ColdFire); cleared `bios/panicasm.S:121,132`; read `:126,135` | `tos-api/src/bdos/process/loader.rs:1217@MEMVALID_MAGIC` | **served** | Static; the kernel compares it at reset (`docs/api/_super-state.md` S10). Oracle read: `bios-core-r1` case 59. |
| `0x424` | `memctrl` | B | `bios/memory.S:572`; read `:146` | none | **hardware** | Copy of the ST MMU bank register `$FFFF8001` (`tosvars.ld:39`); there is no memory controller (CLAUDE.md non-goal: no hardware-level Atari emulation). Value 0. |
| `0x426` | `resvalid` | L | none (cleared `bios/panicasm.S:115`, `util/bootram.S:95`); read `bios/startup.S:504,507` | `tos-api/src/bdos/process/loader.rs:1222@wr_long` (0) | **served** | 0 as on EmuTOS cold boot; reset vector never called (ADR-031). |
| `0x42A` | `resvector` | L | none (cleared `bios/panicasm.S:116`); read `bios/startup.S:510` | `tos-api/src/bdos/process/loader.rs:1223@wr_long` (0) | **served** | As `resvalid`; watched by `tos-api/src/bdos/process/hooks.rs:284@Resvector`. |
| `0x42E` | `_phystop` | L | `bios/memory.S:573`; read `bios/biosmem.c:72` | `tos-api/src/bdos/process/loader.rs:1227@stram_end` | **served** | Static: end of the ST-RAM pool. |
| `0x432` | `_membot` | L | `bios/biosmem.c:67,122`; read `:109,145` | `tos-api/src/bdos/process/loader.rs:1228@stram_start` | **served** | Static; EmuTOS moves it on each `balloc_stram` (`bios/biosmem.c:122`), NewTOS has no `balloc_stram`. |
| `0x436` | `_memtop` | L | `bios/biosmem.c:72,115`; read `:146` | `tos-api/src/bdos/process/loader.rs:1229@stram_end` | **served** | Static, as `_membot` (`bios/biosmem.c:115`). |
| `0x43A` | `_memval2` | L | `bios/memory.S:585`; read `:129,137` | `tos-api/src/bdos/process/loader.rs:1218@MEMVAL2_MAGIC` | **served** | Static; kernel reset compares it (S10). |
| `0x43E` | `_flock` | W | `bios/acsi.c:369,377`, `bios/clock.c:168,187`, `bios/scsi.c:1246,1285`, `bios/floppy.c:1292,1329`; read `bios/floppy.c:1347` | none | **hardware** | Locks the VBL floppy routine against ACSI/SCSI/RTC I/O; no floppy, no ACSI (ADR-012 `_FDC` absent, `docs/COMPATIBILITY.md:1662`; floppy group `:1935`). 0 = EmuTOS rest value. |
| `0x440` | `_seekrate` | W | `bios/floppy.c:309` (`DD_STEPRATE_3MS` = 3, `:79`); read `:294` | none | **hardware** | Floppy step rate; no floppy (`docs/COMPATIBILITY.md:1935`). NewTOS reads 0, EmuTOS 3. |
| `0x442` | `_timer_ms` | W | `bios/mfp.c:196` (= 20); read `bios/vectors.S:541` | none | **missing** | Value 20 is what `Tickcal` answers on NewTOS (`tos-api/src/bios/mod.rs:95@TICKCAL_MS`); the cell reads 0. |
| `0x444` | `_fverify` | W | `bios/floppy.c:308` (= 0xff); read `:1200` | none | **hardware** | Floppy write verify; no floppy (`docs/COMPATIBILITY.md:1935`). NewTOS reads 0, EmuTOS 0xFF. |
| `0x446` | `_bootdev` | W | `bios/bios.c:611,799,810`, `bios/blkdev.c:232`; read `bios/bios.c:827` | `tos-api/src/bdos/process/loader.rs:1234@bootdev` | **served** | Static (`tos-api/src/bdos/fs/drive.rs:55@boot_drive`). Read by `sdk/overlays/xgdos/xos_newtos.c:87@bootdev_s`, `sdk/apps/hypview/init.c:47@_bootdev`. |
| `0x448` | `_palmode` | W | none (`bios/screen.c:607` is a local BOOL, not this cell) | none (page 0 zeroed, `tos-api/src/bdos/process/loader.rs:671@fill`) | **served** | 0 on both sides; no EmuTOS code touches it beyond the boot clear. |
| `0x44A` | `_defshiftmod` | B | none; read `bios/screen.c:1302` | none (zero page) | **served** | 0 on both sides (EmuTOS never writes it). |
| `0x44C` | `_sshiftmod` | B | `bios/screen.c:564,580,1327`, `bios/videl.c:651`; read `bios/screen.c:791,852,1082` | none | **hardware** | Copy of the Shifter mode register `$FFFF8260` (`tosvars.ld:54`); no Shifter, resolution comes from `config.txt` (ADR-017 Revision 2). NewTOS reads 0 (= ST low) at any real resolution. |
| `0x44E` | `_v_bas_ad` | L | `bios/screen.c:656,1107,1139`, `bios/vectors.S:396`, `bios/nova.c:725`; read `bios/conout.c:126,507,712,739`, `bios/vt52.c:930`, the VDI (`vdi/vdi_raster.c:927,942`, `vdi/vdi_misc.c:172`, `vdi/vdi_control.c:618`, `vdi/vdi_fill.c:1175,1187`, `vdi/vdi_textblit.c:1033,1226`, `vdi/vdi_inline.h:16`), `bios/screen.c:1073` (`Logbase`) | none | **missing** | Software variable (logical screen base), the framebuffer exists. Classic idiom `*(LONG*)0x44E`; `Logbase()` answers it on NewTOS. Named in `docs/COMPATIBILITY.md:46`. |
| `0x452` | `_vblsem` | W | `bios/bios.c:553`, `bios/screen.c:501,1158,1163`, `bios/vectors.S:336,486,489` | none | **missing** | EmuTOS boot value 1; NewTOS has no VBL interrupt (`docs/COMPATIBILITY.md:1410`). |
| `0x454` | `_nvbls` | W | `bios/bios.c:366` (= 8); read `bios/vectors.S:440` | none | **missing** | EmuTOS 8; NewTOS 0 (with `_vblqueue` 0 a coherent empty queue, inferred). |
| `0x456` | `_vblqueue` | L | `bios/bios.c:367`, `vdi/vdi_mouse.c:602,631` (slot 0); read `bios/vectors.S:447` | none; watched `tos-api/src/bdos/process/hooks.rs:285@Vblqueue` (default 0) | **hardware** | ADR-031 row "`dump_vec`, `swv_vec`, `exec_os`, `resvector`/`resvalid`, `vblqueue`": not served, warning. NewTOS has no VBL. |
| `0x45A` | `_colorptr` | L | `bios/screen.c:1182`, `bios/videl.c:836,854`, cleared `bios/vectors.S:370`; read `:367` | none | **hardware** | Palette to load into the Shifter at the next VBL; no Shifter palette registers, no VBL (CLAUDE.md non-goal; `docs/COMPATIBILITY.md:1410`). |
| `0x45E` | `_screenpt` | L | none; read `bios/vectors.S:394,424` (VBL base-address load) | none (zero page) | **served** | 0 on both sides: EmuTOS never writes it, and `bios/vectors.S:395` skips the load when it is 0. |
| `0x462` | `_vbclock` | L | `bios/vectors.S:348` (VBL); no C reader | none | **missing** | VBL-executed counter. NewTOS has no VBL interrupt (`docs/COMPATIBILITY.md:1410`) and nothing mirrors a count into `$462`. **Corrects `docs/api/bios/_boot-init.md` section 7c ("keins gefunden").** |
| `0x466` | `_frclock` | L | `bios/vectors.S:330` (VBL); read `bios/screen.c:1217-1218` (`Vsync`), `bios/floppy.c:1351,1359` | none | **missing** | Free-running frame counter; stays 0 on NewTOS, so a program that waits for it to change never leaves its loop. A derived ~60 Hz frame clock exists (`kernel/src/host_env.rs:262@frame_clock`; `Vsync` spins on it, `tos-api/src/xbios/system.rs:35@frame_clock`) but is not mirrored into `$466`. |
| `0x46A` | `_hdv_init` | L | `bios/blkdev.c:98` (= `flop_hdv_init`); never called (`:97`) | none | **missing** | Never called by EmuTOS either (ADR-031 rule 1: no warning). NewTOS reads 0. |
| `0x46E` | `_swv_vec` | L | `bios/bios.c:363,552`; called `bios/screen.c:1329` | `tos-api/src/bdos/process/loader.rs:1242@RESET_ENTRY_VA` | **served** | Static (`reseth`); never called (ADR-031). Measured, `measurements/measurement-2026-10-30-bios16/RUNINFO.TXT` V01. |
| `0x472` | `_hdv_bpb` | L | `bios/blkdev.c:101`; called `bios/bios.c:1122` | none; watched `tos-api/src/bdos/process/hooks.rs:290@HdvBpb` (default 0) | **hardware** | ADR-031 row "`hdv_*` floppy use": not served, warning; no floppy/DMA. NewTOS reads 0. |
| `0x476` | `_hdv_rw` | L | `bios/blkdev.c:102`; called `bios/bios.c:1047` | none; watched `tos-api/src/bdos/process/hooks.rs:291@HdvRw` (default 0) | **hardware** | As `_hdv_bpb`. |
| `0x47A` | `_hdv_boot` | L | `bios/blkdev.c:95`; never called | none | **missing** | Never called by EmuTOS (ADR-031 rule 1). **Corrects `_boot-init.md` section 7c ("boot-abhaengig gesetzt").** |
| `0x47E` | `_hdv_mediach` | L | `bios/blkdev.c:103`; called `bios/bios.c:1186` | none; watched `tos-api/src/bdos/process/hooks.rs:292@HdvMediach` (default 0) | **hardware** | As `_hdv_bpb`; TeraDesk `force_mediach` writes it (`sdk/overlays/teradesk/file.c:28-36@hdv_mediach`). |
| `0x482` | `_cmdload` | W | none; read `bios/bios.c:850` | `tos-api/src/bdos/process/loader.rs:1246@wr_word` (0) | **served** | 0 on both sides. |
| `0x484` | `_conterm` | B | `bios/ikbd.c:1100` (= 7); read `bios/ikbd.c:279,475,495`, `bios/vt52.c:215` | `tos-api/src/bdos/process/loader.rs:1253@CONTERM_DEFAULT`; read per process `tos-api/src/bios/chardev.rs:484@current_conterm` | **served** | Live: the kernel reads the cell, bits 0-3 only (`docs/api/_super-state.md` S10). Read by `sdk/examples/p0kid.c:103@conterm`. |
| `0x486` | `trp14ret` | L | none | none (zero page) | **served** | Unused on EmuTOS (`tosvars.ld:71`); 0 on both sides. |
| `0x48A` | `criticret` | L | none | none (zero page) | **served** | No EmuTOS reference; 0 on both sides. |
| `0x48E` | `_themd` | 16B | `bios/biosmem.c:144-147` (inside `Getmpb`); handed out as `mp_mfl` `:149`; afterwards live BDOS state (head of `pmd.mp_mfl`, split by `bdos/iumem.c:100-135`) | none | **missing** | NewTOS `Getmpb` answers a kernel `THEMD`, not `$48E` (`docs/api/bios/00-getmpb.md` V1); a direct read of `$48E` sees zeroes. Low impact (no in-repo reader). |
| `0x49E` | `____md` | 4B | none | none (zero page) | **served** | No EmuTOS reference; 0 on both sides. |
| `0x4A2` | `_savptr` | L | `bios/bios.c:359`, `bios/kprint.c:587`, `bios/vectors.S:1317,1356`; read `:1283,1335` | none | **hardware** | Register-save stack of the m68k BIOS/XBIOS trampoline; NewTOS dispatches by SVC (`docs/api/bios/_dispatcher.md:167`; ADR-031 row on m68k vectors). |
| `0x4A6` | `_nflops` | W | `bios/floppy.c:319,386`; read `:1343,1358` | none | **hardware** | 0 floppies is the true answer of a machine without floppy controller (ADR-012 `_FDC` absent, `docs/COMPATIBILITY.md:1662`; `:1935`). EmuTOS on a floppy-less machine also computes 0 (`bios/floppy.c:319,386`, inferred). |
| `0x4A8` | `_con_state` | L | `bios/vt52.c:190-331,950`; read `:153-159` | none | **free** | EmuTOS-private VT52 state-machine pointer, read only inside `vt52.c`; points into m68k code (`docs/COMPATIBILITY.md` audit rubric item 2). NewTOS VT52 is native (`docs/api/bios/_vt52.md`). |
| `0x4AC` | `_save_row` | W | `bios/vt52.c:263`; read `:279` | none | **free** | VT52 `ESC Y` scratch, read only in `vt52.c`. |
| `0x4AE` | `sav_context` | L | none | none (zero page) | **served** | No EmuTOS reference; 0 on both sides. |
| `0x4B2` | `_bufl` | 2L | `bdos/fsbuf.c:67,71`; read `bdos/fsio.c:61`, `bdos/bdosmain.c:418`, `bdos/fsopnclo.c:577` | none | **free** | Heads of the EmuTOS BDOS buffer lists, used only in `bdos/`; NewTOS GEMDOS has its own FAT cache (inferred, not read in this run). |
| `0x4BA` | `_hz_200` | L | `bios/vectors.S:503` (Timer C); read by many | `tos-api/src/bdos/process/loader.rs:1262@HZ_200_ADDR` seed; live `tos-api/src/bios/tick.rs:108@publish`, called from `kernel/src/drivers/timer.rs:171@advance` | **served** | Live for the **running** process only (`tos-api/src/bios/tick.rs:125@write_unaligned`); a suspended process sees its last value. Read by Doom (`sdk/apps/doom/doomgeneric_gem.c:675@_hz_200`), PRESENT, `sdk/examples/fsbench.c:82@_hz_200`. |
| `0x4BE` | `the_env` | L | none | none (zero page) | **served** | EmuTOS keeps its environment in a private static (`bios/bios.c:80`); 0 on both sides. |
| `0x4C2` | `_drvbits` | L | `bios/blkdev.c:175,392`, `bios/floppy.c:387`; read `bios/blkdev.c:835`, `aes/gemfslib.c:699`, `bios/initinfo.c:230` | `tos-api/src/bdos/process/loader.rs:1264@drvbits` | **served** | Static (snapshot at load); `Drvmap` answers the live value (`tos-api/src/bios/drive.rs:188@drvbits`). Read by `sdk/apps/apitest/apitest.c:936@drvbits`. Not refreshed after load (no writer found). |
| `0x4C6` | `_dskbufp` | L | `bios/blkdev.c:91`; read `bios/acsi.c:156-670`, `bios/floppy.c:352`, `bios/scsi.c:387`, `bios/scsidriv.c:314`, `bios/ide.c:1379`, `bios/disk.c:162-215`, `bios/blkdev.c:266-309,626-638,766`, `bios/clock.c:1087-1125` | none | **missing** | Disk buffer pointer (3072 bytes in low ST-RAM, `bios/lowstram.c`); readers are the disk drivers, the boot-sector execution and the BPB scan, the RTC code (corrected in BIOS20: not "ACSI/floppy only"). Low impact; no ADR names it (ADR-032 covers XHDI/SCSI API only). |
| `0x4CA` | `_autopath` | L | none | none (zero page) | **served** | EmuTOS iterates `AUTO` itself (`bios/bios.c:846`); 0 on both sides. |
| `0x4CE` | `_vbl_list` | 8L | `bios/bios.c:371` (NULL); `vdi/vdi_mouse.c:602,631` (slot 0 = `vb_draw`) | none | **hardware** | The queue behind `_vblqueue` (ADR-031 row, no VBL). Slots 1-7 equal (NULL); slot 0 is the VDI mouse draw routine on EmuTOS. |
| `0x4EE` | `_dumpflg` | W | `bios/bios.c:357` (-1), `bios/xbios.c:493`, `bios/ikbd.c:596` (`++`), `bios/vectors.S:472`; read `:467` | `tos-api/src/bdos/process/loader.rs:1257@DUMPFLG_ADDR`; `tos-api/src/xbios/system.rs:65@write_unaligned` (`Scrdmp`) | **served** | Boot value -1 and `Scrdmp` as EmuTOS; Alt+Help does not increment it (`docs/DEVIATIONS.md`, KEYS05). |
| `0x4F0` | `_prtabt` | W | none | none (zero page) | **served** | No EmuTOS reference; 0 on both sides. |
| `0x4F2` | `_sysbase` | L | `bios/bios.c:358` | `tos-api/src/bdos/process/loader.rs:1268@OSHEADER_VA` | **served** | Static; read by TeraDesk, Doom, apitest (`sdk/overlays/teradesk/library/utility/gettos.c:31@_shell_p`). |
| `0x4F6` | `_shell_p` | L | none | none (zero page) | **served** | 0 on both sides (`sdk/overlays/teradesk/library/utility/gettos.c:31@_shell_p` reads the neighbour). |
| `0x4FA` | `_end_os` | L | `bios/biosmem.c:66,125` | `tos-api/src/bdos/process/loader.rs:1274@stram_start` | **served** | Static; EmuTOS moves it with `membot` (`biosmem.c:125`). |
| `0x4FE` | `_exec_os` | L | `bios/bios.c:534`; read `:856-863` | none; watched `tos-api/src/bdos/process/hooks.rs:293@ExecOs` (default 0) | **hardware** | ADR-031 row (not served, warning): the AES entry has no EL0 value (`loader.rs` os_magic note). |
| `0x502` | `_dump_vec` | L | `bios/chardev.c:84` (`just_rts`); called `bios/xbios.c:492`, `bios/vectors.S:469` | none; watched `tos-api/src/bdos/process/hooks.rs:294@DumpVec` (default 0) | **hardware** | ADR-031 row (not served). **Value 0, not the `just_rts` stub** (`prt_stat`/`prt_vec` get the stub, `dump_vec` does not). |
| `0x506` | `_prt_stat` | L | `bios/chardev.c:82` | `tos-api/src/bdos/process/loader.rs:1303@RET_STUB_VA` | **served** | Stub like EmuTOS `just_rts` (ADR-031 row `prt_stat`, `prt_vec`). |
| `0x50A` | `_prt_vec` | L | `bios/chardev.c:83` | `tos-api/src/bdos/process/loader.rs:1304@RET_STUB_VA` | **served** | As `_prt_stat`. |
| `0x50E` | `_aux_stat` | L | `bios/chardev.c:78` | none | **missing** | EmuTOS `just_rts`, NewTOS 0: the ADR-031 stub row names `prt_*` only (`tos-api/src/bdos/process/hooks.rs:755@silent` lists it as silent). |
| `0x512` | `_aux_vec` | L | `bios/chardev.c:79` | none | **missing** | As `_aux_stat`. |
| `0x516` | `_pun_ptr` | L | `bios/blkdev.c:161`; read `bdos/fsbuf.c:61` | none | **missing** | AHDI marker; ADR-032 covers XHDI/SCSI, not this cell. A program probing AHDI via `$516` reads 0 = no AHDI (low impact). |
| `0x51A` | `_memval3` | L | `bios/memory.S:586`; read `:132,139` | `tos-api/src/bdos/process/loader.rs:1219@MEMVAL3_MAGIC` | **served** | Static; kernel reset compares it (S10). |
| `0x51E` | `_bconstat_vec` | 8L | `bios/chardev.c:69`, `bios/serport.c:998-1001,1114-1117`; called `bios/bios.c:932` | none; not watched in `PAGE0_HOOKS` | **missing** | EmuTOS dispatches `Bconstat` through this table, so a program that patches a slot redirects the device; NewTOS dispatches natively and neither reads nor warns about it (ADR-031 rule 1 names no such hook). |
| `0x53E` | `_bconin_vec` | 8L | `bios/chardev.c:71`, `bios/serport.c:998-1001,1114-1117`; called `bios/bios.c:976` | none; not watched in `PAGE0_HOOKS` | **missing** | EmuTOS dispatches `Bconin` through this table, so a program that patches a slot redirects the device; NewTOS dispatches natively and neither reads nor warns about it (ADR-031 rule 1 names no such hook). |
| `0x55E` | `_bcostat_vec` | 8L | `bios/chardev.c:73`, `bios/serport.c:998-1001,1114-1117`; called `bios/bios.c:1161` | none; not watched in `PAGE0_HOOKS` | **missing** | EmuTOS dispatches `Bcostat` through this table, so a program that patches a slot redirects the device; NewTOS dispatches natively and neither reads nor warns about it (ADR-031 rule 1 names no such hook). |
| `0x57E` | `_bconout_vec` | 8L | `bios/chardev.c:75`, `bios/serport.c:998-1001,1114-1117`; called `bios/bios.c:1009` | none; not watched in `PAGE0_HOOKS` | **missing** | EmuTOS dispatches `Bconout` through this table, so a program that patches a slot redirects the device; NewTOS dispatches natively and neither reads nor warns about it (ADR-031 rule 1 names no such hook). |
| `0x59E` | `_longframe` | W(L) | `bios/processor.S:261`; read `bios/vectors.S:253,1305,1348`, `bios/bios.c:183` | none (0 = 68000) | **hardware** | m68k stack-frame format; no m68k CPU. Read by EmuTOS own trampolines only. |
| `0x5A0` | `_p_cookies` | L | `util/cookie.c:31`; read `util/cookie.c:37,68` | `tos-api/src/bdos/process/loader.rs:1276@COOKIE_JAR_VA`; jar `tos-api/src/bdos/process/loader.rs:1347@write_cookie_jar` | **served** | Static pointer to the per-process jar mirror (ADR-012; `docs/api/bios/_cookie-jar.md`). Read by apitest, TeraDesk. Oracle read: `bios-core-r1` case 60. |
| `0x5A4` | `_ramtop` | L | `bios/memory2.c:109,119,121`, `bios/memory.S:598` | `tos-api/src/bdos/process/loader.rs:1294@ttram_end` | **served** | Static. Oracle read: case 61. |
| `0x5A8` | `_ramvalid` | L | `bios/memory2.c:124,129`, `bios/memory.S:599` | `tos-api/src/bdos/process/loader.rs:1295@RAMVALID_MAGIC` | **served** | Static. Oracle read: case 62. |
| `0x5AC` | `_bell_hook` | L | `bios/sound.c:90`; called `:253` | none; watched `tos-api/src/bdos/process/hooks.rs:295@BellHook` (default 0) | **hardware** | ADR-031 row (not served, warning; no sound device yet). EmuTOS value is `do_bell`. |
| `0x5B0` | `_kcl_hook` | L | `bios/sound.c:91`; called `bios/vectors.S:1434` | none; watched `tos-api/src/bdos/process/hooks.rs:296@KclHook` (default 0) | **hardware** | ADR-031 row (as `_bell_hook`). |
| `0x6FC` | `_warm_magic` | L | `bios/memory.S:590` (only with `CONF_DETECT_FIRST_BOOT_WITHOUT_MEMCONF`); read `:176` | none | **n/a** | Not in the oracle build: `CONF_DETECT_FIRST_BOOT_WITHOUT_MEMCONF` is 0 for `make 512` (`docs/api/_emutos-config.md:278`, `include/config.h:1414-1418`) and the oracle map has no such symbol. |
| `0x700` | `_pmmutree` | L | `bios/pmmu030.c:152` (68030 only); `CONF_WITH_68030_PMMU` = 1 in `make 512` | none (zero page) | **hardware** | 68030 PMMU tree; NewTOS uses the AArch64 MMU (ADR-028). The area `$700-$767` is free in NewTOS page 0. |

## 5. The "missing" variables: who reads them

17 rows. "EmuTOS reader" is read from the source (cites above); "program" is
the survey of section 7 (none found means none of the searched trees names it).

| Variable | EmuTOS code that reads it | Program in the searched trees | Weight (inferred) |
|---|---|---|---|
| `_v_bas_ad` `$44E` | `bios/conout.c:126,507`, `bios/vt52.c:930`, the VDI (`vdi/vdi_raster.c:927,942`, `vdi/vdi_misc.c:172`, `vdi/vdi_control.c:618`, `vdi/vdi_fill.c:1175,1187`, `vdi/vdi_textblit.c:1033,1226`); also the boot-sector code `util/bootsect.S:77` | none; only declared in `sdk/libc/overlay/include/mint/sysvars.h` and checked in `sdk/libc/tests/sysvars-layout.c:67@_v_bas_ad` | the classic idiom, named in `docs/COMPATIBILITY.md:46`; a read returns 0 |
| `_frclock` `$466`, `_vbclock` `$462` | `Vsync` `bios/screen.c:1217-1218`; `bios/floppy.c:1351,1359` | none | a program that spins on `*_frclock` never ends; a frame clock exists (`kernel/src/host_env.rs:262@frame_clock`) |
| `_vblsem` `$452`, `_nvbls` `$454` | `bios/vectors.S:334-338,440` | none | low; coherent with `_vblqueue` 0 (inferred) |
| `_timer_ms` `$442` | `bios/vectors.S:541` | none (`programs/newtos-gem/src/aes.rs:252@timer_ms` is an unrelated `MU_TIMER` field) | low; `Tickcal` answers the same 20 |
| `_hdv_init` `$46A`, `_hdv_boot` `$47A` | none: EmuTOS never calls them (`bios/blkdev.c:97`) | none | low; ADR-031 rule 1 excludes them from the warning |
| `_bconstat_vec`, `_bconin_vec`, `_bcostat_vec`, `_bconout_vec` `$51E-$59D` | `bios/bios.c:932,976,1161,1009` (the BIOS calls dispatch through them) | none | a program that patches a slot to redirect a device is silent on NewTOS: not served, not warned (see 6.2) |
| `_aux_stat`, `_aux_vec` `$50E`/`$512` | none (never called) | none | low; the stub row of ADR-031 names `prt_*` only |
| `_themd` `$48E` | `Getmpb` `bios/biosmem.c:144-149` | none | low; `Getmpb` itself is served |
| `_dskbufp` `$4C6` | `bios/acsi.c:156-670`, `bios/floppy.c:352`, `bios/scsi.c:387`, `bios/ide.c:1379`, `bios/disk.c:162-215`, `bios/blkdev.c:266-309,626-638`, `bios/clock.c:1087-1125` (drivers, boot sector, BPB scan, RTC) | none | low |
| `_pun_ptr` `$516` | `bdos/fsbuf.c:61` | none | low; a probe reads 0 = no AHDI |

## 6. Findings beyond the table (for the lead)

1. **Corrections to `docs/api/bios/_boot-init.md` section 7c** (not made in
   place: the file is German and a touch would call for translating it, see
   Open points). Against the source:
   * `vbclock` is **not** "keins gefunden": `bios/vectors.S:348` increments it.
   * `frclock` is written by the VBL (`bios/vectors.S:330`); its readers are
     `Vsync` (`bios/screen.c:1217`) and `bios/floppy.c:1351,1359`, **not**
     "Random": `Random` seeds from `hz_200` (`bios/xbios.c:424-425`).
   * `hdv_boot` is set unconditionally at `bios/blkdev.c:95`, not
     "boot-abhaengig", and never called.
   * `screenpt` has **no** writer ("Screen-Umschaltung" is wrong); it is only
     read (`bios/vectors.S:394,424`).
   * `defshiftmod` has **no** writer ("screen-Init" is wrong); it is read at
     `bios/screen.c:1302`.
   * `hz_200` is written by the Timer C handler `bios/vectors.S:503`, not by
     "init_system_timer".
   * `vbl_list[0]` is not NULL after VDI init: `vdi/vdi_mouse.c:602,631`
     stores `vb_draw` there.
   * `dumpflg` is also written by the VBL after a dump (`bios/vectors.S:472`)
     and incremented by the Help key (`bios/ikbd.c:596`).
2. **ADR-031 rule 1 ("never silent") has a gap:** the four `bcon*_vec` tables
   (32 longs) are hooks EmuTOS calls through (`bios/bios.c:932,976,1009,1161`)
   and `PAGE0_HOOKS` (`tos-api/src/bdos/process/hooks.rs:283@PAGE0_HOOKS`) does
   not watch them; the ADR-031 table does not list them either.
3. **ADR-031 stub asymmetry:** `prt_stat`/`prt_vec` hold the `ret` stub like
   EmuTOS `just_rts`; `dump_vec` (`bios/chardev.c:84`) and `aux_stat`/`aux_vec`
   (`bios/chardev.c:78-79`) hold 0 although EmuTOS boots them with
   `just_rts`. A program that chains to the old value branches to address 0.
4. **Zero where EmuTOS has a value, observable by a plain read:** `_nflops` 0,
   `_seekrate` 0 (EmuTOS 3), `_fverify` 0 (EmuTOS 0xFF), `_sshiftmod` 0,
   `_longframe` 0, `_vbclock`/`_frclock` 0, `_v_bas_ad` 0. The first three, the
   shifter copy and `_longframe` are "hardware"; the rest "missing".
5. **`_hz_200` is per process and live only while the process runs** (writer
   4 above): a process that is switched out and back sees its old value until
   the next 200 Hz tick; `_drvbits` and `_bootdev` are snapshots of load time.
6. **COMPATIBILITY.md** (read, not edited). The sentence at `:2150-2152`
   ("NewTOS keeps them **compatible in value and semantics**") holds for the 37
   served rows only. The ticket names `:1880-1907`; that text now stands at
   `:2148-2165`. Proposed wording, to be added after that paragraph:

   > Which variables are served, which are absent by decision and which are
   > not provided yet is the per-variable table in
   > `docs/api/bios/_sysvars-contract.md` (84 assignments of `tosvars.ld`:
   > served 37, hardware/ADR 25, free 3, not provided 17, not variables 2). A
   > program reading a variable of the last group sees 0.

## 7. Program survey (who reads which variable)

Searched with `/usr/bin/grep` (word match of the `_name` and the number forms):
`sdk/apps/`, `sdk/overlays/`, `sdk/examples/`, `sdk/libc`, `sdk/zview`,
`sdk/ldg`, `sdk/libm`, `sdk/gemlib`, `sdk/cflib`, `programs/`, `tosdump/`.
The local-only upstream-source tree (`Applications`, mintlib/windom/etc.) is
not linked into this worktree and was not searched: its headers define every
name, so a hit there says nothing about a ported program.

Hits that matter: `_bootdev` (`sdk/overlays/xgdos/xos_newtos.c:87@bootdev_s`,
`sdk/apps/hypview/init.c:47@_bootdev`), `_hz_200` (Doom `sdk/apps/doom/doomgeneric_gem.c:675@_hz_200`,
PRESENT, `sdk/examples/fsbench.c:82@_hz_200`), `_sysbase`/`_shell_p`
(TeraDesk `sdk/overlays/teradesk/library/utility/gettos.c:31@_shell_p`, Doom, apitest),
`hdv_*` (TeraDesk `sdk/overlays/teradesk/file.c:28-36@hdv_mediach`),
`resvalid`/`resvector` (TeraDesk `sdk/overlays/teradesk/library/utility/cookie.c:104-123@resvalid`),
`_p_cookies`/`_drvbits`/`_membot`/`_memtop`/`_memvalid` (apitest
`sdk/apps/apitest/apitest.c:680@_p_cookies`, `:924@memvalid`, `:936@drvbits`, `:2232@membot`), `etv_term` (`$408`, Doom
`sdk/apps/doom/doomgeneric_gem.c:1881@kbdvec_etv_old`, apitest), `_conterm`
(`sdk/examples/p0kid.c:103@conterm`), `_prt_stat`/`_prt_vec`/`dumpflg`/`etv_*`
(`tosdump/probes/hkprobe.c`, `vdprobe.c`, `vwprobe.c`, test programs). **No
searched program reads a "missing" variable** (`timer_ms` in `programs/newtos-gem/src/aes.rs:252@timer_ms` is an AES event field, not
the `$442` cell).

**"No local source" gap for the standard image.** Every executable of the
standard QEMU image maps to a recipe in `tools/make-sd-image.sh`: the
`REQUIRED` list (`:69-94`: argvtest, libtest, clocktest, setjmptst, MXPROBE,
mathtst, EX_LIB/LDGTEST, ZVIEW, CORE/COREVAL, DUMPVDI, VDIDRAW, QED,
TeraDesk, PRESENT, each with its build command), the `build_example` lines
(hello, big, physbase, scrflip, curdir, cmdline, dtachild, ptermch, ctrlcpar,
fppexec, sysinfo, usbsnap, sysacc: `sdk/examples/*.c`), and `HELLO.NCP`
(generated by `tools/mk-hello-ncp.py`, no sysvar access). Not enumerated in
this run: executables the script copies after `:635` and the `sdk/zview`,
`sdk/libc/tests` sources (only grepped). The gap is empty by construction for
what is listed; I did not read every line of the script (open point).

## 8. Proposal: the read-case type for the served variables (not built)

**Problem.** Of the 37 served rows only 11 sit at a 4-aligned address a
`Setexc(num,-1)` LONG read can reach (`_etv_timer`, `_etv_critic`, `_etv_term`,
`_etv_xtra`, `_memvalid`, `_palmode`, `_conterm`, `_prtabt`, `_p_cookies`,
`_ramtop`, `_ramvalid`), and of these `_palmode`, `_conterm`, `_prtabt` are
narrower than the LONG that `Setexc` returns. The classic layout from `$426`
on is 2-mod-4 for **55 of the 84** cells, so `Setexc` (which reads `4 * num`,
`tos-api/src/bios/vectors.rs:155@cell`; EmuTOS `bios/bios.c:1070-1080`)
cannot read 26 of the 37 served cells. The 8 oracle read cases that exist are
`bios-core-r1.tsv` rows 55-62 (`$400`-`$40C`, `$420`, `$5A0`-`$5A8`).

**Proposal: a `peek` case type through `Supexec`.** One new call kind in the
DUMPBIOS table: `peek:<addr>:<width>` with width 1, 2 or 4 (a WORD/BYTE read
must not be widened into the neighbouring cell). The dumper
reads it inside a `Supexec` callback: on EmuTOS a user-mode read below `$800`
is a bus error (the dumper's own note on `$48E`, `tosdump/core/cases.c:4897@themd`; the same reason
`tosdump/probes/hkprobe.c:132@Supexec` already reads the MFP through `Supexec`), and on NewTOS
the same callback runs unchanged. Choosing
the callback over a plain user-mode peek keeps both sides on the same path;
`Supexec` is served on NewTOS (`docs/api/_super-state.md` S6); user-mode reads of
page 0 are allowed there (S2) but not on EmuTOS. `Setexc` stays the case for the 8 existing rows and for any
aligned LONG.

**Guard (`tosdump/core/cases.c` `call_policy`).** Read-only by construction:
address in `[0x380, 0x800)`, width 1/2/4, no write form. The table must
not read the `proc_*` rows after a crash.

**Return class per served row (a prediction from source; the oracle run
decides):**

| Group | Rows | Class |
|---|---|---|
| fixed magic or zero | `memvalid`, `memval2`, `memval3`, `ramvalid`, `resvalid`, `resvector`, `cmdload`, `conterm`, `dumpflg` (-1), the 11 zero rows (`palmode`, `defshiftmod`, `screenpt`, `trp14ret`, `criticret`, `____md`, `sav_context`, `the_env`, `autopath`, `prtabt`, `shell_p`), `etv_xtra` | `EXACT` |
| address | `etv_timer`, `etv_critic`, `etv_term`, `swv_vec`, `sysbase`, `p_cookies`, `end_os`, `membot`, `memtop`, `phystop`, `prt_stat`, `prt_vec` | `PTR_RAM` (non-null, even, below 2 GiB, the existing class) |
| differs by machine | `ramtop` (TT-RAM: 0 on a profile without it), `bootdev`, `drvbits` | `NONZERO` or `IGNORE` after the first measurement, with the reason in the case comment |
| counter | `hz_200` | two peeks with a wait between: nondecreasing; needs a new class (`MONOTONE`) or a `NONZERO` only |

The `hardware` rows need **no** read case on NewTOS side except as a
documented-difference case (value 0 against the oracle); that choice is the
lead's. `_etv_timer` and `_etv_critic` would measure a deliberate difference
(ADR-031), so they stay `PTR_RAM`.

**A developer ticket would carry:** the `peek` parser and runner in
`tosdump/core/` (`cases.c`, `runner_call.c`), the Supexec callback, the table
`bios-sysvars-r1.tsv` under `tosdump/cases` (one row per served variable, about 37),
references measured on `tt` and `falcontc`, and the tosdiff run on NewTOS.
It is a code ticket, not built here.

## Open points

- STOP: no STOP-block. Everything in the table is read from source; no value
  was measured in this run (no QEMU/Hatari run, by the handover).
- Inferred, not read: `_nflops` on a floppy-less EmuTOS computes 0 (from
  `bios/floppy.c:319,386`); `_nvbls`/`_vblsem` "coherent with an empty queue";
  `_bufl` "NewTOS GEMDOS has its own cache" (the NewTOS BDOS was not read in
  this run); `_dskbufp`/`_pun_ptr`/`_themd` weights; the `free` class itself
  (my reading of the COMPATIBILITY audit rubric, the lead may fold it into
  "missing"); the reason column of the crash-dump rows (ADR-031's CPU-vector row
  is about vectors, not the dump area).
- `tools/make-sd-image.sh` was not read past `:635`, so the "gap is empty" claim
  covers the listed executables only.
- `docs/api/bios/_boot-init.md` section 7c carries the errors of 6.1; I did not
  edit it (German file: a touch asks for the translation, owner rule 2026-11-10).
  The lead decides: translate-and-correct, or replace section 7 by a pointer to
  this file.
- Out of scope: `tos-api/src/xbios/mod.rs:154@Supexec` comment lists `Supexec 0x26`
  among catch-all slots while `docs/api/xbios/26-supexec.md` and
  `docs/api/_super-state.md` describe it as served; `tosdump/cases/bios-r1.tsv`
  and `xbios-system-r1.tsv` NVMaccess comments cite `nvram.c:180-181`, which is
  now `:178-179`/`:181-199` (cosmetic, frozen files).
