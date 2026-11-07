#!/bin/sh
# V18 part 3: tosdump/probes/wkprobe.c, run from the main tree, one emulator at a time.
#   e1 e2  EmuTOS falcontc (the probe feeds its packets itself)
#   n1 n2  NewTOS raspi4b, kernel armed with tools/inject-arm.py launch:WKPROBE=wkprobe-mouse.txt
#          (built by: run-wkprobe.sh kernel <frozen tree> <kernel-out>)
#   np     NewTOS raspi4b, the plain release kernel of the frozen tree (every packet wait times out)
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/ed7bb9a3-0fc2-4b37-91bb-ed70559503a3/scratchpad/t3
M=measurements/measurement-2026-11-07-v18-t3
P=tosdump/probes
export WKPROBE_SCRATCH=$S/wk
KINJ=$S/kinj.img
KPLAIN=$S/head/target/aarch64-unknown-none-softfloat/release/kernel8.img
mkdir -p $M/probe
case "$1" in
kernel) $P/run-wkprobe.sh kernel $S/head $KINJ ;;
e1|e2) $P/run-wkprobe.sh emutos falcontc $M/probe/$1 > $M/probe/$1.stdout 2>&1 ;;
n1|n2) $P/run-wkprobe.sh newtos $M/probe/$1 $KINJ > $M/probe/$1.stdout 2>&1 ;;
np)    $P/run-wkprobe.sh newtos $M/probe/$1 $KPLAIN > $M/probe/$1.stdout 2>&1 ;;
esac
echo finished $1
