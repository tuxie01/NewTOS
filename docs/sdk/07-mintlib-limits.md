# 07 — What the ported mintlib does not provide

Derived from `sdk/libc/build-libc.sh` (what is compiled at all),
`sdk/libc/expected-failures.txt` (what is known not to compile), the shim
headers in `sdk/libc/newtos-arch/`, and an `llvm-nm` pass over the `libc.a`
built on 2026-09-17 (613 objects compiled, 11 failed, 6 excluded —
`sdk/libc/build/{compiled,failed,excluded}.txt`); re-measured 2026-09-21
after TASK/T07 added `setjmp` and the libm (§3, §4, §6 updated; the numbers
that did not change are left as measured), and again 2026-09-25 after
TASK/PORT01 added `newtos-arch/cmdargs.c` (§1, §5, §6). TASK/PORT10
(2026-10-02) updated §1, §5, §6 and added §8: `spawnve.c`/`spawnvp.c` compile
(9 failures now, not 11), `environ` is the empty environment, and the GMP limb
width behind `printf("%f")` is fixed. Later the same day §8: the program-name
search uses NewTOS's extensions (`ntp`, `nap`, `ncp`); and then the execute
bits of `stat()`/`access()` follow the same native extensions (§8). TASK/PORT-WAITH
(2026-10-06) added §9: the `<sys/wait.h>` status macros read the status word
correctly on little-endian AArch64 (header overlay). TASK/PORT09 (2026-10-06)
updated §1 and §5: `newtos-arch/cmdargs.c` is now all of `parseargs()` —
`environ` from `p_env`, the `ARGV=` extension with `NULL:`,
`program_invocation_name`. TASK/MEM05 part B (2026-10-13) updated §5
(`malloc`) and added §10: the overlay `mintlib/malloc.c` lets malloc's hunks
keep doubling up to 16 MiB. TASK/SDK01 A (2026-10-20) updated §1, §2, §4,
§6 and added §11: `pwdgrp/` is compiled (the temp-file family, `glob`,
`wordexp`, `getlogin` link), `__sys_fstat` (`ttyname`, `ctermid`),
`gets`/`getwd`/`siggetmask` from overlays, `newtos-arch/nf_ops.c`, no test
program's `main` in `libc.a`; 627 compiled, 5 failed, 14 excluded.
TASK/SDK01 E (2026-10-21) updated §4, §6 and §11: the SDK has a compiler
runtime now — LLVM compiler-rt's builtins, `sdk/compiler-rt/
libclang_rt.builtins.a`, linked by `newtos-cc` — so the binary128 and
complex-arithmetic helpers resolve: in `libm.a` 21 → 3 functions that do not
link (`scalbln`/`scalblnf`/`scalblnl`, which need `scalbnl`), in `libc.a`
12 → 7; `qecvt` & Co. link but print wrong digits (§11). TASK/SDK01 J
(2026-10-21) updated §4, §5, §6 and §11: `long double` (binary128) is real —
libc without `-D__NO_LONG_DOUBLE_MATH`, binary128 overlays for mintlib's
m68k-only long double code, OpenLibm's `ld128/` `…l` functions built;
`libm.a` 3 → 0 functions that do not link, `qecvt` & Co. print correct
digits; and the float word order of `<bits/endian.h>` is little-endian, so
`printf("%f", INFINITY)` prints `inf`.

## 1. mintlib files that do not compile (recorded, not repaired)

`sdk/libc/expected-failures.txt` is the authoritative list; the build stops
on any failure not in it (`sdk/lib-stale.sh:229-283`). What each costs:

| File | Reason (from the record) | What a program loses |
|------|--------------------------|----------------------|
| `mintlib/linea.c` | m68k inline asm — ST Line-A. **Excluded by ADR-030** (no Line-A on NewTOS), not a failure to repair; listed in `expected-failures.txt` until the build excludes it | the Line-A bindings `linea0()` … `lineaf()`; use the VDI calls instead (`docs/COMPATIBILITY.md`, "No Line-A") |
| ~~`mintlib/nf_ops.c`~~ | m68k inline asm — ARAnyM NatFeats; excluded and replaced by `newtos-arch/nf_ops.c` since TASK/SDK01 A (§2) | — |
| `mintlib/crtinit.c` | m68k asm + two LP64 basepage casts | nothing any more: `newtos-arch/cmdargs.c` is `parseargs()` with AArch64 pointer widths — `argc`/`argv` (PORT01), `environ`, the `ARGV=` extension and `program_invocation_name` (PORT09) — and `newtos-arch/heap_init.c` is `_crtinit`/`_acc_main` (`_base`, `_stksize`, Mshrink; T05, E9) — see §5 |
| ~~`mintlib/spawnve.c`, `spawnvp.c`~~ | compile since TASK/PORT10 (`sdk/libc/overlay/mintlib/`, `va_arg` instead of `va_list` arithmetic) | — the `spawn*` family works (synchronous `Pexec(0)`, QEMU-verified); `system()` is mintlib's own since TASK/PROC01 — §8 |
| ~~`stdio/gets.c`~~, ~~`mintlib/getwd.c`~~, ~~`signal/siggetmask.c`~~ | `.stabs` directive (`link_warning`) rejected by the integrated assembler; compile since TASK/SDK01 A from overlays without that line (`sdk/libc/overlay/{stdio,mintlib,signal}/`) | — `gets`, `getwd`, `siggetmask` are in `libc.a` (without the GNU-ld link-time warning) |
| `mintlib/syscall.c` | needs the generated `syscall-list.h` | the generic `syscall()` |
| `gmp/support.c` | `mathimpl.h` not on the include path | part of the GMP float-printing helpers (the rest of `gmp/` compiles) |
| `tz/dumpstate.c` | struct members missing | `tz` debug dump only |

## 2. mintlib files deliberately replaced by lean shims

`EXCLUDE` in `build-libc.sh:111-135`; the replacement lives in
`sdk/libc/newtos-arch/`. "Lean" means: the real TOS call is made, the Unix
tail that has no TOS analogue is dropped.

| mintlib file | Shim | What is different |
|--------------|------|-------------------|
| `mintlib/do_fstat.c` | `do_fstat.c` | `__sys_fstat` (since TASK/SDK01 A; `ttyname`, `ttyname_r`, `ctermid` need it) = the `Ffstat64` trap only, as `__sys_stat` below — NewTOS answers `EINVFN` (= `-ENOSYS`); `ttyname` reaches it only on MiNT (`__mint >= 9`), on NewTOS it answers `/dev/con` (`/dev/aux` for -2). `__do_fstat` fills `st_mode` (`S_IFCHR\|0600` for a standard handle 0–5 that `__isatty` reports as a terminal — the same `__open_stat[]` cache and `Fseek` probe as `isatty()`, so a standard handle `Fforce`d to a file is `S_IFREG`, as in mintlib's own TOS branch `mintlib/do_fstat.c:102-121`, TASK/LIBC-FSTAT; `S_IFREG\|0644` otherwise, handles ≥ 6 without the probe), an honest `st_size` via `Fseek`, `st_blksize` (**32 KiB for a handle ≥ 0 that is not a terminal and whose size `Fseek` could read, `BUFSIZ` = 1024 otherwise** — named deviation, see below), `st_blocks`, `st_nlink = 1`; **times** (since TASK/SDK01 F, only for `fstat()` — the stdio/`read()`/`dup2()` callers pass `exact` 0 and make no extra trap): as mintlib's TOS branch (`mintlib/do_fstat.c:102-119`) — for a handle that is not a terminal `Fdatime(get)` and `st_mtime = st_atime = st_ctime = __unixtime(time, date)`, for a terminal or a handle `Fdatime` rejects (device, not open) `time() - 2`; **owner stays zero** (`:29-45`) |
| `mintlib/do_stat.c` | `do_stat.c` | `__sys_stat` = the `Fstat64` trap (mintlib's further `Fxattr` try is dropped — NewTOS answers it `EINVFN` too); `__do_stat` (`stat`, `lstat`) = mintlib's own TOS branch (`mintlib/do_stat.c:146-336`) since TASK/SDK01 F: the `Fsfirst` walk of `__quickstat` **plus the timestamps** — `st_mtime = st_atime = st_ctime = __unixtime(dta_time, dta_date)` (`do_stat.c:281-282`); a drive root (`C:`, `C:\`, `\`, or a `.`/`..` search that lands in one) gets mintlib's `OLDDATE = __unixtime(0, 0)` (= 1979-11-30 00:00 with TZ unset), a device name (`/dev/con`) `time() - 2`; exec bits from NewTOS's `.ntp`/`.nap`/`.ncp` as in `__quickstat` (TASK/PORT10); mintlib's `_x_Bit_set_in_stat` (m68k magic probe) is not carried over (`:1-48`). Before SDK01 F `__do_stat` was `__quickstat` and every time was 0 |
| ~~`mintlib/isatty.c`~~ | built again since TASK/LIBC-TTY (2026-10-13); the shim (`isatty(fd)` = `fd >= 0 && fd <= 2`, `__open_stat[].flags` never set) is gone | mintlib's own `__isatty`, as on TOS: `Fcntl(fd, .., TIOCGPGRP)` is `EINVFN` (= `-ENOSYS`), so the `Fseek` probe decides — `Fseek(1, fd, SEEK_CUR) == 0` (a character device, since TASK/FS10) = tty, a position or an error = file, the position is put back (`mintlib/isatty.c:28-48`); a tty gets `__open_stat[].status = FH_ISATTY`, `flags = CRMOD\|ECHO` (`:54-58`), which sends `read()` on the console through `Fread` (§5 "console `read()`"); a standard handle `Fforce`d to a file is a file; `isatty(-1)` is 1 (CON:); the answer is cached per handle until `open`/`close`/`dup`/`dup2` (an `Fforce` after the first query is not seen — upstream). The file also defines `__open_stat[]` |
| `mintlib/nf_ops.c` | `nf_ops.c` (TASK/SDK01 A) | NatFeats are an m68k emulator-host interface (opcodes 0x7300/0x7301, `__NF` cookie with m68k code pointers); there is no host on AArch64, so `nf_init()` = NULL ("not available", `nf_ops.h`); `nf_get_id`, `nf_catch_sigsys` keep mintlib's bodies. mintlib's own `nf_*.c` then give `nf_version()` 0, `nf_get_name()` NULL, `nf_debug()` 0, `nf_debugprintf()` -1/`ENOSYS`, `nf_shutdown()` 0, `nf_exec()` -32 |
| `unix/ioctl.c` | `ioctl.c` | forwards to `Fcntl(fd, arg, request)`; the m68k tty line-discipline path is gone (`:1-16`) |
| `unix/access.c` | `access.c` | checks the owner `rwx` bits from `__quickstat` only; no group/passwd database (`:1-19`) |
| ~~`mintlib/thread.c`~~ | builds since TASK/PROC01 from `sdk/libc/overlay/mintlib/thread.c` (LP64 casts for the 32-bit basepage fields, plus one fix: `tfork` frees the saved `Pexec(PE_CBASEPAGE)` environment instead of `b->p_env`, §8) | `tfork`, `__waitval`, `__waittime` — `vfork`'s TOS branch, `wait4` |

### stdio buffer size of a regular file: 32 KiB (named deviation, TASK/FS08 part 4)

mintlib's stdio gives a stream the buffer size `st_blksize` reports
(`stdio/stdio_init.c:72-77`, allocated in `internals.c` `init_stream`). mintlib's
own TOS branch reports 1024 (`mintlib/do_fstat.c:150`, `quickstat.c:90/248`),
and so did our shim (`BUFSIZ`). **Since 2026-10-07 (user decision: "a larger
buffer is no problem, we have more memory than a real Atari") the shim reports
32 KiB for a GEMDOS file handle (≥ 6) whose size `Fseek` could read**; the
console (fds 0–2, line-buffered as before), the other standard handles 3–5 and
negative (BIOS device) handles keep 1024. Since TASK/LIBC-FSTAT (2026-10-17) a
standard handle 0–5 `Fforce`d to a file counts as a file too (`S_IFREG`,
32 KiB, not line-buffered, seekable); at its device (CON, AUX, PRN) it is
`S_IFCHR` with 1024 — handles 3–5 included, which were `S_IFREG` with size 0
before. `stat()` (`__do_stat`, `__quickstat`) still says
1024 — only the open-stream path sizes buffers. A program that wants another
size calls `setvbuf()`. Host-checked by `sdk/libc/tests/fstat-host.sh`
(through mintlib's own `stdio_init.c`).

**The first `printf` keeps that size (TASK/LIBC-BUF, 2026-10-22, overlay
`sdk/libc/overlay/stdio/vfprintf.c`).** Upstream, a stream that is already
initialised but has no buffer yet goes through `buffered_vfprintf`
(pristine `stdio/vfprintf.c:1326-1329`, `:2237-2258`): a 1-KiB stack buffer, written
at once, `__bufsize` left 0, so the buffer the next `putc`/`fwrite` allocates
is `BUFSIZ` = 1 KiB. The redirected `stdout` is such a stream —
`mintlib/main.c:195-198` wants it fully buffered with `setvbuf(stdout, NULL,
_IOFBF, 0)`, which `setvbuf.c:63-67` rejects (size 0), and `init_stream_mode`'s
`fileno()` initialises it with `__bufsize` = 32 KiB but no buffer; so is a
`fopen` stream whose first call is `fileno()`. The overlay allocates the
`__bufsize` buffer first, as `ARGCHECK` does for a stream not yet
initialised. Unchanged: a stream unbuffered by request (`stderr`,
`setvbuf(_IONBF)`) and a line-buffered one — the console `stdout` still
writes a `printf` at once while it has no buffer, and is line-buffered with
1 KiB once a `putc`/`fputs` has given it one. Evidence:
`measurements/measurement-2026-10-22-libc-buf/RUNINFO.TXT`.

Why: on the real Pi 4 every 1-KiB `fread` on a file became one 1-KiB `Fread`,
i.e. one SD command: 1.35 MB/s against 11.7 MB/s for one large `Fread`
(`measurements/measurement-2026-10-07-fs08-pi-after/`). Why 32 KiB and not 64
(`measurements/measurement-2026-10-07-stdiobuf/RUNINFO.TXT`): with the Pi's
measured cost (~0.53 ms per `Fread` command, ~11.7 MiB/s streaming) a 10-MiB
sequential read costs ≈ 1.21 s at 16 KiB, 1.05 s at 32 KiB, 0.96 s at 64 KiB
(0.88 s for one whole-file `Fread`, 7.6 s at 1 KiB) — but every `fseek`
discards the buffer and the next `fread` below the buffer size refills a whole
buffer. DOOM reads every lump with `fseek` + `fread` (doomgeneric
`w_file_stdc.c`): in the same 90 s of QEMU it read 3.0 MB at 1 KiB, 15 MB at
16, 28 MB at 32 and 55 MB at 64 KiB (≈ +1.9 / +3.0 / +5.2 s on the Pi model).
32 KiB is the smallest size within ~0.1 s per 10 MiB of 64 KiB.

Costs to know when porting:

| What | Effect |
|------|--------|
| memory | one `malloc` of 32 KiB per open file stream, at its first transfer, freed at `fclose`; if that much is not free, mintlib halves the request down to 128 bytes and finally runs unbuffered (`internals.c:63-80`) — no failure, only a smaller buffer. GEMDOS allows 16 open files system-wide (`tos-api/src/bdos/fs/file.rs` `OPEN_FILES`), so stdio buffers cannot exceed 512 KiB in total |
| random access | a `fread` smaller than the buffer after an `fseek` reads up to 32 KiB (see DOOM above); a program that seeks a lot and reads little should `setvbuf()` a small buffer |
| text mode | a stream opened without `"b"` writes through `__stdio_text_write` in chunks of `min(st_blksize, 8192)` bytes on the **stack** (`sysd-stdio.c:205-221`, `alloca`): 8 KiB now instead of 1 KiB, well inside the 64 KiB default stack; reads ask for `bufsize - 1` bytes per `Fread` |
| LDG modules | a module's `malloc` is libshare's (16 KiB static block, then `ldg_Malloc` in the caller's context, empty hunks returned with `ldg_Free`): a codec's `fopen` (JPG, PNG, GIF) gets its 32 KiB through `ldg_Malloc` and gives it back at `fclose`. The exit-time console streams of a module stay at 1024 (`sdk/libshare/tests/libshare-host.sh`) |
| `fpathconf(fd, _PC_PIPE_BUF)` | returns `st_blksize`, i.e. 32768 for a file (`unix/fpathconf.c:24-32`) |

## 3. mintlib assembly that is never assembled

`build-libc.sh` compiles `.c` files only (`:149-169`). The 24 `.S` files in
the core directories are therefore absent unless a shim replaces them:

| Replaced by a shim | Missing — **undefined symbol if used** |
|--------------------|----------------------------------------|
| `string/bcopy.S`, `memset.S`, `bzero.S` → `memops.c`; `mintlib/libc_exit.S` → `proc.S`; `mintlib/getcookie.S` → `getcookie.c`; `mintlib/getsysvar.S` → `getsysvar.c`; `startup/crt0.S` → `sdk/libc/crt0.S`; **`mintlib/setjmp.S` → `setjmp.S`** (since 2026-09-21, TASK/T07 — see below); **`unix/vfork.S` → `vfork.S`, `mintlib/setstack.S` → `setstack.S`** (since 2026-10-05, TASK/PROC01 — §8) | `frexp.S`, `ldexp.S`, `modf.S` (the libm supplies `frexp`/`ldexp`/`modf` since T07, §4); `stdlib/alloca.S` (clang's `__builtin_alloca` covers `alloca()` when `<alloca.h>` maps to it — **unverified**); `getcontext.S`, `makecontext.S`, `setcontext.S`, `swapcontext.S`; `stkover.S`, `checkcpu.S`, `_mon.S`, `elfattr.S`, `_infinitydf.S`, `_normdf.S` |

**`setjmp`/`longjmp` (T07, 2026-09-21).** ADR-019 listed "setjmp (aarch64,
when pulled)" (`docs/MEMORY.md:1139`); it is pulled: `sdk/libc/newtos-arch/
setjmp.S` is the AArch64 retarget of mintlib's m68k `setjmp.S`, exporting the
same four symbols `sigsetjmp`, `setjmp`, `siglongjmp`, `longjmp` (mintlib's
`<setjmp.h>` builds `_setjmp`/`_longjmp` and the `setjmp()` macro on those;
under `_GNU_SOURCE` `setjmp(jb)` is `sigsetjmp(jb, 1)`). The control flow is
the m68k source's, instruction for instruction: the mask slot holds
`savemask`, then — when non-zero — `_sigmask | 1` (mintlib's TOS-emulation
mask, `signal/initsig.c`), or `Psigblock(0) | 1` when `__mint` is non-zero;
`longjmp` calls `Psigreturn()` when `__mint` is non-zero, restores `_sigmask`
(bit 0 cleared, plus `Psigsetmask` under MiNT) when the slot is non-zero, and
returns `val` or 1. On NewTOS `__mint` is 0 (only `crtinit.c` writes it, and
it does not compile), so the GEMDOS calls are never issued. The buffer is
`typedef long jmp_buf[22]` on AArch64 (overlay `include/setjmp.h`; m68k keeps
`[14]`): x19–x28, x29, x30, sp, d8–d15, sigmask — 176 bytes, layout table in
`setjmp.S`. Verified twice: on the host (`sdk/libc/tests/setjmp-host.sh`,
13 checks, the register check proven by sabotage — removing the `d8–d15` or
`x19/x20` restore fails exactly it) and on NewTOS (`sdk/examples/setjmptst.c`
→ `C:\AUTO\SETJMPT.NCP`, QEMU raspi4b serial: `setjmptst: ALL PASS (13
checks)`, `measurements/measurement-2026-09-21-t07/`).

## 4. mintlib directories not compiled at all

`DIRS` (`build-libc.sh`) covers `stdio stdlib string mintlib unix syscall
time misc multibyte signal termios locale dirent gmp posix tz pwdgrp`
(`pwdgrp` since TASK/SDK01 A). Not compiled:
`argp`, `conf`, `crypt`, `login`, `shadow`, `socket`, `startup`,
`stdiio`, `sunrpc`, `sysvipc`. Consequences: no sockets (`socket`,
`gethostbyname` are dangling references), no `crypt`, no SysV IPC, no
`argp`, no integer-only `stdiio` (the full `printf` is linked instead,
`build-qed.sh:30-31`).

The passwd/group database (`pwdgrp/`) links since TASK/SDK01 A; it reads
`/etc/passwd` and `/etc/group`, which a NewTOS volume does not carry, so
`getpwuid()`/`getpwnam()` return NULL and `getgrent()` finds nothing —
mintlib's behaviour on a plain TOS. `getlogin()` answers mintlib's fallback
(`user` in the SDK01 A QEMU run). Of `syscall/` only the host generator's
sources exist (`syscall/SRCFILES` = the generated trap files, which NewTOS
does not generate), so none of it goes into `libc.a` any more.

**libm: OpenLibm (T07, 2026-09-21).** mintlib never contained a libm nor a
`<math.h>` (on m68k both came from the compiler's PML/fdlibm; its
`include/tcmath.h` is a placeholder that `#error`s), and `libc.a` defines none
of `sin`, `sqrt`, `pow`, `floor`, `fmod`, … (measured, unchanged). The SDK's
libm is **OpenLibm 0.8.8** (lead decision 2026-09-21; the FreeBSD msun /
OpenBSD libm / fdlibm-5.3-derived standalone libm the Julia project maintains,
MIT + ISC + 2-clause BSD + the SunPro fdlibm notice — `sdk/libm/
LICENSE.openlibm.md`, provenance and SHA-256 in `sdk/libm/openlibm-sources.sha256`),
built exactly like the other three libraries: pinned tarball + `overlay/`
(one file, `src/s_scalbln.c`, SDK01 J) + `sdk/libm/build-libm.sh` → `sdk/libm/libm.a`, with the
`expected-failures.txt` gate (empty) and a place in `lib-stale.sh`'s chain
(`libc → libm`, a second branch off libc; gemlib/cflib do not depend on it).

