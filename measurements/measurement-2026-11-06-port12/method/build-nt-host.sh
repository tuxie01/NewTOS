#!/bin/bash
# PORT12 Q2/Q3a: the upstream host test programs built with the NewTOS SDK, source unchanged,
# against upstream's fake VDI (hostvdi.c) and stdio OS layer (xos_host.c). No kernel VDI involved.
# usage: build-nt-host.sh <repo> <xgdos clone> <outdir> [extra flags]
set -eu
REPO=$1; X=$2; OUT=$3; shift 3
CC="$REPO/sdk/bin/newtos-cc -ncp -DXG_HOST -w -O1 $*"
M=$REPO/measurements/measurement-2026-11-06-port12/method
mkdir -p "$OUT/o"
objs=""
for s in xfix sintab charmap spd raster xstd heap config fontmgr gcache text vdi pdf soft mem meta sysfont; do
  $CC -c "$X/src/$s.c" -o "$OUT/o/$s.o"; objs="$objs $OUT/o/$s.o"
done
$CC -Dfopen=nt_fopen -c "$X/host/hostvdi.c" -o "$OUT/o/hostvdi.o"
$CC -c "$X/host/xos_host.c" -o "$OUT/o/xos_host.o"
for t in vdi dev; do
  T=$(echo $t | tr a-z A-Z)
  $CC -Dmain=xg_test_main -Dfopen=nt_fopen -c "$X/host/test_$t.c" -o "$OUT/o/test_$t.o"
  $CC "-DOUTFILE=\"C:\\\\XGT$T.OUT\"" -c "$M/nt-wrap.c" -o "$OUT/o/wrap_$t.o"
  $CC "$OUT/o/wrap_$t.o" "$OUT/o/test_$t.o" "$OUT/o/hostvdi.o" "$OUT/o/xos_host.o" $objs -o "$OUT/XGT$T.NCP"
done
ls -la "$OUT"/*.NCP
