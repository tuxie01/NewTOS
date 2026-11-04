#!/usr/bin/env bash
# run-test.sh <scratch> <stage> [X] -- verification of A127 D1; only calls the A127 method scripts.
# scratch layout: <scratch>/head = git archive of bae52dbd, <scratch>/base = of 4d44a616.
# <scratch>/<W>-src holds the pristine kernel/src/{host_env,launch,shell_loop}.rs of that commit.
# stages: setup | rpi4 <A|B|C> <head|base> | virt <head|base>
set -euo pipefail
REPO=$(cd "$(dirname "$0")/../../.." && pwd)
S=${1:?scratch}; STAGE=${2:?}
M=$REPO/measurements/measurement-2026-11-04-a127-d1-test
A=$REPO/measurements/measurement-2026-11-04-a127/method
case "$STAGE" in
setup)
  cc -O2 -o "$S/qmpshot" "$REPO/tools/qmpshot.c"
  MKCARD_SIZE_MIB=128 "$REPO/tools/mkcard.sh" "$S/base.img" "$A/NEWDESK-W.INF"
  cp "$S/base.img.manifest" "$M/base.img.manifest"; shasum -a 256 "$S/base.img" > "$M/serial/base.img.sha256" ;;
rpi4)
  X=${3:?}; W=${4:?head|base}
  case "$X" in
    A) SCRIPT=s-a-hover-leave-reenter-click.script; SHOTS=$(seq 16 36) ;;
    B) SCRIPT=s-b-slow-phases.script; SHOTS=$(seq 16 44) ;;
    C) SCRIPT=s-c-fast-clicks.script; SHOTS=$(seq 16 40) ;;
  esac
  "$A/build-kernel.sh" "$S/$W-src" "$S/$W" desktop="$A/$SCRIPT" > "$M/serial/run$X-$W.kernel.sha256"
  cp "$S/$W/target/aarch64-unknown-none-softfloat/release/kernel8.img" "$S/kernel-$X-$W.img"
  "$A/run.sh" "$S/kernel-$X-$W.img" "$S/base.img" "run$X-$W" "$M/serial" "$S" $SHOTS
  mv "$M/serial/run$X-$W"-*.png "$M/screendumps/" ;;
esac
