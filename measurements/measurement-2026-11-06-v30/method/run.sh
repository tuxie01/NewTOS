#!/bin/sh
# V30: the probe on both sides twice, the tables once; run from the main tree. S = session scratchpad.
S=${S:?set S to the session scratchpad}
M=measurements/measurement-2026-11-06-v30
R=tosdump/reference
K=target/aarch64-unknown-none-softfloat/release/kernel8.img
set -x
cargo xtask build --board rpi4 --release
cp -c $K $S/kernel.img
for r in e1 e2; do VDPROBE_SCRATCH=$S tosdump/probes/run-vdprobe.sh emutos falcontc $S/$r; done
for r in n1 n2; do VDPROBE_SCRATCH=$S tosdump/probes/run-vdprobe.sh newtos $S/$r $S/kernel.img; done
python3 tosdump/probes/vdprobe-compare.py $S/e1 $S/n1 --maps $S/maps > $M/probe/compare-e1-n1.txt
cargo xtask tosdump vdi tosdump/cases/vdi-v18-t2.tsv --out $M/runs/newtos-vdi-v18t2 --reference $R/EMUTOS-1a7484ab-VDI-v18t2-falcontc.BIN --scratch $S/xt
cargo xtask tosdump vdi tosdump/cases/vdi-all.tsv --out $M/runs/newtos-vdi-all --reference $R/EMUTOS-1a7484ab-VDI-all-falcontc.BIN --scratch $S/xt
# the spans of tos-api/tests/vdi_reference/cases_v30.rs, cases TV*/TWA:
python3 $M/method/gen-spans.py $S/e1
