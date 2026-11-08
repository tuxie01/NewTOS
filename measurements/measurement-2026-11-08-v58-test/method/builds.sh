#!/bin/bash
# Tester builds: both boards fresh (dev) + clippy, release rpi4 kernels, in the two git-archive trees. S=scratch.
set -u
S=${S:?}; A=${A:?}
: > $A/builds-rc.txt
for T in base chg; do
  cd $S/t/$T || exit 1
  for B in rpi4 qemu-virt; do
    rm -rf target/aarch64-unknown-none-softfloat
    cargo xtask build --board $B > $A/build-$T-$B.log 2>&1; echo "build $T $B rc $?" >> $A/builds-rc.txt
    cargo clippy -p kernel --features board-$B > $A/clippy-$T-$B.txt 2>&1; echo "clippy $T $B rc $?" >> $A/builds-rc.txt
  done
  cargo clippy -p tos-api > $A/clippy-$T-tos-api.txt 2>&1; echo "clippy $T tos-api rc $?" >> $A/builds-rc.txt
  rm -rf target/aarch64-unknown-none-softfloat
  cargo xtask build --board rpi4 --release > $A/build-$T-rpi4-release.log 2>&1; echo "build $T rpi4 release rc $?" >> $A/builds-rc.txt
  cp target/aarch64-unknown-none-softfloat/release/kernel8.img $S/k/$T-rpi4-release.img
done
shasum -a 256 $S/k/*.img | tee $A/kernels.sha256
for f in $A/build-*.log; do echo "$(basename $f): $(grep -E 'generated [0-9]+ warning' $f | tr '\n' ' ')"; done > $A/warnings.txt
for f in $A/clippy-*.txt; do echo "$(basename $f): $(grep -E 'generated [0-9]+ warning' $f | sed 's/ (run.*//' | tr '\n' ' ')"; done >> $A/warnings.txt
echo BUILDSDONE >> $A/builds-rc.txt
