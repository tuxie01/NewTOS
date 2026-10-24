#!/bin/sh
# regress.sh <tag> <kernel.img>: qemu-virt mouse, Ctrl+Z (keyboard) and key-repeat runs on the default
# topology (= cargo xtask run) and extra (= cargo xtask run --usb-extra). Serial -> ../serial/<tag>-<topo>-*.log,
# screendumps -> $SHOTS (job scratch; the caller copies the evidence PNGs). Copied from
# measurements/measurement-2026-10-24-usb02-10-11/method; USB02 item 10 adds the roll run (qmp_roll.py, default topology).
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
T=$1; K=$2
: "${SCR:?set SCR to a job-scratch run dir}"; : "${SHOTS:?set SHOTS to a job-scratch shot dir}"
export SCR; mkdir -p "$SHOTS"
for topo in default extra; do
  sh $HERE/run-virt.sh $K $HERE/../serial/$T-$topo-mouse.log 150 $topo $HERE/qmp_mouse.py $SHOTS $T-$topo-mouse
  sh $HERE/run-virt.sh $K $HERE/../serial/$T-$topo-key.log 150 $topo $HERE/qmp_key.py $SHOTS $T-$topo-key -
  sh $HERE/run-virt.sh $K $HERE/../serial/$T-$topo-hold.log 150 $topo $HERE/qmp_hold.py $SHOTS $T-$topo-hold
done
sh $HERE/run-virt.sh $K $HERE/../serial/$T-default-roll.log 150 default $HERE/qmp_roll.py $SHOTS $T-default-roll
