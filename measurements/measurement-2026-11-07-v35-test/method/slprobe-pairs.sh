#!/bin/bash
# slprobe: every pair of the six runs (3 x head = before fix, 3 x fix); lists the differing shot names
R=$1
names="slprobe-base slprobe2-base slprobe3-base slprobe-fix slprobe2-fix slprobe3-fix"
for a in $names; do for b in $names; do [[ "$a" < "$b" ]] || continue
  n=0; d=0; l=""; for f in $R/$a/shots/*.png; do g=$R/$b/shots/$(basename $f); n=$((n+1)); cmp -s $f $g || { d=$((d+1)); l="$l $(basename $f .png | cut -c1-6)"; }; done
  echo "$a vs $b: $n shots, $d differ:$l"; done; done
