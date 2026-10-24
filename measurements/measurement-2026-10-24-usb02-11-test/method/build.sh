#!/bin/sh
# build.sh <srcdir> <tag> <outdir>
cd $1; T=$2; O=$3
for prof in dev rel; do for b in qemu-virt rpi4; do
  fl=""; d=debug; [ $prof = rel ] && { fl="--release"; d=release; }
  n=$T-$b-$prof
  cargo xtask build --board $b $fl > $O/build-$n.log 2>&1; echo "$n rc=$?" >> $O/rc-$T.txt
  [ -f kernel8.img ] && mv kernel8.img $O/kernel8-$n.img
  [ -f target/aarch64-unknown-none/$d/kernel.img ] && cp target/aarch64-unknown-none/$d/kernel.img $O/kimg-$n.img
done; done
echo done >> $O/rc-$T.txt
