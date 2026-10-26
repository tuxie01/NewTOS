# 00 — Quick start: from a fresh machine to your own program on NewTOS

This page takes you from nothing to a program of your own running on NewTOS:
install the tools, build the SDK libraries once, compile a program, put it on
a card (or into QEMU), and — further on — port an existing Atari C program.
You do **not** need to build the NewTOS kernel: the
[v0.6 release](https://github.com/tuxie01/NewTOS/releases/tag/v0.6) contains
a ready kernel, and programs built with the SDK run on it.

**What was verified, and where.** Every macOS command on this page was run on
2026-10-20 on an Apple Silicon Mac (macOS 27) in a *clean* shell: a
`git clone` from GitHub (`main` at `f9350fac`), a fresh `rustup` in an empty
home, and a `PATH` of only `/usr/bin:/bin:/usr/sbin:/sbin` plus rustup's
`~/.cargo/bin` — no Homebrew. The times quoted are from that run. The QEMU
steps additionally used Homebrew's `qemu` (11.0.2) and `mtools` (4.0.49).
**Linux commands are untested** (marked so); the scripts are written to run
there, but nobody has done it yet. Windows is not covered.
*Since TASK/SDK01 K (2026-10-22)* Rust is no longer needed for the SDK: the
scripts take `ld.lld`, `llvm-ar` & co. from the LLVM packages and fall back
to the Rust toolchain only when those are absent (§1.2). On the Mac this was
checked with the Rust fallback and with an LLVM directory on `PATH` and no
Rust at all; Homebrew's own `llvm`/`lld` packages were **not** installed for
that check, and Linux remains untested.

The other chapters ([README](README.md)) explain the details; this page links
to them where you need them.

---

## 1. Get the repository and the tools

### 1.1 Host tools

**macOS** — the Xcode Command Line Tools provide `clang`, `make`, `git`,
`patch`, `tar` (reads `.tar.xz` itself) and `python3`; the ELF linker and
the LLVM archiver come from Homebrew's `lld` and `llvm` (Apple ships
neither):

```sh
xcode-select --install          # skip if `clang --version` already works
brew install llvm lld           # ld.lld, llvm-ar, llvm-nm, ... (untested, see §1.2)
```

Homebrew's `llvm` is *keg-only*: it is not put on `PATH`, and it does not
need to be — the SDK looks in `$(brew --prefix)/opt/llvm/bin` and
`…/opt/lld/bin` by itself (§1.2). Apple's `clang` stays the compiler.

Only for running programs in QEMU (step 4b) you also need:

```sh
brew install qemu mtools        # Homebrew: https://brew.sh
```

**Linux, Debian/Ubuntu (untested):**

```sh
sudo apt install git curl clang lld llvm make patch xz-utils python3 build-essential
sudo apt install qemu-system-arm mtools        # only for step 4b
```

**Linux, Fedora (untested):**

```sh
sudo dnf install git curl clang lld llvm make patch xz python3 gcc
sudo dnf install qemu-system-aarch64 mtools    # only for step 4b
```

`lld` gives `ld.lld`, `llvm` gives `llvm-ar`, `llvm-nm`, `llvm-readobj`,
`llvm-objdump` — the linker and tools the SDK uses (§1.2).

`build-essential` (Fedora: `gcc`) gives the host `cc` that
`sdk/run-host-tests.sh` uses.

**No AArch64 cross compiler is needed** — not on an x86_64 PC either, so no
`aarch64-linux-gnu-gcc`. `clang` is a cross compiler by itself and is used
purely as one (`-target aarch64-unknown-none-elf`); the distribution packages
normally include the AArch64 back end. Check it with:

```sh
clang -print-targets | grep aarch64            # expect a line "aarch64 - AArch64 (little endian)"
```

The linker is LLVM's `ld.lld`, not the system's `ld`; libc and the runtime
are built by the SDK itself from the sources in the repository.

