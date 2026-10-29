#!/bin/sh
# probe-runs.sh <scratch> <runs-dir> <kernel-after.img> <kernel-before.img>
# -- the probe calls of this run (TASK/FS22). The probes, their builds and
# runners are permanent in tosdump/ (tosdump/probes/pxprobe.c + run-pxprobe.sh,
# pxprobe-compare.py, pxprobe-trace-check.py; tosdump/probes/gcprobe.c +
# run-gcprobe.sh, gcprobe-compare.py, gcprobe-trace-check.py from FS20;
# tosdump/Makefile targets pxprobe-m68k / pxprobe-newtos, gcprobe-*).
# Both kernels: cargo xtask build --board rpi4 --release --features
# bios-arg-trace, "after" in this tree (FS22 change uncommitted), "before" in a
# `git archive` copy of the base commit in scratch.
set -eu
S=$1; R=$2; KA=$3; KB=$4
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
cd "$REPO"
export PXPROBE_SCRATCH="$S" GCPROBE_SCRATCH="$S"
# pxprobe (acceptance): oracle twice, NewTOS after and before
tosdump/probes/run-pxprobe.sh emutos "$R/px-emutos-1"
tosdump/probes/run-pxprobe.sh emutos "$R/px-emutos-2"
tosdump/probes/run-pxprobe.sh newtos "$R/px-newtos-after" "$KA"
tosdump/probes/run-pxprobe.sh newtos "$R/px-newtos-before" "$KB"
# gcprobe (FS20, no-regression): oracle and NewTOS after
tosdump/probes/run-gcprobe.sh emutos "$R/gc-emutos"
tosdump/probes/run-gcprobe.sh newtos "$R/gc-newtos-after" "$KA"
for d in px-emutos-1 px-emutos-2 px-newtos-after px-newtos-before gc-emutos gc-newtos-after; do
    fsck_msdos -n "$R/$d/testpart-after.img" > "$R/$d/fsck-after.txt" 2>&1; echo "exit $?" >> "$R/$d/fsck-after.txt"
done
python3 tosdump/probes/pxprobe-compare.py emutos-1="$R/px-emutos-1" emutos-2="$R/px-emutos-2" > "$R/px-compare-emutos-1-vs-2.txt" || true
python3 tosdump/probes/pxprobe-compare.py emutos="$R/px-emutos-1" newtos-after="$R/px-newtos-after" > "$R/px-compare-after.txt" || true
python3 tosdump/probes/pxprobe-compare.py emutos="$R/px-emutos-1" newtos-before="$R/px-newtos-before" > "$R/px-compare-before.txt" || true
python3 tosdump/probes/pxprobe-trace-check.py "$R/px-newtos-after" > "$R/px-trace-check-after.txt" || true
python3 tosdump/probes/pxprobe-trace-check.py "$R/px-newtos-before" > "$R/px-trace-check-before.txt" || true
python3 tosdump/probes/gcprobe-compare.py emutos="$R/gc-emutos" newtos-after="$R/gc-newtos-after" > "$R/gc-compare-after.txt" || true
python3 tosdump/probes/gcprobe-trace-check.py "$R/gc-newtos-after" > "$R/gc-trace-check-after.txt" || true
# the partition images are scratch (gitignored *.img): keep their sha256 (run.txt), drop them
find "$R" -name 'testpart-*.img' -delete
