#!/bin/sh
# builds.sh <label>: dev build + clippy both boards, then the rpi4 release kernel copied to $S/kernel-<label>.img
S=${S:?set S to the session scratchpad}
L=$1
cd /Volumes/Daten/Development/PiRusTOS
for B in rpi4 qemu-virt; do
  rm -rf target/aarch64-unknown-none-softfloat
  cargo xtask build --board $B > $S/builds/$L/build-$B.log 2>&1; echo "build $B rc=$?"
  cargo clippy -p kernel --features board-$B > $S/builds/$L/clippy-$B.log 2>&1; echo "clippy $B rc=$?"
done
rm -rf target/aarch64-unknown-none-softfloat
cargo xtask build --board rpi4 --release > $S/builds/$L/release-rpi4.log 2>&1; echo "release rc=$?"
cp -c target/aarch64-unknown-none-softfloat/release/kernel8.img $S/kernel-$L.img && shasum -a 256 $S/kernel-$L.img
