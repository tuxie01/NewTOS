#!/bin/bash
# slprobe repeat (noise class), desktop with injector at 640x480, 1920x1080, 640x240; boot screen + console at 3 geometries; base and head
cd /Volumes/Daten/Development/PiRusTOS
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/v33t
M=measurements/measurement-2026-11-06-v33-test
sh $M/method/slprobe-repeat.sh
export DESK_SCRATCH=$S/dk DESK_FILES=$S/files
for t in head base; do $M/method/desk-inj.sh newtos $M/runs/desk-$t $S/k-$t.img $M/method/desk-NEWDESK.INF $M/method/desk-shots.txt | tail -3; done
bash $M/method/geom-runs.sh
K=target/aarch64-unknown-none-softfloat/release/kernel8.img
for g in "480:" "240:-global bcm2835-fb.yres=240" "1080:-global bcm2835-fb.xres=1920 -global bcm2835-fb.yres=1080"; do
  n=${g%%:*}; x=${g#*:}
  for t in head base; do bash $M/method/boot-console.sh $M/runs/bc$n-$t $S/$t/$K "$x" | tail -3; done
done
echo finished
