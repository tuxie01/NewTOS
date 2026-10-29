#!/usr/bin/env bash
#
# cookie-host.sh -- build and run the host harness for TeraDesk's find_cookie()
# jar walk (TASK/SDK04, finding C6): cookie-host.c includes TeraDesk's
# library.h with the flags of the -D_TOS_ONLY build, redirects Setexc to a fake
# jar in the kernel's wire format, and #includes the cookie.c under test
# unchanged.
#
# Default cookie.c: the patch sdk/overlays/teradesk/library/utility/cookie.c
# (patch site SDK04-C6). COOKIE_SRC=<file> builds that file instead -- for the
# counter-check against the vendored sdk/apps/teradesk/library/utility/cookie.c
# (upstream walk with `COOKIE *`, 16 bytes per element on LP64).
#
# Include order as sdk/gemlib/tests/aesglobal_fields.c (the build's shadow
# tree: overlay directories before the vendored ones, then gemlib, mintlib).
#
# Exit status: the number of failed checks (0 = all passed); 77 = skipped
# (vendored TeraDesk, gemlib tree or mintlib headers absent, or host not LP64
# little-endian); 1 = did not build.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"          # .../sdk/overlays/teradesk/tests
SDK="$(cd "$HERE/../../.." && pwd)"                            # .../sdk
TERA="$SDK/apps/teradesk"
OVL="$SDK/overlays/teradesk"
GT="$SDK/gemlib/gemlib-0.44.0"
MINT="$SDK/libc/mintlib/include"
SRC="${COOKIE_SRC:-$OVL/library/utility/cookie.c}"

CC="${CC:-cc}"
command -v "$CC" >/dev/null 2>&1 || { echo "error: $CC not found" >&2; exit 1; }
[ -f "$TERA/library/xdialog/library.h" ] || {
  echo "cookie-host: skipped -- vendored TeraDesk source absent ($TERA)"; exit 77; }
[ -f "$GT/mt_gem.h" ] || {
  echo "cookie-host: skipped -- overlaid gemlib tree absent ($GT); run build-gemlib.sh"; exit 77; }
[ -f "$MINT/mint/ssystem.h" ] || {
  echo "cookie-host: skipped -- mintlib headers absent ($MINT); run build-libc.sh"; exit 77; }
[ -f "$SRC" ] || { echo "error: $SRC missing" >&2; exit 1; }
SRC="$(cd "$(dirname "$SRC")" && pwd)/$(basename "$SRC")"
echo "cookie.c under test: $SRC"

W="$(mktemp -d)"
trap 'rm -rf "$W"' EXIT

"$CC" -O1 -g -Wall -Wno-macro-redefined -Wno-unused-function \
  -D__NEWTOS__ -D_TOS_ONLY -funsigned-char -DCOOKIE_C="\"$SRC\"" \
  -I "$OVL" -I "$OVL/library/xdialog" -I "$OVL/library/utility" \
  -I "$TERA" -I "$TERA/library/xdialog" -I "$TERA/library/utility" \
  -I "$GT" -I "$MINT" \
  "$HERE/cookie-host.c" -o "$W/cookie-host" || exit 1

"$W/cookie-host"
