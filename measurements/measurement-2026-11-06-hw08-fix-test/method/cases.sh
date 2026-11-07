#!/usr/bin/env bash
# all cases: after, before, emutos (sequential; timing-sensitive busy cases)
export WFPROBE_SCRATCH=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/wf
M=/Volumes/Daten/Development/PiRusTOS/measurements/measurement-2026-11-06-hw08-fix-test
A="a1 a2 a3 a4 a5 b1 b2 b3 b4 b5 d1 o1 n1 n2 n3 n4 p1 p2 p3 p4 r1 t1 t2 u1 u2 u3 u4 u5 u6 w1"
B="a1 a2 a3 a4 a5 b1 b2 b3 b4 b5 d1 o1 n1 n2 n3 n4 t1 t2 u1 u2 u3 u4 u5 u6 w1"
$M/method/run-all.sh newtos /private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/k/fix-rpi4-sdfault.img $M/results/after $A
$M/method/run-all.sh newtos /private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/k/base-rpi4-sdfault.img $M/results/before $B
$M/method/run-all.sh emutos - $M/results/emutos $A
echo CASES-DONE
