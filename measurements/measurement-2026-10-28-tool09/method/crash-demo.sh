#!/bin/bash
# crash-demo.sh <scratch> -- a live program crash ended by the command (TOOL09 item 3):
# the XCIOFL variant of the XBIOS character device audit's probe
# (measurements/measurement-2026-10-28-xbios-chardev-audit/method/xcprobe.c, -DVARIANT=1,
# read only; its expected Bus Error on the page-0 write) is built for NewTOS exactly as
# that audit's build-xcprobe.sh builds its .NAP, but named DUMPBIOS.NAP, and passed to
# `cargo xtask tosdump` as a prebuilt dumper. The probe ignores BIOS.TSV; the run must end
# on the crash screen with its own message, long before the time limit.
set -euo pipefail
S=$1
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; REPO="$(cd "$HERE/../../.." && pwd)"
SRC="$REPO/measurements/measurement-2026-10-28-xbios-chardev-audit/method/xcprobe.c"
P="$S/crash-probe"; mkdir -p "$P"
SDK="$REPO/sdk"; LIBC="$SDK/libc"; MINT="$LIBC/mintlib/include"
RES="$(clang -print-resource-dir)/include"
SYSROOT="$(rustc --print sysroot)"; BIN="$SYSROOT/lib/rustlib/$(rustc -vV | sed -n 's/host: //p')/bin"
CF=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc -D__NEWTOS__ -std=c99 -Wall -Wextra -Werror)
clang -target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc \
    -I"$SDK/include" -I"$MINT" -isystem "$RES" -c "$LIBC/crt0.S" -o "$P/crt0.o"
clang "${CF[@]}" -DVARIANT=1 -I"$MINT" -isystem "$RES" -c "$SRC" -o "$P/xcprobe.o"
"$BIN/rust-lld" -flavor gnu -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
    --build-id=none "$P/crt0.o" "$P/xcprobe.o" "$LIBC/libc.a" -o "$P/DUMPBIOS.NAP"
cd "$REPO"
cargo xtask tosdump "$P/DUMPBIOS.NAP" tosdump/cases/bios-r1.tsv \
    --out measurements/measurement-2026-10-28-tool09/runs/crash-head --scratch "$S" || echo "tosdump exit $?"
rm -rf "$P"
