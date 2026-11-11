#!/usr/bin/env bash
# build-nap.sh <repo-with-sdk-libc> <src.c> <out.NAP> [extra clang -D...] -- NAP half of
# measurement-2026-10-29-bios06/method/build.sh (aarch64 only; no m68k side needed)
set -euo pipefail
REPO=$1; SRC=$2; OUT=$3; shift 3
SDK="$REPO/sdk"; LIBC="$SDK/libc"; MINT="$LIBC/mintlib/include"
RES="$(clang -print-resource-dir)/include"
SYSROOT="$(rustc --print sysroot)"; BIN="$SYSROOT/lib/rustlib/$(rustc -vV | sed -n 's/host: //p')/bin"
T=$(mktemp -d "${TMPDIR:-/tmp}/bn.XXXX")
CF=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc -D__NEWTOS__ -std=c99 -Wall -Wextra -Werror)
clang -target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc \
    -I"$SDK/include" -I"$MINT" -isystem "$RES" -c "$LIBC/crt0.S" -o "$T/crt0.o"
clang "${CF[@]}" "$@" -I"$MINT" -isystem "$RES" -c "$SRC" -o "$T/p.o"
"$BIN/rust-lld" -flavor gnu -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
    --build-id=none "$T/crt0.o" "$T/p.o" "$LIBC/libc.a" -o "$OUT"
shasum -a 256 "$OUT"; rm -rf "$T"
