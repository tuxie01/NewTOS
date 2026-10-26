# 12 — WinDom 1.21.3: the GEM window library zView is written against

Stand 2026-10-06, TASK/LDG-M3. Why WinDom: zView links `-lwindom1`
(`sdk/zview/z-tools-04805b7/zview/Makefile:17`, `sdk/zview/z-tools-04805b7/zview/general.h:30` includes
`<windom1.h>`), and LDG's demo client is a WinDom program
(`#include <windom.h>` at line 11 of demo/client/client.c in the pinned tarball `sdk/ldg/ldg-svn-r133.tar.xz`). Design input: `docs/analysis/zview-ldg-2026-10-04.md`
(P-WINDOM). Evidence for every "verified" below:
`measurements/measurement-2026-10-06-ldg-m3/RUNINFO.TXT`.

WinDom (Dominique Bereziat & Arnaud Bercegeay, LGPL 2.1) is a static GEM
library: windowed forms, menus in windows, toolbars, frames, extended object
types drawn as `G_USERDEF`, its own config file. It is pure C (no assembler).

## 1. The pieces

| What | Where |
|---|---|
| Pinned source | `sdk/windom/windom-1.21.3.zip` — the SourceForge release, unchanged (SHA-1/MD5 match SourceForge; 397 files, byte-identical to `Applications/windom-1.21.3`); hashes, origin, licence in `sdk/windom/windom-sources.sha256` |
| zView's patch | `sdk/windom/patches/zview-windom-1.21.3.patch` — zView's `sdk/zview/z-tools-04805b7/zview/howto_build/windom-1.21.3.patch`, unchanged (z-tools commit `a556a68`): variadic WORDs read as `int`, `av_ping` copies the name into `app.pipe`, three gcc-4 cast-lvalue fixes |
| Build | `sdk/windom/build-windom.sh` (also via `lib_ensure windom`, chain libc → gemlib → windom, `sdk/lib-stale.sh`) — every run: extract zip, apply patch, copy `sdk/windom/overlay/`, compile `sdk/windom/windom-1.21.3/src/files.mak`'s `KERNEL` list (198 files), archive |
| Archive | `sdk/windom/libwindom1.a` (the name zView links, howto step 3) |
| Headers | `sdk/windom/include/` — `windom1.h` **and** `windom.h` (the same patched file), `av.h`, `scancode.h` |
| Overlays | `sdk/windom/overlay/src/*.c` — 10 files, §3 |

Link order for a WinDom program (what `sdk/ldg/build-ldg-client.sh` uses):

```sh
--start-group sdk/windom/libwindom1.a [sdk/ldg/libldg.a] sdk/gemlib/gemlib.a \
              sdk/libm/libm.a sdk/libc/libc.a --end-group
```

with `-I sdk/windom/include` before gemlib's and mintlib's include
directories. Resource files go through the kernel's `rsrc_load`
(`RsrcLoad`, `sdk/windom/windom-1.21.3/src/rsrc_load.c:47-73`), so they are converted with
`tools/mkrsc.py convert` like any other application's ([05](05-abi-pitfalls.md) B8);
`RsrcLoad` then reads `rsh_ntree` from the converted file's native `RSHDR`
preamble.

## 2. Flags (no source change)

