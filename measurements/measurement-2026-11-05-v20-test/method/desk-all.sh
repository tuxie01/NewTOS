#!/bin/sh
# V20 acceptance item 5, complete (pin never crosses the menu bar): armed kernels per commit, NewTOS head/base for the
# main scenes and the edge scenes, EmuTOS oracle for the edge scenes (twice). Run from the main tree.
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/15767204-8d89-43e0-aae0-d8149a16a17d/scratchpad/v20t
M=measurements/measurement-2026-11-05-v20-test
export DESK_SCRATCH=$S/dk
set -x
for p in "head bec04bab" "base 20d15c8c"; do
  set -- $p
  DESK_FILES= $M/method/desk-inj.sh kernel $2 $M/method/edge-mouse.txt $S/ke-$1.img | tail -1
  DESK_FILES= $M/method/desk-inj.sh newtos $M/runs/edge-$1 $S/ke-$1.img $M/method/edge-NEWDESK.INF $M/method/edge-shots.txt
  $M/method/desk-inj.sh kernel $2 $M/method/desk-mouse.txt $S/k-$1.img | tail -1
  DESK_FILES=$S/files $M/method/desk-inj.sh newtos $M/runs/desk-$1 $S/k-$1.img $M/method/desk-NEWDESK.INF $M/method/desk-shots.txt
done
for n in edge-emutos edge-emutos-2; do
  $M/method/run-deskedge-emutos.sh $M/runs/$n $M/method/edge-mouse-emutos-lead3000.txt $M/method/edge-shots-emutos-lead3000.txt $M/method/edge-NEWDESK.INF
done
echo finished
