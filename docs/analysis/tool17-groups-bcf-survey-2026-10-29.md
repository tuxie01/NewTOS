# TOOL17 unmapped groups B, C, F: survey of users and counterparts (2026-10-29)

Dated snapshot, no classification decision. Scope: the three groups of
`unmapped` rows in `docs/analysis/deps/mapping.tsv` that
`TASK/TOOL17-STATUS-DONE.MD` ("Unklare Faelle" B, C, F) hands over.
Marks: **READ** = seen in a file this session, **DERIVED** = concluded from
READ facts (reasoning stated).

Search method (READ): `/usr/bin/grep -rIniE` over `sdk/` (incl. `sdk/apps`,
`sdk/overlays`, `sdk/examples`, libc, gemlib, cflib, windom, zview, ldg),
`programs/`, `tosdump/`, `kernel/`, `tos-api/`, `tos-types/`, `tools/`,
and the sources under `Applications/` (thing, qed, cflib-git, windom,
z-tools, ldg-src, doomgeneric, pml, mintlib-src). `build` directories excluded.

## Card manifest check (READ / DERIVED)

- Standard image: `tools/make-sd-image.sh` builds the executables from
  local sources via its `REQUIRED` list (`tools/make-sd-image.sh (line 60-81)`:
  argvtest, libtest, clocktest, setjmptst, MXPROBE, mathtst, EX_LIB/LDGTEST,
  ZVIEW, CORE/COREVAL, DUMPVDI, VDIDRAW, QED, TeraDesk DESKTOP, PRESENT)
  plus `build_example` calls for `sdk/examples/*.c`
  (`tools/make-sd-image.sh (line 220-317)`, further ones not enumerated here).
  Every `REQUIRED` entry names a recipe over a local source. I did not read
  the whole script to its end, so "every executable maps" for the standard
  image is **not** fully checked below line ~320. The script has no entry
  for Thing, HighWire, HypView or Doom (grep `thing|doom|HYPVIEW|HIGHWIRE`:
  only `ZVIEW`/`PRESENT` hit).
- Card manifests of runs (READ): `measurements/measurement-2026-10-29-bios15/
  cards/td.img.manifest` lists `C:\Apps\TERADESK\DESKTOP.NAP` + rsc/inf;
  `measurements/measurement-2026-10-29-bios15/cards/doom.img.manifest` lists `C:\DOOM\DOOM.NAP` + `DOOM1.WAD`
  (built by `sdk/apps/build-doom.sh`). Both executables map to a local
  source recipe. `optional_missing none` in both.
- No executable on these manifests or in the `REQUIRED` list has no local
  source. The only program that uses XHDI/SCSI (Thing, below) is **not**
  on any manifest and not built.

## Group B: XHDI (22 rows) and SCSIDRV (16 rows)

Rows (mapping.tsv, READ): XHDI = `XHDOSLimits XHDriverSpecial XHDrvMap XHEject
XHGetCapacity XHGetVersion XHInqDev XHInqDev2 XHInqDriver XHInqTarget
XHInqTarget2 XHLastAccess XHLock XHMediumChanged XHMiNTInfo XHNewCookie
XHReaccess XHReadWrite XHReserve XHStop init_XHDI_drvmap xhdi_handler`
(`bios/xhdi.c`); SCSIDRV = `get_bus_info get_bus_maxlen get_handle_entry
scsidriv_CheckDev scsidriv_Close scsidriv_Error scsidriv_In scsidriv_InOut
scsidriv_InquireBus scsidriv_InquireSCSI scsidriv_Open scsidriv_Out
scsidriv_RescanBus scsidriv_init valid_bus valid_bus_and_device`
(`bios/scsidriv.c`). Note column of the XHDI rows: "XHDI driver interface:
not implemented in NewTOS".

### (1) Users

