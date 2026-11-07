#!/bin/bash
# base vs fix, PNG byte for byte, per run pair
G=measurements/measurement-2026-11-07-v34-test/runs/gui
for r in shprobe shprobe-desk slprobe slprobe2 slprobe3 desk480 desk1080 bc480 bc1080; do
  b=$G/$r-base; f=$G/$r-fix; n=0; d=0; l=""
  for p in $b/shots/*.png; do n=$((n+1)); q=$f/shots/$(basename $p); cmp -s $p $q || { d=$((d+1)); l="$l $(basename $p .png | cut -c1-6)"; }; done
  echo "$r: $n shots base, $(ls $f/shots/*.png | wc -l | tr -d ' ') fix, $d differ:$l  | exc/panic base $(grep -a -ciE 'exception|panic' $b/BOOT-SERIAL.txt) fix $(grep -a -ciE 'exception|panic' $f/BOOT-SERIAL.txt)"
done
