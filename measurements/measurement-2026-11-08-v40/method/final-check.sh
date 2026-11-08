#!/bin/bash
# V40: the committed state. The fix tree gets the last edits of the main tree's V40 files (a long line split into a
# binding, a cite, doc text), then: builds + warnings (label fix2, $S/t/fix2 -> $S/t/fix), probe n-fix2 against n-fix,
# clippy --tests, cargo xtask test (full) in the fix tree.
set -x
cd /Volumes/Daten/Development/PiRusTOS
S=${S:?}
M=measurements/measurement-2026-11-08-v40/method
for f in $(cat $S/fix-files.txt); do cmp -s $f $S/t/fix/$f || { cp -p $f $S/t/fix/$f; echo "updated $f"; }; done
ln -sfn $S/t/fix $S/t/fix2
$M/builds.sh fix2
VDPROBE_SCRATCH=$S/scr tosdump/probes/run-vdprobe.sh newtos $S/n-fix2 $S/kernel-fix2.img | tail -2
cmp $S/n-fix/VDPROBE.BIN $S/n-fix2/VDPROBE.BIN && echo "n-fix2 BIN == n-fix BIN"
cmp $S/n-fix/VDPROBE.LOG $S/n-fix2/VDPROBE.LOG && echo "n-fix2 LOG == n-fix LOG"
(cd $S/t/fix && cargo clippy -p tos-api --tests) > $S/builds/fix2/clippy-tos-api-tests.log 2>&1
/usr/bin/grep -E "\(lib test\)|\"vdi_reference\"" $S/builds/fix2/clippy-tos-api-tests.log
(cd $S/t/fix && cargo xtask test) > $S/xtask-test.txt 2>&1; echo "xtask test rc=$?"
/usr/bin/grep -E "^test result" $S/xtask-test.txt | awk '{p+=$4; f+=$6; i+=$8} END {print "passed", p, "failed", f, "ignored", i, "(" NR " result lines)"}'
echo FINALFINISHED
