#!/bin/sh
# V21 acceptance: shprobe, slprobe and desktop run, base (70f9b734) and head (f1006588), sequential.

S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/15767204-8d89-43e0-aae0-d8149a16a17d/scratchpad/v28t
M=measurements/measurement-2026-11-06-v28-test
P=tosdump/probes
export SHPROBE_SCRATCH=$S/sh SLPROBE_SCRATCH=$S/sl
set -x
for T in base head; do
  C=$( [ $T = base ] && echo 70f9b734 || echo f1006588 )
  $P/run-shprobe.sh kernel $C $S/kernel-sh-$T.img
  $P/run-slprobe.sh kernel $C $S/kernel-sl-$T.img
done
for T in base head; do
  $P/run-shprobe.sh newtos $M/runs/newtos-shprobe-$T $S/kernel-sh-$T.img
  $P/run-shprobe.sh newtos-desk $M/runs/newtos-desk-$T $S/$T/target/aarch64-unknown-none-softfloat/release/kernel8.img
  $P/run-slprobe.sh newtos $M/runs/newtos-slprobe-$T $S/kernel-sl-$T.img
done
echo finished
