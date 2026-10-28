#!/usr/bin/env bash
# build-probe.sh <repo> <scratch> -- build bcprobe.c for both systems, after
# measurements/measurement-2026-10-28-bios-storage-audit/method/build-probe.sh:
#   BCPROBE.PRG / BCPROBE.NAP          main probe (groups K S T B G)
#   BCHANGn.PRG (n = 1, 3, 6)          blocking-Bconin variant HANGLIST=0,n (EmuTOS:
#                                      one run per handle, the first block ends it)
#   BCHANG.NAP                         HANGLIST=0,1,3,6 (NewTOS: one run)
# m68k: m68k-atari-mint-gcc 7.5.0 from <repo>/tools/oracle/oracle.env + mintlib.
# AArch64: clang + <repo>/sdk/libc (crt0.S, libc.a, mintlib headers, sdk/newtos.ld),
# the link path of tosdump/build-dumpbios-newtos.sh.
set -euo pipefail
REPO=$1; S=$2
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; SRC="$HERE/bcprobe.c"
. "$REPO/tools/oracle/oracle.env"
m68k() { "${ORACLE_CROSS_MINT}gcc" -O2 -Wall -Wextra -Werror "$@"; }
m68k -o "$S/BCPROBE.PRG" "$SRC"
for n in 1 3 6; do m68k -DHANGLIST=0,$n -o "$S/BCHANG$n.PRG" "$SRC"; done
SDK="$REPO/sdk"; LIBC="$SDK/libc"; MINT="$LIBC/mintlib/include"
RES="$(clang -print-resource-dir)/include"
SYSROOT="$(rustc --print sysroot)"; BIN="$SYSROOT/lib/rustlib/$(rustc -vV | sed -n 's/host: //p')/bin"
CF=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc -D__NEWTOS__ -std=c99 -Wall -Wextra -Werror)
clang -target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc \
    -I"$SDK/include" -I"$MINT" -isystem "$RES" -c "$LIBC/crt0.S" -o "$S/crt0.o"
nap() { # <out> <extra cflags...>
    local o=$1; shift
    clang "${CF[@]}" "$@" -I"$MINT" -isystem "$RES" -c "$SRC" -o "$S/bcprobe.o"
    "$BIN/rust-lld" -flavor gnu -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
        --build-id=none "$S/crt0.o" "$S/bcprobe.o" "$LIBC/libc.a" -o "$o"
}
nap "$S/BCPROBE.NAP"
nap "$S/BCHANG.NAP" -DHANGLIST=0,1,3,6
shasum -a 256 "$S"/BCPROBE.PRG "$S"/BCHANG?.PRG "$S"/BCPROBE.NAP "$S"/BCHANG.NAP
# kprobe.c (Bconin(2) with key input): KPROBE.PRG / KPROBE.NAP
m68k -o "$S/KPROBE.PRG" "$HERE/kprobe.c"
clang "${CF[@]}" -I"$MINT" -isystem "$RES" -c "$HERE/kprobe.c" -o "$S/kprobe.o"
"$BIN/rust-lld" -flavor gnu -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
    --build-id=none "$S/crt0.o" "$S/kprobe.o" "$LIBC/libc.a" -o "$S/KPROBE.NAP"
shasum -a 256 "$S/KPROBE.PRG" "$S/KPROBE.NAP"
