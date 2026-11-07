#!/usr/bin/env bash
#
# build-vdprobe-newtos.sh -- build VDPROBE.NAP, the NewTOS/AArch64 half of the
# VDI drawing probe tosdump/probes/vdprobe.c (TASK/V18). The m68k half is
# `make -C tosdump vdprobe-m68k`; same source: the GEMDOS calls go
# through the mintlib bindings (osbind.h) as a ported TOS program does, the AES
# calls through the tosdump ABI backend (abi/newtos_svc.c here,
# abi/m68k_trap.c there).
#
# The link path of the other NewTOS dumpers (build-dumpbios-newtos.sh): clang
# -target aarch64-unknown-none-elf, the libc-aware crt0, mintlib's libc.a and
# sdk/newtos.ld.
#
# Usage:  tosdump/build-vdprobe-newtos.sh [output.NAP]
set -uo pipefail

HERE=/Volumes/Daten/Development/PiRusTOS/.claude/worktrees/agent-a76fdd16f4fbe1958/tosdump
ROOT="$(cd "$HERE/.." && pwd)"
SDK="$ROOT/sdk"
LIBC="$SDK/libc"
BUILD=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/ed7bb9a3-0fc2-4b37-91bb-ed70559503a3/scratchpad/x51/build
OBJ="$BUILD/obj-newtos-vdprobe"
OUT="${1:-$BUILD/VDPROBE.NAP}"
mkdir -p "$OBJ"

# --- libc ---------------------------------------------------------------------
if [ ! -f "$LIBC/libc.a" ]; then
  echo "libc.a absent -- running build-libc.sh first…"
  "$LIBC/build-libc.sh" >/dev/null 2>&1 || { echo "error: build-libc.sh failed" >&2; exit 1; }
fi
MINT="$LIBC/mintlib/include"
[ -d "$MINT" ] || { echo "error: mintlib headers missing ($MINT); run build-libc.sh" >&2; exit 1; }

# --- toolchain ----------------------------------------------------------------
CLANG="${CLANG:-clang}"
command -v "$CLANG" >/dev/null 2>&1 || { echo "error: clang not found" >&2; exit 1; }
RES="$($CLANG -print-resource-dir)/include"
SYSROOT="$(rustc --print sysroot 2>/dev/null)"
BIN="$SYSROOT/lib/rustlib/$(rustc -vV | sed -n 's/host: //p')/bin"
LLD="${LLD:-$BIN/rust-lld}"
NM="$BIN/llvm-nm"
[ -x "$LLD" ] || { echo "error: missing rust-lld ($LLD)" >&2; exit 1; }

CFLAGS=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2
        -fPIE -fno-builtin -nostdinc -D__NEWTOS__
        -std=c99 -Wall -Wextra -Werror)

echo "clang : $($CLANG --version | head -1)"
echo "lld   : $LLD"
echo

rm -f "$OBJ"/*.o
"$CLANG" "${CFLAGS[@]}" -I"$HERE/core" -I"$MINT" -isystem "$RES" \
    -c "/Volumes/Daten/Development/PiRusTOS/.claude/worktrees/agent-a76fdd16f4fbe1958/measurements/measurement-2026-11-07-v51-test/method/vdprobe-x51.c" -o "$OBJ/vdprobe.o" || { echo "error: vdprobe.c failed to compile" >&2; exit 1; }
echo "  CC  probes/vdprobe.c"
"$CLANG" "${CFLAGS[@]}" -I"$HERE/core" -isystem "$RES" \
    -c "$HERE/abi/newtos_svc.c" -o "$OBJ/newtos_svc.o" || { echo "error: newtos_svc.c failed to compile" >&2; exit 1; }
echo "  CC  abi/newtos_svc.c"

"$CLANG" -target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 \
    -fPIE -fno-builtin -nostdinc -I"$SDK/include" -I"$MINT" -isystem "$RES" \
    -c "$LIBC/crt0.S" -o "$OBJ/crt0.o" || exit 1

rm -f "$OUT"
"$LLD" -flavor gnu -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
    --build-id=none "$OBJ/crt0.o" "$OBJ/vdprobe.o" "$OBJ/newtos_svc.o" "$LIBC/libc.a" \
    -o "$OUT" 2>"$BUILD/link-newtos-vdprobe.err" || {
    echo "LINK FAILED:" >&2; cat "$BUILD/link-newtos-vdprobe.err" >&2; exit 1; }

echo
echo "LINK OK -> $OUT ($(wc -c < "$OUT" | tr -d ' ') bytes)"
u=$("$NM" -u "$OUT" 2>/dev/null | grep -c . || true)
echo "undefined symbols in the final image (goal 0): $u"
[ "$u" -eq 0 ] || exit 1
