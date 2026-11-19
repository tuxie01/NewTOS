# zDesk: can it be built for NewTOS? (PORT13, step 1)

Dated snapshot, 2026-11-19. Researcher. Ticket: `TASK/PORT13.md`. Scope: the zDesk tree in
`references/z-tools/zdesk/` (90 files, gitignored, read only). Labels: **[R]** read in source this run,
**[T]** tried (compile or link in a scratch directory), **[I]** inferred. Nothing was run on NewTOS or in QEMU.

Cites into `references/z-tools/zdesk/...` are written `references/z-tools/zdesk/<file>:<line>`; cites into NewTOS use symbol anchors.
doc-check cannot resolve the gitignored `references/` tree inside a worktree where `worktree-setup` did not link
it; see Open points for the result of the run.

## 0. Verdict in three lines

1. **Buildable with patches, not as is.** 52 translation units compile and link (zero undefined symbols, ELF64
   AArch64, 314 112 bytes) after three build-level changes [T]; the program then still cannot run: it refuses to
   start without the `MiNT` cookie (`references/z-tools/zdesk/appl.c:74`) and every directory read uses MiNT calls that NewTOS
   answers with `EINVFN` (`tos-api/src/gemdos/mod.rs:240@MAX_FN`).
2. **Runnable port = about 15 patched files**, of which about 7 replace MiNT file-system calls by Fsfirst/Fsnext
   and lose links, permissions and creation time; 3 are LP64 pointer/struct widths, 2 endianness (the same two
   as in the zView port), 3 are build-level (type header, one include, `USE_ASM`), 2 are LDG (suffix, `mem.ldg`).
3. **The source is a v0.1 browser prototype, not a desktop**: it cannot start a program, copy, move or delete
   (section 5.4), and it has no licence file, with a header that names the GPL and the LGPL in one text
   (section 1). Whether the port is worth its cost is an owner question (section 7).

## 1. Origin and licence

| Statement | Evidence |
|---|---|
| Version string `"0.1"` | [R] `references/z-tools/zdesk/defs.h:1` (`ZDESK_VERSION`) |
| One licence header in the whole tree: `zDesk. Copyright (c) 2010 Zorro ( zorro270@yahoo.fr)` | [R] `references/z-tools/zdesk/general.h:1-17` (grep for `licen|copyright|GPL|author` over all files finds only this header) |
| The header text says "GNU General Public License ... version 2.1 ... Lesser General Public License ... You should have received a copy of the GNU General Public License ... this library" | [R] `references/z-tools/zdesk/general.h:6-16`; it mixes the GPL and the LGPL 2.1 wording. It is the same boilerplate as lines 1-17 of zView's own general.h in the pinned z-tools tarball (copyright 2004-2005) |
| Other source files carry **no** header | [R] same grep; only `references/z-tools/zdesk/general.h` matches |
| No licence or copying file in the zDesk tree | [R] file list, 90 files |
| The Makefile says "this is shared along all Zorro's products but let's use separate copy here" | [R] `references/z-tools/zdesk/Makefile:28` |
| zDesk's `references/z-tools/zdesk/types2b.h` is an **older** copy than the pinned common/types2b.h (no `HAVE_INTS_DEFINED`, no `HAVE_BOOLEAN`) | [R/T] `diff` of `references/z-tools/zdesk/types2b.h` against common/types2b.h of the extracted pin (`sdk/zview/z-tools-04805b7.tar.xz`; the extracted tree is gitignored) |
| The z-tools pin records: "License: zView GNU GPL (source headers, e.g. zview general.h lines 1-17; the copying.gpl in zview/_dist/doc); each codec directory carries the GNU LGPL 2.1" | [R] `sdk/zview/zview-sources.sha256` (last block). Its licence texts are the copying.gpl in zview/_dist/doc and the file zview/copyright (LGPL 2.1 text) in the pin |
| The pinned z-tools root has no licence file; its README says it is "a dump of my former SVN repository at https://sourceforge.net/projects/z-tools ... It also contains all the archives released by me" and points to th-otto/zview as the maintained zView | [R] README.md lines 1-4 of the pinned tarball (read from the extracted copy) |
| Upstream `github.com/mikrosk/z-tools`, branch shown by default: top level `common, zcodeclib, ztask, zview, zweather` plus `README.md common.mak prerequisites.txt`; **no `zdesk` directory and no licence file; no licence declared** | **fetched** one page, `https://github.com/mikrosk/z-tools`, asked only for licence and top-level names (no licence text exists there to fetch) |

**Reading.** The author (Zorro) states a free-software intent in the header of `references/z-tools/zdesk/general.h`, but the text is
inconsistent (GPL versus LGPL) and covers formally one file. Whether the codec-facing parts are LGPL (as the
zView codecs are) or all of zDesk is GPL is **not decidable from the tree** [I]. The origin of this state
(SVN revision, release archive) is **not found**: it is not in the pinned git commit `04805b7`, not at the top
level of the upstream default branch; the README's "SVN dump with all released archives" suggests it was an
archive or an SVN directory [I]. A port that ships (a binary on the card or release) needs the owner to decide
how to treat this (question Q1).

## 2. Dependencies

### 2.1 Libraries and plugins

