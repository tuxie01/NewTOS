#!/bin/bash
# V53 tester: desktop with the V34 IKBD injector scene (640x480, 1920x1080) and boot screen + APITEST console, base vs fix,
# one QEMU at a time. Kernels from `desk-inj.sh kernel <commit>` (clean git archive, no target/ copied).
set -euo pipefail
cd /Volumes/Daten/Development/PiRusTOS
S=${S:?}
V=measurements/measurement-2026-11-07-v34-test/method
export DESK_SCRATCH=$S/dk S
mkdir -p $S/dk
[ -f $S/k-desk-base.img ] || $V/desk-inj.sh kernel 3380004f $V/desk-mouse.txt $S/k-desk-base.img
[ -f $S/k-desk-fix.img ]  || $V/desk-inj.sh kernel 4cc7005c $V/desk-mouse.txt $S/k-desk-fix.img
[ -n "${KERNELS_ONLY:-}" ] && exit 0
mkdir -p $S/files; for i in $(seq -w 1 40); do printf 'V53 desktop file %s\r\n' "$i" > "$S/files/LONGNM$i.TXT"; done
export DESK_FILES=$S/files
mkdir -p $S/runs/gui
for g in "480:" "1080:-global bcm2835-fb.xres=1920 -global bcm2835-fb.yres=1080"; do
  n=${g%%:*}; x=${g#*:}
  for T in base fix; do QEMU_EXTRA="$x" $V/desk-inj.sh newtos $S/runs/gui/desk$n-$T $S/k-desk-$T.img $V/desk-NEWDESK.INF $V/desk-shots.txt | tail -3; done
  for T in base fix; do bash $V/boot-console.sh $S/runs/gui/bc$n-$T $S/kernel-$T.img "$x" | tail -3; done
done
echo GUIFINISHED
