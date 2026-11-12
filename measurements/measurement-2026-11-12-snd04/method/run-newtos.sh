#!/bin/sh
# run-newtos.sh <run dir> <frozen tree> <scratch> -- the NewTOS side of TASK/SND04.
# 1. bios-r1.tsv (DMA sound inquiry rows 230-235) via cargo xtask tosdump --tree, bios-arg-trace.
# 2. the probe on the kernel that run built (bios-arg-trace kernel: the serial log carries the input trace).
M=$1; T=$2; S=$3
sh "$M/method/newtos-bios-r1.sh" "$T" "$S" "$M/newtos/bios-r1"
SNPROBE_SCRATCH=$S tosdump/probes/run-snprobe.sh newtos "$M/newtos/snprobe" \
  "$T/target/aarch64-unknown-none-softfloat/release/kernel8.img"
