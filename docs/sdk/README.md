# NewTOS SDK — developer documentation

**New here? Start with [00-quickstart.md](00-quickstart.md)** — from a
fresh Mac (Linux: untested) to your own program on a Raspberry Pi 4 or in
QEMU, without building the kernel.

This is the entry point for anyone who wants to build a C program for NewTOS
or port an existing TOS/GEM C application to it. Every statement in these
pages is taken from a file in this repository (cited as `path:line`) or from a
command that was actually run while writing them (the command and its output
are quoted). Where something could not be verified it is marked
**unverified**.

Repository revision the documentation was written against: the chapters were
written one after the other between 2026-09-17 (first version, revision
`49b2c2ce29458119ee064eb993efe1efff66bd0e`, branch `desk-rebuild`) and
2026-10-20; the audit and correction pass of 2026-10-20 (TASK/SDK01 package B)
checked them against `main` at `a900ff0b`. A chapter's own header gives the
date of its measurements; numbers such as file sizes and compile counts move
with every library rebuild.

**What this SDK is today, and what is missing.** It is a set of build scripts
that work from a checkout of this repository on **macOS**. Since TASK/SDK01
package C there is a quick start from nothing — installing the tools,
building the libraries, obtaining the application sources, running on the
release card — [00-quickstart.md](00-quickstart.md), with Linux commands
that are **untested**. There is still **no** install outside the repository
(headers live in the extracted build trees) and **no** Linux way to run
`tools/make-sd-image.sh` (macOS-only; TASK/SDK01 package G); Windows is not
covered. Since TASK/SDK01 package D there **is** a compiler wrapper,
`sdk/bin/newtos-cc` (target flags, include order, `crt0`, linker script and
archives in one command), a Makefile include `sdk/mk/newtos.mk` and two
templates in `sdk/templates/` — [16-newtos-cc.md](16-newtos-cc.md); it runs
from the repository's `sdk/` directory and needs the libraries built once.

`path:line` citations into scripts and sources were exact on the day each
chapter was written; the scripts have changed since and the citations were
**not** all re-checked on 2026-10-20 (only the ones in the statements corrected
by that pass). If a line number does not match, search by the quoted text.

## Pages

