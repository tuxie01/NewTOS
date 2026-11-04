#!/usr/bin/env bash
# run.sh <kernel8.img> <card.img> <tag> <outdir> <scratch> <shot-wall-seconds...>
# One QEMU raspi4b (headless, -append serial=on), per-run APFS clone of the card
# (cp -c), serial to <outdir>/serial/<tag>.log, QMP screendumps at the given wall
# seconds after the QEMU start (<outdir>/screendumps/<tag>-<sec>.png); the clone's
# sha256 after the run to serial/<tag>.img-after.sha256, then it is deleted.
set -uo pipefail
K=$1; CARD=$2; TAG=$3; OUT=$4; SCR=$5; shift 5
HERE=$(cd "$(dirname "$0")" && pwd)
IMG=$SCR/$TAG.img; SOCK=q-$TAG.sock; cd "$SCR"   # relative: a unix socket path must be < 104 bytes
mkdir -p "$OUT/serial" "$OUT/screendumps"
cp -c "$CARD" "$IMG"; rm -f "$SOCK"; : > "$OUT/serial/$TAG.log"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -append serial=on -kernel "$K" \
  -drive file="$IMG",if=sd,format=raw -serial file:"$OUT/serial/$TAG.log" \
  -qmp unix:"$SOCK",server=on,wait=off &
pid=$!
t0=$SECONDS
for s in "$@"; do
  while [ $((SECONDS-t0)) -lt "$s" ]; do sleep 0.2; kill -0 $pid 2>/dev/null || break 2; done
  python3 -I "$HERE/qmpshot.py" "$SOCK" "$SCR/$TAG-$s.ppm" >/dev/null 2>&1 && \
    sips -s format png "$SCR/$TAG-$s.ppm" --out "$OUT/screendumps/$TAG-$s.png" >/dev/null 2>&1
  rm -f "$SCR/$TAG-$s.ppm"
done
kill $pid 2>/dev/null; sleep 1; kill -9 $pid 2>/dev/null; wait $pid 2>/dev/null
shasum -a 256 "$IMG" > "$OUT/serial/$TAG.img-after.sha256"
rm -f "$IMG" "$SOCK"
