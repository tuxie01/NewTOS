#!/usr/bin/env bash
# newtos-runs.sh <scratch> <name> <kernel8.img> <E1|-> -- one NewTOS run of tosdump/probes/mnprobe.c
# as run (A127 oracle measurement), from the repository root. The kernels were built with
#   MNPROBE_SCRATCH=<scratch> tosdump/probes/run-mnprobe.sh kernel bae52dbd <scratch>/kernel-head.img
#   MNPROBE_SCRATCH=<scratch> tosdump/probes/run-mnprobe.sh kernel 4d44a616 <scratch>/kernel-base.img
#   ... kernel bae52dbd <scratch>/kernel-head-dbg.img dbg ; ... kernel 4d44a616 <scratch>/kernel-base-dbg.img dbg
set -euo pipefail
S=$1; NAME=$2; K=$3; E1=$4
export MNPROBE_SCRATCH=$S
[ "$E1" = - ] && E1=
MNPROBE_E1=$E1 tosdump/probes/run-mnprobe.sh newtos "$S/runs/$NAME" "$K" > "$S/last-$NAME.out" 2>&1
grep -E "^(H00|Z99)" "$S/runs/$NAME/MNPROBE.LOG"; grep '^qemu' "$S/runs/$NAME/run.txt"
