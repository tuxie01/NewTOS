#!/bin/sh
# V18 part 2: vdprobe (cases T* appended after k17), oracle falcontc twice, NewTOS (4a8e1ae3 release kernel) twice;
# run from the main tree, sequential.
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/15767204-8d89-43e0-aae0-d8149a16a17d/scratchpad/v18t2
M=measurements/measurement-2026-11-06-v18-t2
P=tosdump/probes
export VDPROBE_SCRATCH=$S/vp
K=$S/head/target/aarch64-unknown-none-softfloat/release/kernel8.img
mkdir -p $M/probe
set -x
case "${1:-all}" in
e1)  $P/run-vdprobe.sh emutos falcontc $M/probe/e1 > $M/probe/e1.stdout 2>&1 ;;
n1)  $P/run-vdprobe.sh newtos $M/probe/n1 $K > $M/probe/n1.stdout 2>&1 ;;
e2)  $P/run-vdprobe.sh emutos falcontc $M/probe/e2 > $M/probe/e2.stdout 2>&1 ;;
n2)  $P/run-vdprobe.sh newtos $M/probe/n2 $K > $M/probe/n2.stdout 2>&1 ;;
ett) $P/run-vdprobe.sh emutos tt $M/probe/ett > $M/probe/ett.stdout 2>&1 ;;
esac
echo finished $1