- **Only user found: Thing (desktop), source only.** READ:
  - XHDI client calls `XHGetVersion`/`XHInqDev`/`XHInqTarget`:
    `Applications/thing-neo-codeSVN-r62/Source/Thing/src/SYSTEM.C (line 1743-1745)`;
    `XHGetVersion`/`XHInqDev`/`XHEject`:
    `Applications/thing-neo-codeSVN-r62/Source/Thing/src/dlmenu.c (line 3784-3787)`; include `..\xhdi\xhdi.h`:
    `Applications/thing-neo-codeSVN-r62/Source/Thing/src/SYSTEM.C` line 34 and `Applications/thing-neo-codeSVN-r62/Source/Thing/src/dlmenu.c` line 39.
  - Cookie lookup `Getcookie(C_XHDI, ...)` with magic check `0x27011992`:
    `Applications/thing-neo-codeSVN-r62/Source/XHDI/xhdi.c (line 25,39-46)`.
  - 'SCSI' cookie (only together with 'MgMc', MagiC-Mac): `Getcookie(C_SCSI)`
    at `Applications/thing-neo-codeSVN-r62/Source/Thing/src/windesk.c (line 811)`. No `scsidrv.h` use found.
  - Thing is in `Applications/` only: no recipe in `tools/make-sd-image.sh`,
    not on any manifest, no `sdk/apps` or `programs/` entry (READ: greps above;
    `docs/analysis/machine-identity-adr-proposal-2026-10-28.md (line 131)` says
    the same: "nur Quelle in Applications/, nicht gebaut").
- **No user in anything that is built.** READ: zero hits for
  `xhdi|XHInq|XHGetVersion|scsidrv|scsidriv|C_SCSI|'SCSI'` in `programs/`,
  `kernel/`, `tos-types/`, `tosdump/` code, `sdk/apps/*` (TeraDesk, QED,
  HypView, HighWire, Doom, present, apitest, coremark), `sdk/examples`,
  `sdk/overlays`, `sdk/zview`, `sdk/ldg`, cflib, windom, gemlib.
- Definitions only (no use): `#define C_SCSI`, `C_XHDI`, `NF_ID_XHDI`,
  `NF_ID_SCSI` in `sdk/libc/mintlib/include/mint/cookie.h (line 423,572)` and
  `sdk/libc/mintlib/include/mint/arch/nf_ops.h (line 57-58)` (READ; header constants, nothing calls them).
  `PC_SCSI` in `sdk/gemlib/gemlib-0.44.0/mt_gemx.h (line 512)` and
  `sdk/cflib/cflib-21/wdlgpdlg.h (line 188)` is a printer-port flag, unrelated.
- TeraDesk: no XHDI. Its `Shutdown()` call (`sdk/apps/teradesk/main.c (line 2073)`)
  is GEMDOS 0x151 (`sdk/libc/mintlib/syscall/syscalls.master:460`, a
  FreeMiNT call, not EmuTOS XHDI) and sits inside `#if _MINT_`
  (`main.c:~2066`); the NewTOS build defines `_MINT_ 0`
  (`sdk/overlays/teradesk/library/xdialog/library.h (line 110)`; recipe comment
  `sdk/apps/build-teradesk.sh (line 95)` "-D_TOS_ONLY -> _MINT_=0"), so it is not
  compiled. (READ + DERIVED for "not compiled".)
- NewTOS side (READ): `tos-api/src/cookie.rs` defines only NTOS, `_MCH`,
  `_IDT`, `_AKP`, EdDI (`cookie.rs (line 25-37)`); no XHDI/SCSI cookie.
  Only comments mention XHDI: `tos-api/src/bios/bpb.rs (line 194)`,
  `tos-api/src/bios/tick.rs (line 35)`.
  `docs/analysis/machine-identity-adr-proposal-2026-10-28.md (line 44)` lists
  `_FRB, XHDI, SCSI` as "fehlen" on NewTOS.
- Compiled/reachable: nothing is compiled that uses it; a program that
  looked for the cookie would find none (DERIVED from cookie.rs).

### (3) EmuTOS pinned make-512 config (READ + DERIVED)

