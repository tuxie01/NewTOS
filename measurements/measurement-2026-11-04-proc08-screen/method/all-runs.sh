#!/usr/bin/env bash
# all-runs.sh <scratch> <kernel8.img> -- the runs of this measurement, one
# crash screen each (run-shot.sh): every cwkid case at the Pi's 1920x1080
# with an empty command line (serial off), case d once more with serial=on,
# cases j and d at QEMU's default 640x480. Output into this run directory
# (runs/<name>/, screendumps/<name>.png).
set -euo pipefail
SCR=$1; K=$2
R=$(cd "$(dirname "$0")/.." && pwd)
run() { bash "$R/method/run-shot.sh" "$SCR" "$R/runs/$1" "$R/screendumps" "$K" "$2" "$3" "$4" 45; }
run s1-j-1080 "" j 1920x1080
run s2-d-1080 "" d 1920x1080
run s3-g-1080 "" g 1920x1080
run s4-w-1080 "" w 1920x1080
run s5-x-1080 "" x 1920x1080
run s6-d-1080-serial "serial=on" d 1920x1080
run s7-j-640 "" j 640x480
run s8-d-640 "" d 640x480
