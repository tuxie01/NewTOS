#!/bin/bash
# icrun.sh <tag> <kernel-name>...: icount runs, 5 at a time
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/v44
cd /Volumes/Daten/Development/PiRusTOS
export TXB_NAP=$S/txb/TXBENCH.NAP TXB_SCR=$S
tag=$1; shift; i=0
for n in "$@"; do
  $S/scripts/txbench-run.sh $S/runs/$tag-$n $S/kernels/$n.img -icount shift=0,sleep=off >/dev/null 2>&1 &
  i=$((i+1)); [ $((i%5)) = 0 ] && wait
done; wait
