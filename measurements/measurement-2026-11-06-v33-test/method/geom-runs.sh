#!/bin/bash
# desktop with the IKBD injector at 640x240 and 1920x1080 (QEMU bcm2835-fb geometry), base and head
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/v33t
M=measurements/measurement-2026-11-06-v33-test
export DESK_SCRATCH=$S/dk DESK_FILES=$S/files
for g in "240:-global bcm2835-fb.yres=240" "1080:-global bcm2835-fb.xres=1920 -global bcm2835-fb.yres=1080"; do
  n=${g%%:*}; x=${g#*:}
  for t in head base; do
    QEMU_EXTRA="$x" $M/method/desk-inj.sh newtos $M/runs/desk$n-$t $S/k-$t.img $M/method/desk-NEWDESK.INF $M/method/desk-shots.txt | tail -3
  done
done
echo finished
