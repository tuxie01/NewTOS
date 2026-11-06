#!/bin/sh
# V21 acceptance: desktop with IKBD injector, base (587ddbd5) and head (877a14b9). Run from the main tree.
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/v33t
M=measurements/measurement-2026-11-06-v33-test
export DESK_SCRATCH=$S/dk
set -x
for p in "head 877a14b9" "base 587ddbd5"; do
  set -- $p
  $M/method/desk-inj.sh kernel $2 $M/method/desk-mouse.txt $S/k-$1.img | tail -1
  DESK_FILES=$S/files $M/method/desk-inj.sh newtos $M/runs/desk-$1 $S/k-$1.img $M/method/desk-NEWDESK.INF $M/method/desk-shots.txt
done
echo finished
