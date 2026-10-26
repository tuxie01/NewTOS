# 17 — Linux test: does the SDK build on your PC?

The SDK has only ever been built on macOS (Apple Silicon, Apple clang 21).
Since TASK/SDK01 K it no longer needs Rust: it links with the `ld.lld` and
`llvm-ar` of your distribution's LLVM packages. Nobody has run that on Linux
yet. This page is the test, step by step, and what to send back.

Time: about 10 minutes, most of it `apt install` and the first build.
Nothing here needs a Raspberry Pi; step 6 is an optional extra on the Pi.

What the test runs: one script, `sdk/host-report.sh`. It installs nothing,
clones nothing, needs no `sudo`, writes only inside the checkout (into
target/host-report/, which git ignores) and collects every result into one
file you attach to the GitHub issue.

## 1. Packages

Debian 12 / LMDE 6 / Ubuntu:

```sh
sudo apt install git clang lld llvm make patch xz-utils python3 build-essential file
```

Fedora:

```sh
sudo dnf install git clang lld llvm make patch xz python3 gcc file
```

What each is for: `clang` compiles for AArch64 (it is a cross compiler by
itself, no `aarch64-linux-gnu-gcc` needed), `lld` gives `ld.lld`, `llvm` gives
`llvm-ar`, `llvm-nm`, `llvm-readobj`, `llvm-objdump`; `build-essential` (`gcc`)
gives the host `cc` for the host tests; `python3` converts the resource files
and checks `long double` results (**3.11 or newer**: Debian 12 has 3.11.2,
fine). No Rust, no `rustup`.

Quick check before you go on:

```sh
clang --version | head -1
clang -print-targets | grep -i aarch64     # expect: aarch64 - AArch64 (little endian)
ld.lld --version
llvm-ar --version | grep -i version
python3 --version
```

If `ld.lld` or `llvm-ar` is "command not found" although the packages are
installed: Debian sometimes installs them only as `ld.lld-14`,
`llvm-ar-14`. The SDK also looks in `/usr/lib/llvm-*/bin` and next to the
real `clang`, so the build may still find them — step 3 shows which one it
took.

## 2. The sources

```sh
git clone https://github.com/tuxie01/NewTOS.git NewTOS
cd NewTOS
git clone https://github.com/freemint/qed.git      sdk/apps/qed
git -C sdk/apps/qed checkout f12055f
git clone https://github.com/freemint/teradesk.git sdk/apps/teradesk
git -C sdk/apps/teradesk checkout b5532b8
```

The path must not contain spaces. QED (text editor) and TeraDesk (desktop)
are the real-world part of the test: if they build, compiler, linker and all
SDK libraries work together. Without them the script skips step 6 of the
report and says so.

## 3. Run the test

```sh
sdk/host-report.sh
```

It prints the report while it runs and ends with

```
report:  …/NewTOS/target/host-report/report.txt
archive: …/NewTOS/target/host-report.tar.gz
```

Run it with your normal environment. Do **not** set `NEWTOS_TOOLCHAIN`; the
default (`auto`) is what we want to see working.

## 4. What the report should say

Reference: the same script on the author's Mac, fresh checkout, 2026-10-22
(about 2 minutes). Your numbers will differ in the places marked.

