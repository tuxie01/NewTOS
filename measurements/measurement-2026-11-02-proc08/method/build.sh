#!/usr/bin/env bash
#
# build-ldg-demo.sh -- build LDG's demo library ex_lib as a native LDG module
# EX_LIB.NLD and the M1 acceptance client LDGTEST.NCP (TASK/LDG-M1,
# docs/analysis/ldg-after-arch03-2026-10-06.md section 7 steps S3/S4).
#
# A .NLD is built exactly like a program (G5 of the design): PIE, the SDK's
# crt0.S (PRGFLAGS note PF_STANDARD) and newtos.ld, linked against libldg.a,
# gemlib.a (ex_lib's main() calls appl_init/form_alert when it is started
# without OFFSETLDG), libm.a (sin, cos) and libc.a. The only difference is the
# file name: `.NLD` = native LDG library (ADR-018 addendum, docs/MEMORY.md),
# not startable from the desktop. The kernel loads it through LDG's own
# Pexec(3) and relocates its R_AARCH64_RELATIVE entries (PROC LibFunc[] holds
# absolute function pointers in .data; -z text keeps relocations out of .text).
#
# libshare (TASK/LDG-M3-B2): EX_LIB.NLD is a module, so it is linked with
# sdk/libshare/libshare.a FIRST in its archive group -- the link zView's
# codecs use (`-lshare -lldg`). LDG's own demo makefiles do not link it for
# ex_lib; that is a toolchain decision taken here, not a source change: with
# mintlib's malloc the module's exit() leaves its free list pointing into a
# hunk its own Pterm frees (measurements/measurement-2026-10-06-ldg-b2,
# EX_LIB: latent, it never mallocs again; POPUP.NLD: Bus Error). The link map
# is checked (sdk/libshare/libshare-link.sh): no mintlib allocator member may
# be pulled. LDGTEST.NCP is a client (a normal program) and keeps libc's
# malloc.
#
# Sources, all UNMODIFIED:
#   EX_LIB.NLD   <LDG>/demo/libs/ex_lib.c
#   LDGTEST.NCP  sdk/examples/ldgtest.c (ours) + <LDG>/demo/libs/exlibbnd.c
#                (the demo's binding, init_ldg_math) with the demo client
#                header <LDG>/demo/libs/ldg/ex_lib.h
# <LDG> = sdk/ldg/ldg-r133, extracted by build-ldg.sh from the pinned tarball.
#
# Usage:  sdk/ldg/build-ldg-demo.sh
# Output: sdk/ldg/build/EX_LIB.NLD, sdk/ldg/build/LDGTEST.NCP
#         (tools/make-sd-image.sh copies them to C:\LDG\EX_LIB.NLD and
#         C:\LDGTEST.NCP, plus \AUTO\LDGTEST.NCP on SD_LDGTEST_AUTO=1)
set -uo pipefail

SDK=/Volumes/Daten/Development/PiRusTOS/sdk      # .../sdk
LDGDIR="$SDK/ldg"
TOP="$LDGDIR/ldg-r133"
LIBC="$SDK/libc"
GEM="$SDK/gemlib"
LIBM="$SDK/libm"
GEMTOP="$GEM/gemlib-0.44.0"
LIBMTOP="$LIBM/openlibm-0.8.8"
BUILD=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/15767204-8d89-43e0-aae0-d8149a16a17d/scratchpad/proc08/probe/build
DOBJ="$BUILD/demo"
mkdir -p "$DOBJ"

# --- libraries: libc -> gemlib, libc -> libm, libc -> ldg -> libshare --------
. "$SDK/lib-stale.sh"
. "$SDK/libshare/libshare-link.sh"
lib_ensure gemlib   || exit 1
lib_ensure libm     || exit 1
lib_ensure ldg      || exit 1
lib_ensure libshare || exit 1
MINT="$LIBC/mintlib/include"
for d in "$MINT" "$GEMTOP" "$LIBMTOP" "$TOP"; do
  [ -d "$d" ] || { echo "error: $d missing (library build did not extract it)" >&2; exit 1; }
