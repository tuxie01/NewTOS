#!/bin/bash
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/v33t
M=measurements/measurement-2026-11-06-v33-test
export DESK_SCRATCH=$S/dk DESK_FILES=$S/files
for p in head:877a14b9 base:587ddbd5; do t=${p%%:*}; c=${p##*:}
  $M/method/desk-inj.sh kernel $c $M/method/desk240-mouse.txt $S/k240-$t.img | tail -1 | cut -c1-120
  QEMU_EXTRA="-global bcm2835-fb.yres=240" $M/method/desk-inj.sh newtos $M/runs/desk240b-$t $S/k240-$t.img $M/method/desk-NEWDESK.INF $M/method/desk-shots.txt | tail -3
done
echo finished
