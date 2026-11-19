#!/usr/bin/env bash
# pr-run.sh <side> <label> "<append>" -- PRESENT navigation run, screendumps keyed to the injector byte lines
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; . "$HERE/env.sh"; SIDE=$1; LBL=$2; APP=$3; SC=nav
ST=('\[inj\] up:PRESENT armed@1=2:arm.png')
while read -r n t; do N=$(python3 "$HERE/fx.py" "$HERE/$SC.script" "$t"); ST+=("\\[inj\\] up:PRESENT feed@$N=0.8:$n.png"); done < "$HERE/$SC.script.marks"
QDELAY=2 bash "$HERE/run.sh" "$S/bin/k-$SC.img" "$S/cards/pr-$SIDE.img" "$SIDE-$LBL" "$APP" "${ST[@]}"
