#!/usr/bin/env bash
# run-8k.sh <side> <kernel8.img|-> <out-dir>: case n1 on a test disk with 8-KiB clusters as on the user's
# cards (16 sectors, FATs of 32 sectors), everything else as tools/mktestdisk.sh. Copies of the two scripts
# in the scratch, patched by sed, nothing in the tree changes. WFPROBE_SCRATCH must be set.
set -eu
SIDE=$1; K=$2; O=$3
REPO=$(cd "$(dirname "$0")/../../.." && pwd)
T=$WFPROBE_SCRATCH/8k; mkdir -p "$T"
sed -e 's/-c 4 -r 32 -L 128/-c 16 -r 32 -L 32/' -e "s#^SCRIPT_DIR=.*#SCRIPT_DIR=$REPO/tools#" "$REPO/tools/mktestdisk.sh" > "$T/mktestdisk.sh"
sed -e "s#^HERE=.*#HERE=$REPO/tosdump/probes#" -e "s#tools/mktestdisk.sh#$T/mktestdisk.sh#" "$REPO/tosdump/probes/run-wfprobe.sh" > "$T/run-wfprobe.sh"
chmod +x "$T/mktestdisk.sh" "$T/run-wfprobe.sh"
if [ "$SIDE" = newtos ]; then "$T/run-wfprobe.sh" newtos n1 "$O" "$K"; else "$T/run-wfprobe.sh" emutos n1 "$O"; fi
