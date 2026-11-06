#!/bin/sh
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/15767204-8d89-43e0-aae0-d8149a16a17d/scratchpad/v32t
M=measurements/measurement-2026-11-06-v32-test
export SLPROBE_SCRATCH=$S/sl
for T in base head; do tosdump/probes/run-slprobe.sh newtos $M/runs/newtos-slprobe2-$T $S/kernel-sl-$T.img; done
echo finished