What is in `libm.a` (256 objects, 322 global symbols since TASK/SDK01 J;
OpenLibm's own `src/`, `bsdsrc/`, `aarch64/` and `ld128/` `Make.files`
manifests expanded by `make`): the C99 `double` and `float` functions, the
complex functions, `tgamma`, the `<fenv.h>` functions over FPCR/FPSR
(`aarch64/fenv.c`) and — since SDK01 J — the `long double` (`…l`) functions:
OpenLibm's own AArch64 configuration outside Darwin (`LONG_DOUBLE_NOT_DOUBLE`,
`ld128/`), AAPCS64 `long double` being binary128, every operation on it a
soft-float libcall into compiler-rt
([05 B12](05-abi-pitfalls.md#b12-long-double-is-128-bit-and-the-compiler-runtime-is-compiler-rt)).
Until SDK01 J the SDK built the Darwin cut without them (TASK/T07 "Arbeit"
2). The 175 `double`/`float` members are byte-identical to that build. Pure C
— OpenLibm has no AArch64 assembly beyond `fenv.c`. One upstream defect is
fixed by the overlay: `scalblnl` passed `(int)n` instead of the clamped `n`
(`scalblnl(1, 1L << 40)` returned 1). Measured on the host against a
130-digit reference (`sdk/libc/tests/ldbl-host.sh`, the target's machine code
run natively): the exact functions (`scalbnl`, `ldexpl`, `frexpl`, `modfl`,
`floorl`, `ceill`, `truncl`, `roundl`, `rintl`, `nextafterl`, `fmodl`,
`remainderl`, `remquol`, `fmal`, …) are bit-exact; `expl`, `exp2l`,
`expm1l`, `logl`, `log2l`, `log10l`, `log1pl`, `powl`, `cbrtl`, `hypotl`,
`sinl`, `cosl`, `tanl`, `atanl`, `sinhl`, `coshl`, `tanhl` within 2 ulp
(maxima 0.49–1.68 ulp); `sqrtl` within 1 ulp but not correctly rounded in
about a quarter of the cases — FreeBSD's `e_sqrtl.c` decides the last bit
with `fetestexcept(FE_INEXACT)` after a division, and compiler-rt's
`__divtf3` raises no flags. With `-lm`, `__isinfl` comes from OpenLibm
(`1` for −∞) rather than from mintlib (`-1`), as before SDK01 J.
The `double`/`float` members that carry `long double` on a NaN/inf path
(`powf`, `remainder`, `remainderf`, `fdim`/`fdimf`, `nexttowardf`) and the
complex ones that need the compiler-rt complex multiply (`casin`, `cacos`,
`casinh`, `cacosh`, `catanh` and the `f` forms, `cpowf`) **link since
SDK01 E** through `sdk/compiler-rt/libclang_rt.builtins.a`; `fdiml` too (it
shares `s_fdim`). `scalbln`/`scalblnf`, which share their object with
`scalblnl` and so need `scalbnl`, link since SDK01 J. §6 lists the
references.

**Upstream accuracy of the complex inverse functions** (measured for SDK01
E, `measurements/measurement-2026-10-21-sdk01e/checks/olm-accuracy.txt`):
OpenLibm 0.8.8's `casin`, `cacos`, `casinh`, `cacosh` and their `f` forms are
the old Cephes algorithms, not FreeBSD's `catrig.c`. Against the macOS
system libm on 50 000 arguments up to |z| ≈ 10⁶: relative errors up to
2·10⁻⁵ for the double forms; the `f` forms return NaN/∞ for about a fifth
of those arguments (|z| beyond ~2¹²) where the result is finite; `cacosh`,
`cacoshf`, `casinhf`, `catanhf` differ in the sign of the imaginary part for
many arguments; `catanh` is within 8·ε·|w| except for 18 arguments. The SDK's
AArch64 build gives bit-for-bit the values of OpenLibm built for the host
from the same sources (`sdk/compiler-rt/tests/rt-host.sh`), so this is
upstream behaviour, not the port. A program that needs accurate complex
inverse trigonometry should not rely on these. Everything else is usable: `sqrt`, `sin`, `cos`,
`tan`, `atan2`, `exp`, `log`, `log10`, `pow`, `floor`, `ceil`, `fmod`,
`frexp`, `ldexp`, `modf`, `fabs`, `hypot`, `cbrt`, `sqrtf`, `isnan`/`isinf`,
… verified on NewTOS (`sdk/examples/mathtst.c` → `C:\AUTO\MATHTST.NCP`,
QEMU raspi4b serial: `mathtst: ALL PASS (23 checks)`,
`measurements/measurement-2026-09-21-t07/`).

Headers: a program includes `<math.h>` (and `<complex.h>`, `<fenv.h>`); the
adapters in `sdk/libm/include/` map those names onto OpenLibm's
`openlibm_math.h`/`openlibm_complex.h`/`openlibm_fenv.h` and turn mintlib's
feature macros (`__USE_MISC`/`__USE_XOPEN`, set by `_GNU_SOURCE`) into the
`__BSD_VISIBLE`/`__XSI_VISIBLE` gates OpenLibm uses for `M_PI` & Co. The app
scripts add `-I sdk/libm/include -I sdk/libm/openlibm-0.8.8/include` after
mintlib's include directory and `libm.a` before `libc.a` in the link group
(`build-qed.sh`, `build-teradesk.sh`; both still link with 0 undefined —
neither uses the libm). `libm.a` needs three libc symbols, `memset`, `_ctype`
(`nan()`) and `__assert_fail` (`j0`/`j1`), hence the group.

## 5. Runtime facts that differ from a MiNT/TOS libc

| Area | On NewTOS | Source |
|------|-----------|--------|
| `argc`/`argv` | mintlib's `parseargs()` rules (TASK/PORT01, TASK/PORT09). **With an `ARGV=` variable in the environment** (Atari's extended argument spec, `references/freemint/doc/programmer/atari/ARGV.txt`): `argv[0]` is the string after `ARGV=` (the launcher's path name), the arguments are the strings after it — unbounded, blanks inside an argument kept; the command tail is not read. As in mintlib the tail's length byte 127 is **not** required (`STRICTLY_COMPATIBLE_WITH_STANDARD` is off, `crtinit.c:61-70`). `ARGV=NULL:i,j,...` cuts the listed entries to `""` (`ARGV-blank.txt`). `program_invocation_name` / `_short_name` become `argv[0]` and its last `\`- or `/`-component. **Without `ARGV=`**: from the command tail `p_cmdlin` — `argv[0]` = `""`, leading blanks skipped, split on runs of blanks/tabs, MagX single-quoting; `1` / `{"", NULL}` when the tail is empty; `program_invocation_name` stays mintlib's `"unknown application"`. Who sends `ARGV=`: mintlib's own `spawn*`/`system()` always (`mintlib/spawn.c:353-420`), TeraDesk for a `PT_ARGV` program type when the command line is not empty (`applik.c:1470`, `startprg.c:543-567`). Host-checked by `tests/cmdargs-host.sh` (21 cases), proved on the machine by `C:\ARGVTEST.NAP` (14 arguments through `ARGV=`: argc 15; `measurements/measurement-2026-10-06-port09/`) | `newtos-arch/cmdargs.c`, `newtos-arch/heap_init.c`, `sdk/libc/crt0.S` |
| `environ`, `getenv` | built from the environment block `p_env` as mintlib does (TASK/PORT09): one entry per string up to the empty one, pointing **into** the block; an `ARGV=` string ends the environment and is cut **in place** (its `A` becomes NUL, so a later `Pexec(.., env = NULL)` child inherits the environment without the arguments); the desktop form `"PATH=\0C:\\\0"` — every `#Z`, desktop and accessory start gets a copy of the AES's (TASK/PPARENT, `docs/api/gemdos/process/_parent-and-environment.md`) — is joined **in place** to `"PATH=C:\\"` (`crtinit.c:332-345`). A program that reads its raw `p_env` block after start-up therefore sees mintlib's rewrite, not the bytes the kernel delivered — as on TOS with mintlib. `p_env == 0` or an empty block give the empty environment (one NULL); `environ` is never NULL (mintlib's `_spawnve` indexes it unconditionally, TASK/PORT10). An accessory gets `environ = {"", NULL}` like mintlib's `_acc_main` (no walk). The arrays (environ, the split tail, argv) lie in the TPA just above the BSS, as mintlib lays them out, and count to what the program keeps (`heap_init.c`, `crtinit.c:167-172`); an environment too large for the TPA stops the start with mintlib's "insufficient memory" | `newtos-arch/cmdargs.c`, `newtos-arch/heap_init.c`, `mintlib/globals.c`, `stdlib/getenv.c`; `kernel/src/launch.rs`, `loader.rs` |
| `_base` (`BASEPAGE *`) | set by `__libc_init_heap` from the basepage address crt0 hands it (TASK/T05; until then it stayed NULL and dereferencing it worked only because the basepage was at VA 0). Since ARCH03 stage 1 the basepage is the start of the TPA block, so `_base` is that address — never 0 | `newtos-arch/heap_init.c`, `sdk/libc/crt0.S`; readers: `unix/getppid.c`, `gmon.c`, TeraDesk's `environm.c` |
| `malloc`/`free`/`realloc` | mintlib's allocator; since ARCH03 E9 the start shrinks the TPA like `_crtinit` and, with the default `_stksize`, malloc takes its memory from GEMDOS `Malloc` (`_split_mem`). Since TASK/MEM05 part B the hunks it asks for keep doubling from 8 KiB up to **16 MiB** instead of stopping at mintlib's 32 KiB — §10 | `newtos-arch/heap_init.c`, `crtinit.c:212-247`, `unix/sbrk.c:54-71`, `overlay/mintlib/malloc.c` |
| start-up (`_main`) | since 2026-10-13 (TASK/LIBC-MAIN) the libc `crt0` ends like mintlib's own start-up, for a program (`_crtinit`, `crtinit.c:257`) and an accessory (`_acc_main`, `crtinit.c:147`) alike: `_main(__libc_argc, __libc_argv, environ)` = `__libc_main` (`mintlib/main.c:77-305`, weak `_main`, so a program can still supply its own). It clears `errno`; finds no `MiNT` cookie (`__mint = 0`); a program calls `Pdomain(1)` and compares `Pgeteuid`/`Pgetuid`, `Pgetegid`/`Pgetgid` — all `EINVFN` on NewTOS as on EmuTOS, so not "secure" and `UNIXMODE` is honoured; clears `__open_stat[]`; with an empty `argv[0]` (no `ARGV=`) and handle 2 a terminal it does `Fforce(2, -1)` (stderr to the console); stdin/stdout/stderr get full/line/no buffering (stdout full when `isatty(1)` is 0, i.e. redirected to a file) and, where `isatty` (mintlib's own since TASK/LIBC-TTY, §2) says terminal, **text mode**; a standard stream redirected to a file gets the `UNIXMODE` default (`main.c:68-71`: `fstat` says `S_IFREG` since TASK/LIBC-FSTAT) — **text** unless `UNIXMODE` contains `b`, i.e. `\n` is written as `\r\n` and `\r\n` read as `\n`, as on TOS; until TASK/LIBC-FSTAT `fstat` reported fds 0-2 as `S_IFCHR` and a redirected stream stayed binary — with a seek function, whose first flush asks `Fseek(0, h, SEEK_CUR)` — 0 on a character device since TASK/FS10, as on EmuTOS (until then every `stdout` write failed with `EBADF` here, which is why crt0 called only `exit(main())` from post-E9 to FS10); and it rewrites `PATH` (or every variable listed in `PCONVERT`) in `environ` to the Unix form (`PATH=C:\` → `PATH=/dev/c/`, `;`/`,` → `:`, lower case, `unx2dos.c:217`) — `spawnvp`/`execvp`/`system()` convert back; the raw `p_env` block is not touched | `mintlib/main.c`, `mintlib/crtinit.c`, `sdk/libc/crt0.S` |
| console `read()`, `getchar`, `fgets` | mintlib's `__read` (`unix/read.c`). With `__open_stat[].flags` as `isatty` leaves them (`CRMOD\|ECHO`) a tty read is `Fread(fd, min(size, 4096))` — on CON the GEMDOS line read with echo and editing (TASK/FS11, `docs/api/gemdos/file/3f-fread.md`); `^D` first = EOF; a line ended by Return gets `\r` appended, `CRMOD` turns it into `\n` and echoes `Cconout('\n')` (`read.c:147-169`). QEMU-verified: `getchar()` = first key, `fgets` = the rest of the line + `\n` (`measurements/measurement-2026-10-13-libc-tty/`). Until TASK/LIBC-TTY the shim left `flags` 0, so every read took the RAW path below. **RAW path** (`flags` with `RAW`/`CBREAK`, or `ECHO` clear — reachable only by a program that sets the flags itself; `ioctl(TIOCSETP)` cannot, TASK/LIBC-IOCTL): `_console_read_byte` → `Fgetchar` (`EINVFN`) → `Crawcin`, no echo, no `CRMOD`, and it **reads two keys per byte, the first one discarded** (`read.c:176` result overwritten at `:183`). That is upstream mintlib-0.60.1 code, byte-identical to the tarball, and stays as it is (user decision 2026-10-13, TASK/LIBC-TTY decision 3) | `mintlib/unix/read.c`, `mintlib/console.c`; `docs/analysis/libc-tty-2026-10-13.md` §3 |
| `exit`/`_exit` | `exit()` is mintlib's: atexit handlers in reverse order, then every stdio stream closed (flushed), then `_exit` = `Pterm(rc)`. A `return` from `main` goes through it: `__libc_main` ends with `exit(main(argc, argv, envp))` (`mintlib/main.c:303`) — since post-E9 (2026-10-05) crt0 did that itself, since TASK/LIBC-MAIN (2026-10-13) through `_main`; before post-E9, crt0 called `Pterm` directly and buffered output and atexit handlers were lost | `stdlib/exit.c:43-59`, `newtos-arch/proc.S`, `sdk/libc/crt0.S` |
| `clock()` | reads `_hz_200` at `$4BA` through `get_sysvar` — a live 200 Hz tick (verified by `sdk/libc/tests/clocktest.c`) | `newtos-arch/getsysvar.c`, `time/clock.c` |
| `time()`/`gettimeofday()` | GEMDOS clock via mintlib's own path (`Tgettimeofday` = `EINVFN`, then `Tgetdate`/`Tgettime` through `mktime`); QEMU-checked by TASK/SDK01 F: `time()` = the GEMDOS clock read as UTC (± 2 s) when `TZ` is not set (`measurements/measurement-2026-10-21-sdk01f/`) | `unix/gettimeofday.c` |
| `stat`/`fstat` times | since TASK/SDK01 F as mintlib on TOS: the FAT entry's DOS stamp (DTA of `Fsfirst` for `stat`/`lstat`, `Fdatime` for `fstat`), `st_mtime = st_atime = st_ctime` (FAT keeps one stamp), 2-second resolution; drive roots 1979-11-30 00:00 (mintlib's `OLDDATE`), devices `time() - 2`. The DOS stamp is **local time** converted by `mktime` with the program's `TZ`: with `TZ` unset (the NewTOS default — no `/etc/localtime`, mintlib falls back to UTC, `tz/localtime.c` `zoneinit("")`) the seconds are the clock face read as UTC, the same rule `time()` uses, so `localtime(&st.st_mtime)` shows the date and time GEMDOS shows. mintlib's `time_t` is 32 bits (`include/bits/types.h:73`): stamps after 2038-01-19 03:14:07 wrap negative, as for `time()`. QED's *Datei → Info* shows them (`measurements/measurement-2026-10-21-sdk01f/`; before: 01.01.1970 00:00:00) | §2 |
| `stat`/`fstat` uid/gid | unchanged by SDK01 F: `stat` from `geteuid`/`getegid`, `fstat` zero; permission bits from the attributes only | §2 |
| `long double` I/O | binary128 since TASK/SDK01 J: `printf`/`scanf` `%Le %Lf %Lg %La` (exact digits, ties to even), `strtold`, `qecvt`/`qfcvt`/`qgcvt` — mintlib's printf/strtod engine with overlays for the binary128 format (mintlib knows only the m68k 96-bit one). Upstream limits kept, for `double` too: `strtod`/`strtold` convert only `(MANT_DIG − bits + 2)/3 + 1` fraction digits exactly and keep the rest as a sticky bit, so an input at or one last-digit unit from a halfway point can round to the wrong neighbour (glibc before 2.17, BZ #3479; measured: 276 of 904 such inputs); `"infinity"` is consumed as `"inf"` (`strtod.c:562` tests the 3-letter form first). Until SDK01 J: `%Lf` read a `double` (`-D__NO_LONG_DOUBLE_MATH`) | `build-libc.sh` header, `overlay/{include/ieee754.h,gmp/,stdlib/strtold.c,stdio/printf_fp*.c}` |
| `printf("%f", INFINITY)` | `inf`/`nan` since TASK/SDK01 J; before, mintlib's `<bits/endian.h>` claimed a big-endian float word order (clang does not predefine `__FLOAT_WORD_ORDER__`), `__isinf`/`__isnan` read the wrong half and the value printed as a 309-digit number | `overlay/include/bits/endian.h` |
| `Getcookie` | over `Ssystem(S_GETCOOKIE)`; `C_FOUND`/`C_NOTFOUND` contract kept | `newtos-arch/getcookie.c` |
| `Super`/`Supexec` | bindings without privilege — see [05](05-abi-pitfalls.md#b4) | `overlay/include/mint/osbind.h` |
| `signal`/`raise`/`kill` | compile and link; whether NewTOS delivers signals to a process is **unverified** here (the kernel has an EL0 upcall mechanism, `loader.rs:735-765`) | — |
| `fork`/`vfork`/`exec*` | `fork` is a weak symbol that compiles; `vfork` works (TOS branch: `tfork` = `Pexec` 5 + 4, QEMU-verified, §8); `execve` is `Pexec(200)` on TOS, which NewTOS answers `EINVFN` like EmuTOS — it fails with `ENOSYS` | `newtos-arch/vfork.S`, `mintlib/thread.c` |

## 6. References `libc.a` leaves unresolved (measured)

Re-measured 2026-10-20 after TASK/SDK01 A (`llvm-nm` over the archive,
referenced by a member, defined by none): **10**, from 23 before SDK01 A
(`measurements/measurement-2026-10-20-sdk01a/`). A link only fails if the
member that references them is pulled in — `build/why.txt` (written by every
link, `--why-extract`) shows the chain.

```
__divtf3 __eqtf2 __getf2 __lttf2 __multf3 __floatsitf       binary128 builtins (qecvt/qfcvt/qgcvt(_r), printf_size): resolved by
                                                            sdk/compiler-rt/libclang_rt.builtins.a since SDK01 E
                                                            (SDK01 J: __trunctfdf2 gone, __floatsitf new; plain printf
                                                            references none of them)
socket gethostbyname                                        socket/ not compiled: if_link, if_*flags, gethostid
main                                                        the program's own (mintlib/main.c calls it)
_end                                                        provided by sdk/newtos.ld at link
(gone with SDK01 A: getpwuid getgrent setgrent endgrent __getpwnam_r __getpwuid_r -- pwdgrp/ compiled;
 __sys_fstat -- newtos-arch/do_fstat.c; nf_init -- newtos-arch/nf_ops.c; getcontext makecontext swapcontext,
 pututline, yyin yyparse -- referenced only by the test programs and the syscall generator, no longer in libc.a)
```

QED (37 files, cflib+gemlib+libc) and TeraDesk (48 files, gemlib+libc) link
with **0** undefined symbols against this archive (`build-qed.sh:167`,
`build-teradesk.sh:187`; `TASK/T01-STATUS-DONE.MD`), so none of the above
is on their paths.

**`libm.a` (T07, measured 2026-09-21, `sdk/libm/build/dangling.txt` — the
build script writes it on every run):** 16 symbols referenced, defined by no
member.

```
__assert_fail _ctype memset                                   libc.a (e_j0/e_j1, s_nan) -- resolve in the link group
__addtf3 __subtf3 __multf3 __divtf3 __gttf2 __netf2
__extendsftf2 __extenddftf2 __trunctfsf2 __trunctfdf2         binary128 compiler-rt: e_powf (nan_mix in long double),
                                                              e_remainder/e_remainderf (a long double NaN path),
                                                              s_fdim (fdiml shares the object), s_nexttowardf
__muldc3 __mulsc3                                             compiler-rt complex multiply: s_casin, s_casinh,
                                                              s_cacosh, s_catanh (+ the f forms)
scalbnl                                                       s_scalbln (scalblnl shares the object with scalbln/scalblnf)
```

Consequence until SDK01 E: `powf`, `remainder`, `remainderf`, `fdim`,
`fdimf`, `scalbln`, `scalblnf`, `nexttowardf`, `casin`, `casinh`, `cacosh`,
`catanh` (+`f`) failed to link — the separate item TASK/T07 named
("Soft-Float-Helfer sind ein eigener Punkt"). Since SDK01 E the 12 builtins
above resolve from `sdk/compiler-rt/libclang_rt.builtins.a` (LLVM
compiler-rt 21.1.8, `newtos-cc` links it after the libraries); `libm.a`
itself is unchanged and still lists them in `dangling.txt` (it does not
contain them). Left until SDK01 J: `scalbnl` (an `l` function, §4) —
`scalbln`, `scalblnf`, `scalblnl`. Since SDK01 J `scalbnl` is a member and
`dangling.txt` lists 25 symbols: the three libc ones and 22 compiler-rt
builtins (the 12 above plus `__eqtf2 __getf2 __letf2 __lttf2 __unordtf2
__fixtfdi __fixtfsi __floatditf __floatsitf __multc3` of the `…l` members).
The `mathtst.NCP` link pulls 28 libm members and none of these.

## 7. What the test programs prove

| Program | Proves | Source |
|---------|--------|--------|
| `sdk/libc/tests/traplink.c` | the retargeted `osbind.h` macros compile and link without `features.h`/`ostruct.h`; `Cconws`, `Fcreate`/`Fwrite`/`Fclose`, `Fopen`/`Fread` round-trip | `:1-17` |
| `sdk/libc/tests/libtest.c` | `printf("%d %ld")`, `malloc`+`free`, `fopen("wb")`/`fwrite`/`fclose` then `fopen("rb")`/`fread` round-trip on `C:` — verified on QEMU (ADR-019, `docs/MEMORY.md:1160-1166`: `-42 305419896`, `heap: heap string`, `roundtrip OK (37 bytes)`) | `:1-14` |
| `sdk/libc/tests/clocktest.c` | mintlib's unmodified `clock()` counts 200 Hz against `Tgettime` edges | `:1-28` |
| `docs/sdk/examples/hello-libc.c` | `printf`, `malloc`/`free`, `Sversion`, `Tgettime` — run 2026-09-17, see [03](03-first-programs.md) | — |
| `sdk/libc/tests/setjmp-host.sh` (+ `setjmp-host.c`, `setjmp-cases.h`) | `newtos-arch/setjmp.S` on the macOS/arm64 host: return values (0 / val / 1 for 0), the sigmask slot exactly as the m68k code, x19–x28 and d8–d15 restored after a longjmp out of a callee that clobbered them, nested buffers, the frame after unwinding four frames — 13 checks, exit status = failures | `setjmp-host.sh:1-29`, `setjmp-cases.h:1-19` |
| `sdk/examples/setjmptst.c` | the same 13 checks on NewTOS through mintlib's `<setjmp.h>` (`_Static_assert(sizeof(jmp_buf) == 176)`), verdicts over `Cconws` — QEMU raspi4b `ALL PASS (13 checks)` | `:1-17` |
| `sdk/examples/mathtst.c` | 23 libm checks against closed-form values with tolerances (`sqrt(2)`, `sin(M_PI/6)`, `pow(2,10)`, `floor(-1.5)`, `frexp(8.0)`, `atan2(1,1)`, `exp`/`log`, `fmod`, `modf`, `hypot`, `cbrt`, `sqrtf`, NaN/-inf) via `<math.h>` — QEMU raspi4b `ALL PASS (23 checks)` | `:1-15` |
| `sdk/libc/tests/waith-host.sh` (+ `waith-probe.c`, `waith-host.c`, `waith-host-mintbind.h`) | the `<sys/wait.h>` macros and `union wait` on known status words and through mintlib's unmodified `wait4.c`/`waitpid.c`/`wait.c` — 74 checks, exit status = failures (§9) | `waith-host.sh:1-19`, `waith-probe.c:1-20` |

## 8. `system()` and `vfork()` (TASK/PORT10, TASK/PROC01)

`system()` is **mintlib's own `posix/system.c`** again (TASK/PROC01,
2026-10-05; from 2026-10-02 to then an overlay without its POSIX branch stood
in). mintlib picks at run time:

* the **POSIX branch** when `/bin/sh` is executable or `$SHELL` is set:
  `vfork()`, the child `execve(shell, {name, "-c", line})`, the parent
  `waitpid()`. On NewTOS, as on EmuTOS, `execve` is `Pexec(200)` (mintlib's
  `_spawnve(P_OVERLAY)`), which the kernel answers with `EINVFN`, so the child
  `_exit(127)`s and `system()` returns the `waitpid` status `127 << 8`,
  i.e. `WEXITSTATUS == 127` — QEMU-verified as the status word
  (`measurements/measurement-2026-10-05-arch03-2/`, VFORKT) and, since the
  §9 header fix, through `WIFEXITED`/`WEXITSTATUS`
  (`measurements/measurement-2026-10-06-waith/`);
* the **TOS branch** (`system_hack`) otherwise — the case of every NewTOS
  program that does not set `$SHELL` (`environ` starts empty): split the line
  into words (`'`/`"`/`` ` `` group), `<`, `>`, `>>` redirect handles 0/1 via
  `Fopen`/`Fcreate` + `Fdup`/`Fforce`, then `spawnvp(P_WAIT, argv[0], argv)` =
  a synchronous `Pexec(0)`; the result is the child's `Pterm` code, `-1` with
  `errno` `ENOENT` when the program is not found, `2` when a redirection file
  cannot be opened. `system(NULL)` is 1. Note: this is the **raw** exit code
  (mintlib `system_hack` returns what `spawnvp(P_WAIT)` returns), not a wait
  status — `WEXITSTATUS()` does not apply to it; `system("X 42")` is `42`, not
  `42 << 8`. That is mintlib's TOS behaviour, unchanged.

`vfork()` is `sdk/libc/newtos-arch/vfork.S`, the AArch64 port of mintlib's
`unix/vfork.S`: with `__mint == 0` (always on NewTOS) the TOS branch,
`tfork(newprog, save)` from `mintlib/thread.c` = `Pexec(PE_CBASEPAGE)`, then
`Pexec(104)` (answered `EINVFN` = `-ENOSYS`), then `Pexec(PE_GO)` on code of
the program itself. The child returns 0 from `vfork` into the caller's frame
and runs **before** the parent continues (TOS `Pexec` is synchronous); the
parent gets the child's pid when the child has ended, and `waitpid`/`wait4`
deliver its exit status from `__waitval`. Differences of the port: the child
resumes on the caller's stack pointer (AArch64 code reaches its locals through
`sp`, m68k through `a6`), and the parent's own `tfork` call runs on a separate
16-KiB stack so the child may use the stack below; the save area and that
stack are static, so a `vfork` from inside a `vfork` child before it ends is
not supported (the m68k original has the same static save area). **One fix
to mintlib's `tfork`** (2026-10-06, `docs/analysis/vfork-malloc-ownership-2026-10-06.md`):
in mintlib 0.60.1 the thread child `Mfree`s its basepage's environment and
adopts the parent's (`b->p_env = parent->p_env`), and after the child the
parent `Mfree`d `b->p_env` — its **own** environment, and on later forks
whatever block had since been allocated at that address (a `malloc` hunk:
VFORKT crashed in `exit()` under memprot=on). The overlay's `tfork` saves the
environment pointer of the `Pexec(PE_CBASEPAGE)` basepage before starting the
child and frees that saved pointer (both the success and the `pid < 0`
branch). Evidence `measurements/measurement-2026-10-06-vforkt-fix/`. Host check
`sdk/libc/tests/system-host.sh` (TOS branch, and that the object carries the
POSIX branch); target checks `sdk/examples/port10.c` (TOS branch) and the
ARCH03 stage-2 probe VFORKT (vfork, waitpid, POSIX branch).

What a caller must know:

* **The search list is NewTOS's, not mintlib's** (user decision
  2026-10-02). A name without extension is tried as given, then with `.ntp`,
  `.nap`, `.ncp` — in that order, replacing mintlib's `.ttp`, `.prg`, `.tos`
  position by position. On NewTOS only the native AArch64 programs are
  executable; `.TTP`/`.PRG`/`.TOS` are m68k and belong to the later m68k
  emulator, which will append them **after** the native three. `.NAC`
  (accessory) is not startable and is not in the list. Overlays:
  `sdk/libc/overlay/mintlib/spawnvp.c` (what `spawnvp`/`spawnlp`/`system()`
  use), `mintlib/spawn.c` (the `#!` interpreter lookup) and `posix/execvp.c`
  (`{ "", .ntp, .nap, .ncp }` — its `.app`/`.gtp` are dropped too, both m68k).
  `system("C:\\P10\\P10KID s3")` now runs `P10KID` (exit code 42, measured,
  `measurements/measurement-2026-10-02-port10c/`); a name whose only file is
  `LEGACY.PRG` is `-1`/`ENOENT`. An explicit extension is used as given —
  also an m68k one (the list does not gate what the caller names; whether
  `Pexec` loads it is the kernel's decision, not measured here). `_buffindfile` appends
  the lower-case string (`P10KID.ncp`); GEMDOS upper-cases names on FAT, so it
  matches `P10KID.NCP`. Host check `sdk/libc/tests/spawn-host.sh`
  (`spawn-search-host`). The PATH part of the search reads `getenv("PATH")`:
  `environ` is built from `p_env` since TASK/PORT09, and since TASK/LIBC-MAIN
  `_main` keeps `PATH` there in mintlib's Unix form (`PATH=C:\` →
  `/dev/c/`, §5 "start-up"); `_buffindfile` splits it on `:`/`,`
  (`buffindf.c:27-28`). The host check fakes `getenv` with the DOS form; a
  program found only through a PATH directory in the Unix form was not
  measured on QEMU (PORT10 names its programs with a path).
* **The execute bits are NewTOS's too** (user decision 2026-10-02,
  follow-up). `stat()`/`lstat()` (`__do_stat` = `__quickstat`), `access()`
  (`newtos-arch/access.c`), `_buffindfile`, `open()` and `glob()` take
  `st_mode` from `__quickstat`; `Fstat64` is `EINVFN` (= mintlib's
  `-ENOSYS`) on NewTOS, so its `Fsfirst` path decides, and it sets `0111` on
  a regular file by the extension **as the caller spelled it**. Pristine
  mintlib: `.app/.gtp/.ttp/.prg/.tos`, `strcmp`, lower case only — a FAT
  name `X.NCP` was not executable, `x.prg` was. The overlay
  `sdk/libc/overlay/mintlib/quickstat.c` replaces the list with `.ntp`,
  `.nap`, `.ncp`, compared with `strcasecmp`; the m68k extensions come back
  with the emulator, after the native ones; `.nac` is not executable.
  Directories keep `0755` whatever their name; `FA_RDONLY`/`FA_HIDDEN`
  still clear the write/read bits. Measured in QEMU raspi4b:
  `access("C:\\P10\\P10KID.NCP", X_OK)` and `access("c:\\p10\\p10kid.ncp",
  X_OK)` are 0, `.NTP`/`.NAP` too; `LEGACY.PRG`/`legacy.prg`, `ACC.NAC`,
  `PLAIN.TXT` are `-1`/`EACCES`; `stat().st_mode` `0100755` vs `0100644`
  (`measurements/measurement-2026-10-02-port10d/`). Host check
  `sdk/libc/tests/quickstat-host.sh` (27 checks; with the pristine file 15
  fail). The other copies of the list are not built: `mintlib/do_stat.c`
  (excluded, replaced by `newtos-arch/do_stat.c`) carries the same old list,
  and the excluded `mintlib/do_fstat.c` set `0111` for an open file whose
  first word is the m68k `0x601a`/`#!` magic — the lean
  `newtos-arch/do_fstat.c` sets no execute bit for `fstat()` at all.
