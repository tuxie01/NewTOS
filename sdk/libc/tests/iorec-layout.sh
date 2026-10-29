#!/usr/bin/env bash
#
# iorec-layout.sh -- compile iorec-layout.c for the NewTOS TARGET with the
# include path of sdk/libc/build-libc.sh (TASK/XBIOS01). The file is all
# _Static_assert, so "compiles" == "the SDK _IOREC has the offsets the kernel
# lays the Iorec records out at". Host-independent: only clang's aarch64
# target is used, nothing is run.
#
# Exit status: 0 = layout right; 1 = an offset or size differs (clang's
# diagnostic names it); 77 = skipped (mintlib tree not extracted).
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"          # .../sdk/libc/tests
LIBC="$(cd "$HERE/.." && pwd)"                                 # .../sdk/libc
TOP="$LIBC/mintlib"
CLANG="${CLANG:-clang}"
[ -f "$TOP/include/mint/ostruct.h" ] || {
  echo "iorec-layout: skipped -- mintlib tree missing ($TOP); run build-libc.sh"
  exit 77
}
RES="$("$CLANG" -print-resource-dir)/include"
W="$(mktemp -d)"; trap 'rm -rf "$W"' EXIT

if "$CLANG" -target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 \
     -fno-builtin -nostdinc \
     -D_LIBC -D_REENTRANT -DHAVE_CONFIG_H -D_GNU_SOURCE -D__NO_LONG_DOUBLE_MATH \
     -Wno-macro-redefined \
     -I"$TOP" -I"$TOP/include" -I"$TOP/mintlib" -I"$TOP/stdlib" -isystem "$RES" \
     -c "$HERE/iorec-layout.c" -o "$W/iorec-layout.o" 2> "$W/err"; then
  echo "iorec-layout: _IOREC and the device-0 record match docs/api/xbios/0e-iorec.md F1"
  exit 0
fi
grep -E "static assertion failed|error:" "$W/err" | sed 's/^/  /'
exit 1
