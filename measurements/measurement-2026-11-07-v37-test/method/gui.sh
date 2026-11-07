#!/bin/bash
# V37 test: desktop with the IKBD injector (640x480 and 1920x1080) and boot screen + AUTO console (both sizes), base vs fix,
# one QEMU at a time. Trees $S/t2/{base,fix} = git archive of f9525a3d / 2bf28d71 (no target dir); scripts of the V34/V35 runs unchanged.
set -euo pipefail
cd /Volumes/Daten/Development/PiRusTOS
S=${S:?}; M=measurements/measurement-2026-11-07-v37-test
V=measurements/measurement-2026-11-07-v34-test/method
D=measurements/measurement-2026-11-07-v35/method
export DESK_SCRATCH=$S/dk DESK_FILES=$S/files S
mkdir -p $S/dk
for T in base fix; do [ -f $S/k-desk-$T.img ] || $D/desk-kernel.sh $S/t2/$T $V/desk-mouse.txt $S/k-desk-$T.img; done
mkdir -p $S/files; for i in $(seq -w 1 40); do printf 'V37 desktop file %s\r\n' "$i" > "$S/files/LONGNM$i.TXT"; done
echo "files $(cat "$S"/files/* | shasum -a 256 | cut -c1-16)"
mkdir -p $M/runs/gui
for g in "480:" "1080:-global bcm2835-fb.xres=1920 -global bcm2835-fb.yres=1080"; do
  n=${g%%:*}; x=${g#*:}
  for T in base fix; do QEMU_EXTRA="$x" $V/desk-inj.sh newtos $M/runs/gui/desk$n-$T $S/k-desk-$T.img $V/desk-NEWDESK.INF $V/desk-shots.txt | tail -3; done
  for T in base fix; do bash $V/boot-console.sh $M/runs/gui/bc$n-$T $S/k/$T-rpi4-release.img "$x" | tail -3; done
done
echo GUIFINISHED