* **Redirection reaches the child (kernel, since TASK/PROC02, `c2cd746b`).**
  `init_pd_files` (`tos-api/src/bdos/process/loader.rs`, EmuTOS
  `bdos/proc.c:387-399`) gives the child every standard handle of the parent
  that is `> 0` via `ixforce`, and `Fforce` counts the use (EmuTOS
  `bdos/fshand.c` `ixforce`, `f_use`), so `system("prog >file")` fills `file`
  and the caller's handle 1 survives `system_hack`'s `Fforce(1, dup)` +
  `Fclose(dup)` (`port10.c` 44/44, `measurements/measurement-2026-10-06-proc02/`). Until PROC02
  the loader set the child's `p_uft` to `-1` and the first `Fclose` freed the
  file, both measured in `port10.c`. The child's stdio sees the inherited file
  as a file (`S_IFREG`, text mode by default, TASK/LIBC-FSTAT;
  `measurements/measurement-2026-10-17-libc-fstat/`).
* `/dev/null` becomes `null:` (`mintlib/unx2dos.c`, no MiNT); `Fcreate` of it
  fails on NewTOS, so `system("... >/dev/null")` returns 2 with a `perror`
  line and runs nothing. That is what DOOM's `I_Error` → `ZenityErrorBox`
  probe sees: DOOM reports the error and exits normally.

