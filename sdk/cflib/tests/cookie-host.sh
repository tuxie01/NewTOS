#!/usr/bin/env bash
#
# cookie-host.sh -- build and run the host harness for cflib's getcookie()
# tag construction (TASK/SDK05): the overlay sdk/cflib/overlay/cookie.c
# (patch site SDK05) compiled for the host with -D__NEWTOS__ -- the define
# build-cflib.sh compiles cflib with -- against cookie-host.c, which plays
# mintlib's Getcookie() and checks the whole long it is handed.
#
# cookie.c includes "intern.h" (all of cflib's GEM headers) only for TRUE and
# FALSE; the file is copied into a work directory, so its quote include finds
# cookie-host-intern.h (copied there as intern.h) instead of cflib's. Its
# <mint/cookie.h> is mintlib's (sdk/libc/mintlib/include, the C_FOUND /
# Getcookie declaration).
#
# COOKIE_SRC=<file> builds that cookie.c instead of the overlay -- for the
# before/after counter-check against the pristine cflib file
# (tar xf sdk/cflib/cflib-21-ce57817.tar.xz cflib-21/cookie.c): with it 17 of
# 26 checks fail -- every tag arrives byte-reversed in the low four bytes with
# stale stack bytes above them (on the macOS arm64 host NTOS ->
# 0x00000001534F544E), so all five jar names miss, and so do the NULL presence
# test and the exact tag of the absent and the >= 0x80 name.
#
# Exit status: the number of failed checks (0 = all passed); 77 = skipped
# (mintlib headers absent, or host not LP64 little-endian); 1 = did not build.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"          # .../sdk/cflib/tests
SDK="$(cd "$HERE/../.." && pwd)"                               # .../sdk
MINT="$SDK/libc/mintlib/include"
SRC="${COOKIE_SRC:-$SDK/cflib/overlay/cookie.c}"

CC="${CC:-cc}"
command -v "$CC" >/dev/null 2>&1 || { echo "error: $CC not found" >&2; exit 1; }
[ -f "$MINT/mint/cookie.h" ] || {
  echo "cookie-host: skipped -- mintlib headers missing ($MINT); run build-libc.sh"
  exit 77
}
[ -f "$SRC" ] || { echo "error: $SRC missing" >&2; exit 1; }
echo "cookie.c under test: $SRC"

W="$(mktemp -d)"
trap 'rm -rf "$W"' EXIT
cp "$SRC" "$W/cookie.c"
cp "$HERE/cookie-host-intern.h" "$W/intern.h"

"$CC" -O0 -g -Wall -Wno-macro-redefined -D__NEWTOS__ -I"$MINT" -c "$W/cookie.c" -o "$W/cookie.o" || exit 1
"$CC" -O1 -g -Wall -Wextra -c "$HERE/cookie-host.c" -o "$W/harness.o" || exit 1
"$CC" "$W/cookie.o" "$W/harness.o" -o "$W/cookie-host" || exit 1

"$W/cookie-host"
