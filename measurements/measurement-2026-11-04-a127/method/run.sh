#!/usr/bin/env bash
# run.sh <kernel8.img> <base-image> <tag> <outdir> <scratch> <shot-wall-seconds...>
# One QEMU raspi4b (headless, -append serial=on), per-run APFS clone of the
# base image (cp -c), serial to <outdir>/<tag>.log, QMP screendumps at the
# given wall-clock seconds after QEMU start (<outdir>/<tag>-<sec>.png). The
# clone is deleted at the end. Read-only observation: no product file touched.
set -uo pipefail
K=$1; BASE=$2; TAG=$3; OUT=$4; SCR=$5; shift 5
IMG=$SCR/$TAG.img; SOCK=q-$TAG.sock; cd "$SCR"
cp -c "$BASE" "$IMG"; rm -f "$SOCK"; : > "$OUT/$TAG.log"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -append serial=on -kernel "$K" \
  -drive file="$IMG",if=sd,format=raw -serial file:"$OUT/$TAG.log" \
  -qmp unix:"$SOCK",server=on,wait=off &
pid=$!
t0=$SECONDS
for s in "$@"; do
  while [ $((SECONDS-t0)) -lt "$s" ]; do sleep 0.2; kill -0 $pid 2>/dev/null || break 2; done
  "$SCR/qmpshot" "$SOCK" "$SCR/$TAG-$s.ppm" >/dev/null 2>&1 && sips -s format png "$SCR/$TAG-$s.ppm" --out "$OUT/$TAG-$s.png" >/dev/null 2>&1
  rm -f "$SCR/$TAG-$s.ppm"
done
kill $pid 2>/dev/null; sleep 1; kill -9 $pid 2>/dev/null; wait $pid 2>/dev/null
shasum -a 256 "$IMG" | cut -c1-16 > "$OUT/$TAG.img-after.sha256"
rm -f "$IMG" "$SOCK"
