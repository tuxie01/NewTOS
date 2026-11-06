#!/bin/bash
# PORT12 Q2: every upstream source through newtos-cc, unpatched, warnings on, one file each. usage: <repo> <xgdos> <obj dir>
REPO=$1; X=$2; O=$3
mkdir -p "$O"
for f in spd raster text fontmgr gcache heap pdf soft charmap sysfont fnt meta xfix sintab xstd config vdi mem lock main xos_tos drv atari.S traps.S crt0.S; do
  case $f in *.S) src=$X/src/$f; o=$O/${f%.S}_S.o;; *) src=$X/src/$f.c; o=$O/$f.o;; esac
  echo "=== $f"
  "$REPO/sdk/bin/newtos-cc" -ncp -Wall -Wextra -Wno-unused-parameter -Wno-sign-compare -Wno-misleading-indentation -Wshorten-64-to-32 -Wpointer-to-int-cast -Wint-to-pointer-cast -c "$src" -o "$o" 2>&1 | /usr/bin/grep -E 'error|warning: ' | sed "s#$X/src/##" | cut -c1-170
  echo "rc ${PIPESTATUS[0]}"
done
