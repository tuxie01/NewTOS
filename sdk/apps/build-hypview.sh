#!/usr/bin/env bash
#
# build-hypview.sh -- compile HypView (freemint/hypview, the FreeMiNT
# ST-Guide hypertext viewer, which also shows plain ASCII files) into a native
# NewTOS GEM application HYP_VIEW.NAP with sdk/bin/newtos-cc (TASK/SDK01 H:
# a port made from docs/sdk/ only).
#
# Upstream: https://github.com/freemint/hypview @ 6f3eeeb (2026-06-18),
# vendored unchanged under sdk/apps/hypview/ (untracked); provenance and
# archive hash in sdk/apps/hypview-sources.sha256.
#
# Upstream's gcc build (Makefile, Makefile.objs, */SRCFILES) compiles the
# COBJS of the top directory and of hyp/, plain/, dragdrop/, bubble/ (bgh.c)
# with -D_GNU_SOURCE -O2 -fomit-frame-pointer, -I<top>, and links -lgem.
# hyp/gfx.s is the Pure C build's assembler module and is not part of the gcc
# build. We keep -D_GNU_SOURCE and -O2 -fomit-frame-pointer, drop the m68k
# CPU flags, and let newtos-cc add the NewTOS target, mintlib and gemlib.
#
# PATCHES: a file under sdk/overlays/hypview/<path> is compiled instead of
# upstream <path>; upstream's directory of that file stays on the include
# path so its quoted relative includes ("../diallib.h") still resolve. Each
# patch file's header says why it exists.
#
# RESOURCE: hyp_view.rsc (English, = hyp_view/en.rsc), patched by
# sdk/overlays/hypview/rsc/recolour-cicons.py into
# build/hyp_view-recoloured.rsc (its 8-plane colour icons were drawn for the
# NVDI palette and are remapped to the TOS one, TASK/A141), then converted
# with tools/mkrsc.py into build/hyp_view.rsc, to sit beside HYP_VIEW.NAP.
#
# Usage:  sdk/apps/build-hypview.sh      -> sdk/apps/hypview/build/
set -uo pipefail

SDK="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="$(cd "$SDK/.." && pwd)"
SRC="$SDK/apps/hypview"
OVL="$SDK/overlays/hypview"
BUILD="$SRC/build"
OBJ="$BUILD/obj"
CC="$SDK/bin/newtos-cc"

[ -f "$SRC/hyp_view.c" ] || { echo "HypView source missing in $SRC (see sdk/apps/hypview-sources.sha256)" >&2; exit 1; }

# the manifest: upstream's own COBJS lists, expanded with make
cobjs() { # $1 = directory relative to $SRC
    make -s -f - -C "$SRC/$1" <<'EOF'
include SRCFILES
all: ; @echo $(COBJS)
EOF
}

FILES=""
for d in . hyp plain dragdrop bubble; do
    for c in $(cobjs "$d"); do
        if [ "$d" = . ]; then FILES="$FILES $c"; else FILES="$FILES $d/$c"; fi
    done
done
[ -n "$FILES" ] || { echo "empty source manifest" >&2; exit 1; }

FLAGS="-nap -D_GNU_SOURCE -O2 -fomit-frame-pointer -Wall -I$SRC"

rm -rf "$OBJ"; mkdir -p "$OBJ"
fail=0; n=0; npatch=0
OBJS=""
for f in $FILES; do
    src="$SRC/$f"
    if [ -e "$OVL/$f" ]; then src="$OVL/$f"; npatch=$((npatch + 1)); fi
    o="$OBJ/$(echo "$f" | tr / _).o"
    if ! "$CC" $FLAGS -I"$(dirname "$SRC/$f")" -c "$src" -o "$o"; then
        echo "FAILED: $f ($src)" >&2; fail=1
    fi
    n=$((n + 1)); OBJS="$OBJS $o"
done
[ "$fail" = 0 ] || { echo "compile failures -- not linking" >&2; exit 1; }
echo "compiled $n files ($npatch from sdk/overlays/hypview)"

"$CC" -nap -o "$BUILD/HYP_VIEW.NAP" $OBJS -Wl,-Map="$BUILD/hyp_view.map" || exit 1
ls -l "$BUILD/HYP_VIEW.NAP"

python3 "$OVL/rsc/recolour-cicons.py" recolour "$SRC/hyp_view.rsc" "$BUILD/hyp_view-recoloured.rsc" \
    > "$BUILD/recolour-cicons.log" || exit 1
head -2 "$BUILD/recolour-cicons.log"
python3 "$ROOT/tools/mkrsc.py" convert "$BUILD/hyp_view-recoloured.rsc" "$BUILD/hyp_view.rsc" || exit 1
ls -l "$BUILD/hyp_view.rsc"
