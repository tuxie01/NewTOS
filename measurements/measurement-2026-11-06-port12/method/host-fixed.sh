#!/bin/bash
# PORT12 Q1/Q3a: upstream host tests on the Mac with method/assign-fonts.sys (fixed font order), for the byte compare with NewTOS.
# usage: host-fixed.sh <xgdos clone> <scratch dir> <out dir> [extra cc flags]
set -u
R=$1; S=$2; OUT=$3; shift 3; EXTRA="$*"
M=$(cd "$(dirname "$0")" && pwd)
mkdir -p "$OUT"; rm -rf "$S"; mkdir -p "$S"; (cd "$R" && tar cf - --exclude=.git .) | tar xf - -C "$S"
cd "$S" || exit 1
HOSTSRC=""; for s in xfix sintab charmap spd raster xstd heap config fontmgr gcache text vdi pdf soft mem meta sysfont; do HOSTSRC="$HOSTSRC src/$s.c"; done
cp "$R"/GEMSYS/*.SPD host/hostfs/C/FONTS/
cp "$M/assign-fonts.sys" host/hostfs/C/ASSIGN.SYS
cc -O1 -DXG_HOST -w $EXTRA -o host/test_vdi host/test_vdi.c host/xos_host.c host/hostvdi.c $HOSTSRC; echo "cc test_vdi rc=$?"
cc -O1 -DXG_HOST -w $EXTRA -o host/test_dev host/test_dev.c host/xos_host.c host/hostvdi.c $HOSTSRC; echo "cc test_dev rc=$?"
cd host; ./test_vdi > "$OUT/XGTVDI.OUT" 2>&1; rc=$?; echo "rc=$rc" >> "$OUT/XGTVDI.OUT"; echo "test_vdi rc=$rc"
./test_dev > "$OUT/XGTDEV.OUT" 2>&1; rc=$?; echo "rc=$rc" >> "$OUT/XGTDEV.OUT"; echo "test_dev rc=$rc"
cp out_screen.pgm "$OUT/OUTSCR.PGM"; cp out_memory.pgm "$OUT/OUTMEM.PGM"
cp hostfs/C/TEST.GEM hostfs/C/GEMFILE.GEM hostfs/C/PRINTS/*.PDF "$OUT/"
