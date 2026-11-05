#!/bin/sh
# V26 acceptance item 4: shprobe, slprobe and desktop run, base (20d15c8c) and head (bec04bab), sequential.
# Run from the main tree. Scratch: /private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/15767204-8d89-43e0-aae0-d8149a16a17d/scratchpad/v20t
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/15767204-8d89-43e0-aae0-d8149a16a17d/scratchpad/v20t
M=measurements/measurement-2026-11-05-v20-test
P=tosdump/probes
export SHPROBE_SCRATCH=$S/sh SLPROBE_SCRATCH=$S/sl
set -x
for T in base head; do
  C=$( [ $T = base ] && echo 20d15c8c || echo bec04bab )
  $P/run-shprobe.sh kernel $C $S/kernel-sh-$T.img
  $P/run-slprobe.sh kernel $C $S/kernel-sl-$T.img
done
for T in base head; do
  $P/run-shprobe.sh newtos $M/runs/newtos-shprobe-$T $S/kernel-sh-$T.img
  $P/run-shprobe.sh newtos-desk $M/runs/newtos-desk-$T $S/$T/target/aarch64-unknown-none-softfloat/release/kernel8.img
  $P/run-slprobe.sh newtos $M/runs/newtos-slprobe-$T $S/kernel-sl-$T.img
done
echo finished
