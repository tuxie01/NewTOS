#!/usr/bin/env bash
# run-virt-test.sh <scratch> <head|base> -- virt build (no injector) + the A127 E/F step files over QMP.
set -euo pipefail
REPO=$(cd "$(dirname "$0")/../../.." && pwd)
S=${1:?}; W=${2:?}
M=$REPO/measurements/measurement-2026-11-04-a127-d1-test
A=$REPO/measurements/measurement-2026-11-04-a127/method
for f in kernel/src/host_env.rs kernel/src/launch.rs kernel/src/shell_loop.rs; do cp "$S/$W-src/$f" "$S/$W/$f"; done
rm -rf "$S/$W/target"
(cd "$S/$W" && cargo xtask build --board qemu-virt --release) > "$S/build-virt-$W.txt" 2>&1
rust-objcopy -O binary "$S/$W/target/aarch64-unknown-none-softfloat/release/kernel" "$S/kernel-virt-$W.img"
shasum -a 256 "$S/kernel-virt-$W.img" > "$M/serial/qvirt-$W.kernel.sha256"
"$A/run-virt.sh" "$S/kernel-virt-$W.img" "$A/q-e-click-holds.steps" qE-$W "$M/serial" "$S" 40
"$A/run-virt.sh" "$S/kernel-virt-$W.img" "$A/q-f-hover-leave-reenter.steps" qF-$W "$M/serial" "$S" 34
mv "$M/serial/"q[EF]-$W-*.png "$M/screendumps/"
