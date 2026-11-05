#!/usr/bin/env bash
# all-runs.sh -- the calls of this run, in order, from the repository root (TASK/A130, GitHub #2).
# SCR = the session scratchpad's a130/ (never /tmp, deleted after the run).
set -euo pipefail
export SLPROBE_SCRATCH=$SCR
R=measurements/measurement-2026-11-05-a130
M=$R/method
# canary (docs/analysis/ORACLE.md section 6) -> runs/canary.txt
sh measurements/measurement-2026-10-28-oracle-canary/method/canary.sh "$SCR/canary-$(date +%Y%m%d-%H%M%S)"

# exploration (geometry of the oracle's scroll bars; output in scratch, not kept)
python3 -I $M/gen-explore.py "$SCR/explore-mouse.txt" "$SCR/explore-shots.txt"
SLPROBE_MOUSE=$SCR/explore-mouse.txt SLPROBE_SHOTS=$SCR/explore-shots.txt \
    tosdump/probes/run-slprobe.sh emutos "$SCR/explore-emutos"

# main input v1 (method/gen-slprobe-input-v1.py -> slprobe-mouse-v1.txt / slprobe-shots-v1.txt)
python3 -I $M/gen-slprobe-input-v1.py tosdump/probes/slprobe-mouse.txt tosdump/probes/slprobe-shots.txt $M/slprobe-cases-v1.tsv
tosdump/probes/run-slprobe.sh kernel HEAD "$SCR/kernel-head-v1.img"
tosdump/probes/run-slprobe.sh emutos $R/runs/emutos-tt-v1-1
tosdump/probes/run-slprobe.sh newtos $R/runs/newtos-head-v1-1 "$SCR/kernel-head-v1.img"
tosdump/probes/run-slprobe.sh newtos $R/runs/newtos-head-v1-2 "$SCR/kernel-head-v1.img"
tosdump/probes/run-slprobe.sh emutos $R/runs/emutos-tt-v1-2

# main input v2 (gen-slprobe-input-v2.py: idle 'before' shot + 100 ticks before the fast-drag press)
python3 -I $M/gen-slprobe-input-v2.py tosdump/probes/slprobe-mouse.txt tosdump/probes/slprobe-shots.txt $M/slprobe-cases-v2.tsv
tosdump/probes/run-slprobe.sh kernel HEAD "$SCR/kernel-head-v2.img"
tosdump/probes/run-slprobe.sh emutos $R/runs/emutos-tt-v2-1
tosdump/probes/run-slprobe.sh newtos $R/runs/newtos-head-v2-1 "$SCR/kernel-head-v2.img"

# flicker input v1 (method/flicker-*-v1.txt; moved off the bar 40 ticks after the press)
SLPROBE_MOUSE=$PWD/$M/flicker-mouse-v1.txt SLPROBE_SHOTS=$PWD/$M/flicker-shots-emutos-v1.txt \
    tosdump/probes/run-slprobe.sh emutos $R/runs/emutos-tt-flicker-v1-1

# main input v3 = tosdump/probes/slprobe-mouse.txt / slprobe-shots.txt (gen-slprobe-input.py:
# 100 ticks still after the fast-drag press); flicker input v2 = method/flicker-*.txt
python3 -I $M/gen-slprobe-input.py tosdump/probes/slprobe-mouse.txt tosdump/probes/slprobe-shots.txt $M/slprobe-cases.tsv
python3 -I $M/gen-flicker-input.py $M/flicker-mouse.txt $M/flicker-shots-emutos.txt $M/flicker-shots-newtos.txt
tosdump/probes/run-slprobe.sh kernel HEAD "$SCR/kernel-head-v3.img"
SLPROBE_MOUSE=$PWD/$M/flicker-mouse.txt tosdump/probes/run-slprobe.sh kernel HEAD "$SCR/kernel-head-flicker2.img"
SLPROBE_MOUSE=$PWD/$M/flicker-mouse.txt SLPROBE_SHOTS=$PWD/$M/flicker-shots-emutos.txt \
    tosdump/probes/run-slprobe.sh emutos $R/runs/emutos-tt-flicker-v2-1
SLPROBE_MOUSE=$PWD/$M/flicker-mouse.txt SLPROBE_SHOTS=$PWD/$M/flicker-shots-newtos.txt \
    tosdump/probes/run-slprobe.sh newtos $R/runs/newtos-head-flicker-v2-1 "$SCR/kernel-head-flicker2.img"
for r in 1 2; do tosdump/probes/run-slprobe.sh newtos $R/runs/newtos-head-v3-$r "$SCR/kernel-head-v3.img"; done
for r in 1 2; do tosdump/probes/run-slprobe.sh emutos $R/runs/emutos-tt-v3-$r; done

# evaluation
python3 $M/compare.py $R/runs/emutos-tt-v1-1/shots $R/runs/newtos-head-v1-1/shots $R/analysis/cmp-v1-1
python3 $M/compare.py $R/runs/emutos-tt-v2-1/shots $R/runs/newtos-head-v2-1/shots $R/analysis/cmp-v2-1
for r in 1 2; do
    python3 $M/compare.py $R/runs/emutos-tt-v3-$r/shots $R/runs/newtos-head-v3-$r/shots $R/analysis/cmp-v3-$r
done
for d in emutos-tt-flicker-v1-1 emutos-tt-flicker-v2-1 newtos-head-flicker-v2-1; do
    python3 $M/flicker.py $R/runs/$d/shots > $R/analysis/flicker-v2/$d.txt
done