| Dependency | Where | In the SDK / zView port? |
|---|---|---|
| gemlib (`mt_gem.h`, `mt_gemx.h`: the `mt_*` AES/VDI bindings with an explicit `aes_global`) | `references/z-tools/zdesk/general.h:28-29`; Makefile `-lgem` (`references/z-tools/zdesk/Makefile:3`) | yes, the SDK's gemlib archive [T: linked] |
| LDG client library (`ldg_open/find/close`) | `references/z-tools/zdesk/general.h:30`, `references/z-tools/zdesk/codecs.c:55,62,...`; `-lldg` | yes, the SDK's libldg archive [T: linked] |
| mintlib (`osbind.h`, `mintbind.h`, mint/cookie.h, mint/dcntl.h, stdio, `math.h`) | `references/z-tools/zdesk/general.h:24-25`, `references/z-tools/zdesk/appl.c:6`, `references/z-tools/zdesk/raster/resample.c:4` | yes, the SDK's libc and libm archives [T] |
| bits/math-68881.h | `references/z-tools/zdesk/raster/resample.c:5` | **no**: an m68k header (not in the SDK include path); [T] compile error, remove the include |
| **windom** | not used: no `windom` or `#include <windom` anywhere | n/a (zView needs it, zDesk does not) |
| libpng, zlib, libjpeg, giflib, libexif, libshare | not linked by zDesk (its codecs are loaded at run time) | not needed in the program |
| **`mem.ldg`**, an LDG module with `ldg_malloc/ldg_realloc/ldg_free/ldg_allocmode` | `references/z-tools/zdesk/codecs.c:55-90`; used for the decode buffers `references/z-tools/zdesk/pic_load.c:48,59,142-147,227,238` | LDG's source (src/mem/mem.c of the SDK's ldg-r133 tree, gitignored extract) is in the SDK, but **no recipe builds it** as `.NLD` (`sdk/ldg/build-ldg.sh` builds `libldg.a`; grep of `sdk/*.sh sdk/*/*.sh` for `MEM.NLD`/`mem.ldg` is empty) [R] |
| Image codecs as LDG modules | `references/z-tools/zdesk/codecs.c:105-142`: every `*.LDG` in the `codecs` directory, symbols `plugin_init`, `reader_init`, `reader_read`, `reader_quit` (`references/z-tools/zdesk/pic_load.c:190-192`) | the zView port builds ten codecs (`sdk/zview/build-zview.sh`, section codecs) with the **same symbol names** (zview/plugins/png/pngldg.c lines 18-19 in the pin) and the **same `img_info` layout** (compared field by field, `references/z-tools/zdesk/pic_load.h:19-66` against zview/plugins/common/zview.h of the pin) [R]; suffix `.NLD`, not `.LDG` (ADR-018 addendum, `sdk/zview/overlay/zview/plugins.c:7@extention`) |

### 2.2 Run-time files

| File | Source | Note |
|---|---|---|
| `zdesk.rsc` (cwd) | `references/z-tools/zdesk/appl.c:80` `mt_rsrc_load("zdesk.rsc")` | new-format RSC with colour icons: header word 0 = 4 (`NEW_FORMAT_RSC`), 165 910 bytes [R]. NewTOS's `rsrc_load` reads the converted NRSC/LP64 form (`tools/mkrsc.py`, as `ZVIEW.RSC`); colour icons are handled (`tos-api/src/aes/resource.rs:56@v11`, A41) |
| `zdesk.rsh`, `zdesk.hrd`, `zdesk.rsm` | `zdesk.rsh` is `#include`d only for index defines (`references/z-tools/zdesk/general.h:34`); `.hrd/.rsm` are ResourceMaster project files | `.rsh`: defines only; nothing to build |
| `zdesk.inf` | `references/z-tools/zdesk/prefs.c:91-99` (read, `$HOME` first, else cwd), `:494-496` (written at exit) | optional; `HOME` via `shel_envrn` |
| `mem.ldg` and `codecs/*.LDG` | `references/z-tools/zdesk/codecs.c:55,62,74-76,117`; fallbacks `C:\gemsys\ldg\mem.ldg`, `\gemsys\ldg\codecs` | if missing, `codecs_init` fails and an alert `ECODEC` is shown, the program goes on (`references/z-tools/zdesk/appl.c:103-106`) |
| desktop folder `<HOME or cwd>\desktop` | `references/z-tools/zdesk/desktop/scan_desktop.c:36-50`: created with `Dcreate` if absent | holds the desktop icons; a directory on the boot drive |
| optional desktop picture | `pref->desk_pic`, `references/z-tools/zdesk/desktop/desktop.c:262` via `pic_load` | needs the codecs |

## 3. Calls

Method: all `mt_*`, `v*_*` and GEMDOS/BIOS/XBIOS/cookie calls found by grep over the 52 `.c` files; the GEMDOS
list was then checked against the object files' undefined symbols [T] (mintlib's GEMDOS bindings are inline
functions in the SDK headers, so they do not appear as undefined symbols; the grep is the inventory).

### 3.1 GEMDOS, BIOS, XBIOS, cookies

NewTOS dispatches GEMDOS functions 0x00-`MAX_FN = 0x57` from a table and answers everything above with `EINVFN`,
except `Ssystem` (`tos-api/src/gemdos/mod.rs:34@MAX_FN`, `:233-241@run_call`) [R]. The MiNT functions are all
above 0x57 (FreeMiNT numbering: `Pdomain` 0x119, `Dpathconf` 0x124, `Dopendir` 0x128, ..., `Dxreaddir` 0x142, ...) [I,
from memory of the numbering; the point that matters, "not in the table", is [R]]. The table does not contain
`Fxattr`, `Dopendir`, `Dreaddir`, `Dclosedir`, `Dxreaddir`, `Freadlink`, `Fchmod`, `Dpathconf`, `Dreadlabel`,
`Pdomain` (`tos-api/src/gemdos/mod.rs:42-102@build_table`); a grep for these names over `tos-api`, `kernel`,
`docs/api` finds none [R].

