#!/bin/bash
# re-run of the two steps of regress.sh that had an empty $SP in the first call (kernel path), same inputs
cd /Volumes/Daten/Development/PiRusTOS
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/rg; M=measurements/measurement-2026-11-06-hw08-fix-test; P=tosdump/probes
export SHPROBE_SCRATCH=$S/sh XDPROBE_SCRATCH=$S/xd
for N in base fix; do $P/run-shprobe.sh newtos-desk $M/results/gui/shprobe-desk-$N /private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/k/$N-rpi4-rel.img; done
$P/run-xdprobe.sh emutos $M/results/gui/xdprobe-emutos2 >/dev/null 2>&1
for N in base fix; do $P/run-xdprobe.sh newtos $M/results/gui/xdprobe-$N /private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/k/$N-rpi4-rel.img; done
echo R2-DONE
