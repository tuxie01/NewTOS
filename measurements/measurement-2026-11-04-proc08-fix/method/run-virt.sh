#!/usr/bin/env bash
# run-virt.sh <tag> <append> <seconds> : QEMU virt (the `cargo xtask run
# --board qemu-virt --headless` machine: virt,gic-version=2, cortex-a72,
# 272M, ramfb, qemu-xhci + usb-kbd + usb-mouse), serial to a file, QMP
# screendump at the end. qemu-virt has no drive (docs/sdk/06-image-and-testing.md).
set -uo pipefail
F=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/15767204-8d89-43e0-aae0-d8149a16a17d/scratchpad/proc08-fix
T=$1; APP=$2; SECS=$3
rm -f $F/serial/$T.log q-$T.sock
cd $F
qemu-system-aarch64 -M virt,gic-version=2 -cpu cortex-a72 -m 272M -device ramfb \
  -device qemu-xhci -device usb-kbd -device usb-mouse -display none \
  -serial file:$F/serial/$T.log -no-reboot -append "$APP" -kernel $F/kernel-virt.img \
  -qmp unix:q-$T.sock,server=on,wait=off &
P=$!
sleep $SECS
python3 -I $F/method/qmp.py q-$T.sock "{\"execute\":\"screendump\",\"arguments\":{\"filename\":\"$F/screendumps/$T.png\",\"format\":\"png\"}}"
kill $P 2>/dev/null; sleep 1; kill -9 $P 2>/dev/null
rm -f q-$T.sock
echo "end $T after ${SECS}s"
