#!/bin/bash
# run-newtos.sh <scratch> <out-dir> -- BSTPROBE.NAP on NewTOS, QEMU raspi4b, headless.
# <scratch> holds kernel8.img (cargo xtask build --board rpi4 --release) and card.img
# (mkcard-call.sh). One APFS clone of card.img is the SD card (#Z autorun of
# C:\BSTPROBE.NAP); it is kept as <scratch>/newtos-after.img for the image comparison.
set -u
S=$1; D=$2
OFF=32256            # partition start LBA 63 * 512
mkdir -p "$D"
IMG=$S/newtos-after.img; rm -f "$IMG"; cp -c "$S/card.img" "$IMG"
: > "$D/BOOT-SERIAL.txt"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$D/BOOT-SERIAL.txt" \
   -append serial=on -kernel "$S/kernel8.img" -drive file="$IMG",if=sd,format=raw &
pid=$!
for i in $(seq 1 240); do
  sleep 2
  /usr/bin/grep -q "desktop up" "$D/BOOT-SERIAL.txt" && break
  kill -0 $pid 2>/dev/null || break
done
sleep 3; kill $pid 2>/dev/null; sleep 1; kill -9 $pid 2>/dev/null; wait $pid 2>/dev/null
export MTOOLS_SKIP_CHECK=1
mcopy -o -i "$IMG@@$OFF" ::/BSTPROBE.LOG "$D/BSTPROBE.LOG"
mcopy -o -i "$IMG@@$OFF" ::/BSTPROB2.LOG "$D/BSTPROB2.LOG"
ls -l "$D"
