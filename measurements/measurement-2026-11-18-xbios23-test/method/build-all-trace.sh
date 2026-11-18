#!/usr/bin/env bash
# build the 4 trace kernels (bios-arg-trace) per side
set -u
A=/Volumes/Daten/Development/PiRusTOS/target-agents/agent-a9830dbeb107ba8e9; M=/Volumes/Daten/Development/PiRusTOS/measurements/measurement-2026-11-18-xbios23-test/method
for side in wt wtmain; do
  if [ $side = wt ]; then S=$A/scratch; else S=$A/scratch-main; fi
  for sc in vid-vdo vid-vdo-ext apps apps2; do
    WS=$A/$side FEAT=bios-arg-trace bash $M/build-trace.sh $S $sc 2>&1 | tail -2
  done
done
echo ALLDONE
