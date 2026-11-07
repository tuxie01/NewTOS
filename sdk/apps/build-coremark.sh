#!/usr/bin/env bash
#
# build-coremark.sh -- build EEMBC CoreMark 1.0 as native NewTOS/AArch64
# console programs (`.NCP`, ADR-018) against the ported mintlib (libc.a).
#
# WHAT IS BUILT
#   sdk/apps/coremark/build/CORE.NCP     performance run, seeds 0,0,0x66
#                                        (the one that carries the score)
#   sdk/apps/coremark/build/COREVAL.NCP  validation run, seeds 0x3415,0x3415,0x66
#
# Both are required by CoreMark's run rules: "All validation must succeed for
# seeds 0,0,0x66 and 0x3415,0x3415,0x66, buffer size of 2000 bytes total."
# Two executables rather than one, because NewTOS cannot pass a program a
# command tail interactively (see the port's core_portme.h, delta 2) -- which
# is the exact case the CoreMark README covers with
#     make XCFLAGS="-DPERFORMANCE_RUN=1" ... / XCFLAGS="-DVALIDATION_RUN=1" ...
#
# LAYOUT (same as build-qed.sh / build-teradesk.sh)
#   sdk/apps/coremark/            vendored CoreMark source tree, UNTRACKED
#                                 (sdk/.gitignore `/apps/*/`) and never edited.
#                                 Provenance + pinned MD5s: sdk/apps/coremark-sources.md5
#   sdk/overlays/coremark/newtos/ OUR porting layer (core_portme.{c,h}), tracked.
#                                 It is a normal CoreMark port directory, the
#                                 same role as the vendored atari/ or linux/
#                                 ones, and is compiled from where it lies --
#                                 nothing is copied into the vendored tree.
#
# The run rules forbid changing any source outside core_portme*, so this script
# MD5-checks the five benchmark translation units and coremark.h against
# sdk/apps/coremark-sources.md5 and refuses to build on any mismatch. That file
# also records where the tree came from, so a lost checkout can be re-fetched
# and re-verified.
#
# WHY .NCP
#   CoreMark writes a text report and needs no GEM. `.NCP` gets the VT52 text
#   screen. The desktop does NOT wait after the program exits: like EmuTOS'
#   desktop it clears the text screen and redraws at once (measured on both,
#   measurements/measurement-2026-11-07-desk70/RUNINFO.TXT). The "[ Press any
#   key to return to the desktop ]" pause this comment used to promise lived
#   in the old built-in desktop's launch_native and was removed with it
#   (DESK43 U3), so the promise was wrong since then. So the port
#   waits itself: portable_fini() in sdk/overlays/coremark/newtos/core_portme.c
#   prints "Press any key to exit." and waits for a key after the timed region,
#   but only when the current directory is not the root. A desktop launch
#   from C:\COREMARK\ therefore waits; a run from C:\AUTO\ (which starts in the
#   root) does not, so measurement images that boot both programs from AUTO
#   still reach the desktop (TASK/DESK70). `.NTP` would open the desktop's
#   parameter dialog first, for a command tail CoreMark does not read.
#
# FLAGS
#   The NewTOS freestanding set shared with the other build-*.sh, plus -O2 and
#   -fomit-frame-pointer -- deliberately the same optimization level the Atari
#   port uses (sdk/apps/coremark/atari/core_portme.mak: -O2 -mcpu=68000
#   -fomit-frame-pointer), so a NewTOS number and an Atari number differ in the
#   machine and not in the aggressiveness of the build. The full string is
#   compiled in via -DFLAGS_STR and PRINTED BY THE BENCHMARK, as the reporting
#   rules require.
#
# Usage:  sdk/apps/build-coremark.sh
# Env:    ITERATIONS=<n>  fixed iteration count; 0 (default) lets CoreMark
#                         scale the run itself to the mandated 10-100 s.
set -uo pipefail

SDK="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"        # .../sdk
CM="$SDK/apps/coremark"                                       # vendored tree
PORT="$SDK/overlays/coremark/newtos"                          # our port dir
LIBC="$SDK/libc"
BUILD="$CM/build"
OBJ="$BUILD/obj"
MD5REF="$SDK/apps/coremark-sources.md5"
ITERATIONS="${ITERATIONS:-0}"
mkdir -p "$OBJ"

# The five benchmark translation units + the header the run rules freeze.
CORE_SRCS="core_list_join.c core_main.c core_matrix.c core_state.c core_util.c"
FROZEN="$CORE_SRCS coremark.h"

[ -d "$CM" ] || { echo "error: CoreMark source tree missing at $CM" >&2
                  echo "       re-fetch it per the header of $MD5REF." >&2; exit 1; }
[ -d "$PORT" ] || { echo "error: NewTOS port directory missing at $PORT" >&2; exit 1; }

# --- run-rule integrity check: nothing outside the port dir may have changed --
md5_of() {
    if command -v md5 >/dev/null 2>&1; then md5 -q "$1"
    else md5sum "$1" | awk '{print $1}'; fi
}
echo "CoreMark tree : $CM"
echo "port dir      : $PORT"
echo "checking the files the run rules freeze against $(basename "$MD5REF")…"
bad=0
for f in $FROZEN; do
    [ -f "$CM/$f" ] || { echo "  MISSING  $f" >&2; bad=1; continue; }
    want=$(awk -v f="$f" '/^#/ {next} $2 == f {print $1}' "$MD5REF")
    got=$(md5_of "$CM/$f")
    if [ -z "$want" ]; then
        echo "  NO REFERENCE MD5 for $f" >&2; bad=1
    elif [ "$want" != "$got" ]; then
        echo "  CHANGED  $f  ($got, expected $want)" >&2; bad=1
    else
        echo "  ok       $f  $got"
    fi
