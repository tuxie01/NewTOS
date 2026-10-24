#!/bin/sh
# regress2.sh <prefix> <kernel.bin>: button click + key repeat on default and extra topology
. "$(dirname "$0")/env.sh"
HERE=$(cd "$(dirname "$0")" && pwd)
P=$1; K=$2
export SCR=$S/run
for topo in default extra; do
  sh $HERE/run-virt.sh $K $HERE/../serial/$P-$topo-btn.log 150 $topo $HERE/qmp_btn.py $S/shots $P-$topo-btn 40 10 40 30
  sh $HERE/run-virt.sh $K $HERE/../serial/$P-$topo-hold.log 150 $topo $HERE/qmp_hold.py $S/shots $P-$topo-hold
done
