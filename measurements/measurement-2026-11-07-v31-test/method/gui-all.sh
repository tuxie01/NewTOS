#!/bin/bash
# V31 acceptance: pictures before (head = deaad5df) and with the fix (35d052a9). Run from the repo root; S = session scratchpad.
cd /Volumes/Daten/Development/PiRusTOS
S=${S:?}
M=measurements/measurement-2026-11-07-v31-test
P=tosdump/probes
K=target/aarch64-unknown-none-softfloat/release/kernel8.img
export SHPROBE_SCRATCH=$S/sh SLPROBE_SCRATCH=$S/sl DESK_SCRATCH=$S/dk DESK_FILES=$S/files S
set -x
for p in head:deaad5df fix:35d052a9; do T=${p%%:*}; C=${p#*:}
  $P/run-shprobe.sh kernel $C $S/kernel-sh-$T.img
  $P/run-slprobe.sh kernel $C $S/kernel-sl-$T.img
  $M/method/desk-inj.sh kernel $C $M/method/desk-mouse.txt $S/k-$T.img | tail -1
done
mkdir -p $M/runs/gui
for T in head fix; do
  $P/run-shprobe.sh newtos $M/runs/gui/shprobe-$T $S/kernel-sh-$T.img
  $P/run-shprobe.sh newtos-desk $M/runs/gui/shprobe-desk-$T $S/trees/$T/$K
  $P/run-slprobe.sh newtos $M/runs/gui/slprobe-$T $S/kernel-sl-$T.img
done
for n in 2 3; do for T in head fix; do $P/run-slprobe.sh newtos $M/runs/gui/slprobe$n-$T $S/kernel-sl-$T.img; done; done
for g in "480:" "1080:-global bcm2835-fb.xres=1920 -global bcm2835-fb.yres=1080"; do
  n=${g%%:*}; x=${g#*:}
  for T in head fix; do QEMU_EXTRA="$x" $M/method/desk-inj.sh newtos $M/runs/gui/desk$n-$T $S/k-$T.img $M/method/desk-NEWDESK.INF $M/method/desk-shots.txt | tail -3; done
done
for g in "480:" "1080:-global bcm2835-fb.xres=1920 -global bcm2835-fb.yres=1080"; do
  n=${g%%:*}; x=${g#*:}
  for T in head fix; do bash $M/method/boot-console.sh $M/runs/gui/bc$n-$T $S/trees/$T/$K "$x" | tail -3; done
done
echo ALLFINISHED
