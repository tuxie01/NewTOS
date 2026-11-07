#!/bin/bash
# V35 test: both boards fresh-built + clippy, base (3a9221eb) and fix (a862f764) in git-archive trees. S = scratchpad.
S=${S:?}; M=/Volumes/Daten/Development/PiRusTOS/measurements/measurement-2026-11-07-v35-test; A=$M/analysis
mkdir -p $S/k
for T in base fix; do
  cd $S/t/$T
  for B in rpi4 qemu-virt; do
    rm -rf target/aarch64-unknown-none-softfloat
    cargo xtask build --board $B > $A/build-$T-$B.log 2>&1; echo "build $T $B rc $?" >> $A/builds-rc.txt
    cargo clippy -p kernel --features board-$B > $A/clippy-$T-$B.txt 2>&1; echo "clippy $T $B rc $?" >> $A/builds-rc.txt
  done
  rm -rf target/aarch64-unknown-none-softfloat
  cargo xtask build --board rpi4 --release > $A/build-$T-rpi4-release.log 2>&1; echo "build $T rpi4 release rc $?" >> $A/builds-rc.txt
  cp -c target/aarch64-unknown-none-softfloat/release/kernel8.img $S/k/$T-rpi4-release.img
done
echo BUILDSDONE >> $A/builds-rc.txt
