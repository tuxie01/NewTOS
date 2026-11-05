#!/usr/bin/env bash
# v2-runs.sh <scratch> -- the probe-v2 runs (mnprobe.c with the MNPROBE.MFS option and a 2048-entry
# observer), as run, from the repository root: the M-first order on both sides, one default-order run
# per side (v2 default == v1 default), the --cmd-fifo cross-check with all 10 clicks logged.
set -uo pipefail
S=$1
M=measurements/measurement-2026-11-04-a127-oracle/method
export MNPROBE_SCRATCH=$S
r() { echo "$1 rc=$? $(grep -h '^M01\|^H00' "$S/runs/$1/MNPROBE.LOG" | tr '\n' ' ') $(grep -c '^R' "$S/runs/$1/MNPROBE.LOG") R $(grep -h '^hatari\|^qemu' "$S/runs/$1/run.txt")"; }
# oracle (emulated time; run in parallel)
MNPROBE_E1=1A MNPROBE_MFIRST=1 tosdump/probes/run-mnprobe.sh emutos tt "$S/runs/v2-emutos-tt-mfirst-e1a" > "$S/last-v2a.out" 2>&1 &
MNPROBE_E1=1A tosdump/probes/run-mnprobe.sh emutos tt "$S/runs/v2-emutos-tt-e1a" > "$S/last-v2b.out" 2>&1 &
MNPROBE_E1=1A MNPROBE_INJ=$PWD/$M/inj-empty.txt MNPROBE_FIFO=$PWD/$M/fifo-cross.txt \
    tosdump/probes/run-mnprobe.sh emutos tt "$S/runs/v2-emutos-tt-fifo-e1a" > "$S/last-v2c.out" 2>&1 &
MNPROBE_E1=1B MNPROBE_INJ=$PWD/$M/inj-empty.txt MNPROBE_FIFO=$PWD/$M/fifo-cross.txt \
    tosdump/probes/run-mnprobe.sh emutos tt "$S/runs/v2-emutos-tt-fifo-e1b" > "$S/last-v2d.out" 2>&1 &
wait
for n in v2-emutos-tt-mfirst-e1a v2-emutos-tt-e1a v2-emutos-tt-fifo-e1a v2-emutos-tt-fifo-e1b; do r $n; done
# NewTOS (wall-clock; one at a time)
MNPROBE_E1=1A MNPROBE_MFIRST=1 tosdump/probes/run-mnprobe.sh newtos "$S/runs/v2-newtos-head-mfirst-e1a" "$S/kernel-head.img" > "$S/last-v2e.out" 2>&1; r v2-newtos-head-mfirst-e1a
MNPROBE_E1=1A MNPROBE_MFIRST=1 tosdump/probes/run-mnprobe.sh newtos "$S/runs/v2-newtos-base-mfirst-e1a" "$S/kernel-base.img" > "$S/last-v2f.out" 2>&1; r v2-newtos-base-mfirst-e1a
MNPROBE_E1=1A tosdump/probes/run-mnprobe.sh newtos "$S/runs/v2-newtos-head-e1a" "$S/kernel-head.img" > "$S/last-v2g.out" 2>&1; r v2-newtos-head-e1a
echo V2-DONE
