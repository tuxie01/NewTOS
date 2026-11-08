#!/usr/bin/env bash
# Build LNPROBE.NAP exactly like tosdump/build-vdprobe-newtos.sh, from lnprobe.c next to this script.
# usage: build-lnprobe.sh <tosdump dir of a tree> <output.NAP> <objdir>
set -uo pipefail
TD=$1; OUT=$2; OBJ=$3; HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$TD/.." && pwd)"; SDK="$ROOT/sdk"; LIBC="$SDK/libc"; MINT="$LIBC/mintlib/include"
mkdir -p "$OBJ"
CLANG=clang; RES="$($CLANG -print-resource-dir)/include"
SYSROOT="$(rustc --print sysroot)"; BIN="$SYSROOT/lib/rustlib/$(rustc -vV | sed -n 's/host: //p')/bin"; LLD="$BIN/rust-lld"
CFLAGS=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc -D__NEWTOS__ -std=c99 -Wall -Wextra)
"$CLANG" "${CFLAGS[@]}" -I"$TD/core" -I"$MINT" -isystem "$RES" -c "$HERE/${SRC:-lnprobe.c}" -o "$OBJ/lnprobe.o" || exit 1
"$CLANG" "${CFLAGS[@]}" -I"$TD/core" -isystem "$RES" -c "$TD/abi/newtos_svc.c" -o "$OBJ/newtos_svc.o" || exit 1
"$CLANG" -target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc -I"$SDK/include" -I"$MINT" -isystem "$RES" -c "$LIBC/crt0.S" -o "$OBJ/crt0.o" || exit 1
"$LLD" -flavor gnu -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 --build-id=none \
  "$OBJ/crt0.o" "$OBJ/lnprobe.o" "$OBJ/newtos_svc.o" "$LIBC/libc.a" -o "$OUT" || exit 1
echo "built $OUT"
