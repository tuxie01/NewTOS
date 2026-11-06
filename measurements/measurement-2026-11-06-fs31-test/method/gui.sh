#!/bin/bash
# FS31 test: shprobe (12 shots + 2 desk) and the V44 desktop+IKBD scene, base (6b53dc5b) against fix (20f87d1d). Run from the main tree.
cd /Volumes/Daten/Development/PiRusTOS
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/fs31t
M=measurements/measurement-2026-11-06-fs31-test
V=measurements/measurement-2026-11-06-v44-test/method
P=tosdump/probes
export SHPROBE_SCRATCH=$S/sh DESK_SCRATCH=$S/dk DESK_FILES=$S/files
mkdir -p $S/sh $S/dk $M/runs/gui
set -x
for p in base:6b53dc5b fix:20f87d1d; do T=${p%%:*}; C=${p#*:}
  $P/run-shprobe.sh kernel $C $S/k/sh-$T.img
  $V/desk-inj.sh kernel $C $V/desk-mouse.txt $S/k/desk-$T.img | tail -1
done
for T in base fix; do
  $P/run-shprobe.sh newtos $M/runs/gui/shprobe-$T $S/k/sh-$T.img
  $P/run-shprobe.sh newtos-desk $M/runs/gui/shprobe-desk-$T $S/k/$T-rpi4.img
  $V/desk-inj.sh newtos $M/runs/gui/desk480-$T $S/k/desk-$T.img $V/desk-NEWDESK.INF $V/desk-shots.txt | tail -3
done
echo ALLFINISHED
