#!/usr/bin/env bash
# build-fsprobe.sh <out.NAP> -- FSPROBE.NAP from method/alertprobe.c, the link path
# of tosdump/build-akprobe-newtos.sh (clang aarch64, mintlib libc.a, crt0,
# sdk/newtos.ld, tosdump/abi/newtos_svc.c). Objects go next to <out>.
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd); ROOT=$(cd "$HERE/../../.." && pwd)
TD=$ROOT/tosdump; SDK=$ROOT/sdk; LIBC=$SDK/libc; MINT=$LIBC/mintlib/include
OUT=$1; OBJ=$(dirname "$OUT")/obj-alertprobe; mkdir -p "$OBJ"
CLANG=${CLANG:-clang}; RES="$($CLANG -print-resource-dir)/include"
SYSROOT="$(rustc --print sysroot)"; BIN="$SYSROOT/lib/rustlib/$(rustc -vV | sed -n 's/host: //p')/bin"
LLD=$BIN/rust-lld
CFLAGS=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc
        -D__NEWTOS__ -std=c99 -Wall -Wextra -Werror)
"$CLANG" "${CFLAGS[@]}" -I"$TD/core" -I"$MINT" -isystem "$RES" -c "$HERE/alertprobe.c" -o "$OBJ/alertprobe.o"
"$CLANG" "${CFLAGS[@]}" -I"$TD/core" -isystem "$RES" -c "$TD/abi/newtos_svc.c" -o "$OBJ/newtos_svc.o"
"$CLANG" -target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc \
    -I"$SDK/include" -I"$MINT" -isystem "$RES" -c "$LIBC/crt0.S" -o "$OBJ/crt0.o"
"$LLD" -flavor gnu -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
    --build-id=none "$OBJ/crt0.o" "$OBJ/alertprobe.o" "$OBJ/newtos_svc.o" "$LIBC/libc.a" -o "$OUT"
shasum -a 256 "$OUT"
