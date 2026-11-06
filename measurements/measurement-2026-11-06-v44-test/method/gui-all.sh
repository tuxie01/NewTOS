#!/bin/bash
# V44 acceptance: pictures before the fix (head = e933d13c) against the fix (6e96e91c). Run from the repo root.
cd /Volumes/Daten/Development/PiRusTOS
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/v44t
M=measurements/measurement-2026-11-06-v44-test
P=tosdump/probes
K=target/aarch64-unknown-none-softfloat/release/kernel8.img
export SHPROBE_SCRATCH=$S/sh SLPROBE_SCRATCH=$S/sl DESK_SCRATCH=$S/dk DESK_FILES=$S/files
set -x
for p in head:e933d13c fix:6e96e91c; do T=${p%%:*}; C=${p#*:}
  $P/run-shprobe.sh kernel $C $S/kernel-sh-$T.img
  $P/run-slprobe.sh kernel $C $S/kernel-sl-$T.img
  $M/method/desk-inj.sh kernel $C $M/method/desk-mouse.txt $S/k-$T.img | tail -1
done
mkdir -p $M/runs/gui
for T in head fix; do
  $P/run-shprobe.sh newtos $M/runs/gui/shprobe-$T $S/kernel-sh-$T.img
  $P/run-shprobe.sh newtos-desk $M/runs/gui/shprobe-desk-$T $S/$T/$K
  $P/run-slprobe.sh newtos $M/runs/gui/slprobe-$T $S/kernel-sl-$T.img
done
for T in head fix; do $P/run-slprobe.sh newtos $M/runs/gui/slprobe2-$T $S/kernel-sl-$T.img; done
for T in head fix; do $P/run-slprobe.sh newtos $M/runs/gui/slprobe3-$T $S/kernel-sl-$T.img; done
for T in head fix; do $M/method/desk-inj.sh newtos $M/runs/gui/desk480-$T $S/k-$T.img $M/method/desk-NEWDESK.INF $M/method/desk-shots.txt | tail -3; done
for g in "240:-global bcm2835-fb.yres=240" "1080:-global bcm2835-fb.xres=1920 -global bcm2835-fb.yres=1080"; do
  n=${g%%:*}; x=${g#*:}
  for T in head fix; do QEMU_EXTRA="$x" $M/method/desk-inj.sh newtos $M/runs/gui/desk$n-$T $S/k-$T.img $M/method/desk-NEWDESK.INF $M/method/desk-shots.txt | tail -3; done
done
for g in "480:" "240:-global bcm2835-fb.yres=240" "1080:-global bcm2835-fb.xres=1920 -global bcm2835-fb.yres=1080"; do
  n=${g%%:*}; x=${g#*:}
  for T in head fix; do bash $M/method/boot-console.sh $M/runs/gui/bc$n-$T $S/$T/$K "$x" | tail -3; done
done
echo ALLFINISHED