Which `clang` version is the minimum is not known: the SDK was only ever
built with Apple clang 21.0.0. If your distribution's `clang` is old (e.g.
clang 14 on Debian 12 / LMDE 6) and something fails, a newer one comes from
the LLVM project's own packages, <https://apt.llvm.org>. Please report what
worked or failed (issue on GitHub, with the error message). The whole Linux
check in one script, with the expected results and what to send back:
[17-linux-test.md](17-linux-test.md).

### 1.2 Linker and LLVM tools — Rust is optional

The SDK links with `ld.lld` and archives with `llvm-ar`; it also calls
`llvm-nm`, `llvm-readobj`, `llvm-objdump` and `llvm-size`. Every script and
`newtos-cc` finds them through `sdk/toolchain.sh`, per tool, in this order
(TASK/SDK01 K):

1. `NEWTOS_LLD`, `NEWTOS_AR`, `NEWTOS_NM`, `NEWTOS_READOBJ`,
   `NEWTOS_OBJDUMP`, `NEWTOS_SIZE` — an explicit file (the older `LLD` and
   `AR` still work too);
2. the directory `NEWTOS_LLVM_BIN`, if set;
3. `PATH` (`ld.lld`, `llvm-ar`, … — the distribution packages);
4. the directory `clang` really lives in, symlinks followed (on Debian
   e.g. `/usr/lib/llvm-14/bin`);
5. Homebrew's `opt/llvm/bin` and `opt/lld/bin` under `$HOMEBREW_PREFIX`,
   `/opt/homebrew`, `/usr/local`, `/home/linuxbrew/.linuxbrew`;
