#!/bin/sh
# V34 test: vdprobe (tosdump/probes/vdprobe.c, unchanged by V34) on the oracle and on base / fix kernels. S = scratchpad.
S=${S:?}; M=measurements/measurement-2026-11-07-v34-test; P=tosdump/probes
export VDPROBE_SCRATCH=$S/vp; mkdir -p $VDPROBE_SCRATCH
set -x
$P/run-vdprobe.sh newtos $M/probe/n-base $S/k/base-rpi4-release.img > $M/probe/n-base.stdout 2>&1
$P/run-vdprobe.sh newtos $M/probe/n-fix1 $S/k/fix-rpi4-release.img > $M/probe/n-fix1.stdout 2>&1
$P/run-vdprobe.sh newtos $M/probe/n-fix2 $S/k/fix-rpi4-release.img > $M/probe/n-fix2.stdout 2>&1
$P/run-vdprobe.sh newtos $M/probe/n-fix-trace $S/k/fix-rpi4-trace.img > $M/probe/n-fix-trace.stdout 2>&1
for p in "e1 n-base" "e1 n-fix1" "n-base n-fix1"; do set -- $p
  python3 $P/vdprobe-compare.py $M/probe/$1 $M/probe/$2 > $M/probe/compare-$1-$2.txt
  diff $M/probe/$1/VDPROBE.LOG $M/probe/$2/VDPROBE.LOG > $M/probe/log-$1-$2.diff
done
echo finished
