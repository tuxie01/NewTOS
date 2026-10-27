#!/usr/bin/env bash
#
# build-dumpbios-newtos.sh -- build DUMPBIOS.NAP, the NewTOS/AArch64 half of the
# TOS reference dumper (docs/TOS-REFERENCE-DUMPER.md, task T4).
#
# Same portable core as the TT build (core/*.c), same case table, only
# abi/newtos_svc.c instead of abi/m68k_trap.c. It follows the proven client
# link path of the other NewTOS apps (sdk/apps/build-qed.sh): clang
# -target aarch64-unknown-none-elf, the libc-aware crt0, mintlib's libc.a, and
# sdk/newtos.ld. No gemlib: the dumper issues the BIOS and XBIOS traps itself
# through its own thin backend, so no library sits between the measurement
# and the kernel.
#
# A .NAP like the other dumpers, so the desktop's #Z autorun starts it the
# same way; unlike them it makes no AES or VDI call (core/dumpbios.c).
#
# Usage:  tosdump/build-dumpbios-newtos.sh [output.NAP]
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"        # .../tosdump
ROOT="$(cd "$HERE/.." && pwd)"
SDK="$ROOT/sdk"
LIBC="$SDK/libc"
BUILD="$HERE/build"
OBJ="$BUILD/obj-newtos-bios"
OUT="${1:-$BUILD/DUMPBIOS.NAP}"
mkdir -p "$OBJ"

# --- libc ---------------------------------------------------------------------
if [ ! -f "$LIBC/libc.a" ]; then
  echo "libc.a absent -- running build-libc.sh first…"
  "$LIBC/build-libc.sh" >/dev/null 2>&1 || { echo "error: build-libc.sh failed" >&2; exit 1; }
fi
MINT="$LIBC/mintlib/include"
[ -d "$MINT" ] || { echo "error: mintlib headers missing ($MINT); run build-libc.sh" >&2; exit 1; }

# --- toolchain ----------------------------------------------------------------
CLANG="${CLANG:-clang}"
command -v "$CLANG" >/dev/null 2>&1 || { echo "error: clang not found" >&2; exit 1; }
RES="$($CLANG -print-resource-dir)/include"
SYSROOT="$(rustc --print sysroot 2>/dev/null)"
BIN="$SYSROOT/lib/rustlib/$(rustc -vV | sed -n 's/host: //p')/bin"
LLD="${LLD:-$BIN/rust-lld}"
READOBJ="$BIN/llvm-readobj"
NM="$BIN/llvm-nm"
[ -x "$LLD" ] || { echo "error: missing rust-lld ($LLD)" >&2; exit 1; }

# The header's build id field is exactly 4 bytes; git widens --short when 4
# hex digits are not unique, so cut it to what actually gets stored.
BUILD_ID="$(cd "$ROOT" && git rev-parse HEAD 2>/dev/null | cut -c1-4)"
[ -n "$BUILD_ID" ] || BUILD_ID='----'

# AArch64 is little-endian, so the record writer swaps on write. This is the
# only compile-time difference from the TT build (spec section 2.1).
CFLAGS=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2
        -fPIE -fno-builtin -nostdinc -D__NEWTOS__
        -std=c99 -Wall -Wextra -Werror
        -DTD_BIG_ENDIAN_TARGET=0 -DTD_BUILD_ID="\"$BUILD_ID\"")
INCS=(-I"$HERE/core" -I"$MINT" -isystem "$RES")

echo "clang : $($CLANG --version | head -1)"
echo "lld   : $LLD"
echo "build : $BUILD_ID"
echo

SRC=(core/record.c core/cases.c core/dump.c core/log.c core/runner_call.c
     core/dumpbios.c abi/newtos_svc.c)

rm -f "$OBJ"/*.o
for s in "${SRC[@]}"; do
    o="$OBJ/$(echo "${s%.c}" | tr '/' '__').o"
    if ! "$CLANG" "${CFLAGS[@]}" "${INCS[@]}" -c "$HERE/$s" -o "$o"; then
        echo "error: $s failed to compile" >&2
        exit 1
    fi
    echo "  CC  $s"
done

"$CLANG" -target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 \
    -fPIE -fno-builtin -nostdinc -I"$SDK/include" -I"$MINT" -isystem "$RES" \
    -c "$LIBC/crt0.S" -o "$BUILD/crt0-newtos-bios.o" || exit 1

rm -f "$OUT"
"$LLD" -flavor gnu -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
    --build-id=none --why-extract="$BUILD/why-newtos-bios.txt" \
    "$BUILD/crt0-newtos-bios.o" "$OBJ"/*.o "$LIBC/libc.a" \
    -o "$OUT" 2>"$BUILD/link-newtos-bios.err"
lr=$?

if [ $lr -ne 0 ]; then
    echo
    echo "LINK FAILED -- residual undefined symbols:"
    grep -oE "undefined symbol: [^ ]+" "$BUILD/link-newtos-bios.err" \
        | awk '{print $3}' | sort -u | sed 's/^/    /'
    exit 1
fi

echo
echo "LINK OK -> $OUT ($(wc -c < "$OUT" | tr -d ' ') bytes)"
"$READOBJ" --file-headers "$OUT" | grep -iE 'Class:|DataEncoding:|Type:|Machine:|Entry:'
u=$("$NM" -u "$OUT" 2>/dev/null | grep -c . || true)
echo "undefined symbols in the final image (goal 0): $u"
[ "$u" -eq 0 ] || exit 1
