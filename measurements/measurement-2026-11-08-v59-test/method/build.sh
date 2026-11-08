#!/bin/bash
# tester V59: trees + builds (own script)
set -u
W=/Volumes/Daten/Development/PiRusTOS/.claude/worktrees/agent-a194b0b2cc3f6d929
MAIN=/Volumes/Daten/Development/PiRusTOS
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/ed7bb9a3-0fc2-4b37-91bb-ed70559503a3/scratchpad/v59t
R=$W/measurements/measurement-2026-11-08-v59-test/analysis; mkdir -p $R
: > $R/builds-rc.txt
for pair in base:6821179c chg:5a3052ec; do
  T=${pair%%:*}; C=${pair#*:}
  rm -rf $S/t/$T; mkdir -p $S/t/$T
  (cd $W && git archive $C) | tar -x -C $S/t/$T
  ln -s $MAIN/references $S/t/$T/references
  echo "tree $T $(cd $W && git rev-parse $C)" >> $R/trees.txt
  cd $S/t/$T
  for B in rpi4 qemu-virt; do
    rm -rf target/aarch64-unknown-none-softfloat
    cargo xtask build --board $B > $R/build-$T-$B.log 2>&1; echo "build $T $B rc $?" >> $R/builds-rc.txt
    cargo clippy -p kernel --features board-$B > $R/clippy-$T-kernel-$B.txt 2>&1; echo "clippy kernel $T $B rc $?" >> $R/builds-rc.txt
  done
  cargo clippy -p tos-api > $R/clippy-$T-tosapi.txt 2>&1; echo "clippy tos-api $T rc $?" >> $R/builds-rc.txt
  cargo clippy -p tos-api --tests > $R/clippy-$T-tosapi-tests.txt 2>&1; echo "clippy tos-api --tests $T rc $?" >> $R/builds-rc.txt
  # release kernels
  rm -rf target/aarch64-unknown-none-softfloat
  cargo xtask build --board rpi4 --release > $R/build-$T-rpi4-release.log 2>&1; echo "build $T rpi4 release rc $?" >> $R/builds-rc.txt
  cp target/aarch64-unknown-none-softfloat/release/kernel8.img $S/k/$T-rpi4.img
  cp target/aarch64-unknown-none-softfloat/release/kernel $S/k/$T-rpi4.elf
  rm -rf target/aarch64-unknown-none-softfloat
  cargo xtask build --board qemu-virt --release > $R/build-$T-virt-release.log 2>&1; echo "build $T virt release rc $?" >> $R/builds-rc.txt
  OC=$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/^host: //p')/bin/rust-objcopy
  $OC -O binary target/aarch64-unknown-none-softfloat/release/kernel $S/k/$T-virt.img
  cp target/aarch64-unknown-none-softfloat/release/kernel $S/k/$T-virt.elf
  rm -rf target/aarch64-unknown-none-softfloat
done
(cd $S/k && shasum -a 256 *.img) > $R/kernels.sha256
echo BUILDSDONE >> $R/builds-rc.txt