| Call | Used where | NewTOS | Verdict |
|---|---|---|---|
| `Getcookie(C_MiNT)` | `references/z-tools/zdesk/appl.c:74`; on `C_NOTFOUND` the program shows alert `EMINT` and exits (`:76-79`, `:170-175`) | the jar is seeded with `NTOS, _MCH, _IDT, _AKP, EdDI, _VDO` only (`tos-api/src/cookie.rs:223-229@seed_defaults`); no `MiNT` | **missing: program exits at start** |
| `Getcookie(C_EdDI)` | `references/z-tools/zdesk/raster/vdi.c:16` (`-1` if absent) | present (`tos-api/src/cookie.rs:228@seed_defaults`, `docs/api/vdi/eddi-vq_scrninfo.md`) | present |
| `Pdomain(1)` | `references/z-tools/zdesk/appl.c:97`, result ignored | not in the table, `EINVFN` | missing, harmless [I: result not read] |
| `Dgetdrv`, `Dsetdrv`, `Dgetpath`, `Dsetpath`, `Dcreate`, `Dfree` | `references/z-tools/zdesk/appl.c:99-101`, `references/z-tools/zdesk/file/file.c:50-54`, `references/z-tools/zdesk/desktop/scan_desktop.c:48`, `references/z-tools/zdesk/file/get_disk_size.c:27`, `references/z-tools/zdesk/codecs.c:62,74-76` | present (`docs/api/gemdos/drive/`, `docs/api/gemdos/directory/`; table slots 0x0E, 0x19, 0x36, 0x39, 0x3B, 0x47) | present |
| `Frename(0, old, new)` | `references/z-tools/zdesk/file/rename.c:20` | present (slot 0x56, `docs/api/gemdos/file/56-frename.md`) | present |
| `Fopen/Fread/Fwrite/Fclose` on a program file (PH header) | `references/z-tools/zdesk/form/info.c:231-236,384-394` | present | present, but the data meaning is TOS-executable specific (section 4.4) |
| `Fattrib(name, 1, attr)` | `references/z-tools/zdesk/form/info.c:204` | present (slot 0x43) | present |
| `Tgetdate` | `references/z-tools/zdesk/form/info.c:323`, `references/z-tools/zdesk/file/file.c:107` | present (slot 0x2A) | present |
| `Drvmap` (BIOS 0x0A) | `references/z-tools/zdesk/desktop/scan_desktop.c:58` | present (`tos-api/src/bios/mod.rs:112@DRVMAP`, `docs/api/bios/0a-drvmap.md`) | present |
| `Getrez` (XBIOS 4) | `references/z-tools/zdesk/raster/vdi.c:21`, result used as `work_in[0]` of `v_opnvwk` | present (`tos-api/src/xbios/mod.rs:75@GETREZ`); NewTOS `Getrez` value differs by design from the Falcon mode (`docs/DEVIATIONS.md`, XBIOS section); for a virtual workstation `work_in[0]` is a device id [I: not read as an input by the VDI] | present, unverified effect |
| `Fxattr(0, path, &xattr)` | `references/z-tools/zdesk/file/file.c:26,170`, `references/z-tools/zdesk/form/info.c:180,287` | **missing** (`EINVFN`) | missing |
| `Dopendir`, `Dreaddir`, `Dclosedir` | `references/z-tools/zdesk/codecs.c:105,110,142`; `references/z-tools/zdesk/file/file.c:102,194,226` | **missing** | missing |
| `Dxreaddir` | `references/z-tools/zdesk/file/file.c:111` (the directory-listing loop: names plus xattr in one call) | **missing** | missing |
| `Freadlink` | `references/z-tools/zdesk/file/file.c:172` | **missing** | missing |
| `Fchmod` | `references/z-tools/zdesk/form/info.c:177` (permission changes in the info box) | **missing** | missing |
| `Dpathconf(dir, 3)` | `references/z-tools/zdesk/winfile/WinFile_Keyb.c:52` (maximum file name length, used to limit rename input) | **missing**: returns `EINVFN` = -32, so `win->edit.len < max_len` is never true and renaming accepts no character [I] | missing |
| `Dreadlabel` | `references/z-tools/zdesk/desktop/scan_desktop.c:106`; on non-zero return the drive gets a generic name | **missing**; the fallback branch (`"Drive X"` from the RSC) runs [R] | missing, graceful |
| `Mxalloc` via `ldg_Malloc`, `Pexec(3/4)` via `ldg_open` | inside `libldg.a` (the zView port uses the same) | present (`docs/sdk/10-ldg-modules.md`) | present |
| BIOS/XBIOS others, `Super`, `Supexec`, `Kbshift`, `Setexc` | not used (grep) | n/a | n/a |

Struct hazards in the same area: `XATTR` is declared locally with m68k 2-byte packing assumptions
(`references/z-tools/zdesk/general.h:62-80`: `uint16 mode; int32 ino; ...` at offset 2) and `PH` (`:37-47`: `int16` then `int32`)
is read with `Fread(sizeof(PH))` (`references/z-tools/zdesk/form/info.c:388`). On AArch64 natural alignment pads both, so `sizeof`
and field offsets differ from the on-wire layout [I, from the C rules]; needs `__attribute__((packed))` or a
patch. For `XATTR` this is moot until a `Fxattr` exists.

### 3.2 AES

