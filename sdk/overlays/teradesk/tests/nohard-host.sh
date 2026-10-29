#!/usr/bin/env bash
#
# nohard-host.sh -- build and run the host harness for TeraDesk's overscan
# gate (TASK/SDK07, patch site SDK07): nohard-host.c includes TeraDesk's
# library.h with the flags of the -D_TOS_ONLY build, redirects Setexc to a fake
# jar in the kernel's wire format, and #includes the overlay cookie.c
# (find_cookie, SDK04-C6) and the overlay nohard.h under test unchanged.
# Construction and include order as cookie-host.sh beside this file.
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

CC="${CC:-cc}"
command -v "$CC" >/dev/null 2>&1 || { echo "error: $CC not found" >&2; exit 1; }
[ -f "$TERA/library/xdialog/library.h" ] || {
  echo "nohard-host: skipped -- vendored TeraDesk source absent ($TERA)"; exit 77; }
[ -f "$GT/mt_gem.h" ] || {
  echo "nohard-host: skipped -- overlaid gemlib tree absent ($GT); run build-gemlib.sh"; exit 77; }
[ -f "$MINT/mint/ssystem.h" ] || {
  echo "nohard-host: skipped -- mintlib headers absent ($MINT); run build-libc.sh"; exit 77; }

W="$(mktemp -d)"
trap 'rm -rf "$W"' EXIT

"$CC" -O1 -g -Wall -Wno-macro-redefined -Wno-unused-function \
  -D__NEWTOS__ -D_TOS_ONLY -funsigned-char \
  -DCOOKIE_C="\"$OVL/library/utility/cookie.c\"" -DNOHARD_H="\"$OVL/nohard.h\"" \
  -I "$OVL" -I "$OVL/library/xdialog" -I "$OVL/library/utility" \
  -I "$TERA" -I "$TERA/library/xdialog" -I "$TERA/library/utility" \
  -I "$GT" -I "$MINT" \
  "$HERE/nohard-host.c" -o "$W/nohard-host" || exit 1

"$W/nohard-host"
