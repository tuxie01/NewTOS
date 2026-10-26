#!/usr/bin/env bash
# (A115: copied from measurements/measurement-2026-10-21-gh1-reg/method/series.sh; outputs one level up from method/reg/)
# series.sh <side head|wt> <mode on|off|strict|verify> [scen...] -- the four injector scenarios (nd td qed hv) of one kernel side under one memprot mode, one QEMU at a time.
# tag = <side>-<mode>-<scen>; dumps -> ../screendumps/<tag>-<mark>.png, serial -> ../serial/<tag>-serial.log. Kernel $S/bin/kernel8-<side>A.img, cards $S/cards.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; . "$HERE/env.sh"
SIDE=$1; MODE=$2; shift 2; SC="${*:-nd td qed hv}"
K="$S/bin/kernel8-${SIDE}A.img"
case $MODE in on) EXTRA="";; *) EXTRA="memprot=$MODE";; esac
export APPEND_EXTRA="$EXTRA" DL=0.5 QDELAY=1
for sc in $SC; do
  case $sc in nd) C=gh1; TR=desktop;; td) C=td; TR=up:DESKTOP;; qed) C=qed; TR=up:QED;; hv) C=hv; TR=up:HYP_VIEW;; esac
  case $sc in nd) T=500;; *) T=200;; esac
  TMO=$T bash "$HERE/drive.sh" "$K" "$S/cards/$C.img" "$SIDE-$MODE-$sc" inj "$TR=$HERE/$sc.script" 2>&1 | tail -1
done
