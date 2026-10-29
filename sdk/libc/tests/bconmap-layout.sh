#!/usr/bin/env bash
#
# bconmap-layout.sh -- compile bconmap-layout.c for the NewTOS TARGET with the
# include path of sdk/libc/build-libc.sh, the overlay include directory first
# (TASK/XBIOS05; as sysvars-layout.sh). The file is all _Static_assert, so
# "compiles" == "the SDK's _BCONMAP/_MAPTAB have the offsets the kernel lays
# the Bconmap table out at". Host-independent: only clang's aarch64 target is
# used, nothing is run.
#
# Exit status: 0 = layout right; 1 = an offset or size differs (clang's
# diagnostic names it); 77 = skipped (mintlib tree not extracted).
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"          # .../sdk/libc/tests
LIBC="$(cd "$HERE/.." && pwd)"                                 # .../sdk/libc
TOP="$LIBC/mintlib"
CLANG="${CLANG:-clang}"
[ -f "$TOP/include/features.h" ] || {
  echo "bconmap-layout: skipped -- mintlib tree missing ($TOP); run build-libc.sh"
  exit 77
}
RES="$("$CLANG" -print-resource-dir)/include"
W="$(mktemp -d)"; trap 'rm -rf "$W"' EXIT

if "$CLANG" -target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 \
     -fno-builtin -nostdinc \
     -D_LIBC -D_REENTRANT -DHAVE_CONFIG_H -D_GNU_SOURCE -D__NO_LONG_DOUBLE_MATH \
     -Wall -Wno-macro-redefined \
     -I"$LIBC/overlay/include" \
     -I"$TOP" -I"$TOP/include" -I"$TOP/mintlib" -I"$TOP/stdlib" -isystem "$RES" \
     -c "$HERE/bconmap-layout.c" -o "$W/bconmap-layout.o" 2> "$W/err"; then
  echo "bconmap-layout: _BCONMAP and _MAPTAB match docs/api/structures/bconmap.md (LP64)"
  exit 0
fi
grep -E "static assertion failed|error:" "$W/err" | sed 's/^/  /'
exit 1
