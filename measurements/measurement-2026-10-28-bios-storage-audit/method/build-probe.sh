#!/usr/bin/env bash
# build-probe.sh <scratch> PSIZE DSIZE -- build bstprobe.c twice:
#   <scratch>/BSTPROBE.PRG  m68k-atari-mint-gcc 7.5.0 (tools/oracle/oracle.env) + mintlib
#   <scratch>/BSTPROBE.NAP  clang aarch64 + sdk/libc (crt0.S, libc.a, mintlib headers,
#                           sdk/newtos.ld), the link path of tosdump/build-dumpbios-newtos.sh
set -euo pipefail
S=$1; PS=$2; DS=$3
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; REPO="$(cd "$HERE/../../.." && pwd)"
. "$REPO/tools/oracle/oracle.env"
DEFS=(-DPSIZE=${PS}L -DDSIZE=${DS}L -DPSTART=63L -DIDEUNIT=18)
"${ORACLE_CROSS_MINT}gcc" -O2 -Wall -Wextra -Werror "${DEFS[@]}" -o "$S/BSTPROBE.PRG" "$HERE/bstprobe.c"
SDK="$REPO/sdk"; LIBC="$SDK/libc"; MINT="$LIBC/mintlib/include"
RES="$(clang -print-resource-dir)/include"
SYSROOT="$(rustc --print sysroot)"; BIN="$SYSROOT/lib/rustlib/$(rustc -vV | sed -n 's/host: //p')/bin"
CF=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc -D__NEWTOS__ -std=c99 -Wall -Wextra -Werror)
clang "${CF[@]}" "${DEFS[@]}" -I"$MINT" -isystem "$RES" -c "$HERE/bstprobe.c" -o "$S/bstprobe.o"
clang -target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc \
    -I"$SDK/include" -I"$MINT" -isystem "$RES" -c "$LIBC/crt0.S" -o "$S/crt0.o"
"$BIN/rust-lld" -flavor gnu -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
    --build-id=none "$S/crt0.o" "$S/bstprobe.o" "$LIBC/libc.a" -o "$S/BSTPROBE.NAP"
shasum -a 256 "$S/BSTPROBE.PRG" "$S/BSTPROBE.NAP"
