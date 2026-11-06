#!/bin/bash
R=measurements/measurement-2026-11-06-v33-test/runs
for p in "t2 v18t2" "t4 v18" "t4 all" "t4 vdi-r3" "t2 v18t2-raster" "t4 raster" "t4 raster-screen" "t4 v18-raster" "t5 r2" "t5 r3" "t5 r4"; do set -- $p; h=$R/$1-head-$2; b=$R/$1-base-$2; for f in $h/*.BIN; do n=$(basename $f); if cmp -s $f $b/$n; then echo "$1-$2 $n base==head"; else echo "$1-$2 $n base!=head ($(cmp -l $f $b/$n | wc -l) bytes)"; fi; done; done
