#!/bin/sh
# A131 runs, sequential (every NewTOS run alone on the host). From the main tree.
# S = session scratchpad/a131. Kernels:
#   $S/kernel-shprobe.img  run-shprobe.sh kernel HEAD (f657cfe4, armed launch:SHPROBE)
#   $S/tree/.../kernel8.img plain `cargo xtask build --board rpi4 --release` of a git clone of f657cfe4
#                           (built by the xtask tosdump run, method/newtos-palette.sh)
# probe-1 runs: emutos tt, emutos falcontc, newtos (first, by hand, same commands)
set -x
R=measurements/measurement-2026-11-05-a131/runs
P=tosdump/probes/run-shprobe.sh
export SHPROBE_SCRATCH=$S/sh
$P emutos tt $R/emutos-tt-probe-2
$P emutos falcontc $R/emutos-falcontc-probe-2
$P newtos $R/newtos-probe-2 $S/kernel-shprobe.img
$P emutos-desk tt $R/emutos-tt-desk-1
$P emutos-desk falcontc $R/emutos-falcontc-desk-1
$P newtos-desk $R/newtos-desk-1 $S/tree/target/aarch64-unknown-none-softfloat/release/kernel8.img
