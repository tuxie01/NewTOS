#!/bin/bash
# compare PNG shots base vs head per probe; usage: shots-compare.sh <runs-dir> <probe-dir-stem> ...
R=$1; shift
for k in "$@"; do
  n=0; d=0; list=""
  for f in $R/$k-base/shots/*.png; do
    [ -f "$f" ] || continue
    g=${f/$k-base/$k-head}; n=$((n+1))
    cmp -s "$f" "$g" || { d=$((d+1)); list="$list $(basename $f)"; }
  done
  echo "$k: $n shots, $d differ:$list"
done