`misc/syslog.c` referenced the missing `vfork` until TASK/PROC01; every
symbol its object references is defined in `libc.a` now (`llvm-nm`,
2026-10-05) — syslog itself is not exercised.

`printf("%f")` (`stdio/printf_fp.c`) printed wrong digits and could fault
until TASK/PORT10: `stdlib/gmp-mparam.h` declared 32-bit GMP limbs while
`mp_limb_t` (`unsigned long`) is 64 bits on LP64. The overlay
`sdk/libc/overlay/stdlib/gmp-mparam.h` derives the width from `long`; host
check `sdk/libc/tests/gmp-host.sh`.

## 9. `<sys/wait.h>` status macros (TASK/PORT-WAITH)

The status word `wait`/`wait3`/`wait4`/`waitpid` store is an `int` built by
mintlib's `unix/wait4.c` with plain integer arithmetic: from the kernel's
`(pid << 16) | code` (on TOS from `__waitval`, which `tfork` sets to
`(pid << 16) | Pexec's return`, `mintlib/thread.c:146`) it swaps the low two
bytes, so the **exit code / stop signal is in bits 8..15, the core flag in
bit 7, the terminating signal in bits 0..6** — the same layout
`W_EXITCODE(ret, sig) = ret << 8 | sig` and `W_STOPCODE(sig) = sig << 8 | 0x7f`
in the header describe. That library code is layout-correct on AArch64
(the rebuilt `libc.a` is byte-identical with and without the fix).

