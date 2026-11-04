#!/usr/bin/env bash
# run-r1.sh <scratch> <head|base> <script> <tag> <card> -- dbg-instrumented injector run of a script
set -euo pipefail
REPO=$(cd "$(dirname "$0")/../../.." && pwd)
M=$REPO/measurements/measurement-2026-11-04-a127-d1-test; A=$REPO/measurements/measurement-2026-11-04-a127/method
S=$1; W=$2; SC=$3; TAG=$4; CARD=${5:-$S/base.img}
"$M/method/dbg-build.sh" "$S/$W-src" "$S/$W" "$SC" > "$M/serial/$TAG.kernel.sha256"
cp "$S/$W/target/aarch64-unknown-none-softfloat/release/kernel8.img" "$S/kernel-$TAG.img"
"$A/run.sh" "$S/kernel-$TAG.img" "$CARD" "$TAG" "$M/serial" "$S" $(seq 16 2 66)
mv "$M/serial/$TAG"-*.png "$M/screendumps/"