| Page | What it answers |
|------|-----------------|
| [00-quickstart.md](00-quickstart.md) | From a fresh machine to your own program: tools, building the libraries, templates and `newtos-cc`, running on the v0.6 release card or in QEMU, fetching QED/TeraDesk, troubleshooting |
| [01-overview.md](01-overview.md) | What the toolchain is, what it produces (`.NCP` / `.NAP` / `.NTP`), what it cannot do |
| [02-prerequisites.md](02-prerequisites.md) | Which compiler, linker and host tools the scripts use, and where they look for them |
| [03-first-programs.md](03-first-programs.md) | Hello World (`.NCP`), a mintlib-linked program, and a minimal GEM application (`.NAP`) — built and run for this document |
| [04-porting-anatomy.md](04-porting-anatomy.md) | Where a fix belongs (general: once in the SDK; one program: a per-program patch file), the list of patches that exist and why, how libc/gemlib/cflib are built, how QED, TeraDesk and CoreMark are built, resource files |
| [05-abi-pitfalls.md](05-abi-pitfalls.md) | The trap ABI, and every LP64 / address-space / semantic trap that has bitten a port so far — with symptom and countermeasure |
| [06-image-and-testing.md](06-image-and-testing.md) | Getting a binary onto the SD image, `AUTO`, `NEWDESK.INF` `#Z`, running in QEMU and on a real Pi 4, screendumps, serial log |
| [07-mintlib-limits.md](07-mintlib-limits.md) | What the ported mintlib does not provide, derived from `sdk/libc/expected-failures.txt` and the test programs; `setjmp` and the libm (OpenLibm) since T07 |
| [08-porting-checklist.md](08-porting-checklist.md) | The short, tickable list for porting an application |
| [09-port-material.md](09-port-material.md) | What source material we hold, in which version, under which licence — and the age trap a frozen third-party library is |
| [10-ldg-modules.md](10-ldg-modules.md) | LDG run-time libraries as native `.NLD` modules: libldg, building a module, loading it from a client, what is not there (TSR, `ldg_callback`, search path) — TASK/LDG-M1; libshare 0.1.1 as the allocator of every module (licence unclear, §6) — TASK/LDG-M3-B2 |
| [11-codec-libraries.md](11-codec-libraries.md) | zlib, libpng, libjpeg (IJG), giflib, libexif as static NewTOS libraries for zView's codecs: versions, pins, how to link, which codec needs what, the LIBTEST.NCP round-trip test — TASK/LDG-M2 |
| [12-windom.md](12-windom.md) | WinDom 1.21.3 (+ zView's patch) as `libwindom1.a`: pin, flags, the 10 LP64/endianness overlays, the LP64 sweep, LDG's WinDom demo client, open problems (mouse clicks, module heap) — TASK/LDG-M3 |
| [13-zview.md](13-zview.md) | zView (LDG version) as `ZVIEW.NAP` + ten codec modules `CODECS\*.NLD`, without PDF: pin, build rules, the 13 overlays (endianness incl. the browser font, two m68k-asm retargets, `.NLD`, PDF omission), samples, what is verified, open kernel blocker K1 (`graf_handle` workstation not open) — TASK/ZVIEW-M4 |
| [14-present.md](14-present.md) | `PRESENT.NAP`, our own full-screen slide show: slide file format (`SLIDES.TXT`), keys, how it draws (system font read back via `vro_cpyfm`, Scale2x/3x, one RGB565 frame blit, libpng pictures), build + install — TASK/PRESENT01 |
| [16-newtos-cc.md](16-newtos-cc.md) | The compiler wrapper `sdk/bin/newtos-cc`, the Makefile include `sdk/mk/newtos.mk`, the templates `sdk/templates/{gem-hello,console-hello}`: options, libraries, PRGFLAGS/protection, byte identity with the scripts, what is untested — TASK/SDK01 D |
| [15-host-checklist.md](15-host-checklist.md) | Checklist for a second Mac: which gitignored files to copy, which tools and versions, proof builds — written to be handed to an agent for verification |
| [17-linux-test.md](17-linux-test.md) | The Linux test of the Rust-free C path: packages, `sdk/host-report.sh` (builds libraries, host tests, two programs, QED, TeraDesk into one report), expected results against the Mac reference, clang 14 vs. a newer LLVM, what to send back — TASK/SDK01 K |

## Thirty-second version

```sh
# your own program, one command (16): libc always, -l for the rest
sdk/bin/newtos-cc -nap -o MYAPP.NAP main.c          # GEM app (gemlib + libc)
sdk/bin/newtos-cc -o HELLO.NCP hello.c              # console program (libc)
make -C sdk/templates/gem-hello                     # template with sdk/mk/newtos.mk

# freestanding console program (no libc): sdk/build.sh
sdk/build.sh sdk/examples/hello.c /path/to/HELLO2.NCP

# libc-, gemlib- and cflib-linked programs: the per-app scripts
sdk/libc/build-libc.sh   [client.c] [out.NCP]     # builds libc.a, links client
sdk/gemlib/build-gemlib.sh [client.c] [out.NCP]   # builds gemlib.a, links client
sdk/cflib/build-cflib.sh   [client.c] [out.NCP]   # builds cflib.a, links client
sdk/libm/build-libm.sh     [client.c] [out.NCP]   # builds libm.a (OpenLibm), links client + libc
sdk/ldg/build-ldg.sh                               # libldg.a (LDG r133, tarball + overlay/)
sdk/libshare/build-libshare.sh                     # libshare.a (libshare 0.1.1, every .NLD links it; 10 §6)
sdk/ldg/build-ldg-demo.sh                          # EX_LIB.NLD (LDG demo module) + LDGTEST.NCP
sdk/windom/build-windom.sh                         # libwindom1.a (WinDom 1.21.3 + zView patch, 12)
sdk/ldg/build-ldg-client.sh                        # LDGDEMO.NAP (LDG's WinDom demo client) + POPUP.NLD
sdk/zlib/build-zlib.sh  sdk/libpng/build-libpng.sh  sdk/libjpeg/build-libjpeg.sh
sdk/giflib/build-giflib.sh  sdk/libexif/build-libexif.sh   # codec libraries (11)
sdk/examples/libtest/build-libtest.sh              # LIBTEST.NCP (their round-trip test)
sdk/apps/build-qed.sh                              # QED.NAP
sdk/apps/build-teradesk.sh                         # DESKTOP.NAP + dist/
sdk/apps/build-coremark.sh                         # CORE.NCP, COREVAL.NCP
sdk/apps/build-hypview.sh                          # HYP_VIEW.NAP + hyp_view.rsc (newtos-cc, SDK01 H)

# put everything on an SD image and boot it in QEMU; the script stops with an
# error if a program it installs is not built (order: 06-image-and-testing.md)
tools/make-sd-image.sh /tmp/sd.img 64
cargo xtask run --board rpi4 --sd /tmp/sd.img       # add --window to see the screen
```

The rule that governs every port: **upstream sources are kept unchanged**
(as archives, or as untracked checkouts under `sdk/apps/`) and nothing is
edited inside them. A problem that every 64-bit (LP64) port would meet is
fixed **once in the SDK** — the libc overlays `sdk/libc/overlay/`, the
AArch64 files `sdk/libc/newtos-arch/`, headers, `crt0`, and the library
overlays `sdk/{gemlib,cflib,ldg,libshare,libjpeg,libexif,windom}/overlay/` (OpenLibm needs none: `sdk/libm/include/` holds header adapters, `build-libm.sh` supports an overlay/ directory that does not exist).
What **only one program** needs is a **per-program patch file** in
`sdk/overlays/<program>/` (zView: `sdk/zview/overlay/`), layered over the
upstream tree at build time and carrying a header that says why. Such patches
exist for QED (2: LP64), TeraDesk (7: 1 m68k asm, 6 LP64), zView (13: m68k
asm, endianness, LP64, the `.NLD` suffix, PDF left out) and HypView (3: m68k
asm, big-endian file format, LP64; TASK/SDK01 H, built with `newtos-cc`); CoreMark's
`sdk/overlays/coremark/` is its own port layer, not a patch. The full list
with reasons is in [04-porting-anatomy.md](04-porting-anatomy.md#the-rule-upstream-stays-unchanged-fixes-go-where-they-belong).

## Where the scripts and sources live

```
sdk/
├── build.sh              freestanding .NCP builder (no libc)
├── crt0.S                startup for freestanding programs
├── newtos.ld             linker script for every native program
├── include/newtos.h      clean-room trap helpers (Cconws/Cconin/Pterm/...)
├── lib-stale.sh          library dependency chain + staleness test
├── toolchain.sh          finds ld.lld/llvm-* (LLVM packages first, pinned Rust nightly as fallback), whatever the cwd
├── libc/                 mintlib-0.60.1 port (tarball + overlay/ + newtos-arch/, 15 files there)
├── gemlib/               gemlib-0.44.0 port (tarball + overlay/)
├── cflib/                cflib port (tarball pinned at commit ce57817 + overlay/)
├── libm/                 OpenLibm 0.8.8 port (tarball + include/ math.h adapters, no overlay/), T07
├── ldg/                  LDG r133 devel library port (tarball + overlay/), .NLD demo + client, LDG-M1
├── zlib/ libpng/ libjpeg/ giflib/ libexif/   codec libraries for zView (release tarball + config overlay), LDG-M2
├── vendor-lib.sh         shared build steps of those five
├── libshare/ windom/ zview/   libshare 0.1.1, WinDom 1.21.3 (zip + patch + overlay/), zView (z-tools tarball + overlay/, per-program patches)
├── overlays/{qed,teradesk,coremark,hypview}/   per-program patch files (zView's are in zview/overlay/)
├── apps/                 build scripts + vendored app trees (untracked)
└── examples/             example programs (examples/libtest/: the codec-library test, LDG-M2)
tools/make-sd-image.sh    builds the FAT16 SD image, installs the binaries
tools/mkrsc.py            converts classic .RSC files to the native format
```
