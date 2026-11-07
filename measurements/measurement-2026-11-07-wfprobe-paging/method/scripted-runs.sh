#!/usr/bin/env bash
# scripted-runs.sh <scratch> <out> -- the scripted wfprobe (WFCASE) before and
# after the paging change, same kernel, QEMU raspi4b (tosdump/probes/run-wfprobe.sh).
#   before: a git clone of HEAD (wfprobe.c without paging) in <scratch>, the
#           built libc of the main tree linked in
#   after:  the working tree
# Kernel: the Pi kernel measurements/measurement-2026-11-06-hw08-fix-kernel/kernel8.img
# (rpi4 --release --features crash-walk,sd-fault, 16bfa004).
set -euo pipefail
SCR=$1; OUT=$2
REPO=$(cd "$(dirname "$0")/../../.." && pwd)
KERNEL=$REPO/measurements/measurement-2026-11-06-hw08-fix-kernel/kernel8.img
CASES=${CASES:-u2 a3}
mkdir -p "$SCR" "$OUT"
if [ ! -d "$SCR/clone-before" ]; then
    git clone -q "$REPO" "$SCR/clone-before"
    ln -s "$REPO/sdk/libc/libc.a" "$SCR/clone-before/sdk/libc/libc.a"
    ln -s "$REPO/sdk/libc/mintlib" "$SCR/clone-before/sdk/libc/mintlib"
fi
echo "clone HEAD $(git -C "$SCR/clone-before" rev-parse HEAD)" > "$OUT/clone.txt"
for c in $CASES; do
    WFPROBE_SCRATCH="$SCR/runs" "$SCR/clone-before/tosdump/probes/run-wfprobe.sh" newtos "$c" "$OUT/before-$c" "$KERNEL"
    WFPROBE_SCRATCH="$SCR/runs" "$REPO/tosdump/probes/run-wfprobe.sh" newtos "$c" "$OUT/after-$c" "$KERNEL"
done
