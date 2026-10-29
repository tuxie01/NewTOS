#!/bin/bash
# run-bcprobe-newtos.sh <scratch> <kernel8.img> <out-dir> -- the BIOS-core audit's probe
# binary measurements/measurement-2026-10-28-bios-core-audit/probe/BCPROBE.NAP (source
# method/bcprobe.c there; groups K S T B G, the S lines S01-S08 are the Setexc cases of
# BIOS07) on NewTOS, QEMU raspi4b, headless, -append serial=on. After that run's
# method/run-newtos.sh: card from tools/mkcard.sh with C:\BCPROBE.NAP and NEWDESK.INF
# "#R 02 / #Z 01 C:\BCPROBE.NAP@", booted as an APFS clone; logs copied with mcopy.
set -u
S=$1; K=$2; D=$3
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
NAP=$REPO/measurements/measurement-2026-10-28-bios-core-audit/probe/BCPROBE.NAP
[ ! -e "$D" ] || { echo "out exists: $D" >&2; exit 1; }
mkdir -p "$D"; D=$(cd "$D" && pwd)
IMG="$S/bc-card.img"; rm -f "$IMG" "$IMG.manifest" "$S/bc-boot.img"
printf '#R 02\r\n#Z 01 C:\\BCPROBE.NAP@\r\n' > "$S/z-bcprobe.inf"
cp "$S/z-bcprobe.inf" "$D/NEWDESK.INF"
(cd "$REPO" && tools/mkcard.sh "$IMG" "$S/z-bcprobe.inf" "BCPROBE.NAP=$NAP" >/dev/null 2>&1) || { echo "mkcard failed" >&2; exit 1; }
cp "$IMG.manifest" "$D/card.img.manifest"
{ echo "probe  $(shasum -a 256 "$NAP" | cut -d' ' -f1) BCPROBE.NAP"
  echo "kernel $(shasum -a 256 "$K" | cut -d' ' -f1) $(basename "$K")"
  echo "card   $(shasum -a 256 "$IMG" | cut -d' ' -f1) (tools/mkcard.sh)"; } > "$D/run.txt"
cp -c "$IMG" "$S/bc-boot.img"
: > "$D/BOOT-SERIAL.txt"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$D/BOOT-SERIAL.txt" \
   -append serial=on -kernel "$K" -drive file="$S/bc-boot.img",if=sd,format=raw 2>/dev/null &
pid=$!
for _ in $(seq 1 240); do
  sleep 2
  /usr/bin/grep -q "BCPROBE.NAP' exited with rc" "$D/BOOT-SERIAL.txt" &&
    /usr/bin/grep -q "desktop up" "$D/BOOT-SERIAL.txt" && break
  kill -0 $pid 2>/dev/null || break
done
sleep 3; kill $pid 2>/dev/null; sleep 1; kill -9 $pid 2>/dev/null; wait $pid 2>/dev/null
echo "card   after $(shasum -a 256 "$S/bc-boot.img" | cut -d' ' -f1) (the booted clone)" >> "$D/run.txt"
cstart=$(od -An -t u4 -j 454 -N 4 "$S/bc-boot.img" | tr -d ' ')
for f in BCPROBE.LOG BCPROB2.LOG; do
  MTOOLS_SKIP_CHECK=1 mcopy -o -i "$S/bc-boot.img@@$((cstart * 512))" ::/$f "$D/$f" 2>/dev/null
done
rm -f "$IMG" "$IMG.manifest" "$S/bc-boot.img"
ls "$D"
