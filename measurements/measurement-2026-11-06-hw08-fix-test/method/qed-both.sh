#!/bin/bash
R=/Volumes/Daten/Development/PiRusTOS; M=/Volumes/Daten/Development/PiRusTOS/measurements/measurement-2026-11-06-hw08-fix-test; SP=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad
for t in fix base; do
  $M/method/arm-build2.sh $t $M/method/q60-keys.txt $SP/k/arm-$t-q60.img up:QED 2>&1 | tail -1 | cut -c1-100
  rm -rf $SP/qs-$t; mkdir -p $SP/qs-$t
  $M/method/app-run.sh $M/results/qed/$t $SP/k/arm-$t-q60.img $R/sdk/apps/qed/build/QED.NAP QED $M/method/q60-shots.txt $SP/qs-$t qed.rsc=$R/sdk/apps/qed/build/qed.rsc 2>&1 | tail -6
  rm -rf $SP/qs-$t
done
echo QED-DONE
