# NewTOS native C SDK

Builds native AArch64 programs for NewTOS with **clang + LLVM lld**
(`ld.lld` from the LLVM packages, or the pinned Rust nightly's `rust-lld` as
fallback — `toolchain.sh`; `aarch64-unknown-none-elf`) on top of ported
Atari C libraries — the core
three are mintlib-0.60.1, gemlib-0.44.0 and cflib (pinned commit `ce57817`);
OpenLibm, LDG, libshare, WinDom and five codec libraries come on top —
following ADR-019 ("port real Atari C libraries, don't reimplement",
`docs/MEMORY.md`). It works from a checkout of this repository on macOS; the from-scratch quick
start is [`docs/sdk/00-quickstart.md`](../docs/sdk/00-quickstart.md) (Linux
commands there untested); an installable sysroot does not exist. The compiler wrapper `bin/newtos-cc`,
the Makefile include `mk/newtos.mk` and the templates in `templates/`
(TASK/SDK01 D) are described in
[`docs/sdk/16-newtos-cc.md`](../docs/sdk/16-newtos-cc.md). The output is an
ELF64 executable named `.NCP` (console), `.NAP` (GEM application) or `.NTP`
(takes parameters) per ADR-018; the kernel loader parses the ELF directly.

**The developer documentation lives in [`docs/sdk/`](../docs/sdk/README.md).**
It covers prerequisites, a built-and-run Hello World / mintlib / GEM example,
where a fix belongs (once in the SDK, or as a per-program patch file in
`sdk/overlays/<program>/`; upstream sources are never edited), the
build scripts for QED, TeraDesk and CoreMark, the LP64/ABI pitfalls that have
bitten ports, and how a binary gets onto the SD image and into QEMU or a real
Pi 4. This file is only a map of the directory.

## Layout

| Path | Purpose |
|------|---------|
| `bin/newtos-cc` | compiler wrapper (POSIX sh): one command compiles and links a program with the flags, crt0, libraries and linker script the scripts use; `--help` ([16](../docs/sdk/16-newtos-cc.md)) |
| `mk/newtos.mk` | Makefile include: `PROG`/`KIND`/`SRCS`/`LIBS`/`RSC` → `make`, `make card DEST=…`, `make clean` |
| `templates/` | `templates/gem-hello/` (window + alert, clean exit) and `templates/console-hello/`, each a `hello.c` + `Makefile` using `mk/newtos.mk`; `templates/tests/newtos-cc-host.sh` (run by `run-host-tests.sh`) |
| `build.sh` | freestanding `.NCP` builder (no libc): `clang -c crt0.S + source`, `ld.lld -T newtos.ld` |
| `crt0.S` | startup for freestanding programs: basepage from `x0`, `SP = p_hitpa`, `bl main`, `Pterm` |
| `newtos.ld` | linker script for **every** native program, a position-independent executable (`ET_DYN`, `-fPIE` + `-pie --no-dynamic-linker -z text`; the loader applies its `R_AARCH64_RELATIVE` relocations, ARCH03 0d): two `PT_LOAD`s from `0x1000` — `text` (R-X: `.text`+dynamic metadata+`.rodata` → `p_tbase/p_tlen`) and `data` (RW-: `.dynamic`+`.got`+`.data` + `.bss` tail → `p_dbase/p_dlen`, `p_bbase/p_blen`) plus `PT_DYNAMIC`, `_start` first, `__bss_start`/`__bss_end`/`_end` exported |
| `include/newtos.h` | clean-room inline-asm trap helpers + `Cconws`/`Cconin`/`Pterm`/`Pterm0` |
| `examples/*.c` | freestanding proof programs (`hello`, `big`, `physbase`, `scrflip`, `dtachild`, `ptermch`, `ctrlcpar`) installed by `tools/make-sd-image.sh` |
| `lib-stale.sh` | the library dependency chain (`libc → gemlib → cflib`), staleness test and failure bookkeeping shared by every script |
| `libc/` | mintlib port: unchanged upstream tarball, `libc/overlay/` (trap macros, fixed-width TOS structs, LP64/endian fixes), `libc/newtos-arch/` (AArch64 shims and TOS-call replacements, 15 files), libc-aware `crt0.S`, `build-libc.sh`, `expected-failures.txt`, `tests/` |
| `gemlib/` | gemlib port: tarball, `gemlib/overlay/` (`svc #2` primitives, LP64 header corrections), `build-gemlib.sh`, `tests/`; plus `build-gemlib-m68k.sh` for the real-Atari build of the same library |
| `cflib/` | cflib port: tarball, `cflib/overlay/`, `build-cflib.sh`, `tests/` |
| `libm/`, `ldg/`, `libshare/`, `windom/`, `zlib/`, `libpng/`, `libjpeg/`, `giflib/`, `libexif/` | further ported libraries (OpenLibm; LDG; libshare; WinDom; the five codec libraries), each with its pinned archive, build script and, where needed, an overlay/ directory |
| `compiler-rt/` | the compiler runtime (TASK/SDK01 E): LLVM compiler-rt 21.1.8's `lib/builtins` (pinned tarball + signature, `compiler-rt-sources.sha256`), `build-compiler-rt.sh` → `libclang_rt.builtins.a` (binary128 `long double` and complex-arithmetic helpers, linked last by `bin/newtos-cc`), `compiler-rt/tests/rt-host.sh` (run by `run-host-tests.sh`) |
| `zview/` | zView port: z-tools tarball, `zview/overlay/` (13 per-program patch files), `build-zview.sh` |
| `overlays/{qed,teradesk,coremark}/` | per-program patch files: QED 2 (LP64), TeraDesk 7 (1 m68k asm, 6 LP64); CoreMark's is its own port layer, not a patch. zView's patches are in `zview/overlay/`, library patches in each library's overlay/ directory. Reasons: [`docs/sdk/04-porting-anatomy.md`](../docs/sdk/04-porting-anatomy.md) |
| `apps/` | `build-qed.sh`, `build-teradesk.sh`, `build-coremark.sh`, the tracked diagnostics `apps/apitest/`, `apps/gemtest/`, `apps/chaintest/`, and the **untracked** vendored application trees |

## Quick start

```sh
sdk/bin/newtos-cc -nap -o MYAPP.NAP main.c            # your program (libc + gemlib); --help
make -C sdk/templates/gem-hello                        # template -> HELLO.NAP
sdk/build.sh sdk/examples/hello.c /tmp/HELLO2.NCP     # freestanding console program
sdk/libc/build-libc.sh                                 # libc.a + build/libtest.NCP
sdk/compiler-rt/build-compiler-rt.sh                   # libclang_rt.builtins.a (newtos-cc links it)
sdk/gemlib/build-gemlib.sh                             # gemlib.a + build/gemtest.NCP
sdk/cflib/build-cflib.sh                               # cflib.a + build/cftest.NCP
sdk/apps/build-qed.sh                                  # sdk/apps/qed/build/QED.NAP
sdk/apps/build-teradesk.sh                             # sdk/apps/teradesk/build/dist/
tools/make-sd-image.sh /tmp/sd.img 64                  # FAT16 image; fails if a listed program is unbuilt
cargo xtask run --board rpi4 --sd /tmp/sd.img --window # boot it in QEMU raspi4b
```

Full walk-through, exact flags, the ABI contract and the pitfall catalogue:
[`docs/sdk/README.md`](../docs/sdk/README.md).
