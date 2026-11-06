#!/usr/bin/env bash
# run-all.sh <kernel8.img> <out-dir>: every case of tosdump/probes/wfprobe.c on both sides, one
# boot and one fresh clone of the test image per case. WFPROBE_SCRATCH must be set.
set -u
K=$1; O=$2
REPO=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$REPO"
for c in a1 a2 a3 a4 a5 b1 b2 b3 b4 b5 d1 t1 t2 u1 u2 u3 u4 u5 u6 w1 o1; do
    tosdump/probes/run-wfprobe.sh newtos $c "$O/newtos-$c" "$K" >/dev/null 2>&1
    echo "newtos $c $(grep qemu "$O/newtos-$c/run.txt")"
done
for c in a1 a2 a3 a4 a5 b1 b2 b3 b4 b5 t1 t2 c1; do
    tosdump/probes/run-wfprobe.sh emutos $c "$O/emutos-$c" >/dev/null 2>&1
    echo "emutos $c $(wc -l < "$O/emutos-$c/WFPROBE.LOG") lines"
done
