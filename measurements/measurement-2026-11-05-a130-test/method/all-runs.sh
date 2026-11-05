#!/usr/bin/env bash
# a130-test runs, in order, from the repository root. SCR = session scratchpad a130t/ (deleted after).
set -uo pipefail
export SLPROBE_SCRATCH=$SCR
R=measurements/measurement-2026-11-05-a130-test
M=$R/method
tosdump/probes/run-slprobe.sh emutos $R/runs/emutos-tt-v3-1
tosdump/probes/run-slprobe.sh kernel HEAD "$SCR/kernel-head-v3.img"
SLPROBE_MOUSE=$PWD/$M/flicker-mouse.txt tosdump/probes/run-slprobe.sh kernel HEAD "$SCR/kernel-head-flicker2.img"
for r in 1 2; do tosdump/probes/run-slprobe.sh newtos $R/runs/newtos-head-v3-$r "$SCR/kernel-head-v3.img"; done
SLPROBE_MOUSE=$PWD/$M/flicker-mouse.txt SLPROBE_SHOTS=$PWD/$M/flicker-shots-newtos.txt \
    tosdump/probes/run-slprobe.sh newtos $R/runs/newtos-head-flicker-v2-1 "$SCR/kernel-head-flicker2.img"
SLPROBE_MOUSE=$PWD/$M/flicker-mouse.txt SLPROBE_SHOTS=$PWD/$M/flicker-shots-emutos.txt \
    tosdump/probes/run-slprobe.sh emutos $R/runs/emutos-tt-flicker-v2-1
echo ALLDONE
