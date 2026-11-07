#!/bin/sh
# V31 test: vdprobe (tosdump/probes/vdprobe.c of 35d052a9, run from the main tree) on the oracle and on both kernels.
S=${S:?}
M=measurements/measurement-2026-11-07-v31-test
P=tosdump/probes
export VDPROBE_SCRATCH=$S/vp; mkdir -p $VDPROBE_SCRATCH
set -x
$P/run-vdprobe.sh emutos falcontc $M/probe/e1 > $M/probe/e1.stdout 2>&1
$P/run-vdprobe.sh newtos $M/probe/n-head $S/kernels/head-rpi4.img > $M/probe/n-head.stdout 2>&1
$P/run-vdprobe.sh newtos $M/probe/n-fix1 $S/kernels/fix-rpi4.img > $M/probe/n-fix1.stdout 2>&1
$P/run-vdprobe.sh newtos $M/probe/n-fix2 $S/kernels/fix-rpi4.img > $M/probe/n-fix2.stdout 2>&1
python3 $P/vdprobe-compare.py $M/probe/e1 $M/probe/n-head > $M/probe/compare-e1-n-head.txt
python3 $P/vdprobe-compare.py $M/probe/e1 $M/probe/n-fix1 > $M/probe/compare-e1-n-fix1.txt
python3 $P/vdprobe-compare.py $M/probe/n-head $M/probe/n-fix1 > $M/probe/compare-n-head-n-fix1.txt
echo finished
