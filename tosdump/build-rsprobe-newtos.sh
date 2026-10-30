#!/usr/bin/env bash
#
# build-rsprobe-newtos.sh -- build RSPROBE.NAP, the NewTOS/AArch64 half of the
# Rsconf / Iorec(0) probe tosdump/probes/rsprobe.c (TASK/XBIOS06). The m68k
# half is `make -C tosdump rsprobe-m68k`; same source, no backend of its own:
# the probe calls the mintlib bindings (osbind.h) as a ported TOS program does.
#
# The link path of the other NewTOS dumpers (build-dumpbios-newtos.sh): clang
# -target aarch64-unknown-none-elf, the libc-aware crt0, mintlib's libc.a and
# sdk/newtos.ld.
#
# Usage:  tosdump/build-rsprobe-newtos.sh [output.NAP]
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"        # .../tosdump
ROOT="$(cd "$HERE/.." && pwd)"
SDK="$ROOT/sdk"
LIBC="$SDK/libc"
BUILD="$HERE/build"
OBJ="$BUILD/obj-newtos-rsprobe"
OUT="${1:-$BUILD/RSPROBE.NAP}"
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
NM="$BIN/llvm-nm"
[ -x "$LLD" ] || { echo "error: missing rust-lld ($LLD)" >&2; exit 1; }

CFLAGS=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2
        -fPIE -fno-builtin -nostdinc -D__NEWTOS__
        -std=c99 -Wall -Wextra -Werror)

echo "clang : $($CLANG --version | head -1)"
echo "lld   : $LLD"
echo

rm -f "$OBJ"/*.o
"$CLANG" "${CFLAGS[@]}" -I"$MINT" -isystem "$RES" \
    -c "$HERE/probes/rsprobe.c" -o "$OBJ/rsprobe.o" || { echo "error: rsprobe.c failed to compile" >&2; exit 1; }
echo "  CC  probes/rsprobe.c"

"$CLANG" -target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 \
    -fPIE -fno-builtin -nostdinc -I"$SDK/include" -I"$MINT" -isystem "$RES" \
    -c "$LIBC/crt0.S" -o "$OBJ/crt0.o" || exit 1

rm -f "$OUT"
"$LLD" -flavor gnu -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
    --build-id=none "$OBJ/crt0.o" "$OBJ/rsprobe.o" "$LIBC/libc.a" \
    -o "$OUT" 2>"$BUILD/link-newtos-rsprobe.err" || {
    echo "LINK FAILED:" >&2; cat "$BUILD/link-newtos-rsprobe.err" >&2; exit 1; }

echo
echo "LINK OK -> $OUT ($(wc -c < "$OUT" | tr -d ' ') bytes)"
u=$("$NM" -u "$OUT" 2>/dev/null | grep -c . || true)
echo "undefined symbols in the final image (goal 0): $u"
[ "$u" -eq 0 ] || exit 1
