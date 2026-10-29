#!/bin/bash
# run-newtos.sh <repo> <scratch> <nap> <out-dir> <kernel8.img> -- one probe .NAP on NewTOS,
# QEMU raspi4b, headless, -append serial=on. After the audit's run-newtos.sh
# (measurements/measurement-2026-10-28-bios-core-audit/method/run-newtos.sh), changed: the
# kernel is an argument; QEMU boots an APFS clone of the tools/mkcard.sh card (card and
# clone deleted after the logs are copied, sha256 kept); C:'s offset is read from the MBR.
set -u
REPO=$1; S=$2; NAP=$3; D=$4; K=$5
NAME=$(basename "$NAP"); mkdir -p "$D"; D=$(cd "$D" && pwd)
IMG="$S/card-$(basename "$D").img"; BOOT="$S/boot-$(basename "$D").img"
rm -f "$IMG" "$IMG.manifest" "$BOOT"
printf '#R 02\r\n#Z 01 C:\\%s@\r\n' "$NAME" > "$S/z-$NAME.inf"
cp "$S/z-$NAME.inf" "$D/NEWDESK.INF"
(cd "$REPO" && tools/mkcard.sh "$IMG" "$S/z-$NAME.inf" "$NAME=$NAP" >/dev/null 2>&1)
cp "$IMG.manifest" "$D/card.img.manifest"; shasum -a 256 "$IMG" > "$D/card.img.before.sha256"
cp -c "$IMG" "$BOOT"
shasum -a 256 "$K" > "$D/kernel8.img.sha256"
: > "$D/BOOT-SERIAL.txt"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$D/BOOT-SERIAL.txt" \
   -append serial=on -kernel "$K" -drive file="$BOOT",if=sd,format=raw &
pid=$!
for i in $(seq 1 240); do
  sleep 2
  /usr/bin/grep -q "desktop up" "$D/BOOT-SERIAL.txt" && break
  kill -0 $pid 2>/dev/null || break
done
sleep 3; kill $pid 2>/dev/null; sleep 1; kill -9 $pid 2>/dev/null; wait $pid 2>/dev/null
export MTOOLS_SKIP_CHECK=1
cstart=$(od -An -t u4 -j 454 -N 4 "$BOOT" | tr -d ' ')
for f in BCPROBE.LOG BCPROB2.LOG BCHANG.LOG; do mcopy -o -i "$BOOT@@$((cstart * 512))" ::/$f "$D/$f" 2>/dev/null; done
shasum -a 256 "$BOOT" > "$D/card.img.after.sha256"; rm -f "$IMG" "$IMG.manifest" "$BOOT"
ls -l "$D"
