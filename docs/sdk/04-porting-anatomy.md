# 04 — Anatomy of a port: overlays, the library chain, QED, TeraDesk, CoreMark

## The rule: upstream stays unchanged, fixes go where they belong

(Line citations of the form `:NNN` into `build-qed.sh` and
`build-teradesk.sh` on this page were written before the header comments of
both scripts were rewritten on 2026-10-20 (TASK/SDK01 B) and the scripts have
changed since; they can be off by a few lines, search by the text instead.)

(Corrected 2026-10-20, TASK/SDK01 package B. Earlier versions of this page
said "the app source stays pristine; every failure is a toolchain gap; the only
exception is m68k asm". That was not true of the tree: of the nine overlay
files of QED and TeraDesk, one retargets m68k asm and eight are LP64
corrections, one of which — QED's `memory.c` — is a complete rewrite of the
memory manager. See the table below.)

What is true, and what the build enforces:

1. **Upstream sources are kept unchanged, as archives or as untracked
   checkouts.** The libraries are vendored as tarballs/zip
   (`sdk/{libc,gemlib,cflib,…}/*.tar.xz`, `sdk/windom/windom-1.21.3.zip`);
   the application trees `sdk/apps/*/` are **untracked**
   (`sdk/.gitignore:29-33`) and QED and TeraDesk are embedded git checkouts
   (`git remote -v`: `freemint/qed` @ `f12055f`, `freemint/teradesk` @
   `b5532b8`). Nothing is edited inside them: `git status` in those trees shows
   only build/ (checked 2026-10-20, `docs/analysis/sdk-inventory-2026-10-20.md`
   §2).
2. **A problem that every program would meet is fixed once, in the SDK** —
   in the libc overlays (`sdk/libc/overlay/`), the AArch64 files
   (`sdk/libc/newtos-arch/`), the headers, `crt0.S`, or the gemlib/cflib
   overlays. Examples: the trap retarget (`osbind.h`), fixed 32-bit TOS
   structures (`basepage.h`, `ostruct.h`), `uintptr_t`/`SIZE_MAX` on LP64, the
   gemlib `AESGLOBAL` accessors. Look here first: if a compile error or an
   undefined symbol would hit any LP64 port, the SDK is where it belongs.
3. **What only one program needs is a per-program patch file** in
   `sdk/overlays/<program>/` (tracked, outside the untracked source tree). It
   is a patch in every sense — it differs from upstream, it is applied at build
   time, and it must be re-checked when the upstream pin moves — and the
   header of each file says why it exists. zView's patches live in
   `sdk/zview/overlay/` (14 files) and the library overlays next to their
   library, not under `sdk/overlays/`.
4. **A build stops on the first failing file** (`build-qed.sh`,
   `build-teradesk.sh` print the failure and refuse to link), and CoreMark
   goes further and MD5-checks the frozen benchmark files before compiling
   anything (`sdk/apps/build-coremark.sh`, `sdk/apps/coremark-sources.md5`).

The per-program patches that exist today (all files tracked; reasons taken from
the file headers and the inventory of 2026-10-20, section 2):

