#!/bin/sh
# builds.sh <tree> <label>: dev build + clippy both boards (fresh target per board), then the rpi4 release
# kernel copied to $S/kernel-<label>.img. Logs into $S/builds/<label>/.
S=${S:?set S to the session scratchpad}
TREE=$1; L=$2
mkdir -p $S/builds/$L
cd "$TREE" || exit 1
for B in rpi4 qemu-virt; do
  rm -rf target/aarch64-unknown-none-softfloat
  cargo xtask build --board $B > $S/builds/$L/build-$B.log 2>&1; echo "build $B rc=$?"
  cargo clippy -p kernel --features board-$B > $S/builds/$L/clippy-$B.log 2>&1; echo "clippy $B rc=$?"
done
rm -rf target/aarch64-unknown-none-softfloat
cargo xtask build --board rpi4 --release > $S/builds/$L/release-rpi4.log 2>&1; echo "release rc=$?"
cp target/aarch64-unknown-none-softfloat/release/kernel8.img $S/kernel-$L.img && shasum -a 256 $S/kernel-$L.img
