#!/usr/bin/env bash
#
# sysvars-layout.sh -- compile sysvars-layout.c and ostruct-layout.c for the
# NewTOS TARGET with the include path of sdk/libc/build-libc.sh, the overlay
# include directory first (TASK/SDK04 point 3). Both files are all
# _Static_assert, so "compiles" == "the SDK's <mint/sysvars.h> cells,
# OSHEADER, NEWTOS_COOKIE, and <mint/ostruct.h>'s _MD/_MPB/_KBDVECS have the
# layout docs/api names". Host-independent: only clang's aarch64 target is
# used, nothing is run. The overlay directory comes first so the check sees
# the committed overlay even before build-libc.sh has copied it into the
# extracted tree.
#
# Exit status: 0 = layout right; 1 = an address, width, offset or size
# differs (clang's diagnostic names it); 77 = skipped (mintlib tree not
# extracted).
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"          # .../sdk/libc/tests
LIBC="$(cd "$HERE/.." && pwd)"                                 # .../sdk/libc
TOP="$LIBC/mintlib"
CLANG="${CLANG:-clang}"
[ -f "$TOP/include/features.h" ] || {
  echo "sysvars-layout: skipped -- mintlib tree missing ($TOP); run build-libc.sh"
  exit 77
}
RES="$("$CLANG" -print-resource-dir)/include"
W="$(mktemp -d)"; trap 'rm -rf "$W"' EXIT

rc=0
for t in sysvars-layout ostruct-layout; do
  if "$CLANG" -target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 \
       -fno-builtin -nostdinc \
       -D_LIBC -D_REENTRANT -DHAVE_CONFIG_H -D_GNU_SOURCE -D__NO_LONG_DOUBLE_MATH \
       -Wall -Wno-macro-redefined \
       -I"$LIBC/overlay/include" \
       -I"$TOP" -I"$TOP/include" -I"$TOP/mintlib" -I"$TOP/stdlib" -isystem "$RES" \
       -c "$HERE/$t.c" -o "$W/$t.o" 2> "$W/$t.err"; then
    echo "$t: layout matches docs/api"
  else
    echo "$t: FAILED"
    grep -E "static assertion failed|error:" "$W/$t.err" | sed 's/^/  /'
    rc=1
  fi
done
exit $rc
