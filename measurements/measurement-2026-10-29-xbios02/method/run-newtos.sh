#!/bin/bash
# run-newtos.sh <repo> <scratch> <kernel8.img> <nap> <out-dir> [<extra-wait-s>] -- one probe .NAP
# on NewTOS, QEMU raspi4b, headless, -append serial=on. Copied from
# measurements/measurement-2026-10-28-xbios-chardev-audit/method/run-newtos.sh; changes: the kernel
# image is an argument; <extra-wait-s> keeps QEMU running that long after "desktop up" (the
# after-exit key of XBIOS02 item 3 and the aes-trace flush); AESTRACE.BIN is copied out too;
# the card is booted as an APFS clone (cp -c) and both are deleted after.
# Card: tools/mkcard.sh (defaults: 64 MiB, MBR + FAT16) with C:\<NAP> and NEWDESK.INF
# "#Z 01 C:\<NAP>@"; KLINE="#K de\r\n" in the environment adds a layout line before #Z.
set -u
REPO=$1; S=$2; K=$3; NAP=$4; D=$5; EXTRA=${6:-0}
NAME=$(basename "$NAP"); mkdir -p "$D"; D=$(cd "$D" && pwd)
IMG="$S/card-$(basename "$D").img"; CL="$IMG.boot"; rm -f "$IMG" "$IMG.manifest" "$CL"
printf '#R 02\r\n%s#Z 01 C:\\%s@\r\n' "${KLINE:-}" "$NAME" > "$S/z-$NAME.inf"
cp "$S/z-$NAME.inf" "$D/NEWDESK.INF"
(cd "$REPO" && tools/mkcard.sh "$IMG" "$S/z-$NAME.inf" "$NAME=$NAP" >/dev/null 2>&1)
cp "$IMG.manifest" "$D/card.img.manifest"; shasum -a 256 "$IMG" > "$D/card.img.before.sha256"
shasum -a 256 "$K" > "$D/kernel8.img.sha256"
cp -c "$IMG" "$CL"
: > "$D/BOOT-SERIAL.txt"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$D/BOOT-SERIAL.txt" \
   -append serial=on -kernel "$K" -drive file="$CL",if=sd,format=raw &
pid=$!
for i in $(seq 1 240); do
  sleep 2
  /usr/bin/grep -q "desktop up" "$D/BOOT-SERIAL.txt" && break
  kill -0 $pid 2>/dev/null || break
done
sleep $((3 + EXTRA)); kill $pid 2>/dev/null; sleep 1; kill -9 $pid 2>/dev/null; wait $pid 2>/dev/null
export MTOOLS_SKIP_CHECK=1
for f in XCPROBE.LOG XCIOFL.LOG XCKTNL.LOG XKPROBE.LOG AESTRACE.BIN; do mcopy -o -i "$CL@@32256" ::/$f "$D/$f" 2>/dev/null; done
shasum -a 256 "$CL" > "$D/card.img.after.sha256"; rm -f "$IMG" "$IMG.manifest" "$CL"
ls -l "$D"
