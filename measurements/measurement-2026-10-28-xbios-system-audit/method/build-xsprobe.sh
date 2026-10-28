#!/usr/bin/env bash
# build-xsprobe.sh <repo> <scratch> -- build xsprobe.c for both systems, after
# measurements/measurement-2026-10-28-xbios-chardev-audit/method/build-xcprobe.sh:
#   XSPROBE.PRG  m68k-atari-mint-gcc 7.5.0 from <repo>/tools/oracle/oracle.env + mintlib
#   XSPROBE.NAP  clang + <repo>/sdk/libc (crt0.S, libc.a, mintlib headers, sdk/newtos.ld),
#                the link path of tosdump/build-dumpbios-newtos.sh.
set -euo pipefail
REPO=$1; S=$2
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; SRC="$HERE/xsprobe.c"
. "$REPO/tools/oracle/oracle.env"
"${ORACLE_CROSS_MINT}gcc" -O2 -Wall -Wextra -Werror -o "$S/XSPROBE.PRG" "$SRC"
SDK="$REPO/sdk"; LIBC="$SDK/libc"; MINT="$LIBC/mintlib/include"
RES="$(clang -print-resource-dir)/include"
SYSROOT="$(rustc --print sysroot)"; BIN="$SYSROOT/lib/rustlib/$(rustc -vV | sed -n 's/host: //p')/bin"
CF=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc -D__NEWTOS__ -std=c99 -Wall -Wextra -Werror)
clang -target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc \
    -I"$SDK/include" -I"$MINT" -isystem "$RES" -c "$LIBC/crt0.S" -o "$S/crt0.o"
clang "${CF[@]}" -I"$MINT" -isystem "$RES" -c "$SRC" -o "$S/xsprobe.o"
"$BIN/rust-lld" -flavor gnu -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
    --build-id=none "$S/crt0.o" "$S/xsprobe.o" "$LIBC/libc.a" -o "$S/XSPROBE.NAP"
shasum -a 256 "$S/XSPROBE.PRG" "$S/XSPROBE.NAP"
# xsseed.c (Random's first seed): XSSEED.PRG / XSSEED.NAP
"${ORACLE_CROSS_MINT}gcc" -O2 -Wall -Wextra -Werror -o "$S/XSSEED.PRG" "$HERE/xsseed.c"
clang "${CF[@]}" -I"$MINT" -isystem "$RES" -c "$HERE/xsseed.c" -o "$S/xsseed.o"
"$BIN/rust-lld" -flavor gnu -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
    --build-id=none "$S/crt0.o" "$S/xsseed.o" "$LIBC/libc.a" -o "$S/XSSEED.NAP"
shasum -a 256 "$S/XSSEED.PRG" "$S/XSSEED.NAP"
# xshi.c (high half of Gettime's LONG): XSHI.PRG / XSHI.NAP
"${ORACLE_CROSS_MINT}gcc" -O2 -Wall -Wextra -Werror -o "$S/XSHI.PRG" "$HERE/xshi.c"
clang "${CF[@]}" -I"$MINT" -isystem "$RES" -c "$HERE/xshi.c" -o "$S/xshi.o"
"$BIN/rust-lld" -flavor gnu -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
    --build-id=none "$S/crt0.o" "$S/xshi.o" "$LIBC/libc.a" -o "$S/XSHI.NAP"
shasum -a 256 "$S/XSHI.PRG" "$S/XSHI.NAP"
