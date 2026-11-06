#!/bin/sh
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/v33t
M=measurements/measurement-2026-11-06-v33-test
export SLPROBE_SCRATCH=$S/sl
for T in base head; do tosdump/probes/run-slprobe.sh newtos $M/runs/newtos-slprobe2-$T $S/kernel-sl-$T.img; done
echo finished
