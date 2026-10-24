#!/bin/sh
# regress.sh <tag-prefix> <kernel.bin>: mouse + keyboard run on every topology, serial logs -> ../serial/<prefix>-<topo>-{mouse,key}.log
. "$(dirname "$0")/env.sh"
HERE=$(cd "$(dirname "$0")" && pwd)
P=$1; K=$2
export SCR=$S/run
mkdir -p $S/shots
for topo in default extra hubtree mtp overfull; do
  sh $HERE/run-virt.sh $K $HERE/../serial/$P-$topo-mouse.log 150 $topo $HERE/qmp_mouse.py $S/shots $P-$topo-mouse
  sh $HERE/run-virt.sh $K $HERE/../serial/$P-$topo-key.log 150 $topo $HERE/qmp_key.py $S/shots $P-$topo-key -
done
