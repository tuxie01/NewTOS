#!/bin/bash
# run-newtos.sh <repo> <scratch> <nap> <out-dir> [<wait-file>] -- one probe .NAP on
# NewTOS, QEMU raspi4b, headless, -append serial=on.  Card: tools/mkcard.sh
# (defaults: 64 MiB, MBR + FAT16) with C:\<NAP> and NEWDESK.INF "#Z 01 C:\<NAP>@";
# kernel <scratch>/kernel8.img.  The card is an APFS-independent file per run,
# deleted after the logs are copied (sha256 before/after kept).
set -u
REPO=$1; S=$2; NAP=$3; D=$4
NAME=$(basename "$NAP"); mkdir -p "$D"; D=$(cd "$D" && pwd)
IMG="$S/card-$(basename "$D").img"; rm -f "$IMG" "$IMG.manifest"
printf '#R 02\r\n#Z 01 C:\\%s@\r\n' "$NAME" > "$S/z-$NAME.inf"
cp "$S/z-$NAME.inf" "$D/NEWDESK.INF"
(cd "$REPO" && tools/mkcard.sh "$IMG" "$S/z-$NAME.inf" "$NAME=$NAP" >/dev/null 2>&1)
cp "$IMG.manifest" "$D/card.img.manifest"; shasum -a 256 "$IMG" > "$D/card.img.before.sha256"
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
for f in BCPROBE.LOG BCPROB2.LOG BCHANG.LOG FRPROBE.LOG; do mcopy -o -i "$IMG@@32256" ::/$f "$D/$f" 2>/dev/null; done
shasum -a 256 "$IMG" > "$D/card.img.after.sha256"; rm -f "$IMG" "$IMG.manifest"
ls -l "$D"
