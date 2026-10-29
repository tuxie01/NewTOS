#!/usr/bin/env bash
# build-probe.sh <repo> <scratch> -- the audit probe bcprobe.c (copied unchanged from
# measurements/measurement-2026-10-28-bios-core-audit/method/bcprobe.c) for NewTOS only,
# against this tree's SDK (the audit's NAPs were linked against the SDK of 1680d5b2):
#   BCPROBE.NAP   main probe (groups K S T B G; B04/B05 = Bconout(6/7,'A'))
#   BCHANG.NAP    blocking-Bconin variant, HANGLIST=0,1,6,7
# The EmuTOS side is the audit's runs (same source, same pinned ROM).
# Link path: the audit's build-probe.sh (clang + sdk/libc crt0.S, libc.a, sdk/newtos.ld).
set -euo pipefail
REPO=$1; S=$2
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; SRC="$HERE/bcprobe.c"
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
nap "$S/BCHANG.NAP" -DHANGLIST=0,1,6,7
shasum -a 256 "$S/BCPROBE.NAP" "$S/BCHANG.NAP"
