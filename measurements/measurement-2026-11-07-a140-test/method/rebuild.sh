#!/usr/bin/env bash
# env A140_FEATURES: cargo features of the kernel (gemdos-trace,aes-trace for the trace run).
# TASK/A140: re-arm the scratch tree (git archive of the commit under test) with a mouse script and build rpi4 release.
#   rebuild.sh <scratch-tree> <pristine-dir: host_env.rs launch.rs shell_loop.rs before arming> <mouse.txt> <kernel-out>
set -euo pipefail
T=$1; P=$2; M=$3; K=$4
cp "$P/host_env.rs" "$P/launch.rs" "$P/shell_loop.rs" "$T/kernel/src/"
python3 -I /Volumes/Daten/Development/PiRusTOS/tools/inject-arm.py "$T" "desktop=$M" | tail -1
(cd "$T" && cargo xtask build --board rpi4 --release ${A140_FEATURES:+--features "$A140_FEATURES"}) > "$K.build.log" 2>&1 || { tail -30 "$K.build.log" >&2; exit 1; }
cp "$T/target/aarch64-unknown-none-softfloat/release/kernel8.img" "$K"
shasum -a 256 "$K"