| Call | Used where | NewTOS | Verdict |
|---|---|---|---|
| `appl_init`, `appl_exit`, `graf_handle` | `references/z-tools/zdesk/appl.c:66,37,69` | present | present |
| `menu_register(ap_id, "  zDesk ")` | `references/z-tools/zdesk/appl.c:88` (a **program** registering, not an accessory) | opcode 35 present; EmuTOS `mn_register` accepts any `pid >= 0` and stores the title pointer in the Desk-menu slots (`references/emutos/aes/gemmnlib.c:789-816`) | present; effect for a non-accessory (stray Desk-menu entry; unregistered at exit?) **not established**, needs a run |
| `menu_bar` (install/remove), `menu_icheck`, `menu_ienable`, `menu_tnormal` | `references/z-tools/zdesk/appl.c:26,150`, `references/z-tools/zdesk/menu.c` (86 calls), `references/z-tools/zdesk/evnt_loop.c:56` | present (`tos-api/src/aes/mod.rs:927@menu_bar`) | present |
| `rsrc_load`, `rsrc_gaddr`, `rsrc_free` | `references/z-tools/zdesk/appl.c:80,128-142` | present; gemlib returns pointers through the LP64 overlay | present (colour icons: section 4.5) |
| `evnt_multi` (MESAG, TIMER, KEYBD, BUTTON; 2 clicks, 100 ms), `evnt_timer` | `references/z-tools/zdesk/evnt_loop.c:51-53`, `references/z-tools/zdesk/winfile/WinFile_Rubberbox.c:88,97` | present | present |
| `wind_create/open/close/delete/calc/find/update` | `references/z-tools/zdesk/winfile/WinFile_New.c:62,119,226`, `references/z-tools/zdesk/form/about.c:99,147`, `references/z-tools/zdesk/form/info.c:580,639`, `references/z-tools/zdesk/winfile/WinFile_Close.c` | present (`tos-api/src/aes/mod.rs:959@wind_create`) | present |
| `wind_set(0, WF_NEWDESK, ADR(ob), 0, 0)` | `references/z-tools/zdesk/desktop/desktop.c:266`; clear: `:114` | present, as TeraDesk uses it (`sdk/apps/teradesk/icon.c:2511`, `tos-api/src/aes/window.rs:1201@NEWDESK_TREE`, `:1276@set_newdesk`) | present **but the argument packing is wrong under LP64**: `ADR()` splits a 32-bit cast into two words (`references/z-tools/zdesk/defs.h:ADR`), the kernel reads the packed pointer at the caller's width, 4 words (`tos-api/src/aes/window.rs:3371@wf_str_ptr`, doc above it); needs `wind_set_ptr_int` as TeraDesk does [R/I] |
| `wind_get/set` with `WF_WORKXYWH, CURRXYWH, PREVXYWH, FIRSTXYWH, NEXTXYWH, TOP, VSLIDE, VSLSIZE, NAME`; `wind_set_str` | many (`references/z-tools/zdesk/winfile/*.c`) | present: EmuTOS `wm_get`/`wm_set` set (`references/emutos/aes/gemwmlib.c:1517-1579,1645-1727`; NewTOS `tos-api/src/aes/window.rs`) | present |
| `wind_set(WF_BOTTOM)` | `references/z-tools/zdesk/winfile/WinFile_Other_Event.c:33` | **missing**: EmuTOS `wm_set` has no `WF_BOTTOM`; the `default:` arm returns FALSE (`references/emutos/aes/gemwmlib.c:1736-1737`) | missing; the window is not sent to the bottom |
| `wind_set(WF_ICONIFY / WF_UNICONIFY)`, `wind_get(WF_UNICONIFY)`, messages `WM_ICONIFY/WM_UNICONIFY/WM_ALLICONIFY`, window kind `SMALLER` | `references/z-tools/zdesk/winfile/WinFile_Iconify.c:35,73,100`, `references/z-tools/zdesk/defs.h` `WINATTRIB`, `references/z-tools/zdesk/evnt_loop.c:135-141` | **missing**: no such `case` in `wm_set`/`wm_get`; no `SMALLER` in the EmuTOS AES; the messages are never sent (`grep SMALLER|WM_ICONIFY` over `references/emutos/aes include` is empty) | missing; the iconify code (about 300 lines) is unreachable, not harmful |
| `form_popup(tree, x, y)` | `references/z-tools/zdesk/form/info.c:88` (flags popup of the info box) | **missing**: no opcode 135 (`tos-api/src/aes/mod.rs:992@set_ret`, `docs/api/aes/_dispatcher.md` section 3: opcodes >= 128 return -1) | missing; returns -1 |
| `appl_getinfo`, `appl_find`, `shel_write`, `shel_find`, AV protocol, `AP_DRAGDROP`, `appl_control` | **not used** (`references/z-tools/zdesk/evnt_loop.c:57` lists `AP_DRAGDROP` as "to do") | n/a | n/a |
| `shel_envrn("HOME=")` | `references/z-tools/zdesk/prefs.c:80,481`, `references/z-tools/zdesk/desktop/scan_desktop.c:28` | present (opcode 125, `tos-api/src/aes/mod.rs:980@shel_envrn`) | present |
| `form_alert`, `form_dial(START/FINISH)` | `references/z-tools/zdesk/form/alert.c:73`, `references/z-tools/zdesk/winfile/WinFile_Popup.c:112,199` | present | present |
| `objc_draw/draw_grect/find/change` | `references/z-tools/zdesk/form/info.c`, `about.c`, `references/z-tools/zdesk/winfile/WinFile_Popup.c:46` | present | present |
| `graf_mouse` (incl. `FLAT_HAND`, `BUSYBEE`), `graf_mkstate`, `graf_dragbox` | `references/z-tools/zdesk/evnt_loop.c:210`, `references/z-tools/zdesk/winfile/WinFile_Tab.c:273-312`, `WinFile_Frame_Sizer.c` | present (the `FLAT_HAND` form is a standard mouse form number) | present |
| `appl_write` | only inside the helper `ApplWrite` (`references/z-tools/zdesk/misc.c:62-75`), which is never called (grep) | present | unused |
| MyAES points from `bug.txt` ("on MyAES the icon image address is not right", "resize mouse form", "no extended objects") | `references/z-tools/zdesk/bug.txt:12-14` | The code reads icon data through `ob_spec.ciconblk->mainlist` (`references/z-tools/zdesk/icon/draw_icon.c:39-45`) | the extended object types are the colour icons (CICON); NewTOS supports them (A41), so the MyAES notes describe AES **without** CICON, not a dependency on an AES 4.x call [I] |

