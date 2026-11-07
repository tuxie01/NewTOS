#!/usr/bin/env bash
#
# inline-rsh-host.sh -- host test of cflib's built-in images (TASK/A142):
# compares the tables of sdk/cflib/overlay/inline.rsh, as the VDI reads them,
# with upstream's bytes from the vendored tarball, as m68k read them. The
# harness and the statement are in inline-rsh-host.c.
#
# INLINE_RSH_SRC=<file> tests that file instead of the overlay. With upstream's
# file, or the overlay before TASK/A142, 42 of 137 checks fail: 39 tables and the
# 3 fixed rows arrive with the two bytes of every word traded (the other 5
# tables read the same either way).
#
# Exit status: the number of failed checks (0 = all passed); 77 = skipped
# (gemlib or mintlib headers absent, or host not LP64 little-endian);
# 1 = did not build.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"          # .../sdk/cflib/tests
SDK="$(cd "$HERE/../.." && pwd)"                               # .../sdk
GT="$SDK/gemlib/gemlib-0.44.0"
MINT="$SDK/libc/mintlib/include"
TARBALL="$SDK/cflib/cflib-21-ce57817.tar.xz"
SRC="${INLINE_RSH_SRC:-$SDK/cflib/overlay/inline.rsh}"

CC="${CC:-cc}"
command -v "$CC" >/dev/null 2>&1 || { echo "error: $CC not found" >&2; exit 1; }
[ -f "$GT/mt_gem.h" ] && [ -d "$MINT" ] || {
  echo "inline-rsh-host: skipped -- gemlib or mintlib headers missing; run build-gemlib.sh"
  exit 77
}
[ -f "$TARBALL" ] || { echo "error: $TARBALL missing" >&2; exit 1; }
[ -f "$SRC" ] || { echo "error: $SRC missing" >&2; exit 1; }
echo "inline.rsh under test: $SRC"

W="$(mktemp -d)"
trap 'rm -rf "$W"' EXIT
tar xOf "$TARBALL" cflib-21/inline.rsh > "$W/upstream.rsh" && tar xOf "$TARBALL" cflib-21/inline.h > "$W/inline.h" || exit 1
cp "$SRC" "$W/port.rsh"
: > "$W/portab.h"      # upstream's guard includes it; the build's headers have none

FLAGS=(-O0 -g -D__NEWTOS__ -Wno-macro-redefined -I"$W" -I"$GT" -I"$MINT")
"$CC" "${FLAGS[@]}" -DSIDE=side_pristine '-DINLINE_RSH="upstream.rsh"' \
    -c "$HERE/inline-rsh-host.c" -o "$W/pristine.o" || exit 1
"$CC" "${FLAGS[@]}" -DSIDE=side_port '-DINLINE_RSH="port.rsh"' \
    -c "$HERE/inline-rsh-host.c" -o "$W/port.o" || exit 1
"$CC" -O1 -g -Wall -Wextra -c "$HERE/inline-rsh-host.c" -o "$W/main.o" || exit 1
"$CC" "$W/main.o" "$W/pristine.o" "$W/port.o" -o "$W/inline-rsh-host" || exit 1

"$W/inline-rsh-host"
