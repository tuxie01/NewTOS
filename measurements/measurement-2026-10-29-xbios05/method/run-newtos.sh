#!/bin/bash
# run-newtos.sh <repo> <scratch> <nap> <out-dir> -- one probe .NAP on NewTOS, QEMU raspi4b,
# headless, -append serial=on, QMP socket. Card: tools/mkcard.sh (64 MiB, MBR + FAT16) with
# C:\<NAP> and NEWDESK.INF "#Z 01 C:\<NAP>@"; kernel <scratch>/kernel8.img. Ends at
# "desktop up" or, for a crashing probe, 5 s after the kernel's "[crash]" line, with a QMP
# screendump of the crash report (the program waits for a key there). The card is a file per
# run, deleted after the logs are copied (sha256 before/after kept). After
# measurement-2026-10-29-xbios01/method/run-newtos.sh (+ the crash stop and the screendump).
set -u
REPO=$1; S=$2; NAP=$3; D=$4
NAME=$(basename "$NAP"); mkdir -p "$D"; D=$(cd "$D" && pwd)
IMG="$S/card-$(basename "$D").img"; rm -f "$IMG" "$IMG.manifest"
printf '#R 02\r\n#Z 01 C:\\%s@\r\n' "$NAME" > "$S/z-$NAME.inf"
cp "$S/z-$NAME.inf" "$D/NEWDESK.INF"
(cd "$REPO" && tools/mkcard.sh "$IMG" "$S/z-$NAME.inf" "$NAME=$NAP" >/dev/null 2>&1)
cp "$IMG.manifest" "$D/card.img.manifest"; shasum -a 256 "$IMG" > "$D/card.img.before.sha256"
cc -o "$S/qmpshot" "$REPO/tools/qmpshot.c"
cd "$S"; Q="q.$$"; rm -rf "$Q"; mkdir -p "$Q"   # relative: a unix socket path must stay < 104 bytes
: > "$D/BOOT-SERIAL.txt"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$D/BOOT-SERIAL.txt" \
   -append serial=on -kernel "$S/kernel8.img" -drive file="$IMG",if=sd,format=raw \
   -qmp unix:"$Q/qmp.sock",server=on,wait=off &
pid=$!
end=none
for i in $(seq 1 240); do
  sleep 2
  /usr/bin/grep -q "desktop up" "$D/BOOT-SERIAL.txt" && { end=desktop; break; }
  /usr/bin/grep -q '^\[crash\]' "$D/BOOT-SERIAL.txt" && { end=crash; sleep 5;
      "$S/qmpshot" "$Q/qmp.sock" "$S/shot.ppm" && sips -s format png "$S/shot.ppm" --out "$D/crash-report.png" >/dev/null; break; }
  kill -0 $pid 2>/dev/null || break
done
echo "end=$end after ~$((i*2)) s" > "$D/end.txt"
sleep 3; kill $pid 2>/dev/null; sleep 1; kill -9 $pid 2>/dev/null; wait $pid 2>/dev/null
rm -rf "$S/$Q" "$S/shot.ppm"
export MTOOLS_SKIP_CHECK=1
for f in XCPROBE.LOG BMCALL.LOG; do mcopy -o -i "$IMG@@32256" ::/$f "$D/$f" 2>/dev/null; done
shasum -a 256 "$IMG" > "$D/card.img.after.sha256"; rm -f "$IMG" "$IMG.manifest"
ls -l "$D"