### 3.3 VDI

All of these are in the EmuTOS-derived VDI: `v_opnvwk`, `v_clsvwk`, `vq_extnd`, `vq_color`, `vs_clip_pxy`,
`vst_point`, `v_gtext`, `v_pline`, `vr_recfl`, `vro_cpyfm`, `vrt_cpyfm`, `vsf_color/interior/udpat`,
`vsl_color/type/udsty`, `vswr_mode`, `v_show_c`, `v_hide_c` (call counts: `grep -c` over the tree, 32 `vsf_color`,
30 `vr_recfl`, 27 `v_pline`, 12 `v_show_c`/`v_hide_c`, 10 `vrt_cpyfm`). Specs: `docs/api/vdi/workstation/`,
`docs/api/vdi/attributes/`, `docs/api/vdi/output/`, `docs/api/vdi/input/_mouse.md`, `docs/api/vdi/raster/_raster.md`, `docs/api/vdi/inquiry/`. `vq_scrninfo`: not EmuTOS, a NewTOS
extension behind the `EdDI` cookie (`docs/api/vdi/eddi-vq_scrninfo.md`, `-32.md`; `tos-api/src/vdi/eddi.rs`).

Known differences that touch zDesk's use: memory MFDBs of 9..31 planes are refused by `vro_cpyfm`/`vrt_cpyfm`
at the 32-plane setting (`docs/DEVIATIONS.md`, "VDI at 32 planes", V65); zDesk builds memory forms with
`app->nplanes` from `vq_extnd` (`references/z-tools/zdesk/raster/vdi.c:26`, `references/z-tools/zdesk/pic_load.c:20`), which is 16 or 32 on NewTOS, so
the refusal does not apply [I]. `v_get_pixel` is not used. The `vro_cpyfm`/`vrt_cpyfm` calls inside `G_USERDEF`
callbacks (desktop background, icons) run at EL0 as for TeraDesk's `ub_bckbox` (`tos-api/src/aes/window.rs:1213@NEWDESK_APP`).

## 4. Compile attempt

Directory: target-agents/agent-ad10ca848897e8fe8 (scratch, outside the repository; removed at the end,
section 4.6). Flags are those of `sdk/zview/build-zview.sh` (BASE + `ZV_WARN`), run per translation unit with
the include paths of the main tree's SDK (`sdk/gemlib/gemlib-0.44.0`, `sdk/libc/mintlib/include`,
`sdk/libm/...`, `sdk/ldg/ldg-r133/include`): `clang -target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2
-fPIE -fno-builtin -nostdinc -fsigned-char -fgnu89-inline -D__NEWTOS__ -DHAVE_INTS_DEFINED -include
sdk/zview/include/zview-ints.h -Wno-macro-redefined -Wall -Wshadow -Wno-pointer-sign -DDEBUG ...` (zDesk's own
`-DDEBUG -DUSE_ASM`, `references/z-tools/zdesk/Makefile:4`). Host clang: Apple clang 21.0.0. No tracked file changed; "changes" are
symlinks and two edited copies in a scratch shadow tree.

| Attempt | Change | Result |
|---|---|---|
| 1 | as is, zView flags + `-DUSE_ASM` | **52 of 52 fail**, 117 errors in all, 2 per file for the type header: `typedef redefinition with different types ('long' vs 'int')` for `int32`/`uint32` (`references/z-tools/zdesk/types2b.h:6-7` against the prelude). `references/z-tools/zdesk/raster/raster.c` has 14 errors (see attempt 2) |
| 2 | shadow tree: `references/z-tools/zdesk/types2b.h` replaced by the pinned common/types2b.h (which honours `HAVE_INTS_DEFINED`) | **50 of 52 compile** (13 errors left). Fails: `references/z-tools/zdesk/raster/raster.c`, 12 x `invalid input constraint 'a' in asm` (inline **m68k** assembly under `USE_ASM`, `references/z-tools/zdesk/raster/raster.c:8,265,490,803,...`); `references/z-tools/zdesk/raster/resample.c`: `'bits/math-68881.h' file not found` |
| 3 | additionally: `USE_ASM` not defined (the C branches exist for every `#ifdef USE_ASM`, `raster.c:37,370,555,852,...`); a copy of `resample.c` without the `math-68881.h` include | **52 of 52 compile** (0 errors, 23 warnings); **link** with `crt0.o` + `gemlib.a`, `libldg.a`, `libm.a`, `libc.a` in the zView order (`newtos.ld`, `-pie`): **0 undefined symbols**, `zdesk.elf` 314 112 bytes [T]. Not run |

Third-attempt warnings: 10 files, 23 warnings, grouped by cause:

