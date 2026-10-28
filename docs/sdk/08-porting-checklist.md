# 08 — Porting checklist

Tick each line; every line points at the section that explains it.

## Before touching anything

- [ ] The app is C (plus at most inline m68k asm, which has to be rewritten)
      and needs only mintlib / gemlib / cflib / libm — not sockets or
      threads; `setjmp`/`longjmp` and the C99 double/float libm are
      available since T07, `long double` arithmetic and complex `*`/`/`
      (compiler-rt builtins, linked by `newtos-cc`) since SDK01 E, `long
      double` in printf/scanf/`strtold` and the `…l` libm functions since
      SDK01 J
      → [07 §3–4](07-mintlib-limits.md#3-mintlib-assembly-that-is-never-assembled)
- [ ] Toolchain present: `clang`, LLVM's `lld` + `llvm` packages
      (`ld.lld`, `llvm-ar`, …; or, as fallback, the pinned Rust nightly with
      `llvm-tools-preview`), `make`, `python3` — `sdk/bin/newtos-cc
      --toolchain` shows what is used → [02](02-prerequisites.md)
- [ ] `sdk/libc/build-libc.sh`, `sdk/gemlib/build-gemlib.sh`,
      `sdk/cflib/build-cflib.sh`, `sdk/libm/build-libm.sh` run clean:
      `failures match expected-failures.txt`
      → [04 §library chain](04-porting-anatomy.md#the-library-chain-and-its-staleness-test)

## Setting the port up

- [ ] Source vendored under `sdk/apps/<app>/` (untracked), provenance recorded
      like `sdk/apps/coremark-sources.md5` → [04 §skeleton](04-porting-anatomy.md#adding-a-new-port-the-skeleton)
- [ ] `sdk/overlays/<app>/` created when the first per-program patch is needed
      (tracked; the patch files of one program live there)
- [ ] Build route chosen. **Outside the repository** (your own project):
      `newtos-cc` or a `newtos.mk` Makefile, patches in a directory of your
      own → [00 §5](00-quickstart.md#5-porting-an-existing-atari-c-program),
      [16](16-newtos-cc.md). **Inside the repository**: a
      `sdk/apps/build-<app>.sh`, either on `newtos-cc` (model:
      `build-hypview.sh`, TASK/SDK01 H) or copied from `build-qed.sh`
      (cflib app) / `build-teradesk.sh` (header overlays, own dialog lib),
      which call clang and the linker (`newtos_lld`, `sdk/toolchain.sh`)
      directly and keep `lib_ensure`, the
      shared flag set, `-D__NEWTOS__`, `--start-group` link. Either way: any
      failing file stops the build → [04](04-porting-anatomy.md)
- [ ] Output name decided: `.NAP` (GEM), `.NCP` (console), `.NTP` (takes a
      tail — no dialog yet) → [01](01-overview.md#output-native-elf-binaries-with-a-new-extension-family)

## Getting it to compile and link — the rule

- [ ] Every compile error is classified: **would every LP64 port meet it?**
      (a library, header, shim, `crt0` or flag problem) → fix it **once in the
      SDK**; **only this program's own assumption** (m68k asm, 32-bit `long`,
      struct size, big-endian tricks) → a per-program patch file in
      `sdk/overlays/<app>/` whose header says why. Never edit inside the
      upstream tree → [04 §rule](04-porting-anatomy.md#the-rule-upstream-stays-unchanged-fixes-go-where-they-belong)
- [ ] Every undefined symbol traced with the linker's `--why-extract` output (`why.txt` in the app's build directory); classified against
      the dangling-reference list → [07 §6](07-mintlib-limits.md#6-references-libca-leaves-unresolved-measured)
- [ ] `undefined symbols in the final image (goal 0): 0`

## Walking the ABI traps against the app's own code

- [ ] Every TOS structure the app reads or writes itself (`BASEPAGE`, `_DTA`,
      `_DISKINFO`, `OSHEADER`, cookie slots, `_MD`, …) is either one the
      overlay pins to 32-bit fields, or gets a fixed-width declaration with
      `_Static_assert`s → [05 B1](05-abi-pitfalls.md#b1-long-and-pointers-are-8-bytes-tos-structures-are-32-bit-on-the-wire)
- [ ] Cookie jar accessed via `Getcookie()`/`Ssystem`, or as 4-byte reads
      with 8-byte slots → [05 B2](05-abi-pitfalls.md#b2-the-cookie-jar-at-5a0)
- [ ] Numeric `_MCH`/`_VDO` comparisons checked. NewTOS reports
      `_MCH = 0xFFFFFFFF` ("no Atari hardware", ADR-012 amendment
      2026-10-28) and `Getcookie()` zero-extends, so a native port sees a
      large positive value: `mch >= 2`, `(mch >> 16) >= 3` and the like take
      the TT/Falcon ("above Falcon") hardware path. Patch such a test in
      `sdk/overlays/<app>/` (user 2026-10-28)
- [ ] No dereference of `Kbdvbase()`; returned pointers range-checked;
      screen writes assume XRGB8888 → [05 B3](05-abi-pitfalls.md#b3-kernel-addresses-vs-user-addresses)
- [ ] `Super`/`Supexec` bracket idioms reviewed: they run, but grant no
      privilege; shutdown/reset paths become live → [05 B4](05-abi-pitfalls.md#b4-super-supexec-supertouser)
- [ ] Directory probes via `Fattrib` expect the TOS4/FreeMiNT answer
      → [05 B5](05-abi-pitfalls.md#b5-fattrib-on-directories--tos-4--freemint-semantics)
- [ ] `argv` works (PORT01); `environ`/`getenv` and the `ARGV=` extension work
      too (PORT09, see [01](01-overview.md), [07 §5](07-mintlib-limits.md));
      an app that reads its command tail itself can still use `p_cmdlin` or
      `shel_read`
      → [05 B6](05-abi-pitfalls.md#b6-argcargv-the-command-tail-and-the-environment)
- [ ] `uintptr_t`/`intptr_t` are pointer-wide since TASK/PORT10; `SIZE_MAX`,
      `PTRDIFF_MIN/MAX` and `SSIZE_MAX` are the 64-bit values since TASK/SDK01 A
      → [05 B7](05-abi-pitfalls.md#b7-mintlibs-stdinth-gives-a-32-bit-uintptr_t-on-our-lp64-build)
- [ ] `.RSC` converted with `tools/mkrsc.py convert` (kernel loader) or
      `--rshdr` (own loader); shipped beside the `.NAP` → [05 B8](05-abi-pitfalls.md#b8-resource-files-format-alignment-and-who-loads-them)
- [ ] `AESGLOBAL` read only through gemlib's macros → [05 B9](05-abi-pitfalls.md#b9-aesglobal-is-15-words-with-4-byte-pointer-fields)
- [ ] Pointer arguments to `wind_set` go through `wind_set_str`/`_ptr`
      → [05 B10](05-abi-pitfalls.md#b10-pointer-arguments-to-wind_set-and-friends)
- [ ] Bit-fields on `unsigned int`, big-endian word tricks, `ror`/`swap`
      asm reviewed → [05 B11](05-abi-pitfalls.md#b11-bit-fields-int-alignment-and-big-endian-word-tricks)
- [ ] Every AES/VDI call that passes a `long` or a pointer through `int_in`
      goes through a binding that splits it arithmetically, not by casting
      `&arg` to `short *` → [05 B16](05-abi-pitfalls.md#b16-gemlib-bindings-that-split-a-long-with-a-pointer-cast)
- [ ] `long double` works (soft-float binary128, compiler-rt builtins;
      `printf("%Lf")`, `strtold`, the `…l` libm functions since SDK01 J);
      `*`/`/` ignore `fesetround()` and `sqrtl` can be 1 ulp off
      → [05 B12](05-abi-pitfalls.md#b12-long-double-is-128-bit-and-the-compiler-runtime-is-compiler-rt), [07 §4](07-mintlib-limits.md#4-mintlib-directories-not-compiled-at-all)
- [ ] libc `malloc` grows through GEMDOS `Malloc` (mintlib's `_stksize`
      model, ARCH03 E9); a program that needs its own heap sets `_stksize`
      → [05 A3/B15](05-abi-pitfalls.md#b15-a-libc-malloc-that-suddenly-fails)
- [ ] Console output strategy chosen (`.NCP` vs. log file) → [05 B14](05-abi-pitfalls.md#b14-console-output-is-not-the-desktop)
- [ ] Pointers in AES messages (`*(char **)&msg[3]`, AV protocol, BubbleGEM)
      carried as two WORDs, high first → [05 B18](05-abi-pitfalls.md#b18-pointers-inside-aes-messages-appl_write-av-protocol-bubblegem)
- [ ] Binary file formats read with `Fread` into structs decoded byte by
      byte (big-endian, 4-byte `long`, 2-byte alignment) → [05 B19](05-abi-pitfalls.md#b19-m68k-binary-file-formats-read-straight-into-structs)

## On the image and in the machine

- [ ] Program and `.rsc` in their own folder (`C:\APPS\<APP>\`, `.rsc`
      beside the `.NAP`; `.NAP` **never** in `\AUTO\`): on the release card
      or a self-made image → [00 §4](00-quickstart.md#4-run-it); for a port
      that lives **in this repository**, as an install block in
      `tools/make-sd-image.sh` → [06](06-image-and-testing.md#the-sd-image-toolsmake-sd-imagesh)
- [ ] After any manual copy onto the FAT volume: `dot_clean -m` (AppleDouble
      `._X.NCP` files would be launched) → [06](06-image-and-testing.md#the-sd-image-toolsmake-sd-imagesh)
- [ ] Headless QEMU run via `#Z 01 C:\…\APP.NAP@` in `NEWDESK.INF`; serial
      log shows `[shell] sh_ldapp: starting …` and no `[EXC]`/`[PANIC]`;
      screendump via QMP checked → [06](06-image-and-testing.md#newdeskinf-z--a-gem-application-instead-of-the-desktop)
- [ ] Only if you build the kernel yourself (not needed with the release
      `kernel8.img`): built fresh for the board being booted (`--board rpi4`),
      release profile for hardware → [06 footgun](06-image-and-testing.md#the-board-footgun)
- [ ] Interactive behaviour (clicks, keys, menus, closing) verified on a real
      Pi 4 — QEMU `raspi4b` has no input → [06](06-image-and-testing.md#real-raspberry-pi-4)

## Before handing over

- [ ] Every per-program patch file carries a header saying **why** it exists
      and what it changes against upstream (the pattern of `sdk/overlays/*`)
- [ ] **A moved library pin rebases every full-file overlay onto it.** An
      overlay that is a whole copy of an upstream file silently reverts
      everything upstream fixed in that file since the old pin. PORT03:
      `sdk/cflib/overlay/fontsel.c` would have taken back the leak fix of
      `399f7a6`. Diff the old pin's original against the new one first, then
      re-apply the overlay's own change on top
- [ ] **The new tarball actually arrives.** A library's `build-*.sh` must
      re-extract when the tarball is newer than the tree, and `sdk/lib-stale.sh`
      must watch the tarball. cflib has both since PORT03; **mintlib and gemlib
      do not** ([PORT04](../../TASK/PORT04.md)) — until then use
      `MINTLIB_REFRESH=1` / `GEMLIB_REFRESH=1`
- [ ] `sdk/libc/expected-failures.txt` unchanged, or a new entry with a
      reason and a ticket → [04](04-porting-anatomy.md#the-library-chain-and-its-staleness-test)
- [ ] Measured evidence kept (serial log, screendump, hashes), not claims
