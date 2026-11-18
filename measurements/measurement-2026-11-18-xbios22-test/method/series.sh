#!/usr/bin/env bash
# series.sh <scratch> <side novdo|vdo> [script-name] -- one trace run: kernel8-trace[-<script>].img (build-trace.sh) on the
# <side> card; screendumps 2 s after the injector armed (TeraDesk's first evnt_multi: "desk") and 0.8 s after the byte
# closing each mark of <script>.script.marks. Tag trace-<side> for the default script vid, else <script>-<side>.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; S="$1"; SIDE="$2"; SC="${3:-vid}"
if [ "$SC" = vid ]; then K="$S/bin/kernel8-trace.img"; TAG="trace-$SIDE"; else K="$S/bin/kernel8-trace-$SC.img"; TAG="$SC-$SIDE"; fi
ST=('\[inj\] up:DESKTOP armed@1=2:desk.png')
while read -r n t; do N=$(python3 "$HERE/fx.py" "$HERE/$SC.script" "$t"); ST+=("\\[inj\\] up:DESKTOP feed@$N=0.8:$n.png"); done < "$HERE/$SC.script.marks"
S="$S" QDELAY=2 bash "$HERE/run.sh" "$K" "$S/cards/$SIDE.img" "$TAG" "${ST[@]}"
