#!/bin/sh
# V21 acceptance: shprobe, slprobe and desktop run, base (cc5f476b) and head (858e5f68), sequential.

S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/v30t
M=measurements/measurement-2026-11-06-v30-test
P=tosdump/probes
export SHPROBE_SCRATCH=$S/sh SLPROBE_SCRATCH=$S/sl
set -x
for T in base head; do
  C=$( [ $T = base ] && echo cc5f476b || echo 858e5f68 )
  $P/run-shprobe.sh kernel $C $S/kernel-sh-$T.img
  $P/run-slprobe.sh kernel $C $S/kernel-sl-$T.img
done
for T in base head; do
  $P/run-shprobe.sh newtos $M/runs/newtos-shprobe-$T $S/kernel-sh-$T.img
  $P/run-shprobe.sh newtos-desk $M/runs/newtos-desk-$T $S/$T/target/aarch64-unknown-none-softfloat/release/kernel8.img
  $P/run-slprobe.sh newtos $M/runs/newtos-slprobe-$T $S/kernel-sl-$T.img
done
echo finished
