#!/bin/sh
# IKBD-injector-armed release rpi4 kernels (head, base) for the desktop run; run from the main tree
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/v30t
M=measurements/measurement-2026-11-06-v30-test
export DESK_SCRATCH=$S/dk
for p in head:858e5f68 base:cc5f476b; do
  $M/method/desk-inj.sh kernel ${p#*:} $M/method/desk-mouse.txt $S/k-${p%%:*}.img | tail -1
done
