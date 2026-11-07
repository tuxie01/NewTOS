#!/bin/sh
# V38 acceptance: fresh builds of two git-archive trees (S = scratchpad v38t):
#   base = 4cc7005c, fix = e6ffdedc. Per tree and board: debug build and clippy, each after
#   cargo clean -p tos-api -p kernel so the warnings print; then the rpi4 release kernel and the qemu-virt debug kernel are kept.
S=${S:?set S}
M=/Volumes/Daten/Development/PiRusTOS/measurements/measurement-2026-11-07-v38-test
for t in base fix; do
  cd "$S/$t" || exit 1
  for b in rpi4 qemu-virt; do
    cargo clean -p tos-api -p kernel > /dev/null 2>&1
    cargo xtask build --board $b > "$M/builds/build-$t-$b.log" 2>&1; echo "build $t $b exit $?"
    if [ $b = qemu-virt ]; then cp -c target/aarch64-unknown-none-softfloat/debug/kernel "$S/kernel-$t-qemu-virt.elf" 2>/dev/null; fi
    cargo clean -p tos-api -p kernel > /dev/null 2>&1
    cargo clippy -p kernel --features board-$b > "$M/builds/clippy-$t-$b.log" 2>&1; echo "clippy $t $b exit $?"
  done
  cargo xtask build --board rpi4 --release > "$M/builds/build-$t-rpi4-release.log" 2>&1; echo "release $t exit $?"
  cp -c target/aarch64-unknown-none-softfloat/release/kernel8.img "$S/kernel-$t.img"
  shasum -a 256 "$S/kernel-$t.img" "$S/kernel-$t-qemu-virt.elf"
done
