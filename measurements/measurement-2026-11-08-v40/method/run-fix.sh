#!/bin/bash
# V40: the final fix stage, in order: builds (warnings, release kernel), probe on NewTOS, tables, GUI.
set -x
cd /Volumes/Daten/Development/PiRusTOS
S=${S:?}
M=measurements/measurement-2026-11-08-v40/method
$M/builds.sh fix
VDPROBE_SCRATCH=$S/scr tosdump/probes/run-vdprobe.sh newtos $S/n-fix $S/kernel-fix.img | tail -2
for p in "e1 n-fix" "n-base n-fix" "e1 n-base"; do set -- $p; python3 tosdump/probes/vdprobe-compare.py $S/$1 $S/$2 > $S/compare-$1-$2.txt 2>&1; tail -1 $S/compare-$1-$2.txt; done
# tables: fix only (the base tables of the first pass stand, base tree unchanged)
R=tosdump/reference
run() { D=$1; TAB=$2; O=$3; REF=$4
  if [ -n "$REF" ]; then cargo xtask tosdump $D tosdump/cases/$TAB --tree $S/t/fix --out $S/runs/$O --reference $R/$REF.BIN --scratch $S/xt-fix
  else cargo xtask tosdump $D tosdump/cases/$TAB --tree $S/t/fix --out $S/runs/$O --scratch $S/xt-fix; fi; }
run vdi vdi-v18-t2.tsv tab-fix-v18t2 EMUTOS-1a7484ab-VDI-v18t2-falcontc
run vdi vdi-all.tsv tab-fix-all EMUTOS-1a7484ab-VDI-all-falcontc
run aes aes-r2-wind.tsv tab-fix-r2 EMUTOS-1a7484ab-AES-r2wind
run aes aes-r3-wind-state.tsv tab-fix-r3 EMUTOS-1a7484ab-AES-r3
run aes aes-r4-subfunctions.tsv tab-fix-r4
rm -rf $S/xt-fix
$M/gui.sh
echo RUNFIXFINISHED
