#!/usr/bin/env bash
# gcprobe-build.sh <scratch> -- build gcprobe.c twice, the link path of
# measurements/measurement-2026-10-28-bios-storage-audit/method/build-probe.sh:
#   <scratch>/GCPROBE.PRG  m68k-atari-mint-gcc (tools/oracle/oracle.env) + mintlib
#   <scratch>/GCPROBE.NAP  clang aarch64 + sdk/libc (crt0.S, libc.a, mintlib headers, sdk/newtos.ld)
set -euo pipefail
S=$1
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; REPO="$(cd "$HERE/../../.." && pwd)"
. "$REPO/tools/oracle/oracle.env"
"${ORACLE_CROSS_MINT}gcc" -O2 -Wall -Wextra -Werror -o "$S/GCPROBE.PRG" "$HERE/gcprobe.c"
SDK="$REPO/sdk"; LIBC="$SDK/libc"; MINT="$LIBC/mintlib/include"
RES="$(clang -print-resource-dir)/include"
SYSROOT="$(rustc --print sysroot)"; BIN="$SYSROOT/lib/rustlib/$(rustc -vV | sed -n 's/host: //p')/bin"
CF=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc -D__NEWTOS__ -std=c99 -Wall -Wextra -Werror)
clang "${CF[@]}" -I"$MINT" -isystem "$RES" -c "$HERE/gcprobe.c" -o "$S/gcprobe.o"
clang -target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc \
    -I"$SDK/include" -I"$MINT" -isystem "$RES" -c "$LIBC/crt0.S" -o "$S/crt0.o"
"$BIN/rust-lld" -flavor gnu -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
    --build-id=none "$S/crt0.o" "$S/gcprobe.o" "$LIBC/libc.a" -o "$S/GCPROBE.NAP"
shasum -a 256 "$S/GCPROBE.PRG" "$S/GCPROBE.NAP"
