#!/usr/bin/env bash
# run-keep.sh (tester variant of the A129 run.sh: shot errors to serial/<tag>.qmp.err, clone kept) <kernel8.img> <card.img> <tag> <outdir> <scratch> <WxH> <shot-wall-seconds...>
# One QEMU raspi4b (headless, -append serial=on, bcm2835-fb at WxH), per-run APFS
# clone of the card (cp -c), serial to <outdir>/serial/<tag>.log, QMP screendumps
# at the given wall seconds (<outdir>/screendumps/<tag>-<sec>.png); clone deleted.
set -uo pipefail
K=$1; CARD=$2; TAG=$3; OUT=$4; SCR=$5; GEOM=$6; shift 6
HERE=$(cd "$(dirname "$0")/../../measurement-2026-11-04-a129-measure/method" && pwd)
IMG=$SCR/$TAG.img; SOCK=q-$TAG.sock; cd "$SCR"
mkdir -p "$OUT/serial" "$OUT/screendumps"
cp -c "$CARD" "$IMG"; rm -f "$SOCK"; : > "$OUT/serial/$TAG.log"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -append serial=on -kernel "$K" \
  -global bcm2835-fb.xres=${GEOM%x*} -global bcm2835-fb.yres=${GEOM#*x} \
  -drive file="$IMG",if=sd,format=raw -serial file:"$OUT/serial/$TAG.log" \
  -qmp unix:"$SOCK",server=on,wait=off &
pid=$!
t0=$SECONDS
for s in "$@"; do
  while [ $((SECONDS-t0)) -lt "$s" ]; do sleep 0.2; kill -0 $pid 2>/dev/null || break 2; done
  python3 -I "$HERE/qmpshot.py" "$SOCK" "$SCR/$TAG-$s.ppm" 2>>"$OUT/serial/$TAG.qmp.err" && \
    sips -s format png "$SCR/$TAG-$s.ppm" --out "$OUT/screendumps/$TAG-$s.png" >/dev/null 2>&1
  [ -f "$OUT/screendumps/$TAG-$s.png" ] && cp "$SCR/$TAG-$s.ppm" "$SCR/$TAG-$s.keep.ppm"
  rm -f "$SCR/$TAG-$s.ppm"
done
kill $pid 2>/dev/null; sleep 1; kill -9 $pid 2>/dev/null; wait $pid 2>/dev/null
shasum -a 256 "$IMG" > "$OUT/serial/$TAG.img-after.sha256"
rm -f "$SOCK"   # tester variant: keeps the image clone (FSPROBE.LOG readout)