done

# --- toolchain ---------------------------------------------------------------
CLANG="${CLANG:-clang}"
command -v "$CLANG" >/dev/null 2>&1 || { echo "error: clang not found" >&2; exit 1; }
RES="$($CLANG -print-resource-dir)/include"
. "$SDK/toolchain.sh"   # LLVM tools: PATH / LLVM packages first, pinned Rust sysroot as fallback (SDK01 I, K)
LLD="${LLD:-$(newtos_tool ld.lld)}"
NM="$(newtos_tool llvm-nm)"
READOBJ="$(newtos_tool llvm-readobj)"
[ -x "$LLD" ] || { echo "error: missing LLVM tool: $LLD" >&2; exit 1; }

# --- flags (the app set of build-qed.sh) -------------------------------------
CFLAGS=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2
        -fPIE -fno-builtin -nostdinc -funsigned-char -D__NEWTOS__
        -Wno-macro-redefined)
INCS=(-I"$TOP/include" -I"$TOP/demo/libs/ldg" -I"$GEMTOP" -I"$MINT"
      -I"$LIBM/include" -I"$LIBMTOP/include" -isystem "$RES")

rm -f "$DOBJ"/*.o "$DOBJ"/*.err
cc() { # cc <src> <obj>
  if ! "$CLANG" "${CFLAGS[@]}" "${INCS[@]}" -c "$1" -o "$DOBJ/$2" 2>"$DOBJ/${2%.o}.err"; then
    echo "error: $1 did not compile:" >&2; sed 's/^/    /' "$DOBJ/${2%.o}.err" >&2; exit 1
  fi
  if [ -s "$DOBJ/${2%.o}.err" ]; then
    echo "  warnings in $(basename "$1"):"; sed 's/^/      /' "$DOBJ/${2%.o}.err"
  fi
}
cc "$TOP/demo/libs/ex_lib.c"   ex_lib.o
cc "$TOP/demo/libs/exlibbnd.c" exlibbnd.o
cc /private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/15767204-8d89-43e0-aae0-d8149a16a17d/scratchpad/proc08/probe/ldgloop.c   ldgtest.o
"$CLANG" "${CFLAGS[@]}" -I"$SDK/include" -I"$MINT" -c "$LIBC/crt0.S" -o "$DOBJ/crt0.o" || exit 1

link() { # link <module|client> <out> <objs...>
  local kind=$1 out=$2; shift 2
  local share=() map="$BUILD/link-$(basename "$out").map"
  # A module links libshare.a first in the group (see the header).
  [ "$kind" = module ] && share=("$SDK/libshare/libshare.a")
  if ! newtos_lld -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
      --build-id=none "$DOBJ/crt0.o" "$@" \
      --start-group ${share[@]+"${share[@]}"} "$LDGDIR/libldg.a" "$GEM/gemlib.a" "$LIBM/libm.a" "$LIBC/libc.a" --end-group \
      -Map="$map" -o "$out" 2>"$BUILD/link-$(basename "$out").err"; then
    echo "error: link of $(basename "$out") failed:" >&2
    sed 's/^/    /' "$BUILD/link-$(basename "$out").err" >&2; exit 1
  fi
  echo "LINK OK -> $out ($(wc -c < "$out" | tr -d ' ') bytes)"
  "$READOBJ" --file-headers "$out" | grep -E '^ *Type:'
  echo "  undefined symbols (goal 0): $(ls_undefined_count "$out")"
  if [ "$kind" = module ]; then
    ls_check_module_map "$map" "$(basename "$out")" || { rm -f "$out"; exit 1; }
  fi
}
link client "$BUILD/LDGLOOP.NCP" "$DOBJ/ldgtest.o" "$DOBJ/exlibbnd.o"