| Cause | Count | Where | Effect |
|---|---|---|---|
| LP64 pointer through 32-bit | 5 `-Wpointer-to-int-cast` | `references/z-tools/zdesk/desktop/desktop.c:181` (`ub_parm = (int32)app`), `:266` twice (`ADR(ob)`), `references/z-tools/zdesk/mem.c:65,122` (`(int)hdr`, debug hash only) | `ub_parm` truncated: `redraw_desktop` would read a bad `APPVar*` (**crash**); `ADR(ob)` wrong packing (section 3.2); `references/z-tools/zdesk/mem.c` harmless |
| `Getcookie(…, &int32)` where the SDK wants `long *` | 2 `-Wincompatible-pointer-types` | `references/z-tools/zdesk/appl.c:74`, `references/z-tools/zdesk/raster/vdi.c:16` | the callee writes 8 bytes into a 4-byte local [R: `int32` is `int` under the prelude, `long *` parameter]; stack overwrite |
| source style | 9 `=-`/`=+` compound-assignment, 3 misleading indentation, 1 empty body, 1 unused variable | `references/z-tools/zdesk/raster/dither.c`, `references/z-tools/zdesk/appl.c:27`, `references/z-tools/zdesk/winfile/WinFile_Close.c:36`, `references/z-tools/zdesk/winfile/WinFile_Slider.c:216`, `references/z-tools/zdesk/winfile/WinFile_Tabbar.c:220`, `references/z-tools/zdesk/winfile/WinFile_Pathbar.c:22` | `err[0] =- 127` assigns -127 (intended dithering clamp); same in upstream; no effect of the port |
| Latin-1 source bytes | 2 `-Winvalid-source-encoding` | `references/z-tools/zdesk/winfile/WinFile_Keyb.c:50` (`allowed_char` table) | compiles; the characters are CP437/Atari bytes in a UTF-8-hostile file |

Missing header, missing library, missing call: **none at compile or link time** (the missing MiNT calls are inline
traps that fail at run time, section 3.1; `mt_form_popup` links because gemlib has it, the kernel answers -1).

### 4.1 What the compile does not show (silent LP64 and endianness hazards) [R/I]

| Hazard | Where | Same as in the zView port? |
|---|---|---|
| `draw_text` builds 1-plane glyph MFDB words with a byte walk, big-endian assumption | `references/z-tools/zdesk/custom_font.c:60-117` (the `|= (*character++) << pixel_shift` on `uint16 *` and the byte stores) | **yes**: the overlay `sdk/zview/overlay/zview/custom_font.c` (TASK/ZVIEW01) fixes the same pattern; zDesk's variant has the extra shifted branch |
| `remap_color`/`color_lookup` read an `int32` as bytes `[1..3]` and compare `SRGB` as an `int32` | `references/z-tools/zdesk/raster/color.c:51-54,60-63,86-90` | **yes**: `sdk/zview/overlay/zview/zvdi/color.c`; reached only on <= 8 planes (latent at 16/32) |
| `int32` = `long` (64-bit) without the prelude | `references/z-tools/zdesk/types2b.h:6-7`; `Entry.file_size`, `XATTR`, `PH`, `dbuf.ino` | fixed by the pinned header plus prelude (attempt 2) |
| `PH`/`XATTR` struct padding | `references/z-tools/zdesk/general.h:37-47,62-80` | new (zView has no such struct) |

### 4.2 Three-attempt limit

Reached after three attempts at "compile"; link needed no further attempt. What stands in the way of a
**running** program is not a build problem but the run-time list of section 3.1 and 5.

### 4.3 Scratch contents and record

Whole scratch directory 2.9 MiB; produced and removed: shadow (symlink tree + two edited files), out1, out2, out3 (objects and `.err`
files), `crt0.o`, `zdesk.elf` (314 112 bytes, sha256 `d60827355b3c5e518e1f45aeb9c7af17e15f746fab6519fe28deadfc8bbe9c39`), `link.err`, `undef.txt`, the extracted pinned z-tools tarball
(zt), cc.sh, link.sh. No file of the repository was written other than this analysis.

## 5. Structure

### 5.1 Single tasking, accessories, AES level

- No accessory code, no `appl_find`, no `shel_write`, no `AC_OPEN` handling [R: grep]. The event loop is one
  `evnt_multi` (`references/z-tools/zdesk/evnt_loop.c:51-53`) handling `AP_TERM`, `MN_SELECTED`, the `WM_*` of its own windows,
  clicks, keys and a 100 ms timer for the mouse form (`references/z-tools/zdesk/evnt_loop.c:200-212`).
- It assumes no multitasking AES; it takes `AP_TERM` (`:63`), which NewTOS's single-tasking never sends [I].
- AES 4.x features it relies on: **none beyond** `form_popup` (info box) and `WF_BOTTOM`/iconify (dead code) (section 3.2).
  `menu_popup`/`menu_attach`/`wind_set(WF_OPTS)`/`objc_sysvar` are not used [R: grep].
- It keeps **its own desktop as window 0**: a `WINDOW` with `handle == 0` in its list (`references/z-tools/zdesk/desktop/desktop.c:219`),
  `wind_set(0, WF_NEWDESK, tree)` with a `G_USERDEF` background object (`:180-181,266`), mouse events routed by
  `wind_find` returning 0 (`references/z-tools/zdesk/evnt_loop.c:170`). NewTOS has the mechanism and a precedent that works for an EL0 app
  (TeraDesk; `tos-api/src/aes/window.rs:1201@NEWDESK_TREE`), including the sweep when the registrant ends (`tos-api/src/aes/window.rs:1235@NEWDESK_OWNER`).

### 5.2 Run as a program on top, or replace the desktop?

It is an ordinary GEM program (`main` -> `ApplInit` -> `EvntLoop` -> `ApplExit`, `references/z-tools/zdesk/main.c:16-27`) that
installs its own menu bar (`references/z-tools/zdesk/appl.c:150`) and its desktop background through `WF_NEWDESK`; `ApplExit` closes
its windows and clears `WF_NEWDESK` (`references/z-tools/zdesk/desktop/desktop.c:114`, `references/z-tools/zdesk/appl.c:22-40`). That is TeraDesk's model
(ADR-026 "a `#Z` program (TeraDesk) uses the same mechanism", `docs/MEMORY.md:2576-2577`): **it can run on top of
NEWDESK as a program**; it does not need to replace the registry entry. [I: no run on NewTOS.]

