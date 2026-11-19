#!/usr/bin/env bash
# td-open.sh <side wt|main> <planes label> "<append>" <kernel> <script-name> -- TeraDesk Options>Video, screendumps keyed to the injector's byte lines
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; S="${S:?}"; SIDE=$1; LBL=$2; APP=$3; K=$4; SC=$5
ST=('\[inj\] up:DESKTOP armed@1=2:desk.png')
while read -r n t; do N=$(python3 "$HERE/fx.py" "$HERE/$SC.script" "$t"); ST+=("\\[inj\\] up:DESKTOP feed@$N=0.8:$n.png"); done < "$HERE/$SC.script.marks"
QDELAY=2 bash "$HERE/run.sh" "$K" "$S/cards/td-$SIDE.img" "$SC-$SIDE-$LBL" "$APP" "${ST[@]}"
