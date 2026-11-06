#!/bin/sh
# V30 acceptance: vdprobe (385 cases), oracle e1 measured anew (falcontc), NewTOS head twice, base once; run from the main tree.
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/v33t
M=measurements/measurement-2026-11-06-v33-test
P=tosdump/probes
export VDPROBE_SCRATCH=$S/vp
mkdir -p $S/vp
set -x
K=target/aarch64-unknown-none-softfloat/release/kernel8.img
$P/run-vdprobe.sh emutos falcontc $M/probe/e1 > $M/probe/e1.stdout 2>&1
$P/run-vdprobe.sh newtos $M/probe/n-head $S/head/$K > $M/probe/n-head.stdout 2>&1
$P/run-vdprobe.sh newtos $M/probe/n-base $S/base/$K > $M/probe/n-base.stdout 2>&1
$P/run-vdprobe.sh newtos $M/probe/n-head-2 $S/head/$K > $M/probe/n-head-2.stdout 2>&1
echo finished
