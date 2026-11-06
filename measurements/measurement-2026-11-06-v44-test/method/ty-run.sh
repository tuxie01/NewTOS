#!/bin/bash
# V44 item 6: vdprobe with the scratch TY series (cells around 64 x 64) from a scratch copy of the fix tree
# (tosdump/probes/vdprobe.c + method/ty-cases.c inserted before t2(), call ty() after tx()).
cd /Volumes/Daten/Development/PiRusTOS
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/v44t
M=measurements/measurement-2026-11-06-v44-test
P=$S/fix/tosdump/probes
export VDPROBE_SCRATCH=$S/vp
mkdir -p $M/probe
$P/run-vdprobe.sh emutos falcontc $M/probe/ty-e1 > $M/probe/ty-e1.stdout 2>&1
$P/run-vdprobe.sh newtos $M/probe/ty-fix $S/kernels/fix.img > $M/probe/ty-fix.stdout 2>&1
$P/run-vdprobe.sh newtos $M/probe/ty-head $S/kernels/head.img > $M/probe/ty-head.stdout 2>&1
$P/run-vdprobe.sh newtos $M/probe/ty-base $S/kernels/base.img > $M/probe/ty-base.stdout 2>&1
echo finished
