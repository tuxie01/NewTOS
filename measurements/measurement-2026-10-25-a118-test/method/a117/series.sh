#!/usr/bin/env bash
# series.sh <side base|new> <n> -- A115: the A117 method (measurements/measurement-2026-10-25-a117) on one kernel side, run n:
#   a117   NEWDESK + A117ACC.NAC: Desk -> "A117 probe" (AC_OPEN), Desk -> "Desktop info..." twice (a117.script, armed on desktop)
#   a117td TeraDesk + A117ACC.NAC: Desk -> "A117 probe", File -> "Quit/Shutdown" (td.script, armed on up:DESKTOP)
# Serial -> ../../serial/<side>-a117-<n>.log / <side>-a117td-<n>.log, dumps -> ../../screendumps/. Card clones deleted after each run.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; . "$HERE/env.sh"
SIDE=$1; N=$2; SER="$HERE/../../serial"; SHOT="$HERE/../../screendumps"
python3 "$HERE/run.py" "$S/bin/kernel8-$SIDE-a117.img" "$S/cards/a117.img" "$SIDE-a117-$N" "$SER" "$SHOT" 60 \
  510:1.5:accopen 1710:1.5:pick1 2510:1.5:ret1 3210:1.5:pick2 4010:1.5:ret2
rm -rf "$S/cards/run-$SIDE-a117-$N"
python3 "$HERE/run.py" "$S/bin/kernel8-$SIDE-a117td.img" "$S/cards/a117td.img" "$SIDE-a117td-$N" "$SER" "$SHOT" 75 \
  510:1.5:accopen 1710:2:quit
rm -rf "$S/cards/run-$SIDE-a117td-$N"
