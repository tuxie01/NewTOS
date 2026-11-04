#!/usr/bin/env bash
# run-virt.sh <kernel-virt.img> <steps-file> <tag> <outdir> <scratch> [wall-seconds-total]
# qemu-virt (xHCI + usb-kbd + usb-mouse + ramfb, headless), driven over QMP by
# qmp_usb.py; serial to <outdir>/serial/<tag>.log, screendumps to
# <outdir>/screendumps/<tag>-<name>.png. Copied from the a127 run's run-virt.sh.
set -uo pipefail
K=$1; ST=$2; TAG=$3; OUT=$4; SCR=$5; TOT=${6:-60}
HERE=$(cd "$(dirname "$0")" && pwd); mkdir -p "$OUT/serial" "$OUT/screendumps"
cd "$SCR"; SOCK=qv-$TAG.sock; rm -f "$SOCK"; : > "$OUT/serial/$TAG.log"
qemu-system-aarch64 -M virt,gic-version=2 -cpu cortex-a72 -m 272M -device ramfb \
  -device qemu-xhci -device usb-kbd -device usb-mouse -display none -serial file:"$OUT/serial/$TAG.log" -no-reboot \
  -kernel "$K" -qmp unix:"$SOCK",server=on,wait=off &
pid=$!
python3 "$HERE/qmp_usb.py" "$SOCK" "$OUT/screendumps" "$TAG" "$ST" > "$OUT/serial/$TAG.qmp.txt" 2>&1 &
dp=$!
t0=$SECONDS; while [ $((SECONDS-t0)) -lt "$TOT" ] && kill -0 $dp 2>/dev/null; do sleep 1; done
kill $dp 2>/dev/null; kill $pid 2>/dev/null; sleep 1; kill -9 $pid 2>/dev/null; wait 2>/dev/null
for p in "$OUT/screendumps/$TAG"-*.ppm; do [ -f "$p" ] && sips -s format png "$p" --out "${p%.ppm}.png" >/dev/null 2>&1 && rm -f "$p"; done
rm -f "$SOCK"
