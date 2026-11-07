#!/bin/sh
# V31: the probe on both sides, the tables; run from the main tree. S = session scratchpad.
# Stage "head" ran first, on the tree before the product change (only tosdump/probes/vdprobe.c changed):
#   cargo xtask build --board rpi4 --release; cp -c $K $S/kernel-head.img
#   VDPROBE_SCRATCH=$S/scr tosdump/probes/run-vdprobe.sh emutos falcontc $S/e1
#   VDPROBE_SCRATCH=$S/scr tosdump/probes/run-vdprobe.sh newtos $S/n-head $S/kernel-head.img
# Stage "fix" below, on the tree with the change.
S=${S:?set S to the session scratchpad}
M=measurements/measurement-2026-11-07-v31
R=tosdump/reference
K=target/aarch64-unknown-none-softfloat/release/kernel8.img
set -x
cargo xtask build --board rpi4 --release
cp -c $K $S/kernel-fix.img
VDPROBE_SCRATCH=$S/scr tosdump/probes/run-vdprobe.sh emutos falcontc $S/e2
for r in n-fix1 n-fix2; do VDPROBE_SCRATCH=$S/scr tosdump/probes/run-vdprobe.sh newtos $S/$r $S/kernel-fix.img; done
python3 tosdump/probes/vdprobe-compare.py $S/e1 $S/n-head > $M/probe/compare-e1-n-head.txt
python3 tosdump/probes/vdprobe-compare.py $S/e1 $S/n-fix1 --maps $S/maps > $M/probe/compare-e1-n-fix1.txt
python3 tosdump/probes/vdprobe-compare.py $S/n-head $S/n-fix1 > $M/probe/compare-n-head-n-fix1.txt
cargo xtask tosdump vdi tosdump/cases/vdi-v18-t2.tsv --out $M/runs/newtos-vdi-v18t2 --reference $R/EMUTOS-1a7484ab-VDI-v18t2-falcontc.BIN --scratch $S/xt
cargo xtask tosdump vdi tosdump/cases/vdi-all.tsv --out $M/runs/newtos-vdi-all --reference $R/EMUTOS-1a7484ab-VDI-all-falcontc.BIN --scratch $S/xt
# the spans of tos-api/tests/vdi_reference/cases_v31.rs:
python3 $M/method/gen-spans.py "the V18 part 2 run's probe/e1" $S/e18 T14A T14D T14E T14F T14I T14J T14K T14M   # e18 = gunzip of measurements/measurement-2026-11-06-v18-t2/probe/e1/VDPROBE.BIN.gz
python3 $M/method/gen-spans.py "the V31 run's probe/e1" $S/e1 T31A T31B T31C T31D T31E