| Program | File | Reason | Class |
|---------|------|--------|-------|
| TeraDesk | `sdk/overlays/teradesk/library/utility/nf_ops.c` | NatFeats probe executes m68k opcodes `0x7300`/`0x7301`; replaced by "NatFeats absent". Now that libc has `sdk/libc/newtos-arch/nf_ops.c` this overlay could become a build exclusion — not done yet | m68k asm |
| TeraDesk | `environm.c` | `_BasPag->p_env` is a 32-bit field in our `basepage.h`; int-to-pointer cast is an error under clang | LP64 |
| TeraDesk | `window.h` | `WDFLAGS` bit-fields re-based on `unsigned short` (struct size 2 vs 4 bytes) | LP64 |
| TeraDesk | `sdk/overlays/teradesk/library/xdialog/library.h`, `icon.c`, `rsc_load.c` | own `AES_GLOBAL` struct removed; `AESGLOBAL` resource words written/read through gemlib's `_AESrscfile_set`/`_AESrscmem_set` as 4-byte TOS fields (A60) | LP64 |
| TeraDesk | `main.c` | low-memory words (`$4F2`, `$420`, `$43A`, `$426`, `$42A`) as 32-bit, reset-vector read at OSHEADER+0x10 (RST01, X04) | LP64 |
| QED | `memory.c` | upstream sub-allocator assumes `sizeof(pointer)==4` in three places; `MALLOC`/`FREE` re-backed by libc | LP64 (rewrite) |
| QED | `set.c` | `SET` addressed as 32-bit big-endian words; one flat byte numbering instead | LP64 + little-endian |
| HypView | `sdk/overlays/hypview/hyp/prepare.c` | m68k inline asm `reduce()`/`mono_bitmap()` rewritten in C; `HYP_PICTURE` width/height big-endian (TASK/SDK01 H) | m68k asm + big-endian file format |
| HypView | `sdk/overlays/hypview/hyp/hyp.c` | `.HYP` header, index table and extra headers are big-endian m68k structs on disk (12-byte header, `seek_offset` at byte 2); decoded into the native structs ([05 B19](05-abi-pitfalls.md#b19-m68k-binary-file-formats-read-straight-into-structs)) | big-endian file format + LP64 |
| HypView | `sdk/overlays/hypview/hyp/ref.c` | `.REF` file: magic, module headers (`LONG module_len, num_entries`) and label lines are big-endian m68k LONG/WORD; upstream read them through `unsigned long`/`long *`/`short *`, so every REF file was refused; decoded from the bytes (TASK/SDK04 S2, host test `sdk/overlays/hypview/tests/ref-host.sh`) | big-endian file format + LP64 |
| HypView | `dl_init.c` | resource header read as `*(RSHDR **)&aes_global[7]` (8 bytes over a 4-byte field, [05 B9](05-abi-pitfalls.md#b9-aesglobal-is-15-words-with-4-byte-pointer-fields)); now `_AESrscmem` + `rsrc_gaddr` | LP64 |
| CoreMark | `newtos/core_portme.{c,h}` | CoreMark's own port layer — not a patch, this is where a port is meant to go | port layer |
| zView | `sdk/zview/overlay/zview/…` (14 files) | m68k asm retarget (`sdk/zview/overlay/zview/plugins/img/xp32.c`, `sdk/zview/overlay/zview/plugins/jpg/jpgdsp.c`); big-endian file formats/bitmaps (`degasldg.c`, `neoldg.c`, `godldg.c`, `rawldg.c`, `imgldg.c`, `custom_font.c`, `av_prot.c`); big-endian colour words (`sdk/zview/overlay/zview/zvdi/color.c`, TASK/SDK04); LP64 (`jpgldg.c`); `.NLD` module suffix (`plugins.c`, ADR-018); PDF left out (`main.c`, `winimg.c`) | mixed, see [13](13-zview.md) |

Counting: of the 9 QED/TeraDesk files, 1 is m68k asm and 8 are LP64
corrections; none is NewTOS-specific behaviour. The only overlay that is
NewTOS-specific rather than ABI-driven is the `.NLD` suffix in zView's
`plugins.c`.

**Rule for a port, in this order:** (1) is the problem general (would the next
program hit it too)? fix it in the SDK. (2) Is it specific to this program's
assumptions about word size, byte order or m68k code? write a per-program
patch file with a header that says *why* and what it changes against
upstream. (3) Never edit inside the upstream tree. Recurring patterns (raw
cookie-jar reads, `p_env` casts, `long` sysvar reads, pointers in AES
messages) are candidates for a shared SDK header — none exists yet.

An overlay file **replaces a same-named file at build time**, leaving the
upstream tree untouched. Three mechanisms
exist:

| Mechanism | Script | What can be overlaid | How |
|-----------|--------|----------------------|-----|
| per-file substitution | `build-qed.sh:113-118` | `.c` files listed in the app's `COBJS` manifest | `if [ -e "$OVL/$c" ]; then src="$OVL/$c"` — the overlay is compiled *instead of* `src/$c` |
| shadow tree | `build-teradesk.sh:104-125` | `.c` **and `.h`** files, at any relative path | build/tree/ is built from symlinks to every vendored `.c`/`.h`, then `ln -sf "$OVL/$f" "$TREE/$f"` layers the overlays on top; the compile runs from the shadow tree so quoted `#include "…"` finds the overlaid header |
| port directory | `build-coremark.sh:133-140` | the port layer the app itself defines (`core_portme.{c,h}`) | compiled from `sdk/overlays/coremark/newtos/` via `-I"$PORT"`, nothing copied |

The libraries use a third, simpler form: `sdk/{libc,gemlib,cflib}/overlay/`
mirrors the library layout and is **copied over the freshly extracted
tarball** on every run (`sdk/libc/build-libc.sh:77-85`). Set
`MINTLIB_REFRESH=1` / `GEMLIB_REFRESH=1` / `CFLIB_REFRESH=1` to throw the
extracted tree away and start from the tarball again (`build-libc.sh:66-70`,
`build-gemlib.sh:70-73`, `build-cflib.sh:65-69`).

## What kinds of change live in an overlay (examples from the real ones)

| Kind | Example | What it does |
|------|---------|--------------|
| **m68k assembly retarget** | `sdk/overlays/teradesk/library/utility/nf_ops.c` | upstream executes the invalid opcodes `0x7300`/`0x7301` to probe ARAnyM/Hatari NatFeats; the overlay returns the API's documented "not available" results (`nf_init() == NULL`, `nf_get_id() == 0`, `-1/ENOSYS`) with the same public symbols (`:1-24`) |
| **LP64 struct size / alignment** | `sdk/overlays/teradesk/window.h` | `WDFLAGS` bitfields re-based on `unsigned short`, because the config loader reads the field through an `_UWORD *` and upstream's `unsigned int` bitfield is 2 bytes on m68k but 4 on AArch64 (`:23-34`, `:128-133`) |
| **LP64 pointer-size assumption in an allocator** | `sdk/overlays/qed/memory.c` | QED's pool allocator hard-codes `sizeof(pointer)==4` in three places and faulted at `FAR=0xffffffff00000008`; the overlay keeps every observable routine verbatim and re-backs only `MALLOC`/`FREE` with libc `malloc`/`free` (`:8-56`) |
| **LP64 word width + little-endian** | `sdk/overlays/qed/set.c` | `typedef unsigned long SET[32]` is addressed both byte-wise and as hard-coded 32-bit big-endian words; on LP64/LE the two disagree (`setmin` answered 25 instead of 1), so the overlay uses one flat byte numbering (`:7-50`) |
| **int → pointer cast forced by a fixed-width TOS struct** | `sdk/overlays/teradesk/environm.c` | `_BasPag->p_env` is a fixed 32-bit field in the overlay `basepage.h`; assigning it to `const char *` is a hard error under clang, so the two reads are cast through `(const char *)(uintptr_t)` (`:23-33`) |
| **the app's own porting layer** | `sdk/overlays/coremark/newtos/core_portme.{c,h}` | a normal CoreMark port directory; three deltas against the Atari port, each explained (`core_portme.h:19-73`): 64-bit `ee_ptr_int`, `SEED_VOLATILE` (no command tail), clang in `COMPILER_VERSION` |

The library overlays are the same idea one level down — the trap layer
(`sdk/libc/overlay/include/mint/osbind.h:120-160`, `sdk/gemlib/overlay/_gc_aes.c`),
fixed-width TOS structures (`ostruct.h` `_DTA`/`_DISKINFO`, `basepage.h`),
endianness (`sdk/libc/overlay/include/bits/byteswap.h`, `mt_gem.h.in` `BFOBSPEC`), a clang-vs-gcc alias
quirk (`sdk/libc/overlay/stdlib/labs.c`), an LP64 `printf` fix (`sdk/libc/overlay/stdio/vfprintf.c:226-234`).
See [05-abi-pitfalls.md](05-abi-pitfalls.md) for why each was needed.

## The library chain and its staleness test

`libc.a`, `gemlib.a` and `cflib.a` are built by three sibling scripts with
one shape: extract + overlay → compile every file of the library's own
manifest best-effort → compare the failure set with `expected-failures.txt` →
archive → link a test client.

| Script | Manifest | Flags (beyond the shared `-target aarch64-unknown-none-elf -ffreestanding -nostdlib -fPIE -fno-builtin -nostdinc`) | Archive | Test client |
|--------|----------|------|---------|-------------|
| `sdk/libc/build-libc.sh` | the directories in `DIRS` (`:109`), minus `EXCLUDE` (`:135`) and the test/skeleton names (`:151`) | `-O2 -D_LIBC -D_REENTRANT -DHAVE_CONFIG_H -D_GNU_SOURCE` (`:101-104`; `-D__NO_LONG_DOUBLE_MATH` until TASK/SDK01 J); `-DVERSION="0.60.1"` for `ident.c` (`:157-161`) | `sdk/libc/libc.a` (`:221`) | `sdk/libc/tests/libtest.c` → `libtest.NCP` (build dir) |
| `sdk/gemlib/build-gemlib.sh` | gemlib's `SRCFILES` `COBJS`, expanded with `make` (`:129-135`) | `-O2 -D__NEWTOS__` (`:115-117`); `mt_gem.h` generated from `mt_gem.h.in` with `@MAJOR@=0 @MINOR@=44` (`:91-95`) | `sdk/gemlib/gemlib.a` (`:168`) | `sdk/gemlib/tests/gemtest.c` → `gemtest.NCP` (build dir) |
| `sdk/cflib/build-cflib.sh` | cflib's `SRCFILES` `COBJS` (`:119-125`) | `-O2 -D__NEWTOS__` (`:106-108`) | `sdk/cflib/cflib.a` (`:158`) | `sdk/cflib/tests/cftest.c` → `cftest.NCP` (build dir) |
| `sdk/libm/build-libm.sh` (T07) | OpenLibm's src/, aarch64/, bsdsrc/ `Make.files`, expanded with `make` without the long-double block | `-O2 -std=c99 -fno-strict-aliasing -D__BSD_VISIBLE`; tarball SHA-256 verified before extraction | `sdk/libm/libm.a` | `sdk/examples/mathtst.c` → `mathtst.NCP` (build dir; linked with `libc.a`) |

The `sdk/libc/newtos-arch/` shims are compiled and archived into `libc.a` after the
mintlib objects (`build-libc.sh:183-217`): `memops.c` (memcpy/memmove/memset/
bcopy/bzero), `proc.S` (`_exit` via `Pterm`), `heap_init.c`, the lean
`do_fstat.c`/`do_stat.c`/`ioctl.c`/`access.c` (they replace the
`EXCLUDE`d mintlib files; the `isatty.c` shim is gone since TASK/LIBC-TTY,
mintlib's own `sdk/libc/mintlib/mintlib/isatty.c` is built again), `getcookie.c` (`Getcookie` over `Ssystem`),
`getsysvar.c` (a plain 32-bit load — the sysvar area is in the process's own
address space), `softfloat.c` (`__extenddftf2`, the one compiler-rt builtin
`strtold` needs), and since T07 `setjmp.S` (the AArch64 `sigsetjmp`/`setjmp`/
`siglongjmp`/`longjmp`, replacing the m68k `sdk/libc/mintlib/mintlib/setjmp.S`; `jmp_buf` is
`long[22]` via the overlay `include/setjmp.h`). This paragraph lists the
shims as of T07; `sdk/libc/newtos-arch/` holds 15 files today. Added since:
`cmdargs.c` (`parseargs()`, PORT01/PORT09), `vfork.S`, `setstack.S`
(`_setstack`, PROC01) and, since TASK/SDK01 A, `nf_ops.c` (NatFeats "absent",
replacing the `EXCLUDE`d `sdk/libc/mintlib/mintlib/nf_ops.c`); `do_fstat.c` also provides
`__sys_fstat`. `DIRS` also contains `pwdgrp` since SDK01 A
(`build-libc.sh:158-167`).

**Failure accounting (T02).** Each script prints the list of files that did
not compile and compares it with `expected-failures.txt`; an unrecorded
failure **stops the build before the archive is written**, and a recorded file
that started compiling is announced (`sdk/lib-stale.sh:229-283`,
`build-libc.sh:171-181`). libc records four failures with reasons (`sdk/libc/mintlib/mintlib/crtinit.c`, `sdk/libc/mintlib/mintlib/syscall.c`, `sdk/libc/mintlib/gmp/support.c`, `sdk/libc/mintlib/tz/dumpstate.c`; `sdk/libc/mintlib/mintlib/linea.c` is not among them — ADR-030 excludes it deliberately in `build-libc.sh` `EXCLUDE`, it is not a build failure)
(`sdk/libc/expected-failures.txt`); gemlib's and cflib's lists are empty, so
any failure there stops the build (`sdk/gemlib/expected-failures.txt:3-5`).

**Staleness (T03).** Every app script starts with `. "$SDK/lib-stale.sh"` and
`lib_ensure <lib>` (`build-qed.sh:60-61`, `build-teradesk.sh:62-63`,
`build-coremark.sh:109-110`; the library scripts chain the same way,
`build-gemlib.sh:61-62`, `build-cflib.sh:56-57`). `lib_ensure` walks
`libc → gemlib → cflib` and rebuilds an archive that is missing **or older than
any of its declared source inputs** — its own overlay and script, plus the
inputs of every library below it (`lib-stale.sh:128-185`):

```
libc.a    ← sdk/libc/overlay/  sdk/libc/newtos-arch/  sdk/include/  sdk/libc/build-libc.sh
gemlib.a  ← sdk/gemlib/overlay/ sdk/gemlib/build-gemlib.sh          + all of libc's
cflib.a   ← sdk/cflib/overlay/  sdk/cflib/build-cflib.sh            + all of gemlib's
libm.a    ← sdk/libm/overlay/   sdk/libm/build-libm.sh              + all of libc's   (T07; a second branch off libc)
```

Since T07 the app scripts also run `lib_ensure libm` and link `libm.a`
between the GEM libraries and `libc.a` (`build-qed.sh`, `build-teradesk.sh`),
with `-I sdk/libm/include -I sdk/libm/openlibm-0.8.8/include` after mintlib's
include directory so that `<math.h>` resolves; see [07 §4](07-mintlib-limits.md#4-mintlib-directories-not-compiled-at-all).

The extracted vendor trees and the tarballs are deliberately **not** watched
(`lib-stale.sh:40-47`). Consequence for a porter: after editing an overlay
header, just run the app script — the chain rebuilds what depends on it.
Editing a file inside `sdk/libc/mintlib/` (the extracted tree) is not seen by
anything and is lost on `MINTLIB_REFRESH=1`; put the change in the library's overlay directory.

Two test-app scripts predate this and only rebuild a library when its archive
is **absent**: `sdk/apps/gemtest/build-gemtest.sh:37-44` and
`sdk/apps/chaintest/build-chaintest.sh:37-44`. Run the library script yourself
before trusting them after an overlay change.

## QED, step by step (`sdk/apps/build-qed.sh`)

1. `lib_ensure cflib` — libc, gemlib, cflib current (`:60-61`).
2. Toolchain lookup as in [02-prerequisites.md](02-prerequisites.md) (`:68-79`).
3. Flags: the shared freestanding set + QED's own `-funsigned-char -O2`, plus
   `-D__NEWTOS__` and `-Wno-macro-redefined` (`:86-88`). Include order: QED
   its src directory, cflib top, gemlib top, mintlib `include/`, clang's headers (`:91`).
4. File list: QED's own `src/SRCFILES` `COBJS`, expanded with `make` (`:99-104`).
5. Compile every file; an overlay in `sdk/overlays/qed/<name>.c` wins over
   `src/<name>.c` (`:113-118`). Two overlays exist today: `memory.c` and
   `set.c`. **Any** failure stops the build (`:130-139`).
6. Assemble `sdk/libc/crt0.S` (`:143-144`).
7. Link `crt0.o` + all objects + `--start-group cflib.a gemlib.a libm.a
   libc.a --end-group` through `sdk/newtos.ld` → `sdk/apps/qed/build/QED.NAP`
   (`:149-153`); report undefined symbols (goal 0, `:159-167`).

**Resource file.** `tools/make-sd-image.sh` installs QED.NAP **and
qed.rsc** from `sdk/apps/qed/build/`. Since TASK/T06 `build-qed.sh` produces the latter itself,
after a successful link:

```sh
python3 tools/mkrsc.py convert sdk/apps/qed/src/qed.rsc sdk/apps/qed/build/qed.rsc
```

— the way `build-teradesk.sh` stages TeraDesk's resources. The German
`sdk/apps/qed/src/qed.rsc` is the one that matches the shipped artifact (`sdk/apps/qed/en/qed_en.rsc`
converts to 40309 bytes). History of the artifact: 41322 bytes, SHA-256
`5a2cefa2…` up to 2026-09-14; since A40 (2026-09-19) the tool emits char-grid
coordinates — NRSC v9, SHA-256 `3383dd39…`, same size — and since A41 part 2
the same day NRSC v13 with the 40-byte `RSHDR` preamble and the 84-byte
header: **41378 bytes, SHA-256 `30ac6bf2…`**, which is what the script now
writes (verified byte-identical to the previously hand-made file, TASK/T06).

## TeraDesk, step by step (`sdk/apps/build-teradesk.sh`)

1. `lib_ensure gemlib` — TeraDesk bundles its own dialog library
   (`library/xdialog`, `library/utility`) and does **not** use cflib (`:5-9`,
   `:62-63`).
2. Flags: shared freestanding set + TeraDesk's `-Os -fomit-frame-pointer`,
   `-funsigned-char`, `-D__NEWTOS__`, **`-D_TOS_ONLY`** (builds the
   single-TOS variant, `_MINT_ == 0`, until the cooperative kernel lands,
   `:87-94`). Include order: shadow tree root, its `xdialog` and `utility`
   dirs, gemlib top, mintlib, clang (`:97-98`).
3. Shadow tree: build/tree/ = symlinks to every vendored `.c`/`.h`, then the
   overlays layered on top — seven files (`ls -R sdk/overlays/teradesk`):
   `environm.c`, `sdk/overlays/teradesk/library/utility/nf_ops.c`, `window.h` (under sdk/overlays/teradesk/); since A60
   (2026-09-19) `icon.c`, `rsc_load.c` and `sdk/overlays/teradesk/library/xdialog/library.h`, which
   make TeraDesk write and read the `AESGLOBAL` resource words as the 4-byte
   TOS fields at bytes 10/14 through gemlib's `_AESrscfile_set`/
   `_AESrscmem_set`; and since RST01/X04 (2026-10-17) `main.c` (32-bit
   low-memory words and the reset-vector read, see the table above).
4. File list: every `.c` under the tree except `sdk/apps/teradesk/library/utility/malloc.c`
   (libc provides `malloc`) (`:127-134`).
5. Compile all from the shadow tree; any failure stops the build and prints
   the file (`build-teradesk.sh`, the compile loop).
6. Link `crt0.o` + all objects + `--start-group gemlib.a libm.a libc.a
   --end-group` → `build/DESKTOP.NAP` (`:164-187`).
7. **Dist staging** (`:189-205`): build/dist/ receives `DESKTOP.NAP`,
   `desktop.rsc` and `cicons.rsc` converted with `tools/mkrsc.py convert`
   (NRSC v13/LP64, loaded by the **kernel's** `rsrc_load`; `cicons.rsc` are
   the colour icons — a new-format `CICONBLK` file, carried as native
   records since A41), and `icons.rsc` converted with `mkrsc.py convert
   --rshdr` (native-RSHDR, parsed by TeraDesk's **own** `xrsrc_load`).
   TeraDesk's `load_icons` tries `cicons.rsc` through the AES first and
   falls back to its own `icons.rsc` when it is absent (mono icons).

`tools/make-sd-image.sh:282-296` copies those files into
`C:\Apps\TERADESK\` (`cicons.rsc` only when the dist directory has it).

## CoreMark, step by step (`sdk/apps/build-coremark.sh`)

1. MD5-check the five benchmark translation units and `coremark.h` against
   `sdk/apps/coremark-sources.md5`; refuse to build on any mismatch (`:73-104`).
2. `lib_ensure libc` (`:109-110`).
3. Flags: `-O2 -fomit-frame-pointer -ffreestanding -fPIE -fno-builtin`
   (reported by the benchmark via `-DFLAGS_STR`), `-D_GNU_SOURCE`,
   `-DITERATIONS=$ITERATIONS` (`:133-138`); include order: port dir, CoreMark
   root, mintlib, clang (`:140`).
4. Two complete builds — `PERFORMANCE_RUN=1` → `CORE.NCP`,
   `VALIDATION_RUN=1` → `COREVAL.NCP` — because a program cannot be given a
   command tail interactively (`:146-181`, `core_portme.h:41-51`).

## Resource files (`.RSC`)

NewTOS does not load classic big-endian Atari `.RSC` files. ADR-013
(`docs/MEMORY.md:376-455`) defines a native little-endian format ("NRSC")
whose `OBJECT` is the kernel's `repr(C)` 32-byte layout with a 64-bit
`ob_spec`, pointers stored as byte offsets relocated by `rsrc_load`
(`tools/mkrsc.py:1-22`, `tos-api/src/aes/resource.rs:1-110`). Since A41
part 2 (NRSC v13/v14) a kernel-loadable file **begins with a native
`RSHDR`** — the classic header's 18 fields as little-endian `u16`, offsets
from the file's first byte, padded to 40 bytes — and the NRSC header follows
at byte 40. So the image the AES hands an application as `ap_rscmem` is the
file with an `RSHDR` at its head, as on EmuTOS, and an application that
parses that header itself (TeraDesk's `xrsrc_gaddr` on the AES-loaded
`cicons.rsc`) reads what it expects; see [05-abi-pitfalls.md](05-abi-pitfalls.md#b8)
symptom 5. Two converters exist in `tools/mkrsc.py` (`main`):

* `convert SRC OUT` — for resources the **kernel** loads via `rsrc_load`
  (QED's `qed.rsc`, TeraDesk's `desktop.rsc` and `cicons.rsc`);
* `convert --rshdr SRC OUT` — for an application that parses the file with
  its **own** loader onto native LP64 structs (TeraDesk's `icons.rsc`).

The converter also word-aligns every raster (`RASTER_ALIGN = 2`,
`mkrsc.py:87-100`) — the reason is in [05-abi-pitfalls.md](05-abi-pitfalls.md#p8).

Where the file must live: `rsrc_load` resolves the name through `sh_find`,
which looks first in the **application directory** (`p_appdir`, set by the
launcher from the launch path), then the name as given, then the root of the
current drive (`tos-api/src/aes/shell.rs:777-830`,
`resource.rs:1304-1313`). That is why `make-sd-image.sh` puts `qed.rsc` next
to `QED.NAP` in `C:\Apps\QED\` (`:209-211`, `:259-266`).

## Adding a new port: the skeleton

1. Vendor the source under `sdk/apps/<app>/` (untracked; record provenance
   the way `coremark-sources.md5` does).
2. Create `sdk/overlays/<app>/` (tracked) when the first per-program patch is needed.
3. Copy `sdk/apps/build-qed.sh` (cflib app) or `build-teradesk.sh` (own
   dialog lib, header overlays needed) to `sdk/apps/build-<app>.sh`; keep
   `lib_ensure`, the flag set, the "any failure stops" rule.
4. Build. For every compile error decide: does every LP64 port meet this
   (library overlay, shim, header, flag → fix it once in the SDK), or is it
   this program's own assumption (m68k asm, 32-bit `long`, big-endian
   tricks → a per-program patch file in `sdk/overlays/<app>/` with a header
   that says why)? Every undefined symbol at link time is first a library
   question (`why.txt` in the build directory shows the pull chain, `build-qed.sh:173`).
5. Convert the `.RSC` with `tools/mkrsc.py convert`.
6. Add the install lines to `tools/make-sd-image.sh` (`C:\Apps\<APP>\`, the
   `.rsc` beside the `.NAP`; never `\AUTO\` for a `.NAP`).
7. Test per [06-image-and-testing.md](06-image-and-testing.md), then walk
   [05-abi-pitfalls.md](05-abi-pitfalls.md) against the app's own use of TOS
   structures.
