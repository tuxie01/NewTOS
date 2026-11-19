#!/usr/bin/env bash
# zv-run.sh <sample> <label v16|v32> "<append>" -- zView through ZVLAUNCH (#Z autorun, shel_write SHW_EXEC with the sample on the tail)
# on the scratch vditrace kernel (bios-arg-trace + the scratch VDI decoder), screendumps 25 s and 45 s after "starting C:\ZVIEW\ZVIEW.NAP".
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; S="${S:?}"; N=$1; L=$2; APP=$3
QDELAY=1 bash "$HERE/run.sh" "$S/bin/k-vditrace.img" "$S/cards/zv-$N.img" "zv-$N-$L" "$APP" \
  'starting C:.ZVIEW.ZVIEW@1=25:z25.png' 'starting C:.ZVIEW.ZVIEW@1=20:z45.png'
