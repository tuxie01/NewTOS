#!/bin/sh
# gcprobe-runs.sh <scratch> <runs-dir> <kernel-after.img> <kernel-before.img>
# -- TASK/FS21: the FS20 probe (tosdump/probes/gcprobe.c, permanent in tosdump/)
# once on the oracle and on NewTOS before (HEAD) and after (FS21), through
# tosdump/probes/run-gcprobe.sh; the test partitions compared with
# tosdump/probes/gcprobe-compare.py, the input traces with gcprobe-trace-check.py.
# Both kernels: cargo xtask build --board rpi4 --release --features bios-arg-trace,
# "after" in this tree, "before" in a `git archive HEAD` copy in scratch.
set -u
S=$1; R=$2; KA=$3; KB=$4
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
cd "$REPO"
export GCPROBE_SCRATCH="$S"
tosdump/probes/run-gcprobe.sh emutos "$R/gc-emutos" > "$S/gc-emutos.stdout" 2>&1; echo "gc-emutos exit $?"
tosdump/probes/run-gcprobe.sh newtos "$R/gc-newtos-after" "$KA" > "$S/gc-after.stdout" 2>&1; echo "gc-newtos-after exit $?"
tosdump/probes/run-gcprobe.sh newtos "$R/gc-newtos-before" "$KB" > "$S/gc-before.stdout" 2>&1; echo "gc-newtos-before exit $?"
for d in gc-emutos gc-newtos-after gc-newtos-before; do
    fsck_msdos -n "$R/$d/testpart-after.img" > "$R/$d/fsck-after.txt" 2>&1; echo "exit $?" >> "$R/$d/fsck-after.txt"
    mv "$R/$d/testpart-after.img" "$S/$d-testpart-after.img"
    ln -s "$S/$d-testpart-after.img" "$R/$d/testpart-after.img"
done
python3 tosdump/probes/gcprobe-compare.py emutos="$R/gc-emutos" newtos-after="$R/gc-newtos-after" > "$R/compare-after.txt"; echo "compare-after exit $?"
python3 tosdump/probes/gcprobe-compare.py emutos="$R/gc-emutos" newtos-before="$R/gc-newtos-before" > "$R/compare-before.txt"; echo "compare-before exit $?"
python3 tosdump/probes/gcprobe-trace-check.py "$R/gc-newtos-after" > "$R/trace-check-after.txt"; echo "trace-check-after exit $?"
python3 tosdump/probes/gcprobe-trace-check.py "$R/gc-newtos-before" > "$R/trace-check-before.txt"; echo "trace-check-before exit $?"
for d in gc-emutos gc-newtos-after gc-newtos-before; do rm -f "$R/$d/testpart-after.img"; done
