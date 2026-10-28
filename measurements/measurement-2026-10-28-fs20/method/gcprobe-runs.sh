#!/bin/sh
# gcprobe-runs.sh <scratch> <runs-dir> <kernel-after.img> <kernel-before.img>
# -- the calls of this run (TASK/FS20). The probe, its build and its runner are
# permanent in tosdump/ (tosdump/probes/gcprobe.c, tosdump/Makefile targets
# gcprobe-m68k / gcprobe-newtos, tosdump/build-gcprobe-newtos.sh,
# tosdump/probes/run-gcprobe.sh, gcprobe-compare.py, gcprobe-trace-check.py).
# Both kernels: cargo xtask build --board rpi4 --release --features bios-arg-trace,
# "after" in this tree (FS20 change uncommitted on d47be1b4), "before" in a
# `git archive HEAD` copy of a36ea390 in scratch (product code = 45ed73d0).
set -eu
S=$1; R=$2; KA=$3; KB=$4
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
cd "$REPO"
export GCPROBE_SCRATCH="$S"
tosdump/probes/run-gcprobe.sh emutos "$R/gc-emutos-1"
tosdump/probes/run-gcprobe.sh emutos "$R/gc-emutos-2"
tosdump/probes/run-gcprobe.sh newtos "$R/gc-newtos-after" "$KA"
tosdump/probes/run-gcprobe.sh newtos "$R/gc-newtos-before" "$KB"
for d in gc-emutos-1 gc-emutos-2 gc-newtos-after gc-newtos-before; do
    fsck_msdos -n "$R/$d/testpart-after.img" > "$R/$d/fsck-after.txt" 2>&1; echo "exit $?" >> "$R/$d/fsck-after.txt"
done
python3 tosdump/probes/gcprobe-compare.py emutos-1="$R/gc-emutos-1" emutos-2="$R/gc-emutos-2" > "$R/compare-emutos-1-vs-2.txt" || true
python3 tosdump/probes/gcprobe-compare.py emutos="$R/gc-emutos-1" newtos-after="$R/gc-newtos-after" > "$R/compare-after.txt" || true
python3 tosdump/probes/gcprobe-compare.py emutos="$R/gc-emutos-1" newtos-before="$R/gc-newtos-before" > "$R/compare-before.txt" || true
python3 tosdump/probes/gcprobe-trace-check.py "$R/gc-newtos-after" > "$R/trace-check-after.txt" || true
python3 tosdump/probes/gcprobe-trace-check.py "$R/gc-newtos-before" > "$R/trace-check-before.txt" || true
