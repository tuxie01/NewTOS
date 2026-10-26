# 10 — LDG modules (`.NLD`): building and loading native LDG libraries

Stand 2026-10-06, TASK/LDG-M1 (section 6 and the libshare rows: TASK/LDG-M3-B2,
`measurements/measurement-2026-10-06-libshare/RUNINFO.TXT`). Design and evidence trail:
`docs/analysis/ldg-after-arch03-2026-10-06.md` (the mechanism, file:line for
every step), `measurements/measurement-2026-10-06-ldg-m1/RUNINFO.TXT` (the
QEMU runs this page's "verified" statements rest on).

LDG ("GEM Dynamical Libraries", Landemarre/Bereziat/Bercegeay, LGPL 2.1) is
the Atari way of loading a library at run time: the library is an ordinary
program that a client loads with `Pexec(3)` ("load, don't go"), starts once
with `Pexec(4)` so that its `main()` can hand back a descriptor, and whose
functions the client then calls directly through pointers. zView's codecs are
LDG modules; that is why NewTOS has it.

On NewTOS this runs **through LDG's own client code, unchanged in logic** —
there is no NewTOS loader for LDG. The kernel's `Pexec` loads and relocates
the module, makes its code visible to instruction fetch, leaves the module
block owned by the client, and runs `Pexec(4)` as a thread in the client's
address space (`docs/api/gemdos/process/4b-pexec.md`, `PE_GO`).

## 1. The pieces

| What | Where | Built by |
|---|---|---|
| LDG source, pinned | `sdk/ldg/ldg-svn-r133.tar.xz` (SVN r133, sha256 in `sdk/ldg/ldg-sources.sha256`, per-file manifest `ldg-svn-r133.files.sha256`) | — |
| `libldg.a` (client + module side: `ldg_open/find/close/error`, `ldg_init`, `ldg_getpath`, `ldg_Malloc` & Co.) | `sdk/ldg/libldg.a` | `sdk/ldg/build-ldg.sh` (also via `lib_ensure ldg`, `sdk/lib-stale.sh`) |
| Headers | `sdk/ldg/ldg-r133/include/ldg.h` (extracted tree) | `build-ldg.sh` |
| Overlays | `sdk/ldg/overlay/src/devel/{ldgload.c,ldgutil.c}` | — |
| Demo module + acceptance client | `sdk/ldg/build/EX_LIB.NLD`, `sdk/ldg/build/LDGTEST.NCP` | `sdk/ldg/build-ldg-demo.sh` |
| LDG's own WinDom demo client + its second module (TASK/LDG-M3) | `sdk/ldg/build/client/dist/LDGDEMO/` (CLIENT/LDGDEMO.NAP, CLIENT/CLIENT.RSC, LIBS/EX_LIB.NLD, LIBS/POPUP.NLD) | `sdk/ldg/build-ldg-client.sh` ([12](12-windom.md)) |
| `libshare.a` — the allocator every module is linked with (libshare 0.1.1, TASK/LDG-M3-B2, section 6) | `sdk/libshare/libshare.a`; pin `sdk/libshare/libshare-0.1.1.tgz` + `libshare-sources.sha256`; overlays `sdk/libshare/overlay/{sbrk,calloc,malloc}.c`; header `sdk/libshare/libshare-0.1.1/libshare.h` | `sdk/libshare/build-libshare.sh` (also via `lib_ensure libshare`: chain libc → ldg → libshare) |

The two overlays below are the only source differences of `libldg.a` (each
file's header says why, with file:line); `sdk/ldg/ldg-r133/demo/client/client.c` has a third,
for the demo client only (module suffix `.ldg` → `.nld`, TASK/LDG-M3):

* `ldgload.c` — `BASEPAGE.p_env` is a fixed 32-bit field in the SDK
  (`sdk/libc/overlay/include/mint/basepage.h`), so the one assignment to a
  `char *` gets a `(char *)(uintptr_t)` cast.
* `ldgutil.c` — `ldg_cookie` reads the jar through `Getcookie()`
  (`Ssystem(S_GETCOOKIE)`) instead of `*(long *)0x5a0` (8 bytes on LP64,
  the jar holds 32-bit words); `ldg_cpush` is empty because the kernel
  already synchronised the caches inside `Pexec(3)`
  (`tos-api/src/bdos/process/loader.rs:599`).

`ldgapp.c` and `ldglib.c` compile unmodified.

## 2. Building a module

An LDG module is built **exactly like a program** — PIE, `sdk/libc/crt0.S`,
`sdk/newtos.ld` — and only the name differs: **`.NLD`** (ADR-018 amendment
2026-10-06 in `docs/MEMORY.md`; `.LDG` stays reserved for m68k modules and the
later emulator). Its `main()` calls `ldg_init(&descriptor)` and returns.

The link line `build-ldg-demo.sh` uses for `ex_lib` (`ld.lld`, or
`rust-lld -flavor gnu` with the Rust fallback — `sdk/toolchain.sh`):

```sh
ld.lld -T sdk/newtos.ld -pie --no-dynamic-linker -z text \
    -z max-page-size=0x1000 --build-id=none crt0.o ex_lib.o \
    --start-group sdk/libshare/libshare.a sdk/ldg/libldg.a sdk/gemlib/gemlib.a \
                  sdk/libm/libm.a sdk/libc/libc.a --end-group \
    -Map=link-EX_LIB.NLD.map -o EX_LIB.NLD
```

compiled with the application flag set of `sdk/apps/build-qed.sh` and the
include path `-I sdk/ldg/ldg-r133/include` before gemlib's and mintlib's.
**`libshare.a` comes first in the group** — `-lshare -lldg`, as zView's codec
makefiles link — so that its `malloc/free/realloc/calloc/sbrk/chdir/memalign`
win over libc's; the map is then checked by `ls_check_module_map`
(`sdk/libshare/libshare-link.sh`), which stops the build if any of libc's
allocator members (`mintlib__malloc.o`, `…calloc.o`, `…realloc.o`,
`…posix_memalign.o`, `unix__sbrk.o`) was pulled next to libshare's. Why a
module needs libshare: section 6. A **client** links the same archives
**without** `libshare.a` (it needs `libldg.a` for `ldg_open` & Co. and keeps
libc's malloc).

* `PROC` tables with function pointers in `.data` are fine: they become
  `R_AARCH64_RELATIVE` entries, which the kernel applies when it loads the
  module. `-z text` keeps relocations out of `.text` (any other relocation
  type makes `Pexec` fail with `EPLFMT`, `tos-api/src/bdos/process/reloc.rs`).
* `ldg_init` finds the client through `getenv("OFFSETLDG")`. That works since
  TASK/PORT09 (the libc start-up builds `environ` from `p_env`); before it,
  every module failed here.
* A module needs no special `PRGFLAGS`: the default `PF_STANDARD` gives a
  `PROT_P` block whose owner is the **client** that loaded it.

## 3. Loading one (client side)

```c
LDG *ldg = ldg_open("C:\\LDG\\EX_LIB.NLD", aes_global);   /* or a bare name after chdir */
double (*sinus)(Real *) = ldg_find("sinus", ldg);
...
ldg_close(ldg, aes_global);
```

`ldg_open` reads only `gl[2]` (the AES id, stored in `ldg->id`), so a client
that runs before the AES (an `\AUTO\` `.NCP`) may pass a zeroed array of its
own — `sdk/examples/ldgtest.c` does.

## 4. Limits (what is not there, and what that means)

| Limit | Consequence |
|---|---|
| **No LDG TSR (`ldg.prg`), no `LDGM` cookie** | Every `ldg_open` loads the module again (no sharing, no reference count, no `LDG_NOT_SHARED`/`LDG_RESIDENT`/`LDG_LOCKED` handling — those are TSR features, `ldgcook.c`). `ldg_error()` always returns `LDG_NO_TSR` (-11, `sdk/ldg/ldg-r133/src/devel/ldgapp.c:103`): detect failure by `ldg_open` returning `NULL`. `ldg_libpath` returns `LDG_NO_TSR` |
| **No library search path** | Without the TSR `ldg_open` passes the name straight to `Pexec(3)` (`sdk/ldg/ldg-r133/src/devel/ldgapp.c:37-38`): give an absolute path, or `chdir` to the module's directory and pass the bare name (as zView does) |
| **No `ldg_callback`** | `stdcall.s` is m68k assembly and is not built; the symbol does not exist in `libldg.a`, so a program that calls it fails to **link** (lead decision 2026-10-06; no stub) |
| **libshare in every module** (since TASK/LDG-M3-B2; until then this row read "No libshare — not needed by plain modules", which was wrong for every module that mallocs) | Each `.NLD` is linked with `sdk/libshare/libshare.a` first (section 2), also `EX_LIB.NLD` and `POPUP.NLD`, whose LDG makefiles do not link it. Why and what it does: section 6 |
| **One client per module** | The module block is `PROT_P`, owned by the client that loaded it; another process cannot call into it under `memprot=on/strict` |
| **Environment copy ≤ 512 bytes** | `ldg_load` passes `OFFSETLDG=<addr>` followed by a copy of the client's environment to `Pexec(3)`; the kernel reads at most `ENV_BUF_LEN = 512` bytes of it and truncates the rest silently (`kernel/src/arch/aarch64/exceptions.rs:1781`). `OFFSETLDG` comes first, so loading is unaffected; the module just sees a shortened environment |
| **64 KiB stack kept per loaded module** | mintlib's `_crtinit` keeps `_stksize` (default `MINKEEP` = 64 KiB, `sdk/libc/newtos-arch/heap_init.c:63`) in the module block; module functions run on the **client's** stack, so it is unused. Original mintlib behaviour, kept on purpose (lead decision Q6) — a module may set its own `_stksize` |
| **Module `Malloc`s during `main()` are freed at its `Pterm`** | As on TOS: memory the module allocates in `main()` belongs to the module and is released when `Pexec(4)` returns. Memory allocated later, inside a function the client calls, belongs to the client |
| **B2 (closed by libshare, TASK/LDG-M3-B2): with libc's malloc a module's `malloc` after loading read freed memory** | The row above has a consequence libc's malloc hides: the module's `exit()` closes stdio (`__fcloseall` → `fclose` → `init_stream` `malloc` + `free`), so the module's own static free list (`_mchunk_free_list`) ended `main()` pointing into a hunk its `Pterm` frees; the next `malloc` inside the module (called by the client) faulted under `memprot=on` (POPUP.NLD `__get_bkgr`, `measurements/measurement-2026-10-06-ldg-m3/`, cause chain `…-ldg-b2/`) and double-used/hung with `memprot=off`. `EX_LIB.NLD` had the same dangling list (latent — it never mallocs again; this row said "unaffected" until LDG-M3-B2). With libshare: section 6 |
| **Residual (libshare, as on the reference)** | Memory a module mallocs during its **own** run (`main()`/`exit()`) beyond libshare's 16 KiB static block, and does **not** free before returning, still comes from `ldg_Malloc` with the module as owner, is freed at its `Pterm`, and libshare's list keeps pointing into it. LDG's rule for module authors stands: allocate in the module's functions (client context), use `ldg_Malloc` & Co. for memory exchanged with the client (`sdk/ldg/ldg-r133/guides/src/ldg.u:1047-1051`, `sdk/ldg/ldg-r133/guides/src/mem.stg:44-52`) |
| `Pwait()` | LDG calls MiNT's `Pwait` (0x109) after `Pexec(4)` and ignores the result; NewTOS answers `EINVFN` — harmless |

## 5. Testing

`tools/make-sd-image.sh` installs `C:\LDG\EX_LIB.NLD` and `C:\LDGTEST.NCP`
when `sdk/ldg/build-ldg-demo.sh` has been run (QEMU image only), and
`SD_LDGTEST_AUTO=1` additionally puts the client into `\AUTO\`. Its serial
verdict line is `ldgtest: ALL PASS (N checks)` or `ldgtest: FAIL (k of N
checks failed)`; the checks are listed at the top of `sdk/examples/ldgtest.c`.

**Only real hardware can confirm** that the code of a freshly loaded — and
especially a **re-loaded** — module is fetched correctly: QEMU has no caches
(`loader.rs:594-597`), so the I-cache maintenance in `Pexec(3)` is exercised
only on a Pi. `LDGTEST`'s reload step (close, open again, often at the same
address) is the case to watch there.

## 6. libshare: the allocator of every module (TASK/LDG-M3-B2)

**Licence: unclear.** libshare 0.1.1 has no licence file and no licence
header in any source; the only statement is the web page footer "Copyright
(c) 2004-2005 Arnaud BERCEGEAY" (http://arnaud.bercegeay.free.fr/libshare/).
`chdir.c` says "By ERS: it's in the public domain"; `malloc.c`, `calloc.c`,
`realloc.c`, `sbrk.c` are marked "from the TOS GCC library" (the lineage of
mintlib's allocator). It is ported by user decision 2026-10-06 (the original,
LP64/target changes as overlays); **the user will ask the author** about the
licence. Until that is answered, do not publish binaries that contain it as if
its licence were known. Same note in `sdk/libshare/libshare-sources.sha256`.

**Why a module needs it.** A module is a mintlib program that LDG starts once
with `Pexec(4)` and that then lives on inside its client. Memory it gets
during that run belongs to the module and is freed at its `Pterm` (as on
EmuTOS, `bdos/proc.c:92-101,136-140`). libc's malloc keeps a static free list
and takes its memory with GEMDOS `Malloc`: the stdio buffers `exit()` mallocs
and frees leave that list pointing into a hunk the `Pterm` frees — finding B2
(section 4; `docs/analysis/ldg-module-malloc-2026-10-06.md`). libshare
replaces `malloc/free/realloc/calloc/sbrk/chdir` (its `lib.h`: "a dirty patch
to make LDG compiled against mintlib shareable"):

* the **first** block is a static 16 KiB array in the module's BSS
  (`references/libshare/orig-0.1.1/libshare-0.1.1/malloc.c:50-52,120-126`) — part of the module's TPA, which the client
  owns, so it survives the module's `Pterm`;
* every further block comes from `sbrk` → `ldg_Malloc` (`sbrk.c:68-75`), i.e.
  GEMDOS `Mxalloc` in the context of whoever runs — the **client** once the
  module is loaded and the client calls into it;
* `free` hands a hunk that became completely free back with `ldg_Free`
  (`references/libshare/orig-0.1.1/libshare-0.1.1/malloc.c:239-252,271-289`). (libc's mintlib 0.60.1 `free` does not merge a
  freed chunk that is the last one in the list — `if (q && …)`,
  `sdk/libc/mintlib/mintlib/malloc.c:190` — so the exit-path hunk is never
  returned and stays referenced; libshare's copy lacks that `q &&`.)

Measured on the modules of this tree (`measurements/measurement-2026-10-06-libshare`,
instrumented scratch kernel): during a module's `Pexec(4)` run `exit()`
closes stderr then stdout; the first buffer lands in the static block, which
libshare then drops from its list (empty static block, `references/libshare/orig-0.1.1/libshare-0.1.1/malloc.c:271-289`);
the second gets a 16 KiB hunk via `Mxalloc` (owner: the module), which
libshare's `free` returns with `Mfree` **before** the `Pterm`. At the `Pterm`
the module's `libshare_mchunk_free_list.next` is 0. `POPUP.NLD`'s
`__get_bkgr` then mallocs in the client's context (`Mxalloc(0x8000)`, owner
the client) and frees it in `__put_bkgr`; the pop-up works under
`memprot=on`, `strict` and `off`.

**What is built** (`sdk/libshare/build-libshare.sh`): libshare's own object
list (`makefile:28`) with its own flags (`makefile:17-26`), against mintlib's
headers and `ldg.h`; the tarball's prebuilt m68k `libshare.a` and test
binaries are deleted after extraction and never used. Three overlays, each
with line and reason in its header:

| File | Change | Reason |
|---|---|---|
| `sbrk.c:61` | `sbrk(size_t)` → `sbrk(intptr_t)` | mintlib 0.60.1 `<unistd.h>:934` declares `intptr_t`; the pristine file does not compile (th-otto's copy has the same change) |
| `calloc.c:25` | `__bzero` → `bzero` | the SDK's libc defines only `bzero` (`sdk/libc/newtos-arch/memops.c:70-74`), no `__bzero` — undefined symbol otherwise |
| `references/libshare/orig-0.1.1/libshare-0.1.1/malloc.c:51` | `static_mblock` aligned like `struct mem_chunk` | LP64: compiled pristine the array sits at `.bss+0x11`; the chunk header holds 8-byte fields and malloc hands out pointers into it |

Plus **one addition**: mintlib's own `sdk/libc/mintlib/mintlib/posix_memalign.c`
(`memalign`, `posix_memalign`, `aligned_alloc`), unmodified, compiled with
`-D__malloc=malloc` into `libshare.a`. In mintlib 0.60.1 those call
`__malloc` (`sdk/libc/mintlib/mintlib/posix_memalign.c:22,25`), the strong name of libc's allocator,
which libshare (2004) does not replace; without the addition a module calling
them would run two allocators. libc's `valloc` calls `memalign` and so lands
on libshare's too. Link probe: `measurements/measurement-2026-10-06-libshare/checks/link-probes.txt`.

**`libshare_init()` / `libshare_exit()`** are in the archive and do nothing
in 0.1.1 (`sdk/libshare/libshare-0.1.1/libshare.c:36-58`); zView's codecs call them. The SHM functions
(`libshare_getshm/createshm/deleteshm`) are experimental in the original
("recommended to NOT use them", README) and unreachable on a single-TOS
system (`has_shm()` returns 0, `sdk/libshare/libshare-0.1.1/gl_shm.c:66-81`).

**Tests:** `sdk/libshare/tests/libshare-host.sh` (run by
`sdk/run-host-tests.sh`) — the module run / Pterm / client sequence against a
model of `ldg_Malloc`/`ldg_Free` and owner-based freeing, 15 checks.

