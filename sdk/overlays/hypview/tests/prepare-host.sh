#!/usr/bin/env bash
#
# prepare-host.sh -- build and run the host harness for HypView's picture
# loader (TASK/A142): prepare-host.c fakes the VDI and #includes the prepare.c
# under test unchanged.
#
# Default prepare.c: the patch sdk/overlays/hypview/hyp/prepare.c. PREPARE_SRC=<file>
# builds that file instead -- for the counter-check against the patch before
# A142 (`git show`), which handed the file's big-endian words to vr_trnfm.
# Upstream's own file does not compile for a host, it is m68k assembler there.
#
# Include path: the HypView tree, then gemlib; mintlib only AFTER the host's
# system headers (-idirafter), as ref-host.sh.
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
SRC="${PREPARE_SRC:-$SDK/overlays/hypview/hyp/prepare.c}"

CC="${CC:-cc}"
command -v "$CC" >/dev/null 2>&1 || { echo "error: $CC not found" >&2; exit 1; }
[ -f "$HV/hyp.h" ] || {
  echo "prepare-host: skipped -- vendored HypView source absent ($HV)"; exit 77; }
[ -f "$GT/mt_gem.h" ] || {
  echo "prepare-host: skipped -- overlaid gemlib tree absent ($GT); run build-gemlib.sh"; exit 77; }
[ -f "$MINT/osbind.h" ] || {
  echo "prepare-host: skipped -- mintlib headers absent ($MINT); run build-libc.sh"; exit 77; }
[ -f "$SRC" ] || { echo "error: $SRC missing" >&2; exit 1; }
SRC="$(cd "$(dirname "$SRC")" && pwd)/$(basename "$SRC")"
echo "prepare.c under test: $SRC"

W="$(mktemp -d)"
trap 'rm -rf "$W"' EXIT

# prepare.c includes "../diallib.h": -I "$HV/hyp" resolves it to the vendored
# header, whichever directory the file under test lies in.
"$CC" -O1 -g -fsanitize=address,undefined -fno-sanitize-recover=all \
  -Wall -Wno-macro-redefined -Wno-unused-function -Wno-parentheses-equality -Wno-unused-variable -Wno-unused-but-set-variable \
  -D__NEWTOS__ -D_GNU_SOURCE -DPREPARE_C="\"$SRC\"" \
  -I "$HV" -I "$HV/hyp" -I "$GT" -idirafter "$MINT" \
  "$HERE/prepare-host.c" -o "$W/prepare-host" || exit 1

"$W/prepare-host"
