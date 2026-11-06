#!/usr/bin/env bash
# run-all.sh <side> <kernel8.img|-> <out-dir> <case>...: cases of tosdump/probes/wfprobe.c, one boot
# and one fresh clone of the test image per case. WFPROBE_SCRATCH must be set.
#   before: newtos, kernel from bdfa6799 with sd-fault; emutos for the new cases
#   after : newtos, kernel of this run
set -u
SIDE=$1; K=$2; O=$3; shift 3
REPO=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$REPO"
mkdir -p "$O"
for c in "$@"; do
    if [ "$SIDE" = newtos ]; then
        tosdump/probes/run-wfprobe.sh newtos "$c" "$O/newtos-$c" "$K" >/dev/null 2>&1
        echo "newtos $c $(grep qemu "$O/newtos-$c/run.txt")"
    else
        tosdump/probes/run-wfprobe.sh emutos "$c" "$O/emutos-$c" >/dev/null 2>&1
        echo "emutos $c $(wc -l < "$O/emutos-$c/WFPROBE.LOG") lines"
    fi
done