- `:`:
  `CONF_WITH_SCSI_DRIVER` and `CONF_WITH_XHDI` default to
  `!CONF_WITH_EXTERNAL_DISK_DRIVER`; `CONF_WITH_EXTERNAL_DISK_DRIVER`
  defaults to 0 (`config.h (line 1675-1677)`) -> both **1**. The only overrides
  to 0 are inside machine blocks (`config.h (line 262,298)` TARGET_192;
  `:491,524` TARGET_CART; `:737` AMIGA/LISA block) not selected by
  TARGET_512 (DERIVED: `docs/api/_emutos-config.md (line 14)` states `-DTARGET_512`
  alone is the whole truth; table `_emutos-config.md (line 203-222)` shows the
  `0` only in the TARGET blocks). `docs/api/_emutos-config.md (line 590,632)`
  and `docs/analysis/aes-audit/emutos-config-profile.md (line 161-162)` list both
  as 1 for the reference build. So the pinned make-512 EmuTOS **has**
  XHDI cookie and SCSIDRV compiled in. (The "oracle ROM actually exposes the
  cookie" was not measured here.)
- EmuTOS ROM holds the cookie also because both are independent of a real
  SCSI/IDE chip: config.h (line 1679-1686) describes SCSI driver as raw pass-through.

## Group C: OS memory pool and DND tree

Functions (mapping.tsv rows, READ): `getosm osmem_init xmgetblk xmfreblk`
(`bdos/osmem.c`); `free_available_dnds freednd getdnd makdnd process_dnd_tree
snipdnd makofd dopath` (`bdos/fsdir.c`).

### (1) Users

- Internal to the BDOS: no ported program, library or app calls these (they
  are EmuTOS-internal C functions, not a trap or cookie interface).
  Programs observe only the effects (current directory, memory-block
  count). DERIVED (no cited source for a caller outside EmuTOS exists;
  the greps for `DND|osmem` over `tos-api/src`, `docs/api` find only
  NewTOS comments, listed under (2)).
- Program-visible effect worth naming (READ): EmuTOS' fixed OS pool limits MDs
  to 354 shared with DND/OFD (`docs/MEMORY.md` ADR-025 Context, citing
  `bdos/osmem.c (line 36-39,62-64,104-109)`).

### (2) NewTOS counterpart

- **OS pool: MD part replaced, the pool itself absent.**
  `xmgetmd/xmfremd` equivalents in `tos-api/src/bdos/mem/mdpool.rs`
  (header comment "Mirrors EmuTOS bdos/osmem.c ... Growth (MEM05)",
  `mdpool.rs (line 3-23)`: pages from the kernel heap, `pages.rs`). Decided in
  `docs/MEMORY.md` ADR-025 (Decision 1: "The MD supply grows page by
  page"; ADR-025 "replaces the fixed pool"), `docs/COMPATIBILITY.md`
  "No fixed memory-descriptor limit" (cited by `mdpool.rs (line 13)`). READ.
  For `getosm osmem_init xmgetblk xmfreblk` themselves (generic OS-pool
  block alloc + the DMD/DND/OFD users): no same-named NewTOS function
  (grep `fn getosm|fn xmgetblk|fn osmem` over `tos-api/src`: no hit;
  DERIVED). Whether OFD/DMD live in Rust statics, as the mapping note
  claims, was not re-read here (open, not part of the ask).
  ADR-025 does not name these four functions (the mapping note: "ADR-025
  ersetzt nur die MD-Versorgung", `TASK/TOOL17-STATUS-DONE.MD:80`).
- **DND tree: concept absent, replaced by canonical path strings + scanning.**
  READ: `tos-api/src/bdos/fs/curdir.rs (line 36-38)` ("A slot holds the directory
  as its canonical absolute path ... not a DND pointer: NewTOS has no DND
  cache"); `curdir.rs (line 326-336)` `free_drive` ("NewTOS keeps no DND tree, so
  this is all of `freetree`"); path walk by cluster scan:
  `tos-api/src/bdos/fs/path.rs` (`resolve_parent`@62, `findit`@130,
  `current_dir_cluster`@21) and `bdos/fs/dir.rs` (`find_in_dir`@184,
  `slot_of`@207). Per-function pairing:
  - `dopath` (walk DND chain to build the path) <-> `curdir.rs@curdir`
    (stored path string) — DERIVED from the curdir.rs header.
  - `makdnd/getdnd/snipdnd/freednd/free_available_dnds/process_dnd_tree`
    <-> no function; the DND cache has no equivalent — READ (curdir.rs (line 36-38)).
  - `makofd` (OFD for a directory) <-> no function; directories are read
    by cluster (`dir.rs`) — DERIVED.
- **No ADR** says "no DND tree": the decision text lives in
  `curdir.rs` doc comments and in `mapping.tsv` notes only (grep
  `DND|osmem` in `docs/MEMORY.md` hits only ADR-025 context/decision
  lines, 2335-2347). Whether a dedicated ADR exists in `docs/api/` was
  not found: `docs/api/gemdos/process/_run-ownership-curdir.md` §4.5 is
  cited by `curdir.rs (line 327)` for the media-change effect (not re-read).

## Group F: startup / runtime functions

For each: EmuTOS definition (READ), NewTOS counterpart (READ unless marked).

| EmuTOS function | EmuTOS location | NewTOS counterpart / state |
|---|---|---|
| `bios_init`, `biosmain` | `bios/bios.c (line 212)`, `:781`; sequence `docs/api/bios/_boot-init.md (line 62-80)` | `kernel/src/lib.rs@kernel_main_rust` (:226): framebuffer, BDOS pools, `seed_clock_from_kernel_file`@1320, `launch::run_auto_folder`@522, AES start; no per-function mapping, one init path (DERIVED: the sequences are comparable, no function boundary matches) |
| `bmem_init`, `balloc_stram` | `bios/biosmem.c (line 51)`; callers `screen.c (line 647)`, `amiga.c`, `68040_pmmu.S (line 99)` | `kernel/src/bios/machine/layout.rs` (membot/memtop constants, ST/TT pool split: `layout.rs (line 23-86)`); `tos-api/src/bios/getmpb.rs` (Getmpb). `balloc_stram` (boot-time allocation below membot): no counterpart function; the one caller on this build path (screen vram) has none, framebuffer comes from `lib.rs@init_framebuffer`@800/817 (DERIVED) |
| `os_header`, `osxhbootdelay` | `bios/startup.S (line 88-89)` | OSHEADER image is built per process by `tos-api/src/bdos/process/loader.rs` (os_beg etc., `:1158-1165`; date from `tos-api/src/build.rs`); reset entry `tos-api/src/bios/reset.rs` + `kernel/src/reset.rs@restart`. `osxhbootdelay`: no counterpart found (only EmuTOS startup delay; not searched in NewTOS beyond grep for the name: none) |
| `screen_init_address`, `calc_vram_size`, `set_rez_hacked` | `bios/screen.c (line 656)`, `:647` | `kernel/src/lib.rs@init_framebuffer`:800; XBIOS `tos-api/src/xbios/screen.rs@physbase`:121, `@setscreen`:274 (ignores `rez`, per mapping note on ADR-017). No function maps 1:1; `set_rez_hacked` (Falcon rez hack): no counterpart found |
| `int_vbl` | `bios/vectors.S (line 317)`, installed `screen.c (line 500)` | No VBL handler running `vblqueue` slots: `tos-api/src/bdos/process/hooks.rs (line 77,171)` (`Hook::Vblqueue`: "there is no VBL queue running program code", ADR-031, `docs/MEMORY.md (line 3041)`). The tick is `kernel/src/drivers/timer.rs (line 171)` -> `tos_api::bios::tick::advance`@78 (`tick.rs`). VBL counting of `frclock`/`vblsem` is not in a named function (not checked) |
| `flush_data_cache`, `invalidate_data_cache`, `invalidate_instruction_cache` | `bios/processor.S` (called e.g. from `_warm_reset`, `panicasm.S (line 105)`) | `kernel/src/arch/aarch64/mmu.rs@clean_dcache_range`:371, `@clean_dcache_all`:434, used by `kernel/src/reset.rs@restart_board` (comment cites `panicasm.S (line 105)`). No `ic`/instruction-cache invalidate found in `kernel/src`/`tos-api/src` (grep `ic iallu|ic ivau`: none) -> no counterpart for `invalidate_instruction_cache`; whether the loader needs one is **open** (not analysed) |
| `panic` | `bios/panicasm.S` | `kernel/src/panic.rs@on_panic`:16 (`#[panic_handler]`: message to UART, halt); also `kernel/src/arch/aarch64/crash.rs`, `tos-api/src/bios/crash.rs`, `docs/api/bios/_exception-crash.md` (not re-read) |
| `shutdown`, `can_shutdown` | `bios/bios.c (line 738,754)` (`CONF_WITH_SHUTDOWN`); only callers `bios.c (line 870)` and `aes/gemshlib.c (line 179)` | `can_shutdown()` returns FALSE for the Falcon/512 build (`bios.c (line 754-770)`, no NatFeats/Amiga/Firebee/Lisa branch). NewTOS: no shutdown function; `kernel/src/lib.rs (line 547-551)` states the `can_shutdown()`-FALSE reasoning and keeps `halt()` after `ShExit::Shutdown` (:654-656); `shell_loop.rs (line 286)`. Related, different thing: TeraDesk `Shutdown()` (GEMDOS 0x151) not compiled (`_MINT_ 0`, see B) |
| `time_init` | `bdos/time.c (line 160)` (reads `Gettime`, sets `etv_timer = tikfrk`) | `Hook::EtvTimer` "NewTOS keeps time natively, no 20 ms timer hook" (`hooks.rs (line 161)`); clock: `tos-api/src/gemdos/time.rs@xbios_settime`:205 / `@xbios_gettime`:222, seeded by `kernel/src/lib.rs@seed_clock_from_kernel_file`:1320 (rpi4, from the FAT date of KERNEL8.IMG). Mapping note "no RTC" (RTC plan: auto-memory, not READ here) |
| `bdos_trap2` | `bdos/rwa.S (line 135-149)` (comment: trap #2 with d0=0 acts as Pterm0, "undocumented compatibility"), installed `bdos/bdosmain.c (line 326)` (`Setexc(0x22, ...)`) | No `Pterm0`-on-trap-2 path found: `tos-api/src/trap.rs` decodes `Gem = 2` (:32, :45) as the AES/VDI class; I did not read how a call with `d0=0` is decoded, so "d0=0 -> Pterm0 on NewTOS" is **open** |
| `run_auto_program` | `bios/bios.c (line 685)` | `kernel/src/launch.rs@run_auto_folder`:385 (pattern `C:\AUTO\*.NCP`, comment cites `bios/bios.c (line 702-714)`); no per-program function |

Program-visible parts of F (READ):
- `_sysbase` (0x4F2) / OSHEADER read by programs: Doom `kbd_probe`
  (`sdk/apps/doom/doomgeneric_gem.c (line 1836-1866)`; reads os_beg +0x08,
  os_version +0x02, +0x2c 'ETOS'); apitest T03
  (`sdk/apps/apitest/apitest.c (line 896-902)`); TeraDesk idiom cited there.
  Doom is on the `bios15` doom card manifest (not the standard image);
  apitest is a test, not on the `REQUIRED` list. So `os_header` has
  readers; the layout is NewTOS-built (`loader.rs (line 1158-1165)`).
- No program in the greps (`sysbase|vblqueue|CacheCtrl|S_CTRLCACHE`) uses
  `vblqueue`/`_nvbls`/`CacheCtrl` (hits are mintlib header defs only:
  `sdk/libc/mintlib/include/mint/sysvars.h (line 76-77)`,
  `ssystem.h (line 43)`, `syscalls.master:761`).
- Dispatch of GEMDOS Shutdown(0x151) on NewTOS: not searched.

## Open / not done

- Standard-image `build_example` list beyond line ~320 of
  `tools/make-sd-image.sh` not enumerated; programs there are
  `sdk/examples/*.c`, which show no XHDI/SCSI hits (READ via grep), so the
  survey result does not depend on it.
- XHDI/SCSI cookie presence on the pinned oracle ROM is a config reading,
  not a measurement.
- `bdos_trap2`, `invalidate_instruction_cache` need deeper analysis
  (named in the table).

## Out-of-scope observations

- `docs/api/bios/_cookie-jar.md (line 60)` lists `XHDI`/`NVDI` as "out of scope".
- Thing's `windesk.c (line 811)` guards on 'MgMc' (MagiC Mac) as well, so even if
  Thing were ported the 'SCSI' branch is a MagiC-Mac path.
