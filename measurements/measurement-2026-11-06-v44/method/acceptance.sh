#!/bin/bash
# V44 acceptance: txbench without icount (wall clock, _hz_200), kernels by turns, one at a time.
# acceptance.sh <repeats> <kernel-name>...: run from the repo root; $S/kernels/<name>.img
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/v44
M=measurements/measurement-2026-11-06-v44
export TXB_NAP=$S/txb/TXBENCH.NAP TXB_SCR=$S
n=$1; shift
for i in $(seq 1 $n); do for k in "$@"; do
  $M/method/txbench-run.sh $M/runs/txbench/$k-$i $S/kernels/$k.img 2>&1 | /usr/bin/grep -E "ticks200|qemu" | /usr/bin/sed -E 's/ calls.*ticks200=/=/' | tr '\n' ' ' | /usr/bin/sed "s/^/$k-$i: /"; echo "load $(uptime | /usr/bin/sed 's/.*averages: //')"
done; done
