#!/usr/bin/env bash
# zv-matrix.sh -- zView on both samples at the default and at vdiplanes=32 (zv-run.sh)
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; . "$HERE/env.sh"
for n in T24.BMP TRGB.PNG; do
  bash "$HERE/zv-run.sh" $n v16 "serial=on"
  bash "$HERE/zv-run.sh" $n v32 "serial=on vdiplanes=32"
done
echo ZVDONE
