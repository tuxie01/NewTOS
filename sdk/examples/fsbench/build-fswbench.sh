#!/usr/bin/env bash
#
# build-fswbench.sh -- build FSWBENCH.NCP (sdk/examples/fswbench.c), the
# layer-by-layer SD write bench of TASK/HW08, against sdk/libc/libc.a
# (mintlib; L3 of the bench is stdio). Same flag set and link line as
# sdk/examples/libtest/build-libtest.sh, minus the codec libraries.
#
# Usage:  sdk/examples/fsbench/build-fswbench.sh [OUT.NCP]
#         default OUT: sdk/examples/fsbench/build/FSWBENCH.NCP
#
# FSWB_FLAGS=-DFSWB_NOKEY builds the variant for headless QEMU runs (drive D:,
# no key waits).
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SDK="$(cd "$HERE/../.." && pwd)"
OUT="${1:-$HERE/build/FSWBENCH.NCP}"
BUILD="$(dirname "$OUT")"
mkdir -p "$BUILD"
OBJ="$(mktemp -d "${TMPDIR:-/private/tmp}/fswbench.XXXXXX")"
trap 'rm -rf "$OBJ"' EXIT

. "$SDK/lib-stale.sh"
lib_ensure libc || exit 1

MINT="$SDK/libc/mintlib/include"
CLANG="${CLANG:-clang}"
RES="$($CLANG -print-resource-dir)/include"
. "$SDK/toolchain.sh"   # LLVM tools: PATH / LLVM packages first, pinned Rust sysroot as fallback (SDK01 I, K)
LLD="${LLD:-$(newtos_tool ld.lld)}"
NM="$(newtos_tool llvm-nm)"

CFLAGS=(${FSWB_FLAGS:-} -target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2
        -fPIE -fno-builtin -nostdinc -funsigned-char -D__NEWTOS__
        -Wno-macro-redefined -Wall -Wextra)

"$CLANG" "${CFLAGS[@]}" -I"$HERE" -I"$MINT" -isystem "$RES" \
    -c "$SDK/examples/fswbench.c" -o "$OBJ/fswbench.o"
"$CLANG" -target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc \
    -I"$SDK/include" -I"$MINT" -c "$SDK/libc/crt0.S" -o "$OBJ/crt0.o"
newtos_lld -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
    --build-id=none "$OBJ/crt0.o" "$OBJ/fswbench.o" "$SDK/libc/libc.a" -o "$OUT"

echo "FSWBENCH.NCP -> $OUT ($(wc -c < "$OUT" | tr -d ' ') bytes)"
echo "  undefined symbols (goal 0): $("$NM" -u "$OUT" 2>/dev/null | grep -c . || true)"
