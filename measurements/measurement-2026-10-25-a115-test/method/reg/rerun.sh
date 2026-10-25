#!/usr/bin/env bash
# rerun.sh <side head|wt> <run> <scen> -- A115: one more run of a series.sh scenario under memprot on (default), tag <side>-onR<run>-<scen>
# (series.sh takes the memprot mode as its second argument, so a repeat needs its own tag).
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; . "$HERE/env.sh"
SIDE=$1; R=$2; sc=$3; K="$S/bin/kernel8-${SIDE}A.img"
export APPEND_EXTRA="" DL=0.5 QDELAY=1
case $sc in nd) C=gh1; TR=desktop;; td) C=td; TR=up:DESKTOP;; qed) C=qed; TR=up:QED;; std) C=std; TR=desktop; sc=nd;; esac
case $sc in nd) T=500;; *) T=200;; esac
TMO=$T bash "$HERE/drive.sh" "$K" "$S/cards/$C.img" "$SIDE-onR$R-$3" inj "$TR=$HERE/$sc.script" 2>&1 | tail -1
