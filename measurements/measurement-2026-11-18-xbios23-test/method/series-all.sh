#!/usr/bin/env bash
# series-all.sh <scratch> <runsub commit|main> -- the four TeraDesk scenarios (method/series.sh) on one side
M="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; S=$1; export RUNSUB=$2
for x in td:vid-vdo td:vid-vdo-ext tdapps:apps tdapps:apps2; do
  bash $M/series.sh $S ${x%%:*} ${x##*:} 2>&1 | tail -1
done
