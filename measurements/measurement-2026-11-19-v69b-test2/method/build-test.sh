#!/usr/bin/env bash
# build-test.sh -- both board builds (fresh per board) and the clippy warning count, on the branch commit (wt) and on its
# parent main 9d90f0589 (wt-main); then the full cargo xtask test on the branch commit.
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; . "$HERE/env.sh"; L=$S/logs
for side in wt wt-main; do
  T=$A/$side
  for b in qemu-virt rpi4; do
    rm -rf $T/target/aarch64-unknown-none-softfloat/debug/kernel*
    (cd $T && cargo xtask build --board $b) > $L/build-$side-$b.log 2>&1; echo "build $side $b rc=$? warnings: $(grep -c '^warning' $L/build-$side-$b.log)"
  done
  (cd $T && cargo clippy -p xtask --all-targets) > $L/clippy-$side.log 2>&1; echo "clippy xtask $side rc=$? $(grep -E 'generated [0-9]+ warning' $L/clippy-$side.log | tr '\n' ';')"
done
(cd $A/wt && cargo xtask test) > $L/xtask-test.log 2>&1; echo "xtask test rc=$?"; grep -E "test result|passed|failed|ignored" $L/xtask-test.log | tail -15
echo BTDONE