The macros `WIFEXITED`, `WEXITSTATUS`, `WIFSIGNALED`, `WTERMSIG`,
`WIFSTOPPED`, `WSTOPSIG`, `WIFCOREDUMPED` and the BSD `union wait` members
(`w_retcode`, `w_termsig`, `w_coredump`, `w_stopsig`) read the word through
`struct __wait` bit-fields declared `junk:16, retcode:8, coredump:1,
termsig:7`. On big-endian m68k the first field takes the most significant
bits, which gives exactly the layout above; on little-endian AArch64
(AAPCS64) the first field takes the least significant bits, so `retcode` sat
in bits 16..23 and `termsig` in bits 25..31: for every status word wait4
produces (16 bits) `WEXITSTATUS` was 0, `WIFEXITED` true and
`WIFSIGNALED`/`WIFSTOPPED` false (found by VFORKT,
ARCH03 stage 2e). The overlay `sdk/libc/overlay/include/sys/wait.h` declares
the fields in reverse order when `__BYTE_ORDER__ == __ORDER_LITTLE_ENDIAN__`
— same bit positions, same `int` layout, macros unchanged; the big-endian
branch is mintlib's text. Only the header changes; programs that use the
macros must be recompiled (no SDK app or overlay does — QED, TeraDesk, Doom,
cflib, gemlib checked; mintlib's own users are `sunrpc/`, `unix/test-vfork.c`
and `test-skeleton.c`, which `build-libc.sh` does not build).

