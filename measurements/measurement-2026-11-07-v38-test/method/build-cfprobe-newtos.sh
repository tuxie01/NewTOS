#!/usr/bin/env bash
# V38 acceptance: build cfprobe.c as a NewTOS .NAP, the link path of tosdump/build-vdprobe-newtos.sh
# (copied in shape; it must not be edited). Usage: build-cfprobe-newtos.sh <case 0|1> <out.NAP> <objdir>
set -uo pipefail
CASE=$1; OUT=$2; OBJ=$3
ROOT=/Volumes/Daten/Development/PiRusTOS
HERE="$ROOT/tosdump"; SDK="$ROOT/sdk"; LIBC="$SDK/libc"; MINT="$LIBC/mintlib/include"
SRC="$ROOT/measurements/measurement-2026-11-07-v38-test/method/cfprobe.c"
mkdir -p "$OBJ"
CLANG="${CLANG:-clang}"
RES="$($CLANG -print-resource-dir)/include"
SYSROOT="$(rustc --print sysroot 2>/dev/null)"
BIN="$SYSROOT/lib/rustlib/$(rustc -vV | sed -n 's/host: //p')/bin"
LLD="$BIN/rust-lld"
CFLAGS=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc -D__NEWTOS__ -std=c99 -Wall -Wextra -Werror -DCASE=$CASE)
"$CLANG" "${CFLAGS[@]}" -I"$HERE/core" -I"$MINT" -isystem "$RES" -c "$SRC" -o "$OBJ/cfprobe.o" || exit 1
"$CLANG" "${CFLAGS[@]}" -I"$HERE/core" -isystem "$RES" -c "$HERE/abi/newtos_svc.c" -o "$OBJ/newtos_svc.o" || exit 1
"$CLANG" -target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc -I"$SDK/include" -I"$MINT" -isystem "$RES" -c "$LIBC/crt0.S" -o "$OBJ/crt0.o" || exit 1
"$LLD" -flavor gnu -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 --build-id=none "$OBJ/crt0.o" "$OBJ/cfprobe.o" "$OBJ/newtos_svc.o" "$LIBC/libc.a" -o "$OUT" || exit 1
echo "LINK OK -> $OUT"
