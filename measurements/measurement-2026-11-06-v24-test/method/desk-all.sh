#!/bin/sh
# V21 acceptance: desktop with IKBD injector, base (ef3c0b5c) and head (caf77fab). Run from the main tree.
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/15767204-8d89-43e0-aae0-d8149a16a17d/scratchpad/v24t
M=measurements/measurement-2026-11-06-v24-test
export DESK_SCRATCH=$S/dk
set -x
for p in "head caf77fab" "base ef3c0b5c"; do
  set -- $p
  $M/method/desk-inj.sh kernel $2 $M/method/desk-mouse.txt $S/k-$1.img | tail -1
  DESK_FILES=$S/files $M/method/desk-inj.sh newtos $M/runs/desk-$1 $S/k-$1.img $M/method/desk-NEWDESK.INF $M/method/desk-shots.txt
done
echo finished
