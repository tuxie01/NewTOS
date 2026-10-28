#!/bin/bash
# buildall.sh <outdir> [features] -- build 4 kernels (rpi4/qemu-virt x release/dev), raw binaries + sha256
set -u
O=$1; F=${2:-}
mkdir -p "$O"
cd /Volumes/Daten/Development/PiRusTOS
OC=$(rustc --print sysroot)/lib/rustlib/aarch64-apple-darwin/bin/llvm-objcopy
FA=(); [ -n "$F" ] && FA=(--features "$F")
for b in rpi4 qemu-virt; do
  for p in release dev; do
    R=(); [ $p = release ] && R=(--release)
    cargo xtask build --board $b ${R[@]+"${R[@]}"} ${FA[@]+"${FA[@]}"} > "$O/build-$b-$p.log" 2>&1 || { echo "BUILD FAIL $b $p"; tail -20 "$O/build-$b-$p.log"; exit 1; }
    d=$([ $p = release ] && echo release || echo debug)
    "$OC" -O binary target/aarch64-unknown-none-softfloat/$d/kernel "$O/$b-$p.bin"
  done
done
(cd "$O" && shasum -a 256 *.bin)
