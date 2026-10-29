#!/usr/bin/env bash
#
# build-cookie-probe.sh -- build TDCOOKIE.NCP, the on-target probe for
# TeraDesk's find_cookie() (TASK/SDK04, finding C6; probe source
# cookie-probe.c beside this script).
#
# The probe links TeraDesk's library/utility/cookie.c compiled UNCHANGED with
# the flags of sdk/apps/build-teradesk.sh (same CFLAGS, same shadow-tree
# include order), so the find_cookie() it calls is the one in DESKTOP.NAP:
#   - default: the file the TeraDesk build uses -- the overlay
#     sdk/overlays/teradesk/library/utility/cookie.c if it exists, else the
#     vendored sdk/apps/teradesk/library/utility/cookie.c;
#   - a second argument names another cookie.c (e.g. the vendored file, for
#     the before side of a before/after run).
# The shadow tree is built the way build-teradesk.sh builds it (vendored
# files symlinked, overlay files layered on top; this tests/ directory is not
# part of it), in a temporary directory that is removed afterwards.
#
# Link: crt0 + cookie-probe.o + cookie.o + gemlib.a/libm.a/libc.a (as
# build-teradesk.sh links DESKTOP.NAP; Getcookie and Ssystem come from libc).
#
# Usage:  sdk/overlays/teradesk/tests/build-cookie-probe.sh <out.NCP> [cookie.c]
# Prints the cookie.c used and its sha256, and the sha256 of the output.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"          # .../sdk/overlays/teradesk/tests
SDK="$(cd "$HERE/../../.." && pwd)"                            # .../sdk
TERA="$SDK/apps/teradesk"
OVL="$SDK/overlays/teradesk"
GEM="$SDK/gemlib"; LIBC="$SDK/libc"; LIBM="$SDK/libm"
GEMTOP="$GEM/gemlib-0.44.0"; LIBMTOP="$LIBM/openlibm-0.8.8"; MINT="$LIBC/mintlib/include"

OUT="${1:?usage: build-cookie-probe.sh <out.NCP> [cookie.c]}"
if [ -n "${2:-}" ]; then
  SRC="$2"
elif [ -f "$OVL/library/utility/cookie.c" ]; then
  SRC="$OVL/library/utility/cookie.c"
else
  SRC="$TERA/library/utility/cookie.c"
fi
SRC="$(cd "$(dirname "$SRC")" && pwd)/$(basename "$SRC")"

[ -d "$TERA" ]   || { echo "error: TeraDesk source missing ($TERA)" >&2; exit 1; }
[ -f "$SRC" ]    || { echo "error: $SRC missing" >&2; exit 1; }
for f in "$GEM/gemlib.a" "$LIBM/libm.a" "$LIBC/libc.a"; do
  [ -f "$f" ] || { echo "error: $f missing; run sdk/apps/build-teradesk.sh once (lib_ensure)" >&2; exit 1; }
done

CLANG="${CLANG:-clang}"
RES="$($CLANG -print-resource-dir)/include"
. "$SDK/toolchain.sh"
LLD="${LLD:-$(newtos_tool ld.lld)}"

# The CFLAGS of sdk/apps/build-teradesk.sh, unchanged.
CFLAGS=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -Os
        -fomit-frame-pointer -fPIE -fno-builtin -nostdinc -funsigned-char
        -D__NEWTOS__ -D_TOS_ONLY -Wno-macro-redefined)

W="$(mktemp -d)"
trap 'rm -rf "$W"' EXIT
TREE="$W/tree"
# Shadow tree as in build-teradesk.sh: vendored .c/.h symlinked, overlays on top.
( cd "$TERA" && find . -type d ! -path './build*' ! -path './.git*' ) | \
    while IFS= read -r d; do mkdir -p "$TREE/$d"; done
( cd "$TERA" && find . -type f \( -name '*.c' -o -name '*.h' \) ! -path './build*' ) | \
    while IFS= read -r f; do ln -s "$TERA/${f#./}" "$TREE/${f#./}"; done
( cd "$OVL" && find . -type f \( -name '*.c' -o -name '*.h' \) ! -path './tests/*' ) | \
    while IFS= read -r f; do f="${f#./}"; mkdir -p "$TREE/$(dirname "$f")"; ln -sf "$OVL/$f" "$TREE/$f"; done
ln -sf "$SRC" "$TREE/library/utility/cookie.c"

INCS=(-I"$TREE" -I"$TREE/library/xdialog" -I"$TREE/library/utility"
      -I"$GEMTOP" -I"$MINT" -I"$LIBM/include" -I"$LIBMTOP/include" -isystem "$RES")

echo "cookie.c: $SRC"
echo "          $(shasum -a 256 "$SRC" | cut -c1-64)"
"$CLANG" "${CFLAGS[@]}" "${INCS[@]}" -c "$TREE/library/utility/cookie.c" -o "$W/cookie.o" || exit 1
"$CLANG" "${CFLAGS[@]}" "${INCS[@]}" -Wall -c "$HERE/cookie-probe.c" -o "$W/probe.o" || exit 1
"$CLANG" "${CFLAGS[@]}" -I"$SDK/include" -I"$MINT" -c "$LIBC/crt0.S" -o "$W/crt0.o" || exit 1
mkdir -p "$(dirname "$OUT")"
newtos_lld -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
    --build-id=none "$W/crt0.o" "$W/probe.o" "$W/cookie.o" \
    --start-group "$GEM/gemlib.a" "$LIBM/libm.a" "$LIBC/libc.a" --end-group \
    -o "$OUT" || exit 1
echo "probe   : $OUT"
echo "          $(shasum -a 256 "$OUT" | cut -c1-64)"
