#!/bin/bash
# V53 tester: wkprobe on the oracle and on NewTOS (base, fix, fix with C1 late), one emulator at a time.
set -u
S=${S:?}; cd /Volumes/Daten/Development/PiRusTOS
export WKPROBE_SCRATCH=$S/wk
tosdump/probes/run-wkprobe.sh emutos falcontc $S/probe/e1 > $S/probe/e1.stdout 2>&1; tail -2 $S/probe/e1/run.txt
tosdump/probes/run-wkprobe.sh newtos $S/probe/n-base $S/kinj-base.img > $S/probe/n-base.stdout 2>&1; tail -3 $S/probe/n-base/run.txt
tosdump/probes/run-wkprobe.sh newtos $S/probe/n-fix $S/kinj-fix.img > $S/probe/n-fix.stdout 2>&1; tail -3 $S/probe/n-fix/run.txt
tosdump/probes/run-wkprobe.sh newtos $S/probe/n-fix-c1late $S/kinj-fix-c1late.img > $S/probe/n-fix-c1late.stdout 2>&1; tail -3 $S/probe/n-fix-c1late/run.txt
for n in n-base n-fix n-fix-c1late; do python3 -I tosdump/probes/wkprobe-compare.py $S/probe/e1 $S/probe/$n > $S/probe/compare-e1-$n.txt 2>&1; done
python3 -I tosdump/probes/wkprobe-compare.py $S/probe/n-base $S/probe/n-fix > $S/probe/compare-n-base-n-fix.txt 2>&1
echo PROBEDONE
