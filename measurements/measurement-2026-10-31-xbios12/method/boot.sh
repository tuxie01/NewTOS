#!/usr/bin/env bash
# boot.sh <board> <outdir> [SOURCE_DATE_EPOCH] -- XBIOS12: fresh `cargo xtask build`
# for <board> (rpi4 | qemu-virt), raw image via llvm-objcopy into <outdir>, boot it
# headless in QEMU without an SD card for 25 s, kill QEMU, keep the serial log and
# print the [clock] line. QEMU arguments as `cargo xtask run` (raspi4b: -append
# serial=on, SER01; virt: gic-version=2, 272M, ramfb), minus USB and SD.
set -uo pipefail
B=$1; OUT=$2; SDE=${3:-}
REPO=$(cd "$(dirname "$0")/../../.." && pwd)
OC=$(rustc --print sysroot)/lib/rustlib/aarch64-apple-darwin/bin/llvm-objcopy
mkdir -p "$OUT"
if [ -n "$SDE" ]; then export SOURCE_DATE_EPOCH=$SDE; else unset SOURCE_DATE_EPOCH; fi
(cd "$REPO" && cargo xtask build --board "$B") > "$OUT/build-$B.log" 2>&1 || { echo "BUILD FAIL $B"; exit 1; }
"$OC" -O binary "$REPO/target/aarch64-unknown-none-softfloat/debug/kernel" "$OUT/kernel-$B.img"
shasum -a 256 "$OUT/kernel-$B.img"
if [ "$B" = rpi4 ]; then
  qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial stdio \
      -kernel "$OUT/kernel-$B.img" -append serial=on </dev/null > "$OUT/serial-$B.txt" 2>&1 &
else
  qemu-system-aarch64 -M virt,gic-version=2 -cpu cortex-a72 -m 272M -device ramfb \
      -display none -serial stdio -no-reboot -kernel "$OUT/kernel-$B.img" </dev/null > "$OUT/serial-$B.txt" 2>&1 &
fi
pid=$!
sleep 25
kill $pid 2>/dev/null; sleep 1; kill -9 $pid 2>/dev/null
wait $pid 2>/dev/null
echo "qemu pid $pid ended"
grep -a "NewTOS booting\|\[clock\] no RTC" "$OUT/serial-$B.txt"
