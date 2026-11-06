#!/bin/sh
# V29 acceptance: vdprobe (cases incl. TRA-TRM), oracle e1 reused from the V18 part 2 run (vdprobe.c unchanged since eba4122a),
# head twice, base once; run from the main tree.
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/15767204-8d89-43e0-aae0-d8149a16a17d/scratchpad/v29t
M=measurements/measurement-2026-11-06-v29-test
P=tosdump/probes
export VDPROBE_SCRATCH=$S/vp
set -x
K=target/aarch64-unknown-none-softfloat/release/kernel8.img
$P/run-vdprobe.sh newtos $M/probe/n-head $S/head/$K > $M/probe/n-head.stdout 2>&1
$P/run-vdprobe.sh newtos $M/probe/n-base $S/base/$K > $M/probe/n-base.stdout 2>&1
$P/run-vdprobe.sh newtos $M/probe/n-head-2 $S/head/$K > $M/probe/n-head-2.stdout 2>&1
echo finished
