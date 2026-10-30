#!/usr/bin/env bash
#
# run-all.sh -- the six probe runs of this measurement (TASK/XBIOS06, step 1):
# tosdump/probes/run-rsprobe.sh twice per side, oracle tt, oracle falcontc,
# NewTOS raspi4b with a kernel built by
#   cargo xtask build --board rpi4 --release --features bios-arg-trace
# (copied to the scratch first; its sha256 is in each NewTOS run.txt).
#
# Usage: RSPROBE_SCRATCH=<scratch dir> method/run-all.sh <kernel8.img>
# Run from the repository root; writes runs/RSPROBE-* of this run directory.
set -euo pipefail
KERNEL=$1
RUN=measurements/measurement-2026-10-30-xbios06-rsprobe/runs
for n in 1 2; do
    tosdump/probes/run-rsprobe.sh emutos tt       "$RUN/RSPROBE-EMUTOS-tt-$n"
    tosdump/probes/run-rsprobe.sh emutos falcontc "$RUN/RSPROBE-EMUTOS-falcontc-$n"
    tosdump/probes/run-rsprobe.sh newtos          "$RUN/RSPROBE-NEWTOS-raspi4b-trace-$n" "$KERNEL"
done