done
if [ "$bad" -ne 0 ]; then
    echo >&2
    echo "STOP: a CoreMark source the run rules forbid changing does not match." >&2
    echo "      A result built from it would not be reportable. Restore the file," >&2
    echo "      or -- if this is a deliberate upstream update -- re-verify against" >&2
    echo "      eembc/coremark and update $MD5REF together with the note in its" >&2
    echo "      header." >&2
    exit 1
fi
echo

# --- libc ---------------------------------------------------------------------
# What libc.a depends on is declared once in sdk/lib-stale.sh (TASK/T03), and
# lib_ensure reports what the library build did instead of hiding it (T02).
. "$SDK/lib-stale.sh"
lib_ensure libc || exit 1
MINT="$LIBC/mintlib/include"
[ -d "$MINT" ] || { echo "error: mintlib headers missing ($MINT); run build-libc.sh" >&2; exit 1; }

# --- toolchain ----------------------------------------------------------------
CLANG="${CLANG:-clang}"
command -v "$CLANG" >/dev/null 2>&1 || { echo "error: clang not found" >&2; exit 1; }
RES="$($CLANG -print-resource-dir)/include"
. "$SDK/toolchain.sh"   # LLVM tools: PATH / LLVM packages first, pinned Rust sysroot as fallback (SDK01 I, K)
LLD="${LLD:-$(newtos_tool ld.lld)}"
READOBJ="$(newtos_tool llvm-readobj)"
NM="$(newtos_tool llvm-nm)"
[ -x "$LLD" ] || { echo "error: missing LLVM tool: $LLD" >&2; exit 1; }

echo "clang : $($CLANG --version | head -1)"
echo "lld   : $LLD"
echo

# --- flags --------------------------------------------------------------------
# PORT_CFLAGS is what gets REPORTED (CoreMark reporting rule: "C - Compiler
# version and flags"). Keep it to the flags that actually shape the generated
# code; the -nostdinc/-I search path is build plumbing, not a codegen option.
PORT_CFLAGS="-target aarch64-unknown-none-elf -O2 -fomit-frame-pointer -ffreestanding -fPIE -fno-builtin"
FLAGS_STR="$PORT_CFLAGS"

CFLAGS=(${PORT_CFLAGS} -nostdlib -nostdinc
        -D_GNU_SOURCE -DFLAGS_STR="\"$FLAGS_STR\"" -DITERATIONS="$ITERATIONS")
# Include order: our port dir first (core_portme.h), then the CoreMark tree
# root (coremark.h), then mintlib, then clang's freestanding headers.
INCS=(-I"$PORT" -I"$CM" -I"$MINT" -isystem "$RES")

# --- one full build per run type ----------------------------------------------
# CoreMark run rule 4: "All source files must be compiled with the same flags."
# So each run type gets its own complete compile of all six translation units;
# nothing is shared between CORE.NCP and COREVAL.NCP.
build_one() {
    local tag="$1" define="$2" out="$3"
    local o u
    echo "── $tag ($define) ───────────────────────────────────────────"
    rm -rf "$OBJ/$tag"; mkdir -p "$OBJ/$tag"
    for c in $CORE_SRCS; do
        o="$OBJ/$tag/${c%.c}.o"
        "$CLANG" "${CFLAGS[@]}" -D"$define" "${INCS[@]}" -c "$CM/$c" -o "$o" \
            || { echo "error: compiling $c failed" >&2; return 1; }
    done
    "$CLANG" "${CFLAGS[@]}" -D"$define" "${INCS[@]}" \
        -c "$PORT/core_portme.c" -o "$OBJ/$tag/core_portme.o" \
        || { echo "error: compiling core_portme.c failed" >&2; return 1; }
    "$CLANG" "${CFLAGS[@]}" -I"$SDK/include" -I"$MINT" \
        -c "$LIBC/crt0.S" -o "$OBJ/$tag/crt0.o" \
        || { echo "error: assembling crt0.S failed" >&2; return 1; }

    rm -f "$out"
    newtos_lld -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
        --build-id=none --why-extract="$OBJ/$tag/why.txt" \
        "$OBJ/$tag/crt0.o" "$OBJ/$tag"/core_*.o "$LIBC/libc.a" \
        -o "$out" 2>"$OBJ/$tag/link.err"
    if [ $? -ne 0 ]; then
        echo "LINK FAILED -- residual undefined symbols (blockers):" >&2
        grep -oE "undefined symbol: [^ ]+" "$OBJ/$tag/link.err" | awk '{print $3}' | sort -u | sed 's/^/    /' >&2
        echo "See $OBJ/$tag/why.txt for the archive pull chains." >&2
        return 1
    fi
    u=$("$NM" -u "$out" 2>/dev/null | grep -c . || true)
    echo "LINK OK -> $out ($(wc -c < "$out" | tr -d ' ') bytes, undefined symbols: $u)"
    "$READOBJ" --file-headers "$out" | grep -iE 'Class:|Type:|Machine:|Entry:'
    echo
}

build_one perf "PERFORMANCE_RUN=1" "$BUILD/CORE.NCP"    || exit 1
build_one val  "VALIDATION_RUN=1"  "$BUILD/COREVAL.NCP" || exit 1

echo "reported compiler version : (compiled in from __clang_version__)"
echo "reported compiler flags   : $FLAGS_STR"
echo "ITERATIONS                : $ITERATIONS $( [ "$ITERATIONS" = 0 ] && echo '(CoreMark auto-scales the run to 10-100 s)' )"
echo
echo "Install with tools/make-sd-image.sh (installs C:\\COREMARK\\{CORE,COREVAL}.NCP)."
