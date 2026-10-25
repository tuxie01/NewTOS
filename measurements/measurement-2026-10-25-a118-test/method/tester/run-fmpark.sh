#!/usr/bin/env bash
# run-fmpark.sh <card|run <side> <n>> -- Tester: shell-loop fm_error (#Z C:\NOPE.NAP) while an accessory (A118DL.NAC, startup hold)
# holds the window-tree lock for 6 s. EmuTOS: the alert waits (wm_update(BEG_UPDATE) in fm_alert) until END_UPDATE.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; . "$HERE/../reg/env.sh"
case "$1" in
  card)
    mkdir -p "$S/userbin" "$S/cards"
    bash "$WS/sdk/build.sh" "$HERE/a118dl.c" "$S/userbin/A118DL.NAC" >/dev/null
    bash "$WS/sdk/build.sh" "$HERE/a118dl-nohold.c" "$S/userbin/A118DLN.NAC" >/dev/null
    bash "$HERE/../reg/mkcard.sh" "$S/cards/fmpark.img" "$HERE/../fmerr/z-nope.inf" ACC/A118DL.NAC="$S/userbin/A118DL.NAC" >/dev/null
    TD="$WS/sdk/apps/teradesk/build/dist"
    TDF=(Apps/TERADESK/DESKTOP.NAP="$TD/DESKTOP.NAP" Apps/TERADESK/desktop.rsc="$TD/desktop.rsc" Apps/TERADESK/icons.rsc="$TD/icons.rsc"
         Apps/TERADESK/cicons.rsc="$TD/cicons.rsc" Apps/TERADESK/TERADESK.INF="$HERE/../reg/TERADESK.INF")
    bash "$HERE/../a117/mkcard.sh" "$S/cards/dltd.img" "$HERE/../a117/td.inf" ACC/A118DL.NAC="$S/userbin/A118DLN.NAC" "${TDF[@]}" >/dev/null
    bash "$HERE/../a117/mkcard.sh" "$S/cards/dltdh.img" "$HERE/../a117/td.inf" ACC/A118DL.NAC="$S/userbin/A118DL.NAC" "${TDF[@]}" >/dev/null
    shasum -a 256 "$S/cards/fmpark.img" "$S/cards/dltd.img" "$S/cards/dltdh.img" "$S/userbin/A118DL.NAC" "$S/userbin/A118DLN.NAC";;
  run)
    SIDE=$2; N=$3
    python3 "$HERE/timed.py" "$S/bin/kernel8-$SIDE.img" "$S/cards/fmpark.img" "$SIDE-fmpark-$N" "$HERE/../../serial" "$HERE/../../screendumps" 40 \
      2:t02 4:t04 6:t06 8:t08 10:t10 14:t14 20:t20 30:t30
    rm -rf "$S/cards/run-$SIDE-fmpark-$N";;
  tdq)  # run-fmpark.sh tdq <side> <n> <card dltd|dltdh>: TeraDesk quit with A118DL answering AC_CLOSE through BEG_UPDATE
    SIDE=$2; N=$3; CARD=$4
    python3 "$HERE/../a117/run.py" "$S/bin/kernel8-$SIDE-tdq.img" "$S/cards/$CARD.img" "$SIDE-$CARD-tdq-$N" "$HERE/../../serial" "$HERE/../../screendumps" 120 \
      1710:2:quitdlg 2710:0.5:onquit 2710:6:after 2710:12:after12
    rm -rf "$S/cards/run-$SIDE-$CARD-tdq-$N";;
esac
