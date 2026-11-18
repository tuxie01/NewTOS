#!/bin/bash
# lint-rel.sh <tree> <name>: lint.sh (debug builds + clippy) then release kernels per board objcopy'd to kern/
A=/Volumes/Daten/Development/PiRusTOS/target-agents/agent-ab6a60009402c9c41; T=$1; N=$2
bash $(dirname $0)/lint.sh $T $A/checks/lint-$N
cd $T
for B in rpi4 qemu-virt; do
  cargo xtask build --board $B --release > $A/checks/rel-$N-$B.log 2>&1; echo "release $B exit $?" >> $A/checks/lint-$N/summary.txt
  E=$T/target/aarch64-unknown-none-softfloat/release/kernel; K=$A/kern/$N-${B/qemu-virt/virt}.img
  rust-objcopy -O binary $E $K 2>/dev/null || llvm-objcopy -O binary $E $K; cp $K $K.keep
done
echo RELDONE >> $A/checks/lint-$N/summary.txt
