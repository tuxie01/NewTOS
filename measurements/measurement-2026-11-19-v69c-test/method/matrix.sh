#!/usr/bin/env bash
# matrix.sh -- the PRESENT runs. side: wt = d82cf2a23, wt-main = 2d00b4bb5 (negative control), alert = scratch variant refusing 32
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
bash $HERE/pr-run.sh wt v16 "serial=on"
bash $HERE/pr-run.sh wt-main v16 "serial=on"
bash $HERE/pr-run.sh wt v16b "serial=on"
bash $HERE/pr-run.sh wt v16x "serial=on vdiplanes=16"
bash $HERE/pr-run.sh wt v32 "serial=on vdiplanes=32"
bash $HERE/pr-run.sh wt v32b "serial=on vdiplanes=32"
TMO=40 bash $HERE/pr-run.sh wt-main v32 "serial=on vdiplanes=32"
TMO=40 bash $HERE/pr-run.sh alert v32 "serial=on vdiplanes=32"
echo MATRIXDONE