### 5.3 Screen-depth handling

`references/z-tools/zdesk/raster/vdi.c:13-203` picks the raster routines from `vq_scrninfo` (`format`, bits, bit order) after the `EdDI`
check: 1-plane, planar 2/4/8, chunky 8/15/16/24/32 with a "reverse" (`out[16] < out[48]`) and "z-trail" variant
(`:35-36`, `:138-167`). NewTOS answers `vq_scrninfo` with format 2 (packed), 16 planes Falcon order `[14] = 2` by
default and 32 planes usual order `[14] = 1` at `vdiplanes=32` (`tos-api/src/vdi/eddi.rs:64@ORDER_FALCON`,
ADR-038). So both depths go through code paths that exist; the 16-bit path also serves zView, which shares the
lineage of this raster code (zDesk's raster directory is an older copy of zView's zvdi directory, the files differ) [I]. Colour icons: `draw_cicon` copies
`col_data` as a memory form of `app->nplanes` planes with `S_AND_D` (`references/z-tools/zdesk/icon/draw_icon.c:85-87`) and walks the
CICON chain choosing `num_planes <= nplanes` (`:44`); NewTOS's loader keeps one variant per block, converted to the
live depth (`tos-api/src/aes/resource.rs:62@fix_cicons`), so the loop should find it [I]. Not run: **16 and 32 planes untested**.

### 5.4 What the program does (it is a prototype)

Read from the sources, not tried:
- Menu items handled: About, New browser/tab, Close window/tab, Info, List/Icon view, Info bar, Tab bar, Quit
  (`references/z-tools/zdesk/evnt_loop.c:60-95`). Open, Duplicate, Trash, Find, Format, Eject, Shutdown, Cut/Copy/Paste, sort,
  toolbar, path bar, side bar, Preferences have menu entries (`references/z-tools/zdesk/zdesk.rsm`, `defs` in `zdesk.rsh`) and **no handler**
  (`default: break;`).
- Opening an entry (`WinFile_Exec_Entry`, `references/z-tools/zdesk/winfile/WinFile_Exec_Entry.c:20-127`) handles a drive on the
  desktop, a folder and the `..` entry. A **file** or program does nothing; there is no `Pexec`, `shel_write`
  or `shel_find` in the tree.
- File operations: directory listing, rename (`Frename`), `Dcreate` of its own desktop folder. No copy, move,
  delete, no `Fdelete`.
- Drag and drop: only a tab being dragged (`WinFile_Tab_Drag_and_Drop`); `AP_DRAGDROP` and the AV protocol are
  "to do" (`references/z-tools/zdesk/evnt_loop.c:57`). Long file names: names up to `MAXNAMLEN 256` (`references/z-tools/zdesk/defs.h:3-5`) and an
  Aranym-specific workaround (`references/z-tools/zdesk/winfile/WinFile_Keyb.c:86`).
- Its own `bug.txt` lists open problems (rubberbox/menu, rename >128 characters, attribute setting on the Aranym
  filesystem) (`references/z-tools/zdesk/bug.txt:1-21`), written in French.

The port would therefore be a **file browser with icon/list view, tabs and thumbnails**, not a replacement for
NEWDESK's launch, copy and delete functions.

## 6. Cost in terms of the zView port

| Item | zView (precedent) | zDesk (estimate) |
|---|---|---|
| Source pin and pristine tree | tarball + sha256 + extract script | needs a pin: tarball of `references/z-tools/zdesk/` (origin unknown, Q1) |
| Build recipe | `sdk/zview/build-zview.sh` (shadow tree, overlay dir, prelude, library ensure) | similar but smaller: no windom, no codec libraries, no codec build; 52 TUs; links gemlib, ldg, libm, libc; stack size (`Makefile:1` `STACKSIZE 256k`) |
| Patch files | 14 overlay files | about 15 (list below) |
| Libraries | windom, ldg, png/zlib, libshare, 4 image libs | ldg, gemlib (all present) + `mem.nld` recipe or patch |
| Codecs | ten built | reuse the zView `.NLD` set (same interface) in a codecs directory next to the program, plus suffix patch |
| Resource | `tools/mkrsc.py` convert | same tool; colour icons exercised (a risk the zView RSC did not take) |
| New kernel work | K1 (workstation open) | none **if** patched to TOS calls; a FreeMiNT-extension ticket if the owner wants the MiNT calls instead |

Patch list by kind (rough count; each would be a header-commented overlay file, as in zView):

| Kind | Files | Count |
|---|---|---|
| Build level | `references/z-tools/zdesk/types2b.h` (use the pinned one), `references/z-tools/zdesk/raster/resample.c` (drop the 68881 include), `USE_ASM` off (flag) | 2 files + 1 flag |
| LP64 pointer/struct width | `references/z-tools/zdesk/desktop/desktop.c` (`ub_parm`, `WF_NEWDESK` via `wind_set_ptr_int`), `references/z-tools/zdesk/appl.c` and `references/z-tools/zdesk/raster/vdi.c` (`Getcookie` result type, then `MiNT` gate removed), `references/z-tools/zdesk/general.h` (`PH`, `XATTR` packing) | 4 |
| Endianness (same fixes as zView) | `references/z-tools/zdesk/custom_font.c`, `references/z-tools/zdesk/raster/color.c` | 2 |
| MiNT file-system calls -> TOS | `references/z-tools/zdesk/file/file.c` (listing with `Fsfirst/Fsnext`, no `Fxattr`/`Freadlink`), `references/z-tools/zdesk/codecs.c` (module scan), `references/z-tools/zdesk/form/info.c` (`Fxattr`, `Fchmod`, PH flags, `form_popup`), `references/z-tools/zdesk/winfile/WinFile_Keyb.c` (`Dpathconf`), `references/z-tools/zdesk/desktop/scan_desktop.c` (`Dreadlabel`) | 5 |
| LDG naming and `mem.ldg` | `references/z-tools/zdesk/codecs.c` (`.LDG` -> `.NLD`, `mem.ldg`), `references/z-tools/zdesk/pic_load.c` (6 `ldg_malloc` sites -> `gmalloc` as in zView `references/z-tools/zdesk/pic_load.c:90`) | 2 (codecs.c counted above) |
| Optional | `references/z-tools/zdesk/winfile/WinFile_Other_Event.c` (`WF_BOTTOM`, no effect) | 1 |