Checks: host `sdk/libc/tests/waith-host.sh` (in `sdk/run-host-tests.sh`) —
74 checks: every macro on known words (exit 0/1/42/127/255, SIGINT, SIGTERM,
SIGSEGV + core, stopped SIGTSTP/SIGSTOP), `union wait` in both directions,
and mintlib's unmodified `wait4.c`/`waitpid.c`/`wait.c` with the `__waitval`
`tfork` leaves; with the pristine header 28 of the 74 fail. Target: VFORKT
(`waitpid` after `_exit(42)` → `WEXITSTATUS == 42`, `_exit(7)` → 7, POSIX
`system()` → `WIFEXITED && WEXITSTATUS == 127`), QEMU raspi4b,
`measurements/measurement-2026-10-06-waith/`.

## 10. `malloc` hunk sizes: doubling up to 16 MiB (TASK/MEM05 part B)

mintlib's `malloc` asks the OS for memory in **hunks**: one `__sbrk` = one
GEMDOS `Malloc` = one memory descriptor (`unix/sbrk.c:54-73`). A hunk is
`MINHUNK` bytes, 8 KiB at the start; after each hunk `MINHUNK` doubles until
it reaches `MAXHUNK`; a request larger than `MINHUNK` gets a hunk of its own
size and does not advance the doubling (`mintlib/malloc.c:64-92`). mintlib's
`MAXHUNK` is 32 KiB and `static` (`malloc.c:26-27`); its only setter,
`_mallocChunkSize(siz)`, sets **both** values (`:30-32`), so a program could
only choose "every hunk `siz`".

