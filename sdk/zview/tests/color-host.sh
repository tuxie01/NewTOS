#!/usr/bin/env bash
#
# color-host.sh -- build and run the host harness for zView's palette matcher
# (TASK/SDK04, probe S3 / finding 8 of the byte-order survey): color-host.c
# #includes the color.c under test unchanged; this script gives it a scratch
# tree whose general.h, pic_load.h and zvdi/vdi.h include color-host-stubs.h
# (color.c includes them by relative path), and the int prelude
# build-zview.sh uses (sdk/zview/include/zview-ints.h, int32 = 32 bits).
#
# Default color.c: the patch sdk/zview/overlay/zview/zvdi/color.c (patch
# site SDK04-S3). ZVIEW_COLOR_SRC=<file> builds that file instead -- for the
# counter-check against the extracted z-tools tree's zview/zvdi/color.c
# (big-endian byte view of an int32 and of SRGB).
#
# Exit status: the number of failed checks (0 = all passed); 77 = skipped
# (z-tools tree absent, or host not little-endian); 1 = did not build.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"          # .../sdk/zview/tests
ZV="$(cd "$HERE/.." && pwd)"                                   # .../sdk/zview
SRC="${ZVIEW_COLOR_SRC:-$ZV/overlay/zview/zvdi/color.c}"

CC="${CC:-cc}"
command -v "$CC" >/dev/null 2>&1 || { echo "error: $CC not found" >&2; exit 1; }
[ -f "$SRC" ] || {
  echo "color-host: skipped -- $SRC absent (run sdk/zview/build-zview.sh for the z-tools tree)"; exit 77; }
SRC="$(cd "$(dirname "$SRC")" && pwd)/$(basename "$SRC")"
echo "color.c under test: $SRC"

W="$(mktemp -d)"
trap 'rm -rf "$W"' EXIT
mkdir -p "$W/zview/zvdi"
cp "$SRC" "$W/zview/zvdi/color.c"
for h in zview/general.h zview/pic_load.h zview/zvdi/vdi.h; do
  printf '#include "%s"\n' "$HERE/color-host-stubs.h" > "$W/$h"
done

# -fno-sanitize=shift-base: upstream's `(int32)satur << 24` (satur up to 255)
# overflows int32 by design -- m68k gcc and the target's clang both wrap.
"$CC" -O1 -g -fsanitize=address,undefined -fno-sanitize=shift-base -fno-sanitize-recover=all \
  -fsigned-char -Wall -Wno-unused-function \
  -DZVIEW_INTS_H="\"$ZV/include/zview-ints.h\"" \
  -DZVIEW_COLOR_C="\"$W/zview/zvdi/color.c\"" \
  "$HERE/color-host.c" -o "$W/color-host" || exit 1

"$W/color-host"
