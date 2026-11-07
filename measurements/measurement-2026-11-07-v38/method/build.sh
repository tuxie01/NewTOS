#!/bin/sh
# V38: fresh builds of the two scratch trees (S = session scratchpad/v38):
#   base = git archive of 3380004f (HEAD), fix = base + the V38 tos-api/src/vdi/contour.rs;
#   references/ symlinked. Isolated from the uncommitted V53 work in the main tree.
# Per tree and board: debug build and clippy, each after `cargo clean -p tos-api -p kernel`
# so the warnings are printed; then the rpi4 release kernel for the QEMU runs.
S=${S:?set S to the session scratchpad/v38}
M=/Volumes/Daten/Development/PiRusTOS/measurements/measurement-2026-11-07-v38
for t in base fix; do
  cd "$S/$t" || exit 1
  for b in rpi4 qemu-virt; do
    cargo clean -p tos-api -p kernel > /dev/null 2>&1
    cargo xtask build --board $b > "$M/builds/build-$t-$b.log" 2>&1; echo "build $t $b exit $?"
    cargo clean -p tos-api -p kernel > /dev/null 2>&1
    cargo clippy -p kernel --features board-$b > "$M/builds/clippy-$t-$b.log" 2>&1; echo "clippy $t $b exit $?"
  done
  cargo xtask build --board rpi4 --release > "$M/builds/build-$t-rpi4-release.log" 2>&1; echo "release $t exit $?"
  cp -c target/aarch64-unknown-none-softfloat/release/kernel8.img "$S/kernel-$t.img"
  shasum -a 256 "$S/kernel-$t.img"
done
