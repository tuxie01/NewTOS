#!/bin/sh
# V40 builds.sh <label>: in the scratch tree $S/t/<label>: dev build + clippy, both boards, a fresh
# target/aarch64-unknown-none-softfloat per board; then the rpi4 release kernel8.img -> $S/kernel-<label>.img.
# The warning lines go to $S/builds/<label>/warnings.txt.
S=${S:?set S to the session scratchpad}
L=$1
cd "$S/t/$L" || exit 1
mkdir -p "$S/builds/$L"
for B in rpi4 qemu-virt; do
  rm -rf target/aarch64-unknown-none-softfloat
  cargo xtask build --board $B > "$S/builds/$L/build-$B.log" 2>&1; echo "build $B rc=$?"
  cargo clippy -p kernel --features board-$B > "$S/builds/$L/clippy-$B.log" 2>&1; echo "clippy $B rc=$?"
done
rm -rf target/aarch64-unknown-none-softfloat
cargo xtask build --board rpi4 --release > "$S/builds/$L/release-rpi4.log" 2>&1; echo "release rc=$?"
cp target/aarch64-unknown-none-softfloat/release/kernel8.img "$S/kernel-$L.img" && shasum -a 256 "$S/kernel-$L.img"
(cd "$S/builds/$L" && /usr/bin/grep -H "generated [0-9]* warning" *.log) > "$S/builds/$L/warnings.txt"
cat "$S/builds/$L/warnings.txt"
