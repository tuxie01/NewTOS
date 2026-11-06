#!/bin/bash
# txwall.sh <rounds> <kernel-name>...: txbench without icount, kernels by turns, one at a time, from the repo root.
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad
M=measurements/measurement-2026-11-06-v41
export TXB_NAP=$S/v41/txb/TXBENCH.NAP TXB_SCR=$S/v41
n=$1; shift
for i in $(seq 1 $n); do for k in "$@"; do
  rm -rf $S/v41/tb/w$i-$k
  $M/method/txbench-run.sh $S/v41/tb/w$i-$k $S/v41/k/$k.img 2>&1 | /usr/bin/grep -E "ticks200" | /usr/bin/sed -E 's/ calls.*ticks200=/=/' | tr '\n' ' ' | /usr/bin/sed "s/^/$k-$i: /"; echo "load $(uptime | /usr/bin/sed 's/.*averages: //')"
done; done
