#!/bin/bash
# V35: desktop with the IKBD injector (640x480 and 1920x1080) and boot screen + AUTO console (both sizes),
# base vs fix, one QEMU at a time. Uses the V34 test run's scripts and inputs unchanged.
set -euo pipefail
cd /Volumes/Daten/Development/PiRusTOS
S=${S:?}
V=measurements/measurement-2026-11-07-v34-test/method
M=measurements/measurement-2026-11-07-v35/method
export DESK_SCRATCH=$S/dk DESK_FILES=$S/files S
for T in base fix; do $M/desk-kernel.sh $S/t/$T $V/desk-mouse.txt $S/k-desk-$T.img; done
mkdir -p $S/runs/gui
for g in "480:" "1080:-global bcm2835-fb.xres=1920 -global bcm2835-fb.yres=1080"; do
  n=${g%%:*}; x=${g#*:}
  for T in base fix; do QEMU_EXTRA="$x" $V/desk-inj.sh newtos $S/runs/gui/desk$n-$T $S/k-desk-$T.img $V/desk-NEWDESK.INF $V/desk-shots.txt | tail -3; done
  for T in base fix; do K=$S/kernel-head.img; [ $T = fix ] && K=$S/kernel-fix.img; bash $V/boot-console.sh $S/runs/gui/bc$n-$T $K "$x" | tail -3; done
done
echo GUIFINISHED
