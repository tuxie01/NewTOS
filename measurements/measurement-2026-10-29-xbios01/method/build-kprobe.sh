#!/usr/bin/env bash
# build-kprobe.sh <repo> <scratch> -- KPROBE.NAP from method/kprobe.c (copied from
# measurement-2026-10-28-bios-core-audit/method, unchanged), with the AArch64 link path
# of that run's build-probe.sh (lines 18-37 there): clang + sdk/libc crt0.S, libc.a,
# mintlib headers, sdk/newtos.ld.
set -euo pipefail
REPO=$1; S=$2
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SDK="$REPO/sdk"; LIBC="$SDK/libc"; MINT="$LIBC/mintlib/include"
RES="$(clang -print-resource-dir)/include"
SYSROOT="$(rustc --print sysroot)"; BIN="$SYSROOT/lib/rustlib/$(rustc -vV | sed -n 's/host: //p')/bin"
CF=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc -D__NEWTOS__ -std=c99 -Wall -Wextra -Werror)
clang -target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc \
    -I"$SDK/include" -I"$MINT" -isystem "$RES" -c "$LIBC/crt0.S" -o "$S/crt0.o"
clang "${CF[@]}" -I"$MINT" -isystem "$RES" -c "$HERE/kprobe.c" -o "$S/kprobe.o"
"$BIN/rust-lld" -flavor gnu -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
    --build-id=none "$S/crt0.o" "$S/kprobe.o" "$LIBC/libc.a" -o "$S/KPROBE.NAP"
shasum -a 256 "$S/KPROBE.NAP"
