#!/bin/bash
# V35 test, third part: boot-screen timing at 1920x1080 again, host load lower, order reversed every second round (ABC CBA ABC CBA).
cd /Volumes/Daten/Development/PiRusTOS
S=${S:?}; M=measurements/measurement-2026-11-07-v35-test
mkdir -p $M/runs/bct2
n=0
for order in "base fix fixpad" "fixpad fix base" "base fix fixpad" "fixpad fix base"; do n=$((n+1))
  for T in $order; do echo "load before $T-$n: $(uptime | sed 's/.*load averages*: //')" >> $M/runs/bct2/loads-before.txt
    bash $M/method/bc-timing.sh $M/runs/bct2 $S/k/$T-rpi4-release.img $T-$n
  done
done
echo GUI3FINISHED
