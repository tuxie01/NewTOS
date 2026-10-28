#!/usr/bin/env bash
# build-xcprobe.sh <repo> <scratch> -- build xcprobe.c for both systems, after
# measurements/measurement-2026-10-28-bios-core-audit/method/build-probe.sh:
#   XCPROBE.PRG / XCPROBE.NAP    main probe (groups R S B I K V W M)
#   XCIOFL.PRG  / XCIOFL.NAP     -DVARIANT=1 (Iorec(1) keyboard-flush idiom)
#   XCKTNL.PRG  / XCKTNL.NAP     -DVARIANT=2 (Keytbl(NULL,-1,-1) + Bioskeys)
#   XKPROBE.PRG / XKPROBE.NAP    xkprobe.c (input probe)
# m68k: m68k-atari-mint-gcc 7.5.0 from <repo>/tools/oracle/oracle.env + mintlib.
# AArch64: clang + <repo>/sdk/libc (crt0.S, libc.a, mintlib headers, sdk/newtos.ld),
# the link path of tosdump/build-dumpbios-newtos.sh.
set -euo pipefail
REPO=$1; S=$2
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; SRC="$HERE/xcprobe.c"
. "$REPO/tools/oracle/oracle.env"
m68k() { "${ORACLE_CROSS_MINT}gcc" -O2 -Wall -Wextra -Werror "$@"; }
m68k -o "$S/XCPROBE.PRG" "$SRC"
m68k -DVARIANT=1 -o "$S/XCIOFL.PRG" "$SRC"
m68k -DVARIANT=2 -o "$S/XCKTNL.PRG" "$SRC"
SDK="$REPO/sdk"; LIBC="$SDK/libc"; MINT="$LIBC/mintlib/include"
RES="$(clang -print-resource-dir)/include"
SYSROOT="$(rustc --print sysroot)"; BIN="$SYSROOT/lib/rustlib/$(rustc -vV | sed -n 's/host: //p')/bin"
CF=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc -D__NEWTOS__ -std=c99 -Wall -Wextra -Werror)
clang -target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc \
    -I"$SDK/include" -I"$MINT" -isystem "$RES" -c "$LIBC/crt0.S" -o "$S/crt0.o"
nap() { # <out> <extra cflags...>
    local o=$1; shift
    clang "${CF[@]}" "$@" -I"$MINT" -isystem "$RES" -c "$SRC" -o "$S/xcprobe.o"
    "$BIN/rust-lld" -flavor gnu -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
        --build-id=none "$S/crt0.o" "$S/xcprobe.o" "$LIBC/libc.a" -o "$o"
}
nap "$S/XCPROBE.NAP"
nap "$S/XCIOFL.NAP" -DVARIANT=1
nap "$S/XCKTNL.NAP" -DVARIANT=2
shasum -a 256 "$S"/XCPROBE.PRG "$S"/XCIOFL.PRG "$S"/XCKTNL.PRG "$S"/XCPROBE.NAP "$S"/XCIOFL.NAP "$S"/XCKTNL.NAP
# xkprobe.c (Keytbl/Bioskeys/mousevec with real input): XKPROBE.PRG / XKPROBE.NAP
m68k -o "$S/XKPROBE.PRG" "$HERE/xkprobe.c"
clang "${CF[@]}" -I"$MINT" -isystem "$RES" -c "$HERE/xkprobe.c" -o "$S/xkprobe.o"
"$BIN/rust-lld" -flavor gnu -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
    --build-id=none "$S/crt0.o" "$S/xkprobe.o" "$LIBC/libc.a" -o "$S/XKPROBE.NAP"
shasum -a 256 "$S/XKPROBE.PRG" "$S/XKPROBE.NAP"
