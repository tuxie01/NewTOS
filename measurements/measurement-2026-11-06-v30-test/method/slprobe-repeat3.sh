#!/bin/sh
cd /Volumes/Daten/Development/PiRusTOS
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/v30t
M=measurements/measurement-2026-11-06-v30-test
export SLPROBE_SCRATCH=$S/sl
for rt in 3:head 3:base 4:head 4:base; do r=${rt%%:*}; T=${rt#*:}; tosdump/probes/run-slprobe.sh newtos $M/runs/newtos-slprobe$r-$T $S/kernel-sl-$T.img >/dev/null 2>&1; done
echo finished
