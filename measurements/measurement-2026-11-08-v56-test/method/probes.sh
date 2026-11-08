#!/bin/bash
# V56: vdprobe (plain release kernels) and wkprobe (injector-armed kernels, tosdump/probes/run-wkprobe.sh kernel)
# base vs fix, plus wkprobe on the pinned oracle (e1). One emulator at a time; each script kills its own PID.
set -u
S=${S:?}; cd $S/pr
mkdir -p $S/probe
for T in base fix; do VDPROBE_SCRATCH=$S/scr tosdump/probes/run-vdprobe.sh newtos $S/probe/vd-$T $S/k/$T-release.img > $S/probe/vd-$T.stdout 2>&1; tail -2 $S/probe/vd-$T/run.txt; done
python3 -I tosdump/probes/vdprobe-compare.py $S/probe/vd-base $S/probe/vd-fix > $S/probe/compare-vd-base-vd-fix.txt 2>&1; tail -3 $S/probe/compare-vd-base-vd-fix.txt
export WKPROBE_SCRATCH=$S/wk
for T in base fix; do [ -f $S/k/kwk-$T.img ] || tosdump/probes/run-wkprobe.sh kernel $S/t/$T $S/k/kwk-$T.img; done
tosdump/probes/run-wkprobe.sh emutos falcontc $S/probe/e1 > $S/probe/e1.stdout 2>&1; tail -2 $S/probe/e1/run.txt
for T in base fix; do tosdump/probes/run-wkprobe.sh newtos $S/probe/n-$T $S/k/kwk-$T.img > $S/probe/n-$T.stdout 2>&1; tail -3 $S/probe/n-$T/run.txt; done
for n in n-base n-fix; do python3 -I tosdump/probes/wkprobe-compare.py $S/probe/e1 $S/probe/$n > $S/probe/compare-e1-$n.txt 2>&1; done
python3 -I tosdump/probes/wkprobe-compare.py $S/probe/n-base $S/probe/n-fix > $S/probe/compare-n-base-n-fix.txt 2>&1
echo PROBEDONE
