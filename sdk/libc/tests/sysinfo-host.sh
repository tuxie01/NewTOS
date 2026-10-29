#!/usr/bin/env bash
#
# sysinfo-host.sh -- build and run the host harness for mintlib's
# sysinfo(SI_PLATFORM)/(SI_HW_PROVIDER) with the NewTOS overlay (TASK/SDK03):
# the overlay sdk/libc/overlay/unix/sysinfo.c (under AddressSanitizer) and
# sdk/libc/newtos-arch/getcookie.c, compiled with the include path and DEFS of
# sdk/libc/build-libc.sh, <mint/mintbind.h>/<mint/osbind.h> held back and
# sysinfo-host-mintbind.h forced in (Ssystem/Super/SuperToUser/Sversion ->
# sysinfo-host.c, which plays the kernel's cookie jar). The weak_alias line is
# dropped (an ELF .weak directive the Mach-O host assembler rejects); the
# harness calls __sysinfo as sh_sysinfo.
#
# SYSINFO_SRC=<file> builds that sysinfo.c instead of the overlay -- for the
# before/after counter-check against the pristine mintlib file
# (tar xf sdk/libc/mintlib-0.60.1.tar.xz mintlib-0.60.1/unix/sysinfo.c):
# with it 5 of 15 cases fail: _MCH 0xFFFFFFFF without ColdFire (strlen (NULL),
# SEGV); the three families above 5 (platforms[9] past the nine-entry table --
# undefined; clang warns -Warray-bounds and the -O1 build ends in the same
# SEGV in strlen); ColdFire EVB (gets "cfevbatariclone").
#
# Exit status: the number of failed cases (0 = all passed); 77 = skipped
# (mintlib tree not extracted, or host not LP64 little-endian); 1 = did not
# build.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"          # .../sdk/libc/tests
LIBC="$(cd "$HERE/.." && pwd)"                                 # .../sdk/libc
TOP="$LIBC/mintlib"
SRC="${SYSINFO_SRC:-$LIBC/overlay/unix/sysinfo.c}"

CC="${CC:-cc}"
command -v "$CC" >/dev/null 2>&1 || { echo "error: $CC not found" >&2; exit 1; }
[ -f "$TOP/include/mint/cookie.h" ] || {
  echo "sysinfo-host: skipped -- mintlib tree missing ($TOP); run build-libc.sh"
  exit 77
}
[ -f "$SRC" ] || { echo "error: $SRC missing" >&2; exit 1; }
RES="$("$CC" -print-resource-dir)/include"
echo "sysinfo.c under test: $SRC"

W="$(mktemp -d)"
trap 'rm -rf "$W"' EXIT

MFLAGS=(-O1 -g -fno-builtin -nostdinc -fno-blocks -Wall
        -D_LIBC -D_REENTRANT -DHAVE_CONFIG_H -D_GNU_SOURCE -D__NO_LONG_DOUBLE_MATH
        -Wno-macro-redefined
        -D_MINT_MINTBIND_H=1 -D_MINT_OSBIND_H=1 -include "$HERE/sysinfo-host-mintbind.h"
        -D__sysinfo=sh_sysinfo
        -I"$TOP" -I"$TOP/include" -I"$TOP/mintlib" -I"$TOP/stdlib" -isystem "$RES")
sed '/^weak_alias/d' "$SRC" > "$W/sysinfo.c"
"$CC" "${MFLAGS[@]}" -fsanitize=address -c "$W/sysinfo.c" -o "$W/sysinfo.o" || exit 1
"$CC" "${MFLAGS[@]}" -c "$LIBC/newtos-arch/getcookie.c" -o "$W/getcookie.o" || exit 1
"$CC" -O1 -g -Wall -Wextra -c "$HERE/sysinfo-host.c" -o "$W/harness.o" || exit 1
"$CC" -fsanitize=address "$W"/*.o -o "$W/sysinfo-host" || exit 1

"$W/sysinfo-host"
