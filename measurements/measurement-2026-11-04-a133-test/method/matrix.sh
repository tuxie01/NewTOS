#!/usr/bin/env bash
# matrix.sh <scratch-t3> <outdir> -- A133-test point 4: head (fd6c7e3f) vs base (3b1fd066), raspi4b,
# 1920x1080 and 640x480, one QEMU at a time via the A129 run.sh (per-run APFS clone, serial=on).
set -uo pipefail
S=$1; OUT=$2
HERE=$(cd "$(dirname "$0")" && pwd); ROOT=$(cd "$HERE/../../.." && pwd)
RUN=$ROOT/measurements/measurement-2026-11-04-a129-measure/method/run.sh
mkdir -p "$S/mx"
for geom in 1920x1080 640x480; do
 for side in base head; do
  P=$S/k-$side-rpi4.img; I=$S/ki-$side.img; G=${geom%x*}
  r() { local k=$1 card=$2 name=$3; shift 3; "$RUN" "$k" "$card" "$side-$G-$name" "$OUT" "$S/mx" "$geom" "$@"; }
  r "$P" "$S/r133/boot/card.img" boot 30 45
  r "$P" "$S/r133/fs/card.img" fs 55
  r "$P" "$S/al/card.img" alert 45
  r "$P" "$S/karte/card-karte.img" karte 45
  r "$P" "$S/karte/card-zview.img" zview 60
  r "$P" "$S/karte/card-qed.img" qed 60
  r "$I" "$S/r133/boot/card.img" about 40 50
  r "$I" "$S/karte/card-zview.img" zfsel 60 75
  r "$I" "$S/r133/fs/card.img" fedit 40 55
 done
done
echo MATRIX-DONE
