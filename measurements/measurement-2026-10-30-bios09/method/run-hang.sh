#!/bin/bash
# run-hang.sh <repo> <scratch> <nap> <out-dir> <kernel8.img> [<seconds>] -- BIOS09: one BCHANG
# .NAP on NewTOS, QEMU raspi4b, headless, -append serial=on, no serial input. After
# measurements/measurement-2026-10-30-bios08/method/run-newtos.sh, changed: QEMU runs a fixed
# <seconds> (default 60) after the probe's start instead of until "desktop up" -- a probe
# blocked in Bconin never returns to the desktop (the oracle runs were killed after 20 s).
set -u
REPO=$1; S=$2; NAP=$3; D=$4; K=$5; SECS=${6:-60}
NAME=$(basename "$NAP"); mkdir -p "$D"; D=$(cd "$D" && pwd)
IMG="$S/card-$(basename "$D").img"; BOOT="$S/boot-$(basename "$D").img"
rm -f "$IMG" "$IMG.manifest" "$BOOT"
printf '#R 02\r\n#Z 01 C:\\%s@\r\n' "$NAME" > "$S/z-$NAME.inf"
cp "$S/z-$NAME.inf" "$D/NEWDESK.INF"
(cd "$REPO" && tools/mkcard.sh "$IMG" "$S/z-$NAME.inf" "$NAME=$NAP" >/dev/null 2>&1)
cp "$IMG.manifest" "$D/card.img.manifest"; shasum -a 256 "$IMG" > "$D/card.img.before.sha256"
cp -c "$IMG" "$BOOT"
shasum -a 256 "$K" > "$D/kernel8.img.sha256"
shasum -a 256 "$NAP" > "$D/probe.sha256"
: > "$D/BOOT-SERIAL.txt"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$D/BOOT-SERIAL.txt" \
   -append serial=on -kernel "$K" -drive file="$BOOT",if=sd,format=raw &
pid=$!
for i in $(seq 1 120); do
  sleep 1
  /usr/bin/grep -aq "desktop up" "$D/BOOT-SERIAL.txt" && break
  kill -0 $pid 2>/dev/null || break
done
sleep "$SECS"; kill $pid 2>/dev/null; sleep 1; kill -9 $pid 2>/dev/null; wait $pid 2>/dev/null
echo "QEMU killed $SECS s after the first \"desktop up\"" > "$D/run.txt"
export MTOOLS_SKIP_CHECK=1
cstart=$(od -An -t u4 -j 454 -N 4 "$BOOT" | tr -d ' ')
mcopy -o -i "$BOOT@@$((cstart * 512))" ::/BCHANG.LOG "$D/BCHANG.LOG" 2>/dev/null
shasum -a 256 "$BOOT" > "$D/card.img.after.sha256"; rm -f "$IMG" "$IMG.manifest" "$BOOT"
ls -l "$D"