| Flag | Why |
|---|---|
| WinDom's own `gcc.mak` set: `-fomit-frame-pointer -DMODIF_XEDIT -Wall -Wmissing-prototypes -Winline -Wshadow -Wpointer-arith -Wcast-qual -Waggregate-return` | the upstream build |
| `-D__NEWTOS__` | gemlib's retargeted trap path |
| `-fsigned-char` | WinDom was written for m68k compilers, whose `char` is signed; AArch64's is unsigned. Keeps every `char` comparison as upstream ran it (library-internal; programs keep the app flags) |
| `-D__RSXHDR` | `sdk/windom/windom-1.21.3/src/globals.h:244-264` declares WinDom's own `RSXHDR`; gemlib 0.44 (newer than the 0.43 WinDom targets) declares one too behind this guard. No WinDom source uses either |
| `-Wno-error=int-conversion`, `=implicit-function-declaration`, `=incompatible-function-pointer-types` | clang ≥ 15 makes these errors; gcc of WinDom's/zView's time warned. Sites and why each is value-correct on LP64: header of `build-windom.sh` (the three `long = pointer` stores zView's patch leaves; `strcpy`/`strcmp` without `<string.h>`; an `int` userdef routine in a `short` slot) |
| `-Wno-header-guard` | one upstream typo, `sdk/windom/windom-1.21.3/src/globals.h:29-30` |
| `SOURCE_DATE_EPOCH` = the release's time stamp | `sdk/windom/windom-1.21.3/src/globals.c:35-44` stores `__DATE__`/`__TIME__`; pinned so two builds are byte-identical |

Build result: 198 files, 0 failures, 16 files with warnings (120 lines, all
upstream: 60 `-Wmissing-braces` in the `.rsh` initialisers, 17 `-Wshadow`,
17 `-Wmissing-prototypes`, …; list in the run's `measurements/measurement-2026-10-06-ldg-m3/checks/build-windom.txt`).

## 3. Overlays (10 files, each with a header comment: upstream line, change, reason)

| File | Kind | What |
|---|---|---|
| `sdk/windom/overlay/src/evnt_windom.c` | endianness + width | `MN_SELECTED` tree pointer: `*(OBJECT**)&evnt.buff[5]` → `msg[5]<<16 \| msg[6]` (AES 4 layout, WinDom's own senders use `ADR()` = hi, lo) |
| `sdk/windom/overlay/src/av_init.c` | endianness + width | `VA_PROTOSTATUS` server name `*(char **)&msg[6]` → hi/lo (it read 4 bytes past the 8-WORD buffer) |
| `sdk/windom/overlay/src/shel_write.c` | endianness + width | `VA_START` command pointer store `*(char **)&msg[3]` → hi/lo |
| `sdk/windom/overlay/src/debug_windom.c` | endianness + width | `AP_DEBUG` pointer store `*(char **)&buff[4]` → hi/lo |
| `sdk/windom/overlay/src/cookieptr.c` | LP64 sysvar | `_p_cookies` read 4 bytes, not 8 ([05](05-abi-pitfalls.md) B2) |
| `sdk/windom/overlay/src/set_cookie.c` | LP64 + NewTOS contract | jar walk (8-byte read, 16-byte stride, writes into the page-0 mirror) → `Getcookie` + `Ssystem(S_SETCOOKIE)` |
| `sdk/windom/overlay/src/new_cookie.c` | LP64 + NewTOS contract | same; one recorded difference: an existing tag is updated, not shadowed |
| `sdk/windom/overlay/src/vq_tos.c` | LP64 sysvar | `_sysbase` read 4 bytes, not 8 |
| `sdk/windom/overlay/src/cicon.c` | LP64 | 32-bit device pixel stored as `ULONG` (8 bytes) → 4-byte store (the 32-plane path; today's 16-plane VDI takes the 2-byte one) |
| `sdk/windom/overlay/src/objc_str3d.c` | endianness | `te_color` bit-field view declared LSB-first on a 2-byte unit (upstream order would paint the border colour) |

The pointer-in-message overlays use the **32-bit hi/lo word pair**, the
protocol's own layout; that is enough because every NewTOS user address is
below 2 GiB ([05](05-abi-pitfalls.md) A2). A NewTOS-wide convention for
pointers in inter-application messages (AV, BubbleGEM, VA_START) does not
exist yet; this is the protocol-faithful choice.

## 4. The LP64 sweep (what was looked at, and what was left alone)

Classes searched over all 198 compiled files (pristine tree + patch):

* **`(long)`/`(LONG)`/`(ULONG)` casts — 76 lines in `src/*.c`.** 16
  `wind_get.c` + 2 `wind_set.c`: `WindGet`/`WindSet` pass pointers as two
  WORDs (hi, lo) by arithmetic — correct below 2 GiB. 15 `rsrc_gaddr.c`:
  `*(long*)addr = (long)ptr` — `long` is pointer-wide on LP64. 20 value
  arithmetic (slider/frame/scroll maths, `w_getbkgr.c` size) — width-neutral.
  9 pointer→`long` stores into `ub_parm`/`wp_spec`/`ob_spec.index`
  (`sdk/windom/windom-1.21.3/src/menu_attach.c:55`, `sdk/windom/windom-1.21.3/src/objc_dup.c:96,104,129`, `sdk/windom/windom-1.21.3/src/rsrc_userdraw.c:87`,
  `sdk/windom/windom-1.21.3/src/xrsrc.c:152,182`, `cicon.c:118,205`) — the fields are `long`, correct. 1
  `vq_tos.c` — overlay. 2 `appl_tplay` records (`sdk/windom/windom-1.21.3/src/mouse_objc.c:38`,
  `sdk/windom/windom-1.21.3/src/snd_mubutton.c:44`, `long rec[2]`) — the kernel's `appl_tplay` plays
  nothing yet (TASK/A45), so the record layout is not observable; open.
  1 `sdk/windom/windom-1.21.3/src/bubble_conf.c:32` (a 32-bit BHLP cookie value), 1 debug print, 3
  Sozobon-only macros (`cicon.c:70-72`, not compiled), 6 in `rsrc_xload.c`
  (below).
* **Type-punned reads/writes of WORD arrays** (`*(T*)&x[n]`): 6 sites — the
  4 message overlays above, `cicon.c:298,610,614` (byte views of a
  `memcpy`'d device pixel, same address both ways — endian-neutral).
* **Absolute low-memory addresses**: `$5A0`, `$4F2` — overlays.
* **Bit-fields**: `objc_str3d.c` (overlay); `sdk/windom/windom-1.21.3/src/vq_winx.c:38-41` and
  `sdk/windom/windom-1.21.3/src/cookie.h:47` (`struct _config_icfs`) are read only when a WinX / ICFS
  cookie exists, which NewTOS does not publish — left alone, latent.
* **Foreign-system structures behind cookies** (`cookie.h`: `MAGX_INFOS`,
  `AESVARS`, `N_AESINFO`, `NVDI_STRUC`, `BGEM`; `LTMF` byte offset
  `sdk/windom/windom-1.21.3/src/userdef.c:255`): `long`/pointer layouts of MagiC, N.AES, NVDI, BubbleGEM,
  Let 'em Fly. Reached only when the cookie exists; none is published on
  NewTOS. Left alone, latent.
* **Varargs**: zView's patch fixed `ApplWrite`/`ApplSet`; `ObjcAttach`,
  `WindFind`, `FrameSet`, `ConfInquire` read `int`/pointer-sized `va_arg`s —
  correct. (`sdk/windom/windom-1.21.3/src/objc_attach.c:118-119` reads a `data` argument that callers like
  the demo client do not pass — upstream behaviour, harmless while `data` is
  unused.)
* **Inline/m68k assembler**: none.

**`RsrcXload` (`rsrc_xload.c`) is not exercised and not changed.** It reads
a resource file into memory and relocates it in place, i.e. it expects the
file's structures to be the program's structures. A classic Atari `.RSC`
(big-endian, 24-byte `OBJECT`) can never work that way on AArch64; a file
converted with `tools/mkrsc.py convert --rshdr` (native `RSHDR`, LP64
structures — what TeraDesk's own loader reads) matches its pointer
arithmetic for resources **without** colour icons (`--rshdr` refuses those;
the `NRSHDR` extension `sdk/windom/windom-1.21.3/src/rsrc_xload.c:162-168` uses `unsigned long`).
**Unverified** — no program in M3/M4 calls `RsrcXload` (zView and the demo
client use `RsrcLoad`).

## 5. What runs (verified in QEMU raspi4b, memprot on and strict)

LDG's demo client, unmodified apart from the `.nld` suffix overlay
([10](10-ldg-modules.md)), as `C:\LDGDEMO\CLIENT\LDGDEMO.NAP`
(`sdk/ldg/build-ldg-client.sh`): `ApplInit`, `RsrcLoad`,
`RsrcXtype(TRUE)` (WinDom's extended types as userdefs, incl. its built-in
colour icons through `RsrcFixCicon` on the 16-plane VDI), the
`FormBegin`/`FormEnd` load dialog while both modules load, `FormCreate` of the
"LDG Client" window, WinDom's keyboard shortcuts (Alt+underlined letter,
Return = DEFAULT), editing the Value field, `FormAlert(mathinfo())` —
`information()` from `EX_LIB.NLD` ("Example of math library!!!") — and the
`AP_TERM` shutdown (`ldg_close` ×2, `RsrcFree`, `ApplExit`, `exit(0)`). On
and strict screendumps are pixel-identical.

## 6. Known problems (open, not fixed in M3)

| # | What | Where | Effect |
|---|---|---|---|
| ~~B1~~ | `evnt_multi`/`evnt_button` report `EV_MB` = the **live** button after the double-click window; EmuTOS reports the state of the event (`bchange` sets `button = gl_bdesired`, `ev_rets` returns it or `pr_button`, `aes/geminput.c:155-180,328-345`, `aes/gemevlib.c:42-58`) | kernel, `tos-api/src/aes/event.rs` `fill_rets` | WinDom acts on a click only if `evnt.mbut & 1` (`evnt_windom.c:334`) with `bclicks 258`: **every normal mouse click on a WinDom form, toolbar or window menu is dropped**; a press held longer than the double-click window works. Kernel change — reported, not made. **Closed** by `66353bc` (AES records the state at event time like EmuTOS; `measurements/measurement-2026-10-06-aes-evbutton/`) |
| B2 | an LDG module's `exit()` closes stdio (`__fcloseall` → `fclose` → `__flshfp` → `init_stream` → `malloc` → `free`); the hunk is Malloc'd by the module's own process and freed when `Pexec(4)` returns, but the module's static `_mchunk_free_list` keeps pointing into it | libc + LDG module model ([10](10-ldg-modules.md) §4) | the module's next `malloc`, called from the client, reads freed memory: memprot on → Bus Error in the module's `malloc` (POPUP.NLD `__get_bkgr`); memprot off → hang. Every module that mallocs after loading is affected (all zView codecs). **Closed by TASK/LDG-M3-B2:** every `.NLD` is now linked with libshare ([10](10-ldg-modules.md) §6); the pop-up works under memprot on/strict/off (`measurements/measurement-2026-10-06-libshare/`) |
| K1 | `ApplInit` asks the screen depth with `vq_extnd` on the workstation `graf_handle` returns (`sdk/windom/windom-1.21.3/src/appl_init.c:42-46`); on NewTOS that handle (1) is never opened (`programs/desktop/src/gsx.rs:16-26`), the VDI treats it as invalid and writes nothing, so **`app.nplanes = 0` and `app.color = 0`** in every WinDom program (EmuTOS opens it at AES start, `aes/gemgsxif.c:169-185`) | kernel (AES/VDI), found in TASK/ZVIEW-M4 (`measurements/measurement-2026-10-06-zview-m4/checks/k1-evidence.txt`) | WinDom draws as on a monochrome/≤16-colour screen (`app.color < 16` branches), skips its true-colour `FLG_NOPAL` setup; zView builds 0-plane MFDBs (grey windows, heap overrun for pictures wider than 128 px, [13](13-zview.md) §6). Kernel change — reported, not made |
| — | the text caret of `objc_edit` ignores `te_just` (EmuTOS `gr_just` in `pxl_rect`, `aes/gemobed.c:180-182`) | kernel, `tos-api/src/aes/object.rs` `edit_field_t` | caret 18 px left of the text end in the demo's centred field; cosmetic |
