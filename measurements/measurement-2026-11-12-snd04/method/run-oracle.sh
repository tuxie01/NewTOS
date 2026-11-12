#!/bin/sh
# run-oracle.sh <run dir> -- the oracle side of TASK/SND04: tosdump/probes/run-snprobe.sh
# emutos <profile>, two runs per profile (SNPROBE_SCRATCH = the session scratchpad).
M=$1
for p in falcontc tt; do for n in 1 2; do
  tosdump/probes/run-snprobe.sh emutos $p "$M/oracle/$p-run$n"
done; done
# input check: a copy of tools/hatari-oracle.sh with --trace xbios (hatari-oracle-xbiostrace.sh)
for p in falcontc tt; do sh "$M/method/hatari-oracle-xbiostrace.sh" $p tosdump/build/SNPROBE.PRG - "$SNPROBE_SCRATCH/trace-$p"; done
