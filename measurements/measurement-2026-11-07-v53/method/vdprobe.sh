#!/bin/sh
# V53: tosdump/probes/vdprobe.c on NewTOS, the plain release kernels of base and fix (no injector), and the
# base-vs-fix compare. The oracle side is not re-run (the probe source is unchanged; base vs fix is what is asked).
S=${S:?}
cd /Volumes/Daten/Development/PiRusTOS
for T in base fix; do VDPROBE_SCRATCH=$S/scr tosdump/probes/run-vdprobe.sh newtos $S/probe/vd-$T $S/kernel-$T.img > $S/probe/vd-$T.stdout 2>&1; tail -2 $S/probe/vd-$T/run.txt; done
python3 -I tosdump/probes/vdprobe-compare.py $S/probe/vd-base $S/probe/vd-fix > $S/probe/compare-vd-base-vd-fix.txt 2>&1
tail -3 $S/probe/compare-vd-base-vd-fix.txt
