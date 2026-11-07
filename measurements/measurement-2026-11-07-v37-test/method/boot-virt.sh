#!/bin/bash
# qemu-virt headless boot of a tree's own build (fresh target), bounded 100 s. boot-virt.sh <tree> <out-dir>
# (V37 test: kills only its own QEMU, found by the ELF path, because a second worker may run QEMU.)
T=$1; O=$2; mkdir -p $O; cd $T
rm -rf target/aarch64-unknown-none-softfloat
cargo xtask build --board qemu-virt --release > $O/build.log 2>&1 || { echo build failed; exit 1; }
shasum -a 256 target/aarch64-unknown-none-softfloat/release/kernel | tee $O/kernel.sha256
cargo xtask run --board qemu-virt --headless --release > $O/serial.txt 2>&1 &
pid=$!; sleep 100
pkill -f "qemu-system-aarch64.*$T/target"; sleep 1; kill $pid 2>/dev/null; wait $pid 2>/dev/null
grep -a -c -iE "exception|panic" $O/serial.txt | sed 's/^/exception-panic-lines /' | tee $O/summary.txt
grep -a -E "desktop up" $O/serial.txt | tee -a $O/summary.txt
