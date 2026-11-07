#!/bin/bash
# V35 test, second part: desk1080-base rerun (the first try failed in mkcard: "hdiutil ... Simulators are still shutting down", host), then
# boot-screen timing at 1920x1080: base / fix / fixpad (fix + 4 KiB dead data = base's image size), 4 rounds interleaved.
cd /Volumes/Daten/Development/PiRusTOS
S=${S:?}; M=measurements/measurement-2026-11-07-v35-test; G=$M/runs/gui
export DESK_SCRATCH=$S/dk DESK_FILES=$S/files S
mv $G/desk1080-base $G/desk1080-base-failed-mkcard
QEMU_EXTRA="-global bcm2835-fb.xres=1920 -global bcm2835-fb.yres=1080" $M/method/desk-inj.sh newtos $G/desk1080-base $S/k-base.img $M/method/desk-NEWDESK.INF $M/method/desk-shots.txt | tail -3
mkdir -p $M/runs/bct
for r in 1 2 3 4; do for T in base fix fixpad; do
  bash $M/method/bc-timing.sh $M/runs/bct $S/k/$T-rpi4-release.img $T-$r
done; done
echo GUI2FINISHED
