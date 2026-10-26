#!/usr/bin/env bash
# qemu-all.sh -- A118: every QEMU raspi4b run of the regression, one QEMU at a time (kernels from build-all.sh, lint-both.sh and
# a118/run-a118.sh build; cards from mkcards.sh, api/api-card.sh and a118/run-a118.sh card). Per-run APFS clones, deleted after.
#   reg  : series.sh head|wt on nd td qed  (GH1 regression series, memprot on)  + rerun.sh std (make-sd-image card, nd script)
#   a117 : a117/series.sh base|new 1
#   tdq  : a117/tdq.sh run base|new 1      (TeraDesk launch + quit back to NEWDESK)
#   a118 : a118/run-a118.sh run base|new 1  (the effect probe: a file selector opened by a desk accessory)
#   api  : api/api-series.sh (APITEST #Z, memprot on/off/strict, base vs new)
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
for side in head wt; do bash "$HERE/reg/series.sh" $side on nd td qed; done
for side in head wt; do bash "$HERE/reg/rerun.sh" $side 1 std; done
for side in base new; do bash "$HERE/a117/series.sh" $side 1; done
for side in base new; do bash "$HERE/a117/tdq.sh" run $side 1; done
for side in base new; do bash "$HERE/a118/run-a118.sh" run $side 1; done
bash "$HERE/api/api-series.sh"
echo QEMU-ALL DONE
