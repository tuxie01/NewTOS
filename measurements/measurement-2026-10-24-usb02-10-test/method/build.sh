#!/bin/sh
# build.sh <tree> <tag> <outdir>: dev+release x both boards (fresh per build), keep ELF + kernel.img/kernel8.img
T=$1; G=$2; O=$3
OBJ=/Users/ingouhlemann/.rustup/toolchains/nightly-2026-04-15-aarch64-apple-darwin/lib/rustlib/aarch64-apple-darwin/bin/rust-objcopy
cd $T || exit 1
for b in qemu-virt rpi4; do
 for p in dev release; do
  if [ $p = release ]; then F=--release; D=release; else F=; D=debug; fi
  rm -f $T/target/aarch64-unknown-none-softfloat/$D/kernel
  cargo xtask build --board $b $F > $O/$G-$p-$b.log 2>&1; echo "$G $p $b rc=$?" >> $O/rc.txt
  E=$T/target/aarch64-unknown-none-softfloat/$D/kernel
  cp $E $O/$G-$p-$b.elf
  $OBJ -O binary $E $O/$G-$p-$b.img
  [ -f $T/kernel8.img ] && mv $T/kernel8.img $O/$G-$p-$b-kernel8.img
 done
done
echo ALLDONE >> $O/rc.txt
