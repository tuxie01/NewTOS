#!/usr/bin/env bash
set -u
SP=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad
L=$SP/logs; mkdir -p $L
K=target/aarch64-unknown-none-softfloat
for t in fix base; do
  cd $SP/t/$t
  for b in rpi4 qemu-virt; do cargo xtask build-programs --board $b > $L/bp-$t-$b.log 2>&1; echo "bp $t $b rc $?"; done
  # clippy / build warnings, both boards
  for b in rpi4 qemu-virt; do
    cargo clippy -p tos-api > $L/clippy-$t-tos-$b.log 2>&1; echo "clippy-tos $t $b rc $? $(grep -E 'generated [0-9]+ warning' $L/clippy-$t-tos-$b.log | tr '\n' ' ')"
    cargo clippy -p kernel --release --features board-$b > $L/clippy-$t-kernel-$b.log 2>&1; echo "clippy-kernel $t $b rc $? $(grep -E 'generated [0-9]+ warning' $L/clippy-$t-kernel-$b.log | tr '\n' ' ')"
  done
  cargo xtask build --board rpi4 --release > $L/build-$t-rpi4-rel.log 2>&1; echo "build $t rpi4 rel rc $?"; cp $K/release/kernel8.img $SP/k/$t-rpi4-rel.img
  cargo xtask build --board rpi4 --release --features sd-fault > $L/build-$t-rpi4-sdfault.log 2>&1; echo "build $t rpi4 sd-fault rc $?"; cp $K/release/kernel8.img $SP/k/$t-rpi4-sdfault.img
  cargo xtask build --board qemu-virt > $L/build-$t-virt-dev.log 2>&1; echo "build $t virt dev rc $?"; cp $K/debug/kernel8.img $SP/k/$t-virt-dev.img 2>/dev/null
done
shasum -a 256 $SP/k/*.img
echo ALLDONE
