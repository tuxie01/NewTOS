# 02 — Prerequisites: the tools the scripts use, and where they look

Everything below is what the build scripts actually invoke. Versions are the
ones present on the host while this document was written (command output
quoted); the scripts do not pin versions.

## Compiler: clang

Every script uses `${CLANG:-clang}` from `PATH` (`sdk/build.sh:35`,
`sdk/libc/build-libc.sh:88`, `sdk/gemlib/build-gemlib.sh:98`,
`sdk/cflib/build-cflib.sh:87`, `sdk/apps/build-qed.sh:69`,
`sdk/apps/build-teradesk.sh:70`, `sdk/apps/build-coremark.sh:115`). The
target is always `-target aarch64-unknown-none-elf`. Measured on the host:

```
$ which clang && clang --version | head -2
/usr/bin/clang
Apple clang version 21.0.0 (clang-2100.1.1.101)
Target: arm64-apple-darwin27.0.0
```

The freestanding headers (`stddef.h`, `stdarg.h`, `stdint.h` … ) come from
clang's resource directory: `RES="$($CLANG -print-resource-dir)/include"`,
passed as `-isystem "$RES"` after the mintlib include path
(`sdk/libc/build-libc.sh:90,103`). On this host that is
`/Library/Developer/CommandLineTools/usr/lib/clang/21/include`.

