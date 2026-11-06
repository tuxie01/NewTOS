#!/bin/bash
# icrun.sh <tag> <kernel-name>...: icount runs, 5 at a time
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/v44t
cd /Volumes/Daten/Development/PiRusTOS
M=measurements/measurement-2026-11-06-v44-test
export TXB_NAP=$S/txb/TXBENCH.NAP TXB_SCR=$S
tag=$1; shift; i=0
for n in "$@"; do
  $M/method/txbench-run.sh $S/runs/$tag-$n $S/kernels/$n.img -icount shift=0,sleep=off >/dev/null 2>&1 &
  i=$((i+1)); [ $((i%5)) = 0 ] && wait
done; wait
