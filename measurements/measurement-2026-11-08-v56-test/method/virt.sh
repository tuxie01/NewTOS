#!/bin/bash
# V56 test: qemu-virt, base and fix, each from its own fresh `cargo xtask build --board qemu-virt` (debug), flat
# image as xtask run makes it, QEMU headless (xtask's argument list), serial to a file, 60 s, killed by PID.
set -u
S=${S:?}; cd /Volumes/Daten/Development/PiRusTOS
BIN="$(rustc --print sysroot)/lib/rustlib/aarch64-apple-darwin/bin"; mkdir -p $S/virt
for T in base fix; do
 (cd $S/t/$T && rm -rf target/aarch64-unknown-none-softfloat && cargo xtask build --board qemu-virt > $S/virt/build-$T.log 2>&1; echo "build $T rc=$?"
  $BIN/rust-objcopy -O binary target/aarch64-unknown-none-softfloat/debug/kernel $S/virt/kernel-$T.img)
 cd $S/virt
 qemu-system-aarch64 -M virt,gic-version=2 -cpu cortex-a72 -m 272M -device ramfb -device qemu-xhci -device usb-kbd -device usb-mouse \
   -display none -serial file:$S/virt/serial-$T.txt -no-reboot -kernel $S/virt/kernel-$T.img &
 pid=$!; sleep 60; kill $pid; sleep 1; kill -0 $pid 2>/dev/null && kill -9 $pid; wait $pid 2>/dev/null
 cd /Volumes/Daten/Development/PiRusTOS
done
