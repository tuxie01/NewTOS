#!/usr/bin/env bash
# all-runs.sh <scratch> <kernel8.img> -- the QEMU raspi4b runs of the ladder
# (TASK/PROC08 "Stand Leiter"). The card and the run driver are those of
# measurements/measurement-2026-11-04-proc08-screen, called, not copied:
# make-card.sh builds CWONE.NAP + CWKID.NAP and the ONE base image through
# tools/mkcard.sh into <scratch>/base.img; run-shot.sh boots an APFS clone
# (cp -c) of it per run with C:\CWCASE.TXT = the case, takes a QMP screendump
# after 45 s and 5 s later, and deletes the clone. Output: serial/<run>/
# (SERIAL.txt, run.txt, qmp.txt), screendumps/<run>.png.
#   l1-j-1080          ""          j (jump to a freed pool page: instruction abort, far = pc), 1920x1080, serial off
#   l2-j-1080-serial   "serial=on" j, 1920x1080: the serial ladder line, and the screen
#   l3-x-1080-serial   "serial=on" x (call $700: instruction abort, UXN; AT S1E0R ok), 1920x1080
set -euo pipefail
SCR=$1; K=$2
R=$(cd "$(dirname "$0")/.." && pwd)
REPO=$(cd "$R/../.." && pwd)
S=$REPO/measurements/measurement-2026-11-04-proc08-screen/method
bash "$S/make-card.sh" "$SCR"
cp "$SCR/base.img.manifest" "$R/base.img.manifest"
run() { bash "$S/run-shot.sh" "$SCR" "$R/serial/$1" "$R/screendumps" "$K" "$2" "$3" "$4" 45; }
run l1-j-1080 "" j 1920x1080
run l2-j-1080-serial "serial=on" j 1920x1080
run l3-x-1080-serial "serial=on" x 1920x1080
rm -f "$SCR/base.img"
