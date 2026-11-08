#!/bin/sh
# V56: shot-by-shot byte compare base vs fix of every gui run.
S=${S:?}
for r in desk480 desk1080 bc480 bc1080; do
  same=0; diff=0; list=""
  for f in $S/runs/gui/$r-base/shots/*.png; do b=$(basename $f)
    if cmp -s $f $S/runs/gui/$r-fix/shots/$b; then same=$((same+1)); else diff=$((diff+1)); list="$list $b"; fi
  done
  echo "$r: $same identical, $diff differ$list"
done
