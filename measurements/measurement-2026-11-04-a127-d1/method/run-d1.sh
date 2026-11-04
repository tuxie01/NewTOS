#!/usr/bin/env bash
# run-d1.sh <scratch> <stage> -- A127 D1 evidence runs. Only calls: the build,
# tools/mkcard.sh, tools/qmpshot.c, and the A127 run's own method scripts with
# their injector scripts / QMP step files (measurements/measurement-2026-11-04-a127/
# method, called in place, not copied). <scratch> = session scratchpad dir.
#   setup  : scratch copy of the working tree (with the D1 change), qmpshot, the card
#   rpi4 X : raspi4b + injector, X in A B C (the A127 scripts s-a / s-b / s-c)
#   virt   : qemu-virt, real USB mouse over QMP (the A127 q-e / q-f step files)
set -euo pipefail
REPO=$(cd "$(dirname "$0")/../../.." && pwd)
S=${1:?scratch}; STAGE=${2:?stage}
M=$REPO/measurements/measurement-2026-11-04-a127-d1
A=$REPO/measurements/measurement-2026-11-04-a127/method
case "$STAGE" in
setup)
  rsync -a --delete --exclude .git --exclude /target --exclude /references --exclude /measurements \
    --exclude /Applications --exclude /hardware --exclude /graphify-out --exclude /.venv "$REPO/" "$S/tree/"
  cc -O2 -o "$S/qmpshot" "$REPO/tools/qmpshot.c"
  MKCARD_SIZE_MIB=128 "$REPO/tools/mkcard.sh" "$S/base.img" "$A/NEWDESK-W.INF"
  cp "$S/base.img.manifest" "$M/base.img.manifest"
  shasum -a 256 "$S/base.img" > "$M/serial/base.img.sha256"
  ;;
rpi4)
  X=${3:?A|B|C}
  case "$X" in
    A) SCRIPT=s-a-hover-leave-reenter-click.script; SHOTS=$(seq 16 36) ;;
    B) SCRIPT=s-b-slow-phases.script; SHOTS=$(seq 16 44) ;;
    C) SCRIPT=s-c-fast-clicks.script; SHOTS=$(seq 16 40) ;;
  esac
  "$A/build-kernel.sh" "$REPO" "$S/tree" desktop="$A/$SCRIPT" > "$M/serial/run$X.kernel.sha256"
  cp "$S/tree/target/aarch64-unknown-none-softfloat/release/kernel8.img" "$S/kernel-run$X.img"
  "$A/run.sh" "$S/kernel-run$X.img" "$S/base.img" "run$X" "$M/serial" "$S" $SHOTS
  mv "$M/serial/run$X"-*.png "$M/screendumps/"
  ;;
virt)
  # no injector: the three files tools/inject-arm.py patches come back from the repo
  for f in kernel/src/host_env.rs kernel/src/launch.rs kernel/src/shell_loop.rs; do cp "$REPO/$f" "$S/tree/$f"; done
  (cd "$S/tree" && CARGO_TARGET_DIR="$S/tgt-virt" cargo xtask build --board qemu-virt --release) > "$S/build-virt.txt" 2>&1
  rust-objcopy -O binary "$S/tgt-virt/aarch64-unknown-none-softfloat/release/kernel" "$S/kernel-virt.img"
  shasum -a 256 "$S/kernel-virt.img" > "$M/serial/qvirt.kernel.sha256"
  "$A/run-virt.sh" "$S/kernel-virt.img" "$A/q-e-click-holds.steps" qE "$M/serial" "$S" 40
  "$A/run-virt.sh" "$S/kernel-virt.img" "$A/q-f-hover-leave-reenter.steps" qF "$M/serial" "$S" 34
  mv "$M/serial/"q[EF]-*.png "$M/screendumps/"
  ;;
esac
