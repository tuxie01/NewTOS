#!/bin/bash
# PORT12 Q3b: patched XGDOS portable modules + xgscr.c. Upstream stays unchanged: copy, apply sdk/overlays/xgdos/lp64-wire.patch.
# usage: build-xgscr.sh <repo> <xgdos clone> <scratch>   -> <scratch>/nt/XGSCR.NAP, <scratch>/mac/xgscr
set -eu
REPO=$1; X=$2; S=$3
M=$REPO/measurements/measurement-2026-11-06-port12/method
O=$REPO/sdk/overlays/xgdos
rm -rf "$S"; mkdir -p "$S/src" "$S/nt" "$S/mac"
cp "$X"/src/*.c "$X"/src/*.h "$S/src/"
(cd "$S" && patch -s -p0 < "$O/lp64-wire.patch")
MODS="xfix sintab charmap spd raster xstd heap config fontmgr gcache text vdi pdf soft mem meta sysfont fnt"
CC="$REPO/sdk/bin/newtos-cc -nap -DXG_WIRE64 -w -O1 -I$S/src"
objs=""
for s in $MODS lock; do $CC -c "$S/src/$s.c" -o "$S/nt/$s.o"; objs="$objs $S/nt/$s.o"; done
$CC -c "$O/xos_newtos.c" -o "$S/nt/xos_newtos.o"
$CC -c "$M/xgscr.c" -o "$S/nt/xgscr.o"
$CC -c "$M/xgscr_nt.c" -o "$S/nt/xgscr_nt.o"
$CC "$S/nt/xgscr.o" "$S/nt/xgscr_nt.o" "$S/nt/xos_newtos.o" $objs -o "$S/nt/XGSCR.NAP"
ls -la "$S/nt/XGSCR.NAP"
hs=""; for s in $MODS; do hs="$hs $S/src/$s.c"; done
cc -O1 -fno-builtin -DXG_WIRE64 -w -I"$S/src" -o "$S/mac/xgscr" "$M/xgscr.c" "$M/xgscr_fake.c" "$X/host/xos_host.c" $hs
