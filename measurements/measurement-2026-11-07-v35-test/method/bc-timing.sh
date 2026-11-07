#!/bin/bash
# bc-timing.sh <out-dir> <kernel8.img> <tag> : boot screen + APITEST console at 1920x1080, serial-marker timing + dumps.
set -euo pipefail
cd /Volumes/Daten/Development/PiRusTOS
S=${S:?}; OUT=$1; KERNEL=$2; TAG=$3
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
SCR=$(mktemp -d "$S/bt.XXXXXX"); trap 'rm -rf "$SCR"' EXIT
printf '#R 02\r\n' > "$SCR/NEWDESK.INF"
tools/mkcard.sh "$SCR/card.img" "$SCR/NEWDESK.INF" AUTO/APITEST.NCP=sdk/apps/apitest/APITEST.NCP > "$SCR/mk.out" 2>&1 || { cat "$SCR/mk.out"; exit 1; }
[ -f "$OUT/card.img.manifest" ] || cp "$SCR/card.img.manifest" "$OUT/"
cp -c "$SCR/card.img" "$SCR/boot.img"
cd "$SCR"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/$TAG-serial.txt" -global bcm2835-fb.xres=1920 -global bcm2835-fb.yres=1080 -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw -qmp unix:qmp.sock,server=on,wait=off &
pid=$!
python3 -I /Volumes/Daten/Development/PiRusTOS/measurements/measurement-2026-11-07-v35-test/method/bc-timing.py qmp.sock "$OUT/$TAG-serial.txt" "$OUT" "$TAG" || true
kill $pid 2>/dev/null || true; sleep 1; kill -9 $pid 2>/dev/null || true; wait $pid 2>/dev/null || true
echo "$TAG load $(uptime | sed 's/.*load averages*: //')" >> "$OUT/loads.txt"
