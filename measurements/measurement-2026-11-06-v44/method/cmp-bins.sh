#!/bin/bash
# fix against the state before it, byte for byte; AES records that differ: which words, tosdiff decides.
M=measurements/measurement-2026-11-06-v44; R=$M/runs
for p in v18t2 v18 all vdi-r3 v18t2-raster raster raster-screen v18-raster r2 r3 r4; do
  h=$R/tab-fix-$p; b=$R/tab-head-$p
  for f in $h/*.BIN; do n=$(basename $f)
    if cmp -s $f $b/$n; then echo "$p $n fix==before $(shasum -a 256 $f | cut -c1-16)";
    else echo "$p $n fix!=before ($(cmp -l $f $b/$n | wc -l | tr -d ' ') bytes) fix $(shasum -a 256 $f | cut -c1-16) before $(shasum -a 256 $b/$n | cut -c1-16)"; fi
  done
done
for s in n-fix n-prefix e1; do echo "probe $s BIN $(shasum -a 256 $M/probe/$s/VDPROBE.BIN | cut -c1-16) LOG $(shasum -a 256 $M/probe/$s/VDPROBE.LOG | cut -c1-16)"; done
cmp $M/probe/n-fix/VDPROBE.BIN $M/probe/n-prefix/VDPROBE.BIN && echo "probe BIN fix==before"
cmp $M/probe/n-fix/VDPROBE.LOG $M/probe/n-prefix/VDPROBE.LOG && echo "probe LOG fix==before"