6. `/usr/lib/llvm-*/bin`, highest version first (apt.llvm.org's layout);
7. only then the Rust nightly that `rust-toolchain.toml` pins
   (`nightly-2026-04-15` with `llvm-tools-preview`): its `rust-lld` and
   `llvm-*`.

`NEWTOS_TOOLCHAIN=llvm` never uses Rust, `NEWTOS_TOOLCHAIN=rust` uses only
the Rust nightly (the SDK's behaviour before SDK01 K). To see what will be
used:

```sh
sdk/bin/newtos-cc --toolchain
```

If nothing is found, the build stops with a message that names the packages
to install (`sudo apt install clang lld llvm`, `brew install llvm lld`, …).

**Rust is needed only for kernel work** (building NewTOS itself, `cargo
xtask`), or as the fallback when you cannot install LLVM's `lld`/`llvm`.
Then install `rustup`, the same way on macOS and Linux:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
. "$HOME/.cargo/env"            # or open a new terminal
```

What was compared (measurement
`measurements/measurement-2026-10-22-sdk01-k/RUNINFO.TXT`): on the Mac all
13 SDK archives and 51 program files (QED, TeraDesk, HypView, zView with
its codecs, Doom, the templates, …) built once through the Rust fallback
and once with `ld.lld`/`llvm-*` from a `PATH` directory and no `rustc`
reachable came out byte-identical, because both runs used the same LLD
22.1.2. A different lld release (Homebrew's, Debian's) was not compared yet.
Byte comparisons between machines also need the same locale: the archive
member order follows `sort`, so `libc.a` & co. differ between
`LANG=de_DE.UTF-8` and the C locale (same members, other order).

### 1.3 The repository

```sh
git clone --depth 1 https://github.com/tuxie01/NewTOS.git
cd NewTOS
# only with Rust (kernel work, or the fallback of §1.2):
rustup toolchain install        # installs the pinned nightly + components from rust-toolchain.toml
rustc -vV                       # expect: rustc 1.97.0-nightly (a5c825cd8 2026-04-14)
```

Measured: the shallow clone took about 1 minute (774 MB working tree), the
toolchain download 2 min 12 s. Without `--depth 1` the clone carries the full
history.

The SDK scripts can be started from any directory, by absolute or relative
path. When they fall back to Rust, they ask `rustc` from inside the SDK
directory (`sdk/toolchain.sh`): `rustup` chooses the toolchain by the current
directory, so that is the pinned nightly whatever directory you started in.

Everything the SDK libraries are built from is **in the repository** as the
unchanged upstream archives: mintlib, gemlib, cflib, OpenLibm, LDG,
libshare, WinDom, z-tools and the five codec libraries
(`git ls-files 'sdk/*.tar.*' 'sdk/*.tgz' 'sdk/*.zip'`). Building them needs
no network. Only the *application* trees (QED, TeraDesk, …) are not in the
repository; step 5 says where they come from.

---

## 2. Build the SDK libraries (once)

```sh
sdk/cflib/build-cflib.sh                 # builds libc.a (mintlib), gemlib.a, then cflib.a
sdk/libm/build-libm.sh                   # libm.a (OpenLibm)
sdk/compiler-rt/build-compiler-rt.sh     # libclang_rt.builtins.a (compiler runtime)
```

`build-cflib.sh` builds the chain below it first (`sdk/lib-stale.sh`), so
these three commands give you the four core libraries and the compiler
runtime (LLVM compiler-rt's builtins: 128-bit `long double` and complex
arithmetic — `long double` works in `printf`/`scanf` and the `…l` math
functions too since TASK/SDK01 J — which `newtos-cc` links into every program — it stops with
`libclang_rt.builtins.a is not built` until this has run). Measured: 62 s,
9 s and 7 s.
Building them one by one is equivalent: `sdk/libc/build-libc.sh` (33 s),
`sdk/gemlib/build-gemlib.sh` (22 s), `sdk/cflib/build-cflib.sh` (6 s),
`sdk/libm/build-libm.sh` (9 s). Each script ends with a summary and a test
link, for example:

```
failures match libc/expected-failures.txt: 4 today, 4 recorded, 0 new
LINK OK -> …/sdk/libc/build/libtest.NCP (103656 bytes)
```

The other libraries (LDG, libshare, WinDom, zlib, libpng, libjpeg, giflib,
libexif) are built the same way by their own `sdk/<lib>/build-<lib>.sh`, when
you need them ([16 §1](16-newtos-cc.md#libraries) has the table).

**Check that it worked:**

```sh
sdk/run-host-tests.sh
```

Expected (measured in a fresh clone before TASK/SDK01 E: 20 passed, 34 s; it
builds the codec libraries for its own test on the way. SDK01 E added
`rt-host`, about 20 s, and SDK01 J `ldbl-host`, about 20 s (it needs
`python3` for its oracle and skips without it); both pass once step 2 is
done — the fresh-clone run was not repeated):

```
host tests: 22 passed, 0 failed, 3 skipped
```

The three skips are tests whose inputs you do not have yet and are fine:
`aesglobal_fields` (needs the TeraDesk source, step 5), `custom-font-host`
(needs a zView build), `libshare-host` (needs `sdk/libshare/build-libshare.sh`).
On a non-AArch64 host (a Linux PC) `setjmp-host`, `rt-host` and `ldbl-host`
(they run the AArch64 code of the compiler runtime and of libc/libm
natively) skip as well.

---

## 3. Build your first program

### 3a. From a template, with `make`

```sh
make -C sdk/templates/gem-hello          # -> sdk/templates/gem-hello/HELLO.NAP (a GEM window)
make -C sdk/templates/console-hello      # -> sdk/templates/console-hello/HELLO.NCP
```

For a project of your own, copy a template anywhere and tell it where the SDK
is:

```sh
cp -R sdk/templates/gem-hello ~/myapp
cd ~/myapp
make NEWTOS_SDK=/path/to/NewTOS/sdk      # -> HELLO.NAP
```

Edit `PROG`, `KIND`, `SRCS`, `LIBS` and `RSC` in its `Makefile`
([16 §2](16-newtos-cc.md#2-newtosmk)).

### 3b. One file, with `newtos-cc`

`sdk/bin/newtos-cc` compiles and links in one command, like a cross `gcc`.
Put it on your `PATH` or call it by path:

```c
/* hello.c */
#include <stdio.h>
#include <mint/osbind.h>

int main(void)
{
    printf("Hello from NewTOS, GEMDOS %04x\n", (unsigned)(Sversion() & 0xffff));
    printf("Press a key.\n");
    Cconin();
    return 0;
}
```

```sh
/path/to/NewTOS/sdk/bin/newtos-cc -Wall -o HELLO.NCP hello.c      # console program
/path/to/NewTOS/sdk/bin/newtos-cc -nap -o MYAPP.NAP main.c        # GEM application (adds gemlib)
/path/to/NewTOS/sdk/bin/newtos-cc -nap -lcflib -lm -o MYAPP.NAP *.c
```

The result is a native AArch64 ELF (`file HELLO.NCP` → `ELF 64-bit LSB pie
executable, ARM aarch64`). The **extension** tells the desktop how to start
it: `.NAP` GEM application, `.NCP` console program, `.NTP` console program
that asks for a command line, `.NAC` accessory
([01](01-overview.md#output-native-elf-binaries-with-a-new-extension-family)).
Names must be 8.3. All options: `newtos-cc --help` and
[16-newtos-cc.md](16-newtos-cc.md).

Programs are **LP64**: `int` is 32 bits, `long` and pointers 64 bits. Read
[05 Part B](05-abi-pitfalls.md#part-b--the-pitfalls) before you port anything
that touches TOS structures.

---

## 4. Run it

### 4a. On a Raspberry Pi 4, with the release card

1. Download `NewTOS-0.6.zip` from the
   [v0.6 release](https://github.com/tuxie01/NewTOS/releases/tag/v0.6) and
   prepare the card as its `README.TXT` says: MBR partition table, first
   partition **FAT16, at most 2 GB** (FAT32/exFAT do not work), then copy the
   *contents* of the ZIP's `KARTE` folder onto it. The README has the
   partitioning commands for macOS, Windows and Linux.
2. Copy your program into its own folder under `APPS\` on that partition,
   together with its `.RSC` file if it has one (the AES looks for the
   resource in the program's folder first,
   [04](04-porting-anatomy.md#resource-files-rsc)). On macOS, with the card
   mounted as `/Volumes/NEWTOS`:

   ```sh
   mkdir -p /Volumes/NEWTOS/APPS/HELLO
   cp -X HELLO.NAP /Volumes/NEWTOS/APPS/HELLO/
   dot_clean -m /Volumes/NEWTOS        # removes ._* files macOS may have left
   ```

   or, from a project that uses `newtos.mk`:
   `make card DEST=/Volumes/NEWTOS/APPS/HELLO` (copies with `cp -X`).
   On Linux a plain `cp` to the mounted partition is enough.
3. Eject the card, boot the Pi, open *DISK C* → `APPS` → `HELLO` and
   double-click `HELLO.NAP`. A `.NCP` runs in the console and waits for a key
   before the desktop comes back.

Programs from the current SDK (`main`) run on the v0.6 kernel: both templates
were started on the v0.6 `kernel8.img` in QEMU for this page (4b). The step on
real hardware was **not** repeated for this page; it is the release's own
installation procedure plus a file copy.

### 4b. In QEMU (optional)

QEMU's `raspi4b` machine boots the same release `kernel8.img` from an SD-card
image — again no kernel build. Limits: QEMU's `raspi4b` has **no USB**, so
there is no keyboard and no mouse (the log says
`[pcie] RC not present; USB disabled`). You can watch a program, not use it.
Two ways to start something without a click:

* a **console program** (`.NCP` that ends by itself, like the
  `console-hello` template) in `C:\AUTO\` runs at boot, before the desktop;
* a **GEM program** is started instead of the desktop by a `#Z` line in
  `C:\NEWDESK.INF` ([06](06-image-and-testing.md#newdeskinf-z--a-gem-application-instead-of-the-desktop)).
  The line must be in the first 300 bytes of the file, so it goes directly
  after the first line (`#R 02`).

Build a 256 MiB image (MBR + FAT16) from the release card and your programs.
These commands use only `python3` and `mtools`, so they are meant for macOS
and Linux alike (run on macOS; **untested on Linux**):

```sh
unzip NewTOS-0.6.zip                            # -> NewTOS-0.6/KARTE/
K=NewTOS-0.6/KARTE

# your programs onto the card tree
mkdir -p "$K/APPS/HELLO" "$K/AUTO"
cp sdk/templates/gem-hello/HELLO.NAP     "$K/APPS/HELLO/"
cp sdk/templates/console-hello/HELLO.NCP "$K/AUTO/"
# start HELLO.NAP instead of the desktop: #Z as the second line
awk 'NR==1 {print; printf "#Z 01 C:\\APPS\\HELLO\\HELLO.NAP@\r\n"; next} {print}' \
    "$K/NEWDESK.INF" > NEWDESK.tmp && mv NEWDESK.tmp "$K/NEWDESK.INF"

# empty image with one FAT16 partition (type 0x06) from sector 2048
dd if=/dev/zero of=sd.img bs=1048576 count=256
python3 - sd.img <<'EOF'
import struct, sys
with open(sys.argv[1], "r+b") as f:
    sectors = f.seek(0, 2) // 512
    f.seek(446)
    f.write(struct.pack("<B3sB3sII", 0, b"\xfe\xff\xff", 0x06, b"\xfe\xff\xff", 2048, sectors - 2048))
    f.seek(510)
    f.write(b"\x55\xaa")
EOF
mformat -i sd.img@@1M -T 522240 -h 64 -s 32 -H 2048 -v NEWTOS ::   # 522240 = 255 MiB in sectors
mcopy -i sd.img@@1M -s -Q "$K"/* ::/
mdir -i sd.img@@1M ::/                          # check: kernel8.img, APPS, AUTO, NEWDESK.INF …

# boot it (macOS window; on Linux use -display gtk or -display sdl)
qemu-system-aarch64 -M raspi4b -kernel "$K/kernel8.img" -append serial=on \
    -drive file=sd.img,if=sd,format=raw -serial stdio -display cocoa
```

`-append serial=on` turns on the kernel's serial log (it goes to the
terminal); the window shows the screen. Expected in the log:

```
[auto] running C:\AUTO\HELLO.NCP
Hello from a NewTOS console program
…
[shell] sh_ldapp: starting C:\APPS\HELLO\HELLO.NAP, sh_nextapp=2, sh_isgem=1
```

and on the screen the *GEM Hello* window. (Verified with `-display none`
and a QMP screendump; `-display cocoa` is what `cargo xtask run --window`
uses.) Quit with Ctrl-C in the terminal or by closing the QEMU window.
To replace a file later (QEMU not running):
`mcopy -o -i sd.img@@1M HELLO.NAP ::/APPS/HELLO/`.
Remove the `#Z` line to get the desktop back.

On macOS, `hdiutil`/`diskutil` can build the same image without `mtools`;
[06](06-image-and-testing.md) shows that route and `tools/make-sd-image.sh`,
the repository's own test-image builder (macOS only; it needs a kernel built
from the checkout with `cargo xtask`, see the root `README.md`).

---

## 5. Porting an existing Atari C program

There is no m68k emulator yet: an Atari program runs on NewTOS only if it is
**recompiled from its C sources**. m68k binaries (`.PRG`, `.TOS`, `.TTP`,
`.ACC`) do not run.

### 5.1 The recipe

1. Take the program's C sources and its own build flags (Makefile).
   Keep flags that change C semantics (`-funsigned-char`, `-D…`), drop the
   m68k ones (`-m68000`, `-m68020-60`, `-mshort`, `-mcpu=…`).
2. Compile with `newtos-cc`, choosing the libraries it uses: GEM programs
   get gemlib automatically with `-nap`; add `-lcflib`, `-lm`, `-lwindom`,
   `-lldg` … as needed ([16](16-newtos-cc.md#libraries)):

   ```sh
   newtos-cc -nap -lcflib -funsigned-char -o MYAPP.NAP src/*.c
   ```
3. Fix what does not compile or link. Decide each time whether every 64-bit
   port would meet the problem (then it belongs in the SDK, once) or only this
   program (then a patch file for this program, 5.3). Most problems are LP64
   assumptions in the program: 32-bit `long` or pointers in TOS structures,
   big-endian word tricks, m68k assembler — the catalogue with symptoms and
   cures is [05-abi-pitfalls.md](05-abi-pitfalls.md). Sources written for
   Pure C also assume a 16-bit `int`; here `int` is 32 bits.
4. Convert the resource file: NewTOS loads a little-endian native format, not
   the classic `.RSC` ([04](04-porting-anatomy.md#resource-files-rsc)):

   ```sh
   python3 tools/mkrsc.py convert src/myapp.rsc MYAPP.RSC
   ```
   Put the converted file next to the `.NAP`.
5. Copy both into `APPS\<NAME>\` (step 4) and test. Then walk the checklist
   [08-porting-checklist.md](08-porting-checklist.md); the full anatomy of
   real ports is [04-porting-anatomy.md](04-porting-anatomy.md).

### 5.2 The worked examples: QED and TeraDesk

The repository holds the build scripts and patch files for QED (text editor)
and TeraDesk (desktop); the upstream sources are fetched separately, at the
commits the ports were made against. They are also the best **first test of
a new toolchain** (on Linux especially): if both build, clang, `ld.lld` (or
the fallback `rust-lld`) and the SDK libraries work together. Run the commands inside the NewTOS checkout
(step 1.3):

```sh
git clone https://github.com/freemint/qed.git      sdk/apps/qed
git -C sdk/apps/qed checkout f12055f
git clone https://github.com/freemint/teradesk.git sdk/apps/teradesk
git -C sdk/apps/teradesk checkout b5532b8

sdk/apps/build-qed.sh        # -> sdk/apps/qed/build/QED.NAP + qed.rsc
sdk/apps/build-teradesk.sh   # -> sdk/apps/teradesk/build/dist/ (DESKTOP.NAP, desktop.rsc, cicons.rsc, icons.rsc)
```

Measured in the clean environment: both build without error ("undefined
symbols in the final image (goal 0): 0"), about 5 s each, and convert their
resources; the converted `qed.rsc` and `desktop.rsc` are byte-identical to
the ones built on the author's machine. (The `.NAP` files differ in a few
bytes because the build paths end up in the binary.) After the TeraDesk
checkout, `sdk/run-host-tests.sh` also runs `aesglobal_fields`.

CoreMark is obtained the same way (`sdk/apps/coremark-sources.md5` gives the
clone commands); the other gitignored inputs (Doom, reference sources) are
listed in [15-host-checklist.md](15-host-checklist.md) §A.

### 5.3 How the patch files in `sdk/overlays/` work

The upstream tree under `sdk/apps/<program>/` is never edited. What only one
program needs is a **complete replacement file** (not a diff) with the same
name in `sdk/overlays/<program>/`, starting with a comment that says why:

* QED (`build-qed.sh`): for each source file, the copy in `sdk/overlays/qed/`
  is compiled instead of `src/<file>.c` if it exists (2 files today, both
  LP64).
* TeraDesk (`build-teradesk.sh`): a shadow tree of symlinks to the upstream
  files, with the overlay files layered on top, so `.h` patches reach every
  translation unit (7 files: 1 m68k assembler retarget, 6 LP64).
* zView keeps its patches in `sdk/zview/overlay/`.

For a port of your own with `newtos-cc`, do the same by hand: keep the
original sources untouched, put your changed copies in a directory of their
own, pass those `.c` files instead of the originals, and put that directory's
`-I` first so changed headers win. The list of existing patches and the
reason for each is in [04](04-porting-anatomy.md#the-rule-upstream-stays-unchanged-fixes-go-where-they-belong).

Two details that are easy to miss (TASK/SDK01 H, HypView):

* A patched `.c` that includes upstream headers by a **quoted relative path**
  (`#include "../diallib.h"`) no longer finds them once it lives in the patch
  directory: quoted includes are searched relative to the file being
  compiled first. Pass `-I` of the upstream file's **own** directory when
  compiling the patched copy, so the relative path resolves there.
* Many FreeMiNT-style trees list their sources in `SRCFILES`/`Makefile.objs`
  (`COBJS = …`), one per subdirectory. Use that manifest instead of a glob
  (it leaves out test programs and Pure C assembler modules such as
  HypView's hyp/gfx.s); `make` can print it:
  `printf 'include SRCFILES\nall: ; @echo $(COBJS)\n' | make -s -f - -C <dir>`.

`sdk/apps/build-hypview.sh` is a complete `newtos-cc` port script of this
shape (manifest per directory, patch substitution, `.rsc` conversion); the
HypView sources are fetched as `sdk/apps/hypview-sources.sha256` describes.

---

## 6. Troubleshooting

The messages below are real output from this machine.

**1. `newtos-cc: error: headers of 'c' not found -- the library is not built yet; run sdk/libc/build-libc.sh`**
(or `headers of 'gem'`, or `gemlib.a is not built`). Step 2 was skipped or
failed for that library; run the script the message names. After you change
anything under `sdk/`, rebuild the library yourself — `newtos-cc` does not
notice an out-of-date archive ([16](16-newtos-cc.md)).

**2. `ld.lld: error: undefined symbol: …`** (`rust-lld: error: …` with the
Rust fallback).
Either a library option is missing (`-lm` for `<math.h>` functions, `-lcflib`,
…), or the function is one of those the ported libraries do not provide:
the list is [07 §11](07-mintlib-limits.md#11-functions-that-do-not-link-measured-per-symbol-tasksdk01-a)
(libc) and [07 §4/§6](07-mintlib-limits.md) (libm). Symbols such as
`__extendsftf2`, `__addtf3`, `__lttf2`, `__muldc3` mean 128-bit `long
double` or complex arithmetic: `newtos-cc` links their runtime
(`sdk/compiler-rt/libclang_rt.builtins.a`) by itself — if you see them,
the archive is missing a step-2 build, or your own link line leaves it out
([05 B12](05-abi-pitfalls.md#b12-long-double-is-128-bit-and-the-compiler-runtime-is-compiler-rt));
`socket`, `gethostbyname` mean networking, which does not exist.

**3. `fatal error: 'math.h' file not found`** (or another library's header).
`newtos-cc` adds a library's include path only with its `-l` option —
pass `-lm` (also when compiling with `-c`).

**4. `error: incompatible pointer to integer conversion … [-Wint-conversion]`,
or a program that builds but crashes or shows garbage.**
A 32-bit assumption: a pointer stored in an `int`, a `long` in a structure
that TOS defines with 32-bit fields, a pointer split into two 16-bit words.
[05 Part B](05-abi-pitfalls.md#part-b--the-pitfalls) lists each case with
its symptom. `-Wint-conversion` is an error by default in current clang;
fix the code rather than silencing it — on LP64 the conversion really loses
the upper half of the pointer.
The most common runtime case is the **cookie jar**: code that walks `$5A0`
as an array of 32-bit `long` pairs reads garbage on LP64 and crashes. Use
`Getcookie` (`Ssystem`) instead, or read the jar exactly as
[05 B2](05-abi-pitfalls.md#b2-the-cookie-jar-at-5a0) describes.

**5. `error: LLVM tool 'ld.lld' not found (NEWTOS_TOOLCHAIN=auto).`**
(or `'llvm-ar'`, `'llvm-nm'`, …), followed by the install lines.
Neither LLVM's `lld`/`llvm` packages nor the Rust fallback were found
(§1.2). Install the packages the message names; if your LLVM lives
somewhere else, point `NEWTOS_LLVM_BIN` at its bin directory.
`sdk/bin/newtos-cc --toolchain` shows what each tool resolves to.

**Also common:** the card does not boot — it must be MBR + FAT16 ≤ 2 GB
(release `README.TXT`). Files called `._NAME.NAP` on the card — macOS
AppleDouble files; run `dot_clean -m` on the card. A `#Z` line that is
ignored — it is not within the first 300 bytes of `NEWDESK.INF`; put it
directly after `#R`. A double-clicked `.PRG` gives "This is an m68k Atari
program" — m68k binaries need recompiling (section 5).