**Header precedence matters:** `-nostdinc -I<mintlib>/include … -isystem $RES`
means mintlib's own `stdint.h` wins over clang's. Pristine, its `uintptr_t`
was `unsigned int` on a non-`__MSHORT__` build — 32 bits on our LP64 target;
since TASK/PORT10 the overlay `sdk/libc/overlay/include/stdint.h` makes
`intptr_t`/`uintptr_t` `long` on LP64 (with `INTPTR_*`/`UINTPTR_MAX` and the
`PRI*PTR`/`SCN*PTR` macros in `inttypes.h`). Since TASK/SDK01 A
(2026-10-20) `PTRDIFF_MIN/MAX` and `SIZE_MAX` in the same header and
`SSIZE_MAX` (overlay `sdk/libc/overlay/include/bits/posix1_lim.h`) are the 64-bit values too
(`sdk/libc/tests/lp64-limits.sh`). See
[05-abi-pitfalls.md](05-abi-pitfalls.md#b7-mintlibs-stdinth-gives-a-32-bit-uintptr_t-on-our-lp64-build).

## Linker and binutils: LLVM's, from the packages or from Rust

The library and app scripts, `sdk/build.sh` and `sdk/bin/newtos-cc` link with
`ld.lld` and use `llvm-ar`, `llvm-nm`, `llvm-readobj`, `llvm-objdump` and
`llvm-size`. All of them resolve each tool through one file,
`sdk/toolchain.sh` (TASK/SDK01 I, K):

```sh
. "$SDK/toolchain.sh"
AR="${AR:-$(newtos_tool llvm-ar)}"
LLD="${LLD:-$(newtos_tool ld.lld)}"
NM="$(newtos_tool llvm-nm)"
…
newtos_lld -T "$SDK/newtos.ld" -pie …      # "$LLD" (+ -flavor gnu for rust-lld)
```
(e.g. `sdk/apps/build-qed.sh`; the same lines in every script.)

`newtos_tool <name>` searches, per tool, and stops at the first hit:

| # | Where | Notes |
|---|---|---|
| 1 | `NEWTOS_LLD`, `NEWTOS_AR`, `NEWTOS_NM`, `NEWTOS_READOBJ`, `NEWTOS_OBJDUMP`, `NEWTOS_SIZE` (`llvm-<x>` → `NEWTOS_<X>`) | an explicit file; the scripts' older `LLD=` / `AR=` overrides still come before it |
| 2 | `$NEWTOS_LLVM_BIN/<name>` | any LLVM bin directory |
| 3 | `PATH` | Debian/Ubuntu/Fedora packages `lld` + `llvm` |
| 4 | the directory `${CLANG:-clang}` really lives in (symlinks followed) | e.g. Debian's `/usr/lib/llvm-<N>/bin`, or a keg-only Homebrew clang given as `CLANG=…` |
| 5 | `<prefix>/opt/llvm/bin`, `<prefix>/opt/lld/bin` for `$HOMEBREW_PREFIX`, `/opt/homebrew`, `/usr/local`, `/home/linuxbrew/.linuxbrew` | Homebrew's `llvm` is keg-only, not on `PATH` |
| 6 | `/usr/lib/llvm-*/bin`, highest version first | apt.llvm.org / Debian layout |
| 7 | the pinned Rust nightly's sysroot, `rustc` asked from inside `sdk/` | `rust-lld` for `ld.lld`, `llvm-*` from `llvm-tools-preview` |

`NEWTOS_TOOLCHAIN=auto` (default) is steps 1–7, `llvm` steps 1–6 (never
Rust), `rust` steps 1 and 7 (the behaviour before SDK01 K). When no step
finds the tool, `newtos_tool` prints which packages to install
(`sudo apt install clang lld llvm`, `sudo dnf install clang lld llvm`,
`brew install llvm lld`) and the script stops. `sdk/bin/newtos-cc --toolchain`
prints what every tool resolves to.

**`-flavor gnu`.** `rust-lld` is lld's generic driver and needs
`-flavor gnu`; `ld.lld` is the GNU flavour by its name. `newtos_lld` (and
`newtos-cc`) add the flag only for `rust-lld`/`lld`. Measured with the Rust
nightly's gcc-ld/ld.lld wrapper: `ld.lld -flavor gnu --version` →
`rust-lld: error: unknown argument '-flavor'`.

The Rust fallback is the pinned nightly of `rust-toolchain.toml`:

```toml
channel = "nightly-2026-04-15"
components = ["rust-src", "rustfmt", "clippy", "llvm-tools-preview"]
targets = ["aarch64-unknown-none-softfloat"]
```

`llvm-tools-preview` is what puts `rust-lld` & co. into
`<sysroot>/lib/rustlib/<host>/bin`. Measured:

```
$ rustc -vV
rustc 1.97.0-nightly (a5c825cd8 2026-04-14)
host: aarch64-apple-darwin
LLVM version: 22.1.2
$ $BIN/rust-lld -flavor gnu --version
LLD 22.1.2 (…) (compatible with GNU linkers)
```

A rustup proxy on `PATH` (`~/.cargo/bin/rust-lld`, `rust-ar`, …) is never
used: it resolves the toolchain by the caller's directory, so outside the
checkout it would be the *default* toolchain's, not the pinned one.

Measured on this Mac (2026-10-22,
`measurements/measurement-2026-10-22-sdk01-k/RUNINFO.TXT`): the Rust
fallback and an `ld.lld`/`llvm-*` directory on `PATH` with no `rustc`
reachable produced byte-identical archives and programs — both were the same
LLD 22.1.2, so this proves the resolution, not a different lld release.
Homebrew's and the Linux distributions' packages are not yet tested.

## Other host tools the scripts call

| Tool | Used by | For |
|------|---------|-----|
| `make` (GNU Make 3.81 on this host) | `build-gemlib.sh:129-132`, `build-cflib.sh:119-122`, `build-qed.sh:99-102` | expanding the libraries' own `SRCFILES` `COBJS` manifests |
| `tar` (xz-capable) | all three `build-*lib.sh` | extracting the pristine tarballs |
| `python3` (3.14.6 on this host) | `build-teradesk.sh:197-201`, `tools/make-sd-image.sh` | `tools/mkrsc.py` resource conversion; `mk-hello-ncp.py`; MBR sanity check |
| `md5` or `md5sum` | `build-coremark.sh:75-86` | run-rule integrity check |
| `hdiutil`, `diskutil`, `newfs_msdos`, `dot_clean`, `mdutil` | `tools/make-sd-image.sh:13,36-50,291` | building and populating the FAT16 image — **macOS only** |
| `qemu-system-aarch64` (11.0.2 on this host) | `cargo xtask run` (`tools/xtask/src/main.rs:477,536`) | booting the image |
| `cc` | `tools/qmpshot.c:13` | the QMP screendump client |

## Host assumptions baked into the scripts

* **macOS paths and tools.** `tools/make-sd-image.sh` is documented as "macOS
  only (uses hdiutil / diskutil / newfs_msdos)" (`:13`) and mounts the image
  under `/Volumes/NEWTOS` (`:44`). `sdk/build.sh` defaults its output to
  `/tmp/HELLO2.NCP` (`:27`); `make-sd-image.sh` defaults to `/tmp/sd.img`,
  which is also what `cargo xtask run --sd` looks for when no path is given
  (`tools/xtask/src/main.rs:414-421`).
* **Linux is not described and has not been tried.** The library build
  scripts may well run there (they use `ld.lld`/`llvm-*` from the `lld` and
  `llvm` packages or the Rust sysroot, `tar`, `make`, an `md5`/`md5sum`
  switch and `shasum`), but nobody has run them on
  Linux, and the image tool is macOS-only (see above). A Linux path (and a
  step-by-step install guide for any host: `xcode-select --install`, `rustup`
  inside the checkout, …) is planned (TASK/SDK01 packages C and G); it does
  not exist yet. *Update (TASK/SDK01 C):* the install guide is
  [00-quickstart.md](00-quickstart.md); its Linux commands are untested.
* **No network.** "A clone builds with no external network"
  (`sdk/.gitignore:6-7`): the three library tarballs are in the tree. The
  application trees under `sdk/apps/*/` are **not** tracked
  (`sdk/.gitignore:29-33`); QED and TeraDesk are embedded git checkouts,
  CoreMark is re-obtained per `sdk/apps/coremark-sources.md5:5-8`
  (`git clone https://github.com/czietz/coremark.git … checkout b546802`).
  Where they come from is readable from the embedded checkouts
  (`git remote -v`, `git rev-parse --short HEAD`, run 2026-10-20): QED
  `https://github.com/freemint/qed.git` @ `f12055f`, TeraDesk
  `https://github.com/freemint/teradesk.git` @ `b5532b8`. **No provenance
  file in the repository records this**; the clone-and-check-out recipe is
  [00-quickstart.md §5.2](00-quickstart.md#52-the-worked-examples-qed-and-teradesk).
* **`set -uo pipefail` without `-e`** in every library/app script: a failing
  step is handled explicitly (`|| exit 1`), not by the shell.
* **Paths with spaces are not handled** in `build-coremark.sh`'s
  `CFLAGS=(${PORT_CFLAGS} …)` word-split (`:136`); keep the checkout on a
  space-free path.
