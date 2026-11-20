#!/bin/bash
# runapi.sh <kernel> <tag>
K=$1; T=$2; D=$(dirname $0)
cp -c $D/apitest.img $D/api-$T.img
: > $D/api-$T.log
qemu-system-aarch64 -M raspi4b -no-reboot -display none -append serial=on -kernel $K -drive file=$D/api-$T.img,if=sd,format=raw -serial file:$D/api-$T.log &
P=$!
for i in $(seq 1 300); do grep -q "TOTAL" $D/api-$T.log && break; kill -0 $P 2>/dev/null || break; sleep 1; done
sleep 4; kill $P 2>/dev/null; sleep 1; kill -9 $P 2>/dev/null; wait $P 2>/dev/null
grep -E "TOTAL|FAIL" $D/api-$T.log | head; grep -cE "\[EXC\]|PANIC|panicked|hook\] warning" $D/api-$T.log
rm -f $D/api-$T.img
