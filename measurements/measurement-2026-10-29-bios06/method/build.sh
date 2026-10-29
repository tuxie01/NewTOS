#!/usr/bin/env bash
# build.sh <repo> <scratch> -- build gnprobe.c for both systems, the link path of
# measurement-2026-10-28-bios-core-audit/method/build-probe.sh:
#   GNPROBE.PRG  m68k-atari-mint-gcc 7.5.0 (<repo>/tools/oracle/oracle.env) + mintlib
#   GNPROBE.NAP  clang aarch64 + <repo>/sdk/libc (crt0.S, libc.a, mintlib headers incl.
#                the overlay ostruct.h with the SDK04 _MD/_MPB asserts, sdk/newtos.ld)
set -euo pipefail
REPO=$1; S=$2
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; SRC="$HERE/gnprobe.c"
. "$REPO/tools/oracle/oracle.env"
"${ORACLE_CROSS_MINT}gcc" -O2 -Wall -Wextra -Werror -o "$S/GNPROBE.PRG" "$SRC"
SDK="$REPO/sdk"; LIBC="$SDK/libc"; MINT="$LIBC/mintlib/include"
RES="$(clang -print-resource-dir)/include"
SYSROOT="$(rustc --print sysroot)"; BIN="$SYSROOT/lib/rustlib/$(rustc -vV | sed -n 's/host: //p')/bin"
CF=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc -D__NEWTOS__ -std=c99 -Wall -Wextra -Werror)
clang -target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc \
    -I"$SDK/include" -I"$MINT" -isystem "$RES" -c "$LIBC/crt0.S" -o "$S/crt0.o"
clang "${CF[@]}" -I"$MINT" -isystem "$RES" -c "$SRC" -o "$S/gnprobe.o"
"$BIN/rust-lld" -flavor gnu -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
    --build-id=none "$S/crt0.o" "$S/gnprobe.o" "$LIBC/libc.a" -o "$S/GNPROBE.NAP"
shasum -a 256 "$S/GNPROBE.PRG" "$S/GNPROBE.NAP"
