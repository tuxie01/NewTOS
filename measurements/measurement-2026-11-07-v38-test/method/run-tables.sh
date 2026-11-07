#!/bin/sh
# V38 acceptance: vdprobe (954 cases) and the three VDI tables on base and fix, sequential, one QEMU at a time
# (the called scripts stop only the QEMU PID they started). Run from the main tree after build.sh.
# S = scratchpad v38t (base/, fix/ trees; kernel-base.img, kernel-fix.img from build.sh).
S=${S:?set S}
M=measurements/measurement-2026-11-07-v38-test
R=tosdump/reference
O=measurements/measurement-2026-11-07-v37-test/probe/e1   # oracle vdprobe run (same VDPROBE.PRG source, vdprobe.c unchanged since 2bf28d71)
set -x
mkdir -p $M/probe $M/runs
for t in base fix; do
  VDPROBE_SCRATCH=$S/scr tosdump/probes/run-vdprobe.sh newtos $M/probe/n-$t $S/kernel-$t.img > $S/vdprobe-$t.out 2>&1
done
python3 tosdump/probes/vdprobe-compare.py $M/probe/n-base $M/probe/n-fix > $M/probe/compare-n-base-n-fix.txt
python3 tosdump/probes/vdprobe-compare.py $O $M/probe/n-fix > $M/probe/compare-e1-n-fix.txt
python3 tosdump/probes/vdprobe-compare.py $O $M/probe/n-base > $M/probe/compare-e1-n-base.txt
for t in base fix; do
  cargo xtask tosdump vdi tosdump/cases/vdi-all.tsv --tree $S/$t --out $M/runs/$t-vdi-all --reference $R/EMUTOS-1a7484ab-VDI-all-falcontc.BIN --scratch $S/xt-$t
  cargo xtask tosdump vdi tosdump/cases/vdi-v18.tsv --tree $S/$t --out $M/runs/$t-vdi-v18 --reference $R/EMUTOS-1a7484ab-VDI-v18-falcontc.BIN --scratch $S/xt-$t
  cargo xtask tosdump vdi tosdump/cases/vdi-v18-t2.tsv --tree $S/$t --out $M/runs/$t-vdi-v18t2 --reference $R/EMUTOS-1a7484ab-VDI-v18t2-falcontc.BIN --scratch $S/xt-$t
done
echo finished
