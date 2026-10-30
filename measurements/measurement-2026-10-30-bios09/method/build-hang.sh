#!/usr/bin/env bash
# build-hang.sh <repo> <scratch> -- BIOS09: the audit probe bcprobe.c (copied unchanged from
# measurements/measurement-2026-10-30-bios08/method/bcprobe.c = the audit's) as the blocking-
# Bconin variant in the audit's per-handle form (HANGLIST=0,n; the oracle runs
# measurements/measurement-2026-10-28-bios-core-audit/runs/HANG-EMUTOS-tt-1, -6):
#   BCHANG1.NAP  HANGLIST=0,1      BCHANG6.NAP  HANGLIST=0,6
# for NewTOS against this tree's SDK. Link path as
# measurements/measurement-2026-10-30-bios08/method/build-probe.sh.
set -euo pipefail
REPO=$1; S=$2
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; SRC="$HERE/bcprobe.c"
SDK="$REPO/sdk"; LIBC="$SDK/libc"; MINT="$LIBC/mintlib/include"
RES="$(clang -print-resource-dir)/include"
SYSROOT="$(rustc --print sysroot)"; BIN="$SYSROOT/lib/rustlib/$(rustc -vV | sed -n 's/host: //p')/bin"
CF=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc -D__NEWTOS__ -std=c99 -Wall -Wextra -Werror)
mkdir -p "$S"
clang -target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc \
    -I"$SDK/include" -I"$MINT" -isystem "$RES" -c "$LIBC/crt0.S" -o "$S/crt0.o"
nap() { # <out> <extra cflags...>
    local o=$1; shift
    clang "${CF[@]}" "$@" -I"$MINT" -isystem "$RES" -c "$SRC" -o "$S/bcprobe.o"
    "$BIN/rust-lld" -flavor gnu -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
        --build-id=none "$S/crt0.o" "$S/bcprobe.o" "$LIBC/libc.a" -o "$o"
}
nap "$S/BCHANG1.NAP" -DHANGLIST=0,1
nap "$S/BCHANG6.NAP" -DHANGLIST=0,6
shasum -a 256 "$S/BCHANG1.NAP" "$S/BCHANG6.NAP"
