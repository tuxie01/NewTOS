#!/usr/bin/env bash
# run-virt.sh <tag> <kernel-flat.img> <append> <seconds> : the `cargo xtask run --board qemu-virt --headless` machine
# (virt,gic-version=2, cortex-a72, 272M, ramfb, qemu-xhci+usb-kbd+usb-mouse); serial to file; QMP screendump at the end.
set -uo pipefail
F=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/15767204-8d89-43e0-aae0-d8149a16a17d/scratchpad/w
M=/Volumes/Daten/Development/PiRusTOS/measurements/measurement-2026-11-04-proc08-fix-test
T=$1; K=$2; APP=$3; SECS=$4
rm -f $M/serial/$T.log $F/q-$T.sock
cd $F
qemu-system-aarch64 -M virt,gic-version=2 -cpu cortex-a72 -m 272M -device ramfb \
  -device qemu-xhci -device usb-kbd -device usb-mouse -display none \
  -serial file:$M/serial/$T.log -no-reboot -append "$APP" -kernel $K \
  -qmp unix:q-$T.sock,server=on,wait=off &
P=$!
sleep $SECS
python3 -I $M/method/qmp.py q-$T.sock "{\"execute\":\"screendump\",\"arguments\":{\"filename\":\"$M/screendumps/$T.png\",\"format\":\"png\"}}"
kill $P 2>/dev/null; sleep 1; kill -9 $P 2>/dev/null
rm -f $F/q-$T.sock
echo "end $T after ${SECS}s"
