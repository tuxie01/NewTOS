#!/usr/bin/env bash
#
# ref-host.sh -- build and run the host harness for HypView's REF reader
# (TASK/SDK04, probe S2 / finding 6 of the byte-order survey): ref-host.c
# fakes the GEMDOS file calls and #includes the ref.c under test unchanged.
#
# Default ref.c: the patch sdk/overlays/hypview/hyp/ref.c (patch site
# SDK04-S2). REF_SRC=<file> builds that file instead -- for the counter-check
# against the vendored sdk/apps/hypview/hyp/ref.c (magic read into an 8-byte
# little-endian `long`: every REF file refused).
#
# Include path as sdk/apps/build-hypview.sh: the upstream directory of ref.c
# (hyp/) so its quoted "../hyp.h" resolves, the HypView top, then gemlib;
# mintlib only AFTER the host's system headers (-idirafter), for the headers
# the host lacks (<osbind.h>, <compiler.h>, <mint/...>), so the harness's
# own stdio/stdlib/string are the host's.
#
# Exit status: the number of failed checks (0 = all passed); 77 = skipped
# (vendored HypView, gemlib tree or mintlib headers absent, or host not LP64
# little-endian); 1 = did not build.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"          # .../sdk/overlays/hypview/tests
SDK="$(cd "$HERE/../../.." && pwd)"                            # .../sdk
ROOT="$(cd "$SDK/.." && pwd)"
HV="$SDK/apps/hypview"
GT="$SDK/gemlib/gemlib-0.44.0"
MINT="$SDK/libc/mintlib/include"
SRC="${REF_SRC:-$SDK/overlays/hypview/hyp/ref.c}"

CC="${CC:-cc}"
command -v "$CC" >/dev/null 2>&1 || { echo "error: $CC not found" >&2; exit 1; }
[ -f "$HV/hyp/ref.c" ] || {
  echo "ref-host: skipped -- vendored HypView source absent ($HV)"; exit 77; }
[ -f "$GT/mt_gem.h" ] || {
  echo "ref-host: skipped -- overlaid gemlib tree absent ($GT); run build-gemlib.sh"; exit 77; }
[ -f "$MINT/osbind.h" ] || {
  echo "ref-host: skipped -- mintlib headers absent ($MINT); run build-libc.sh"; exit 77; }
[ -f "$SRC" ] || { echo "error: $SRC missing" >&2; exit 1; }
SRC="$(cd "$(dirname "$SRC")" && pwd)/$(basename "$SRC")"
echo "ref.c under test: $SRC"

W="$(mktemp -d)"
trap 'rm -rf "$W"' EXIT

"$CC" -O1 -g -fsanitize=address,undefined -fno-sanitize-recover=all \
  -Wall -Wno-macro-redefined -Wno-unused-function \
  -D__NEWTOS__ -D_GNU_SOURCE -DREF_C="\"$SRC\"" \
  -I "$HV/hyp" -I "$HV" -I "$GT" -idirafter "$MINT" \
  "$HERE/ref-host.c" -o "$W/ref-host" || exit 1

"$W/ref-host" "$ROOT"
