#!/usr/bin/env bash
# build-serrx.sh <repo> <out-dir> -- build serrx.c for both systems, after
# measurements/measurement-2026-10-29-xbios01/method/build-xcprobe.sh:
#   SERRX.PRG  m68k-atari-mint-gcc 7.5.0 (<repo>/tools/oracle/oracle.env) + mintlib
#   SERRX.NAP  clang aarch64 + <repo>/sdk/libc (crt0.S, libc.a, mintlib headers,
#              sdk/newtos.ld), the link path of tosdump/build-dumpbios-newtos.sh
# <repo> is the main tree (the SDK is an instrument here; the measured kernel
# comes from the frozen tree).
set -euo pipefail
REPO=$1; S=$2
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; SRC="$HERE/serrx.c"
. "$REPO/tools/oracle/oracle.env"
mkdir -p "$S"
"${ORACLE_CROSS_MINT}gcc" -O2 -Wall -Wextra -Werror -o "$S/SERRX.PRG" "$SRC"
SDK="$REPO/sdk"; LIBC="$SDK/libc"; MINT="$LIBC/mintlib/include"
RES="$(clang -print-resource-dir)/include"
SYSROOT="$(rustc --print sysroot)"; BIN="$SYSROOT/lib/rustlib/$(rustc -vV | sed -n 's/host: //p')/bin"
CF=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc -D__NEWTOS__ -std=c99 -Wall -Wextra -Werror)
clang -target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc \
    -I"$SDK/include" -I"$MINT" -isystem "$RES" -c "$LIBC/crt0.S" -o "$S/crt0.o"
clang "${CF[@]}" -I"$MINT" -isystem "$RES" -c "$SRC" -o "$S/serrx.o"
"$BIN/rust-lld" -flavor gnu -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
    --build-id=none "$S/crt0.o" "$S/serrx.o" "$LIBC/libc.a" -o "$S/SERRX.NAP"
shasum -a 256 "$S/SERRX.PRG" "$S/SERRX.NAP" "$LIBC/libc.a" "$SRC"
