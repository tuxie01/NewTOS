#!/bin/bash
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/15767204-8d89-43e0-aae0-d8149a16a17d/scratchpad/v32t
M=measurements/measurement-2026-11-06-v32-test
export DESK_SCRATCH=$S/dk DESK_FILES=$S/files
for p in head:937d6361 base:c5fff3be; do t=${p%%:*}; c=${p##*:}
  $M/method/desk-inj.sh kernel $c $M/method/desk240-mouse.txt $S/k240-$t.img | tail -1 | cut -c1-120
  QEMU_EXTRA="-global bcm2835-fb.yres=240" $M/method/desk-inj.sh newtos $M/runs/desk240b-$t $S/k240-$t.img $M/method/desk-NEWDESK.INF $M/method/desk-shots.txt | tail -3
done
echo finished