With 32 KiB hunks a program that keeps allocating needs one GEMDOS block per
32 KiB: `malloc(32)` × 147 605 took 254 blocks — the whole system MD pool
before MEM05 part A (`measurements/measurement-2026-10-06-post-e9/`), and 5138
blocks for 3 000 000 after it, at ~2000 s, because mintlib's free list walks
past one border chunk per hunk on every `malloc`
(`measurements/measurement-2026-10-13-mem05-kernel/`).

**Overlay** `sdk/libc/overlay/mintlib/malloc.c` = the pristine file plus one
function (user decision E6, design `docs/analysis/mem05-design-2026-10-13.md`
§4):

```c
void __newtos_malloc_maxhunk(size_t siz) { MAXHUNK = siz; }
```

It raises only the ceiling; `MINHUNK` and the doubling are mintlib's. The C
start-up calls it once, with **16 MiB**, at the very start of `__libc_crtinit`
(`sdk/libc/newtos-arch/heap_init.c`), before anything can call `malloc` and
before the accessory branch — so programs and accessories (`acc_main`) both
get it. The reference is **weak**: a program or module that does not link
mintlib's `malloc.o` — an LDG module runs on libshare's allocator
(`sdk/libshare/libshare-link.sh`) — keeps the symbol undefined, the static link
resolves it to 0 (no dynamic relocation), and the call is skipped. The
undefined-symbol gates of the module builds (`build-zview.sh`,
`build-ldg-demo.sh`, `build-ldg-client.sh`) therefore count every undefined
symbol except this one weak reference (`ls_undefined_count`).

