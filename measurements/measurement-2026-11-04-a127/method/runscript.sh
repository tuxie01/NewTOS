#!/usr/bin/env bash
# runscript.sh <tag> <script> <shot-wall-seconds...> : build the injector kernel for <script>, run once.
set -euo pipefail
REPO=$(cd "$(dirname "$0")/../../.." && pwd); S=${A127_SCRATCH:?}; M=$REPO/measurements/measurement-2026-11-04-a127
TAG=$1; SCRIPT=$2; shift 2
"$M/method/build-kernel.sh" "$REPO" "$S/tree" desktop="$SCRIPT" | tee "$M/serial/$TAG.kernel.sha256"
cp "$S/tree/target/aarch64-unknown-none-softfloat/release/kernel8.img" "$S/kernel-$TAG.img"
"$M/method/run.sh" "$S/kernel-$TAG.img" "$S/base.img" "$TAG" "$M/serial" "$S" "$@"
