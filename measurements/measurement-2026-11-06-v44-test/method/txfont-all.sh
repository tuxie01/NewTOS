#!/bin/bash
# txfont-all.sh <rounds> <kernel>...: loaded-font timing, wall clock, kernels in turn. Run from the repo root.
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/v44t
M=measurements/measurement-2026-11-06-v44-test
export TXF_NAP=$S/txfont/TXFONT.NAP TXB_SCR=$S
n=$1; shift
for i in $(seq 1 $n); do for k in "$@"; do
  $M/method/txfont-run.sh $M/runs/txfont/$k-$i $S/kernels/$k.img 2>&1 | /usr/bin/grep -E "ticks200|qemu" | /usr/bin/sed -E 's/ calls.*ticks200=/=/' | tr '\n' ' ' | /usr/bin/sed "s/^/$k-$i: /"; echo "load $(uptime | /usr/bin/sed 's/.*averages: //')"
done; done
