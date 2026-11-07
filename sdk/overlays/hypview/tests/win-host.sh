#!/usr/bin/env bash
#
# win-host.sh -- build and run the host harness for HypView's window list
# (TASK/A140, GitHub #22): win-host.c fakes the AES and the GEMDOS block calls
# and #includes the dl_win.c under test unchanged.
#
# Default dl_win.c: the patch sdk/overlays/hypview/dl_win.c. DLWIN_SRC=<file>
# builds that file instead -- for the counter-check against the vendored
# sdk/apps/hypview/dl_win.c (the global `Win` stays on the freed window).
#
# Include path as sdk/apps/build-hypview.sh: the HypView top, then gemlib;
# mintlib only AFTER the host's system headers (-idirafter), as ref-host.sh.
#
# Exit status: the number of failed checks (0 = all passed); 77 = skipped
# (vendored HypView, gemlib tree or mintlib headers absent, or host not LP64
# little-endian); 1 = did not build.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"          # .../sdk/overlays/hypview/tests
SDK="$(cd "$HERE/../../.." && pwd)"                            # .../sdk
HV="$SDK/apps/hypview"
GT="$SDK/gemlib/gemlib-0.44.0"
MINT="$SDK/libc/mintlib/include"
SRC="${DLWIN_SRC:-$SDK/overlays/hypview/dl_win.c}"

CC="${CC:-cc}"
command -v "$CC" >/dev/null 2>&1 || { echo "error: $CC not found" >&2; exit 1; }
[ -f "$HV/dl_win.c" ] || {
  echo "win-host: skipped -- vendored HypView source absent ($HV)"; exit 77; }
[ -f "$GT/mt_gem.h" ] || {
  echo "win-host: skipped -- overlaid gemlib tree absent ($GT); run build-gemlib.sh"; exit 77; }
[ -f "$MINT/osbind.h" ] || {
  echo "win-host: skipped -- mintlib headers absent ($MINT); run build-libc.sh"; exit 77; }
[ -f "$SRC" ] || { echo "error: $SRC missing" >&2; exit 1; }
SRC="$(cd "$(dirname "$SRC")" && pwd)/$(basename "$SRC")"
echo "dl_win.c under test: $SRC"

W="$(mktemp -d)"
trap 'rm -rf "$W"' EXIT

"$CC" -O1 -g -fsanitize=address,undefined -fno-sanitize-recover=all \
  -Wall -Wno-macro-redefined -Wno-unused-function -Wno-parentheses-equality -Wno-unused-variable \
  -D__NEWTOS__ -D_GNU_SOURCE -DDLWIN_C="\"$SRC\"" \
  -I "$HV" -I "$GT" -idirafter "$MINT" \
  "$HERE/win-host.c" -o "$W/win-host" || exit 1

"$W/win-host"
