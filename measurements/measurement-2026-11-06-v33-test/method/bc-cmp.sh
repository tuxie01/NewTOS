#!/bin/bash
R=measurements/measurement-2026-11-06-v33-test/runs
for p in "bc1080-base bc1080r-base" "bc1080-head bc1080r-head" "bc1080r-base bc1080r-head" "bc1080-base bc1080-head"; do set -- $p
 echo "$1 vs $2: $(for f in $R/$1/shots/*.png; do cmp -s $f $R/$2/shots/$(basename $f) || basename $f; done | tr '\n' ' ')"; done
