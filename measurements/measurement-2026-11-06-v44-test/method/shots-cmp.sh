#!/bin/bash
R=measurements/measurement-2026-11-06-v44-test/runs/gui
for k in shprobe shprobe-desk desk480 desk240 desk1080 bc480 bc240 bc1080; do
  n=0; d=0; l=""
  for f in $R/$k-head/shots/*.png; do g=$R/$k-fix/shots/$(basename $f); n=$((n+1)); cmp -s $f $g || { d=$((d+1)); l="$l $(basename $f .png)"; }; done
  echo "$k: $n shots, $d differ:$l"
done
for f in $R/*/run.txt; do echo "$(basename $(dirname $f)): $(/usr/bin/grep -h -o 'exception-panic-lines [0-9]*\|shots [0-9]*\|qemu [a-z-]*' $f | tr '\n' ' ')"; done
