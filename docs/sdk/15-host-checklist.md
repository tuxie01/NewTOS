# 15 — Host checklist: can this Mac build NewTOS and its apps?

A checklist for a second Mac (e.g. a MacBook on the road). Hand it to an
agent: it runs every **Check** command, compares with **Expect**, and reports
a table `item | found | OK/MISSING/DIFFERENT`. It installs nothing and copies
nothing on its own — it reports, the owner decides.

Reference host (Mac mini, 2026-10-09): macOS 27, Apple Silicon. All versions
below were measured there. Other versions may work but are untested; report
them as DIFFERENT, not as a failure.

Detail on why each tool is needed: [02-prerequisites.md](02-prerequisites.md).

## A. Repository and files that are NOT in git

`git clone https://github.com/tuxie01/NewTOS.git` gives everything tracked
(branch `main`, tags `v0.5`, `v0.6`). The items below are gitignored; without them the
listed builds fail. QED and TeraDesk can be re-cloned
([00-quickstart.md §5.2](00-quickstart.md#52-the-worked-examples-qed-and-teradesk)),
CoreMark per `sdk/apps/coremark-sources.md5`; the rest has to be copied
from the Mac mini. The upstream origin of four of them is known (the
embedded git checkouts report it: `git remote -v` / `git rev-parse --short
HEAD`, checked 2026-10-20): QED `github.com/freemint/qed` @ `f12055f`,
TeraDesk `github.com/freemint/teradesk` @ `b5532b8`, CoreMark
`github.com/czietz/coremark` @ `b546802`, doomgeneric
`github.com/ozkl/doomgeneric` @ `dcb7a8d`. The quick start for someone
without access to the Mac mini is [00-quickstart.md](00-quickstart.md).
Everything the other SDK libraries need (mintlib, gemlib, cflib, OpenLibm,
LDG, libshare, WinDom, z-tools, the five codec libraries) **is** tracked as
tarballs/zip in the repository.

| # | Path (repo-relative) | Needed for | Size | Check |
|---|---|---|---|---|
| A1 | repo on a path **without spaces** | `build-coremark.sh` word-splits paths | — | `pwd` contains no space |
| A2 | `sdk/apps/qed/` | QED (`sdk/apps/build-qed.sh`) | 5.5 MB | `test -d sdk/apps/qed/.git` |
| A3 | `sdk/apps/teradesk/` | TeraDesk (`build-teradesk.sh`) | 7.4 MB | `test -d sdk/apps/teradesk/.git` |
| A4 | `sdk/apps/coremark/` | CoreMark (`build-coremark.sh`); alternatively re-clone per `sdk/apps/coremark-sources.md5` | 2.1 MB | `test -f sdk/apps/coremark/core_main.c` |
| A5 | `sdk/apps/highwire/` | not built yet; copy to keep the source | 4.5 MB | `test -d sdk/apps/highwire` |
| A6 | `Applications/doomgeneric/` | Doom (`build-doom.sh`, `build-doom-m68k.sh`) | part of 55 MB | `test -d Applications/doomgeneric/doomgeneric` |
| A7 | `Applications/doom-wad/` | `DOOM1.WAD` for the card | | `ls Applications/doom-wad` |
| A8 | `Applications/ldg-src/` | provenance of the tracked LDG tarball only (`sdk/ldg/build-ldg.sh:10-12`); no build reads it | | `test -d Applications/ldg-src` |
| A9 | rest of `Applications/` (cflib-git, ldg-2, mintlib-src, pml-2.03, qed, thing-neo-codeSVN-r62, windom-1.21.3, z-tools) | reference sources, not built directly | 55 MB total | `ls Applications` |
| A10 | `references/` (emutos, freemint, circle, datasheets, arm-arm, gabi, libshare, rpi4-boot, sd-spec) | the specification sources (CLAUDE.md rule 1/6); no build reads them | 247 MB | `ls references` |
| A11 | Pi firmware for a card: `start4.elf`, `fixup4.dat`, `bcm2711-rpi-4-b.dtb` | writing a real card | small | e.g. `ls sd-card-backup-20261008-ofam/BOOT/` |

Simplest: copy the whole repo folder from the Mac mini (rsync without
`target/`, `.claude/worktrees/`), then `git status` must be clean.

## B. Tools

| # | Tool | Check | Expect (reference host) | Needed for |
|---|---|---|---|---|
| B1 | Xcode Command Line Tools: `clang`, `cc`, `make`, `patch` | `clang --version; make --version \| head -1; patch --version \| head -1` | Apple clang 21.0.0; GNU Make 3.81 | every C build |
| B2 | clang resource headers | `ls "$(clang -print-resource-dir)/include/stdarg.h"` | file exists | freestanding headers |
| B3 | rustup | `rustup --version` | any | Rust toolchain |
| B4 | pinned nightly from `rust-toolchain.toml` | `rustc -vV` (run **inside** the repo) | `rustc 1.97.0-nightly (a5c825cd8 2026-04-14)`, `LLVM version: 22.1.2`, `host: aarch64-apple-darwin` | kernel; linker for apps only as fallback (SDK01 K) |
| B5 | toolchain components | `rustup component list --installed` (inside repo) | includes `rust-src`, `rustfmt`, `clippy`, `llvm-tools` | `rust-lld`, `llvm-ar` for the SDK when B17 is absent |
| B6 | target | `rustup target list --installed` (inside repo) | includes `aarch64-unknown-none-softfloat` | kernel |
| B7 | `rust-lld` in the sysroot | `ls "$(rustc --print sysroot)/lib/rustlib/aarch64-apple-darwin/bin/"` | contains `rust-lld`, `llvm-ar`, `llvm-nm`, `llvm-objdump`, `llvm-readobj` | linking apps when B17 is absent |
| B8 | `python3` | `python3 --version` | 3.14.x | `mkrsc.py`, sample generators, `make-sd-image.sh` |
| B9 | Pillow | `python3 -c "import PIL; print(PIL.__version__)"` | 12.3.0 | `sdk/zview/samples/gen-samples.py`, `sdk/examples/libtest/gen-data.py` |
| B10 | `xz` | `xz --version \| head -1` | XZ Utils 5.8.3 | unpacking the pinned `.tar.xz` sources |
| B11 | `tar`, `shasum`, `md5`, `cmp`, `unzip`, `git` | `which tar shasum md5 cmp unzip git` | all found (macOS built-ins + git) | source pins, checksums |
| B12 | QEMU | `qemu-system-aarch64 --version \| head -1` | QEMU 11.0.2 | `cargo xtask run`, tests |
| B13 | macOS disk tools | `which hdiutil diskutil newfs_msdos dot_clean mdutil` | all found | `tools/make-sd-image.sh` (macOS only) |
| B14 | m68k cross GCC (**optional**) | `/opt/cross-mint/bin/m68k-atari-mint-gcc --version \| head -1` | `m68k-atari-mint-gcc (MiNT 20230908) 7.5.0` | only `build-doom-m68k.sh`, `build-gemlib-m68k.sh` (Doom for a real Atari) |
| B15 | Hatari (**optional**) | `which hatari` | found | TT/Falcon reference runs (`tools/hatari-qed.sh`) |
| B16 | Homebrew (if used to install B10/B12/B17) | `brew --version` | any | installing missing tools |
| B17 | LLVM `lld` + `llvm` (**optional** while B4/B5 exist) | `sdk/bin/newtos-cc --toolchain` | `linker … ld.lld`, `llvm-ar …` paths outside the Rust sysroot; on the reference host (no Homebrew llvm/lld) all point into the Rust sysroot | linking/archiving apps without Rust (TASK/SDK01 K); `brew install llvm lld` |

## C. Proof builds (run in this order; each must succeed)

```sh
cargo xtask test                                   # host tests: 1268 passed / 0 failed / 1 ignored
cargo xtask build --board rpi4 --release           # target/aarch64-unknown-none-softfloat/release/kernel8.img
sdk/run-host-tests.sh                              # 22 passed / 0 failed / 0 skipped (2026-10-20)
sdk/apps/present/build-present.sh                  # PRESENT.NAP
sdk/apps/build-doom.sh                             # DOOM.NAP (needs A6)
sdk/apps/build-qed.sh                              # QED (needs A2)
sdk/zview/build-zview.sh                           # ZVIEW.NAP + codecs (builds zlib/png/jpeg/gif/exif/windom/ldg on demand)
```

Expected: each script ends without error; the libraries build on first use
(`sdk/lib-stale.sh`). Report the first failing step with its last 20 lines
of output. `cargo xtask build --board rpi4` writes `kernel8.img` beside the
kernel ELF in `target/` and prints its path as the last line (TOOL10); the
repository root stays free of build output (CLAUDE.md: nothing new in the root).

## D. Report format

1. Table of A1–A11 and B1–B17 with found value and verdict.
2. Results of section C, one line each.
3. List of what is MISSING, split into "copy from the Mac mini" (section A)
   and "install" (section B, with the install command the owner would run,
   e.g. `xcode-select --install`, `rustup show` inside the repo,
   `brew install xz qemu`, `python3 -m pip install Pillow`).
