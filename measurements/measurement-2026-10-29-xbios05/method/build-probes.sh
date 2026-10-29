#!/usr/bin/env bash
# build-probes.sh <repo> <scratch> -- XBIOS05: XCPROBE.NAP from xcprobe.c (copied unchanged
# from measurement-2026-10-28-xbios-chardev-audit/method; the EmuTOS side of its group B is
# that audit's PROBE-EMUTOS-tt / -falcontc run, same ROM) and BMCALL.NAP from bmcall.c, with
# the AArch64 link path of measurement-2026-10-29-xbios01/method/build-xcprobe.sh
# (clang + sdk/libc crt0.S, libc.a, mintlib headers, sdk/newtos.ld).
set -euo pipefail
REPO=$1; S=$2
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SDK="$REPO/sdk"; LIBC="$SDK/libc"; MINT="$LIBC/mintlib/include"
RES="$(clang -print-resource-dir)/include"
SYSROOT="$(rustc --print sysroot)"; BIN="$SYSROOT/lib/rustlib/$(rustc -vV | sed -n 's/host: //p')/bin"
CF=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc -D__NEWTOS__ -std=c99 -Wall -Wextra -Werror)
clang -target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc \
    -I"$SDK/include" -I"$MINT" -isystem "$RES" -c "$LIBC/crt0.S" -o "$S/crt0.o"
nap() { # <src> <out>
    clang "${CF[@]}" -I"$MINT" -isystem "$RES" -c "$1" -o "$S/probe.o"
    "$BIN/rust-lld" -flavor gnu -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
        --build-id=none "$S/crt0.o" "$S/probe.o" "$LIBC/libc.a" -o "$2"
}
nap "$HERE/xcprobe.c" "$S/XCPROBE.NAP"
nap "$HERE/bmcall.c" "$S/BMCALL.NAP"
shasum -a 256 "$S/XCPROBE.NAP" "$S/BMCALL.NAP"