| Section | Expected on Linux | Reference (Mac) |
|---|---|---|
| 1 host facts | your versions; `ld.lld` and `llvm-ar` **not** "(missing)" | `ld.lld (missing)` — the Mac used the Rust fallback |
| 2 tool resolution | `linker` = your `ld.lld` (e.g. `/usr/bin/ld.lld`) **without** `-flavor gnu`; `llvm-ar` etc. from the same LLVM; `mode NEWTOS_TOOLCHAIN=auto`; no path containing `rustup` | `rust-lld -flavor gnu` from `~/.rustup` |
| 3 libs-cflib | `rc=0`; log ends with `LINK OK -> …/cftest.NCP` | rc=0, 33 s |
| 3 libs-libm | `rc=0`; `undefined symbols in the final image (goal 0): 0` | rc=0, 6 s |
| 3 libs-compiler-rt | `rc=0`; `compiled OK=151 FAILED=0` (the count may differ with another clang); `self-contained: no external references` | rc=0, 5 s |
| 4 host-tests | `rc=0`, `0 failed`. On an x86_64 PC **5 skips are normal**: `setjmp-host`, `rt-host`, `ldbl-host` (they run AArch64 code natively) plus `custom-font-host`, `libshare-host` (their inputs are not built) | `24 passed, 0 failed, 2 skipped` |
| 5 hello, mathtst | `rc=0`, both `ELF 64-bit LSB pie executable, ARM aarch64` | rc=0 |
| 6 qed, teradesk | `rc=0`, `undefined symbols in the final image (goal 0): 0` | rc=0, 3 s each |
| 7 checksums | the four **`.rsc`** lines must be identical to the reference below; the `.NCP`/`.NAP` lines will differ (other linker version, build paths inside the binary) | see below |

Reference checksums of the converted resource files (produced by
`tools/mkrsc.py` from the pinned sources, so they must match on every host):

```
30ac6bf24a28ce2c95fab6a7c663bf29f40de9ad9ee1dd6b80ec36c146264a79  sdk/apps/qed/build/qed.rsc
ee7de7f37985aba9856ccbe01fb852bec47e6ff9d200644ea3aced15231f6241  sdk/apps/teradesk/build/dist/cicons.rsc
409f69a09bf1d1d869bdf9e712510413917cc2355b05b3ba937e60366b0b0e08  sdk/apps/teradesk/build/dist/desktop.rsc
8d4bc8a2b8ca8e6890f318677a0445dd26b476b4cd0bf6ee4fc103a0f4f22e75  sdk/apps/teradesk/build/dist/icons.rsc
```

A step with `rc` other than 0 is a finding, not your mistake: the first
failing step is the interesting one, later ones usually fail because of it.
You do not need to debug it — the report and its logs are enough for us.

## 5. If clang 14 fails

Debian 12 / LMDE 6 ship clang 14; the SDK was built with clang 21 so far.
If section 3 fails with compiler errors (unknown option, unknown builtin,
internal compiler error), please **send that report first**, then try a
newer LLVM from <https://apt.llvm.org> and run again with it first on the
`PATH`:

```sh
wget https://apt.llvm.org/llvm.sh && chmod +x llvm.sh && sudo ./llvm.sh 19
sudo apt install lld-19
mv target/host-report.tar.gz ~/host-report-clang14.tar.gz   # keep the first report
PATH=/usr/lib/llvm-19/bin:$PATH sdk/host-report.sh
```

Section 1 of the new report must then show clang 19 and the `ld.lld` from
`/usr/lib/llvm-19/bin`. Send both reports; together they tell us the minimum
clang version.

## 6. Optional: on the Pi

If you have a NewTOS 0.6.1 card: copy `sdk/apps/qed/build/QED.NAP` and
`sdk/apps/qed/build/qed.rsc` over the ones in `C:\Apps\QED\` (keep the
originals somewhere), and target/host-report/programs/HELLO.NCP to `C:\`.
Boot, start both from the desktop. HELLO prints `Hello from NewTOS, GEMDOS
…` and waits for a key; QED opens its window and you can type. Tell us
whether both did.

## 7. What to send back

Attach target/host-report.tar.gz to the GitHub issue
(<https://github.com/tuxie01/NewTOS/issues/4>) and add three lines:

1. distribution and version, CPU (`x86_64` or `aarch64`);
2. whether the packages of step 1 were all you installed (or what else you
   needed);
3. anything that confused you in this page.

The archive contains `report.txt` and the full log of every step. It holds
your local paths (e.g. `/home/<name>/NewTOS/...`) and tool versions, nothing
else; look at `report.txt` before you post it if you want to be sure.
