#!/usr/bin/env bash
# all-runs.sh -- the calls of this run, in order, from the repository root (TASK/A127 D2/D3).
# SCR = the session scratchpad's a127gap/ (never /tmp, deleted after the run).
set -euo pipefail
export MGPROBE_SCRATCH=$SCR
R=measurements/measurement-2026-11-05-a127-gap
# canary (docs/analysis/ORACLE.md section 6) -> runs/canary.txt
sh measurements/measurement-2026-10-28-oracle-canary/method/canary.sh "$SCR/canary-$(date +%Y%m%d-%H%M%S)"

# v1 input (pin to (0,0); kept as method/mgprobe-mouse-v1.txt / mgprobe-shots-v1.txt)
python3 -I $R/method/gen-mgprobe-input.py tosdump/probes/mgprobe-mouse.txt tosdump/probes/mgprobe-shots.txt   # (v1 version of the generator)
tosdump/probes/run-mgprobe.sh kernel HEAD "$SCR/kernel-head.img"
tosdump/probes/run-mgprobe.sh emutos $R/runs/emutos-tt-1
tosdump/probes/run-mgprobe.sh newtos $R/runs/newtos-head-1 "$SCR/kernel-head.img"

# v2 input (pin to (0,479)); probe v2 = method/mgprobe-v2.c
python3 -I $R/method/gen-mgprobe-input.py tosdump/probes/mgprobe-mouse.txt tosdump/probes/mgprobe-shots.txt
tosdump/probes/run-mgprobe.sh kernel HEAD "$SCR/kernel-head-v2.img"
tosdump/probes/run-mgprobe.sh emutos $R/runs/emutos-tt-v2-2
tosdump/probes/run-mgprobe.sh emutos $R/runs/emutos-tt-v2-3

# probe v3 = v2 + F-lines (objc_find at the probed points), same input, same kernel
for r in 1 2; do tosdump/probes/run-mgprobe.sh emutos $R/runs/emutos-tt-v3-$r; done
for r in 1 2; do tosdump/probes/run-mgprobe.sh newtos $R/runs/newtos-head-v3-$r "$SCR/kernel-head-v2.img"; done

# evaluation
python3 $R/method/compare.py $R/runs/emutos-tt-1/shots $R/runs/newtos-head-1/shots $R/analysis/cmp-emutos1-newtos1
for r in 1 2; do
    python3 $R/method/compare.py $R/runs/emutos-tt-v3-$r/shots $R/runs/newtos-head-v3-$r/shots $R/analysis/cmp-v3-$r
done
