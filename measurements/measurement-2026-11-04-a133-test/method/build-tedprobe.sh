#!/usr/bin/env bash
# build-tedprobe.sh <out.NAP> <out.PRG> -- method/tedprobe.c for both sides, with
# the link recipes of measurements/measurement-2026-11-04-a129-measure/method/
# build-fsprobe.sh (NewTOS: clang aarch64, crt0, sdk/newtos.ld,
# tosdump/abi/newtos_svc.c, mintlib libc.a) and build-fsprobe-m68k.sh (m68k:
# ORACLE_CROSS_MINT gcc, tosdump/abi/m68k_trap.c); only the source differs.
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd); ROOT=$(cd "$HERE/../../.." && pwd)
TD=$ROOT/tosdump; SDK=$ROOT/sdk; LIBC=$SDK/libc; MINT=$LIBC/mintlib/include
NAP=$1; PRG=$2; OBJ=$(dirname "$NAP")/obj-tedprobe; mkdir -p "$OBJ"
CLANG=${CLANG:-clang}; RES="$($CLANG -print-resource-dir)/include"
SYSROOT="$(rustc --print sysroot)"; BIN="$SYSROOT/lib/rustlib/$(rustc -vV | sed -n 's/host: //p')/bin"
CFLAGS=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc
        -D__NEWTOS__ -std=c99 -Wall -Wextra -Werror)
"$CLANG" "${CFLAGS[@]}" -I"$TD/core" -I"$MINT" -isystem "$RES" -c "$HERE/tedprobe.c" -o "$OBJ/tedprobe.o"
"$CLANG" "${CFLAGS[@]}" -I"$TD/core" -isystem "$RES" -c "$TD/abi/newtos_svc.c" -o "$OBJ/newtos_svc.o"
"$CLANG" -target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc \
    -I"$SDK/include" -I"$MINT" -isystem "$RES" -c "$LIBC/crt0.S" -o "$OBJ/crt0.o"
"$BIN/rust-lld" -flavor gnu -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
    --build-id=none "$OBJ/crt0.o" "$OBJ/tedprobe.o" "$OBJ/newtos_svc.o" "$LIBC/libc.a" -o "$NAP"
. "$ROOT/tools/oracle/oracle.env"
"${ORACLE_CROSS_MINT}gcc" -O2 -Wall -Wextra -Werror -I"$TD/core" "$HERE/tedprobe.c" "$TD/abi/m68k_trap.c" -o "$PRG"
shasum -a 256 "$NAP" "$PRG"