| `malloc(32)` × N | hunks before (32 KiB) | hunks now (16 MiB) |
|---|---|---|
| 100 | 1 | 1 |
| 1 000 | 3 | 3 |
| 10 000 | 19 | 7 |
| 147 605 | 254 | 10 (8 KiB … 4 MiB) |
| 1 000 000 | 1 714 | 14 |
| 3 000 000 | 5 138 | 21 |
| 10 000 000 | — | 44 (12 doubling hunks to 16 MiB, then 32 × 16 MiB; 570 MB) |

Measured on QEMU raspi4b in every protection mode
(`measurements/measurement-2026-10-13-mem05-sdk/`, MDSTEP: GEMDOS blocks and
mintlib's own hunk list agree with the design's model at every step); the
"before" column is the post-E9 / part-A measurement and the host harness with
the setter not called. `malloc(32)` × 3 000 000 now takes 1–5 s of QEMU
time (by protection mode) instead of ~2000 s; × 10 000 000 about 37–46 s.

**Small programs stay small**: a hello-world (`printf` + a few small
`malloc`s) takes one 8 KiB hunk, as before (HELLOMEM vs HELLOOLD in the same
run). An accessory gets the same schedule (MDACC: 147 605 × `malloc(32)` = 10
hunks, 8 KiB … 4 MiB).

Costs to know:

| What | Effect |
|------|--------|
| a hunk can fail where a smaller one would not | from the 12th hunk on mintlib asks for 16 MiB at once and does not retry smaller (`malloc.c:90-92`, `sbrk.c:62-72`): with less than 16 MiB free the `malloc` returns NULL. The schedule only gets there after ~32 MiB of heap |
| over-allocation | at most the last hunk unused: up to 16 MiB more than the heap needs, against 32 KiB before |
| `_mallocChunkSize(siz)` | unchanged: a program that calls it gets `siz` for every hunk, as in mintlib |
| `free` | unchanged mintlib: a hunk goes back with `Mfree` only once it is entirely free and merged (`malloc.c:196-243`) |

Checks: host `sdk/libc/tests/malloc-host.sh` (in `sdk/run-host-tests.sh`) —
the overlay `malloc.c` against a recording `__sbrk`: without the call the
counts equal the post-E9 QEMU measurement (1/3/19/87/173/254/255); with it
1/3/7/10/14 and every hunk `8 KiB << i` up to 16 MiB; the accessory settings
give the same; a hello-world takes one 8 KiB hunk and a 32 KiB buffer gets
its own hunk without advancing the doubling; `_mallocChunkSize(64 KiB)` still
wins. With the setter's body emptied 10 of 30 checks fail.

## 11. Functions that do not link (measured per symbol, TASK/SDK01 A)

Method (`docs/analysis/sdk-inventory-2026-10-20.md` §0 step 2): for every
global symbol `libc.a` defines, one program that references exactly that
symbol, linked as an application links (`sdk/libc/crt0.S` + `sdk/newtos.ld` +
`libc.a`). Measured 2026-10-20 (`measurements/measurement-2026-10-20-sdk01a/`):

| | before SDK01 A | after |
|---|---|---|
| global symbols defined (incl. weak aliases) | 1204 | 1229 |
| symbols that do not link | 64 (incl. 9 syscall-generator symbols, the 2 `ucontext.c` test arrays and `main`) | **12** |

Linking since SDK01 A: `tmpfile`, `mkstemp`, `mktemp`, `mkdtemp`, `tmpnam`,
`tmpnam_r`, `tempnam`, `glob`, `globfree`, `glob_pattern_p`, `wordexp`,
`wordfree`, `getlogin`, `cuserid`, `getgroups`, `group_member`, `euidaccess`,
`eaccess`, `sem_open`, `sem_unlink`, the passwd/group functions (`getpwuid`,
`getpwnam`, `getgrent`, … §4), `ttyname`, `ttyname_r`, `ctermid`, `gets`,
`getwd`, `siggetmask`, `nf_init`, `nf_get_id`, `nf_version`, `nf_get_name`,
`nf_get_fullname`, `nf_debug`, `nf_debugprintf`, `nf_debugvprintf`,
`nf_shutdown`, `nf_exec`, `nf_execv` (§2). `tmpfile`, `mkstemp`, `tmpnam`,
`tempnam`, `glob`, `ttyname`, `ctermid`, `getwd`, `getpwuid`, `getlogin`,
`stat` and the `nf_*` absent-path ran on NewTOS in QEMU raspi4b, memprot
on/strict/off: the temp files were really created and removed (a `tmpfile`
in a child is gone after its `exit`, mintlib's TOS path), `fsck` clean.

**TASK/SDK01 E (2026-10-21)** added the compiler runtime
(`sdk/compiler-rt/libclang_rt.builtins.a`, LLVM compiler-rt 21.1.8 builtins,
linked last by `newtos-cc`). Same method, the probe linking `libc.a` +
the builtins archive, and the same for every global symbol of `libm.a`
(`libm.a` + `libc.a` + builtins), `measurements/measurement-2026-10-21-sdk01e/`:

| | before SDK01 E | after |
|---|---|---|
| `libc.a`: symbols that do not link (of 1229) | 12 | **7** |
| `libm.a`: symbols that do not link (of 214) | 21 | **3** |

Linking since SDK01 E: `qecvt`, `qecvt_r`, `qfcvt`, `qfcvt_r`, `qgcvt`;
`powf`, `remainder`, `remainderf`, `fdim`, `fdimf`, `fdiml`, `nexttowardf`,
`cpowf`, `casin`, `casinf`, `cacos`, `cacosf`, `casinh`, `casinhf`,
`cacosh`, `cacoshf`, `catanh`, `catanhf`. The libm ones ran on NewTOS (QEMU
raspi4b, memprot on/strict/off) with the bit patterns of the reference;
the builtins themselves were checked on the host against exact oracles
(`sdk/compiler-rt/tests/rt-host.sh`). Accuracy of OpenLibm's complex inverse
functions is upstream's (§4).

**`qecvt` & Co. linked with SDK01 E but were wrong**: mintlib formats them
through `snprintf("%.*Lf", …, long double)` (`misc/efgcvt_r.c` included by
`misc/qefgcvt_r.c`), and `libc.a` was built with `-D__NO_LONG_DOUBLE_MATH`,
so `vfprintf` took `%Lf` with `va_arg(ap, double)`: `qecvt(1.0L/3, 10)` =
`"-0.000000000"`, decpt −1, `qfcvt(2.5L, 3)` = `""`, `qgcvt(0.125L, 6)` =
`"0"` (`measurements/measurement-2026-10-21-sdk01e/serial/rt-on.log`).
**Since TASK/SDK01 J** they are right — `"3333333333"`/0, `"2500"`/1,
`"0.125"` on NewTOS (`measurements/measurement-2026-10-21-sdk01j/`), and on
the host 410 random arguments each against exact arithmetic
(`sdk/libc/tests/ldbl-host.sh`). Per-symbol link probe after SDK01 J:
`libc.a` 7 of 1231 do not link (the two new symbols are
`__mpn_extract_long_double`/`__mpn_construct_long_double`), `libm.a` **0 of
322** (`scalbln`, `scalblnf`, `scalblnl` and every `…l` function link).

Still not linking (7 in `libc.a`, 0 in `libm.a` since SDK01 J):

| Function | Missing | Reason |
|---|---|---|
| `gethostid`, `__gethostid` | `gethostbyname` | `socket/` not compiled (§4) |
| `if_link`, `if_getifflags`, `if_setifflags`, `if_getlnkflags`, `if_setlnkflags` | `socket` | same |

Not in `libc.a` at all (no symbol, so not in the count): `getcontext`,
`makecontext`, `setcontext`, `swapcontext` (§3); the libm gaps are listed in
§4/§6 of the libm part.