Total about 14-15 distinct files. Loss with TOS calls only: symbolic links, permission/owner bits, creation time and
mode in the info box, the name-length limit (`Dpathconf`), drive labels (`Dreadlabel`).

Where the source and recipe would live, by analogy with zView: pinned tarball `sdk/zdesk/zdesk-<state>.tar.xz` with a
`zdesk-sources.sha256` (origin, command, licence) beside a build script build-zdesk.sh in that directory (not created), overlays in an overlay directory below it,
a prelude shared or copied from `sdk/zview/include/zview-ints.h`, output staged as `ZDESK.NAP`, `ZDESK.RSC`,
`CODECS\*.NLD`, `MEM.NLD`; installed by `tools/make-sd-image.sh` on the QEMU image only. **Not created** (docs-only task).

## 7. What would have to exist first (possible tickets, none filed)

1. **Licence and origin decision** (owner): before a pin, ask the author or accept the inconsistent header (Q1).
2. **`sdk/zdesk` source pin and build recipe** with the overlays above (developer); acceptance by a QEMU run (tester).
3. **`mem.nld` recipe** (from the src/mem directory of LDG's tree), or the `ldg_malloc` -> `gmalloc` patch; the latter is smaller.
4. **Alternative to the MiNT patches (kernel, FreeMiNT extension, ADR-027):** `Dopendir/Dreaddir/Dclosedir/Dxreaddir/Fxattr/Dpathconf/
   Freadlink/Dreadlabel` in `tos-api`. Needs a MiNT cookie decision (`seed_defaults` has none) and, by the roadmap,
   belongs to 2.0 not 1.0; it would cut about 5 of the patch files but not the `MiNT`-cookie question.
5. **Colour-icon check** at 16 and 32 planes for this RSC (A41 covers the loader; this resource is new input).
6. **zDesk functions that do not exist** (launch, copy, delete, drag and drop): a product decision, not a port patch.

## Open points

Each as a question for the owner with the researcher's reading.

- **Q1 Licence and origin.** The only notice is `references/z-tools/zdesk/general.h:1-17` (Zorro, 2010; GPL/LGPL 2.1 mixed), no licence file,
  not in z-tools git `04805b7`, not in the default branch of the upstream repository (fetched page listing). Reading: free
  software intended; formal clarity missing; ask the author, or accept GPL-or-LGPL-2.1 as the pin file does for zView. Is the
  state a branch, an SVN revision or a release archive? The owner's directory may know. [Cited as read; the reading is inferred.]
- **Q2 Is a browse-only program wanted?** Reading: yes it can be built, but as read it cannot launch programs, copy or delete
  (section 5.4); it is no step towards replacing NEWDESK's functions. [Cited; the judgement is inferred.]
- **Q3 MiNT calls: patch to TOS calls or add them to NewTOS?** Reading: patch (5 files, no kernel change); adding them is a
  2.0 FreeMiNT-extension ticket and still leaves the `MiNT` cookie, which NewTOS does not set. [Cited: table slots, `seed_defaults`; the
  preference is inferred.]
- **Q4 `menu_register` by a non-accessory**: stray Desk-menu entry after quit? Not established; needs one QEMU run
  (`references/z-tools/zdesk/appl.c:88`, `references/emutos/aes/gemmnlib.c:789-816`). [Inferred.]
- **Q5 Colour icons at 16 and 32 planes** (`references/z-tools/zdesk/icon/draw_icon.c:39-105` against `tos-api/src/aes/resource.rs:62@fix_cicons`): the
  claim that the loop finds the single converted variant is inferred, not run. [Inferred.]
- **Q6 `Getrez()+2` as `work_in[0]`** (`references/z-tools/zdesk/raster/vdi.c:21`) on NewTOS: expected unused for a virtual workstation, not
  verified. [Inferred.]
- **Q7 doc-check**: 0 failures with the `references/` tree linked by `worktree-setup` (207 path cites); the pin's extracted tree and built archives are gitignored in the worktree, so those are named in prose, not as cites; the
  `references/README.md` layout line for z-tools/zdesk could not be written by the researcher role (role guard: docs/ only).
  Proposed line, for the lead to apply:
  `├── z-tools/zdesk/            # zDesk 0.1 (Zorro), TASK/PORT13; not in the z-tools pin of sdk/zview/; origin unknown; only licence notice general.h:1-17 (docs/analysis/zdesk-newtos-build-feasibility-2026-11-19.md)`
- Out-of-scope observation: the zView overlays for custom_font.c and zvdi/color.c are the model for the two
  endianness patches; nothing in the zView port was changed or re-checked here.
- Out-of-scope observation: `docs/api/gemdos/_dispatcher.md` lists the table slots by EmuTOS names; the NewTOS code
  table is `tos-api/src/gemdos/mod.rs:42-102@build_table` (they agree on the slots used above); not audited beyond this.
