#!/usr/bin/env bash
# oracle-runs.sh <scratch> <name> <profile> <E1|-> -- one oracle run of tosdump/probes/mnprobe.c as run
# (A127 oracle measurement, 2026-11-04/05), from the repository root. <E1> "-" = unset.
set -euo pipefail
S=$1; NAME=$2; PROF=$3; E1=$4
export MNPROBE_SCRATCH=$S
[ "$E1" = - ] && E1=
MNPROBE_E1=$E1 tosdump/probes/run-mnprobe.sh emutos "$PROF" "$S/runs/$NAME" > "$S/last-$NAME.out" 2>&1
grep -E "^(H00|Z99)" "$S/runs/$NAME/MNPROBE.LOG"
