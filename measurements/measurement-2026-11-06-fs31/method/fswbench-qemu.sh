#!/usr/bin/env bash
# FSWBENCH (headless variant) in QEMU raspi4b: log on C:, deletes on D:, the
# pair the Pi run showed. Usage: fswbench-qemu.sh <kernel8.img> <out-dir> <scratch>
# C: = test image (tools/mktestdisk.sh) with NEWDESK.INF, D: = program partition.
set -euo pipefail
KERNEL=$1; OUT=$2; SCR=$3
REPO=$(cd "$(dirname "$0")/../../.." && pwd)
mkdir -p "$OUT" "$SCR"; OUT=$(cd "$OUT" && pwd); cd "$REPO"
FSWB_FLAGS=-DFSWB_NOKEY sdk/examples/fsbench/build-fswbench.sh "$SCR/FSWBENCH.NCP" > "$SCR/build.txt" 2>&1
printf '#R 02\r\n#Z 00 D:\\FSWBENCH.NCP@\r\n' > "$SCR/NEWDESK.INF"
tools/mktestdisk.sh "$SCR/test.img" "NEWDESK.INF=$SCR/NEWDESK.INF" > "$SCR/mktestdisk.txt" 2>&1
MKCARD_TESTDISK="$SCR/test.img" MKCARD_SIZE_MIB=512 tools/mkcard.sh "$SCR/card.img" "$SCR/NEWDESK.INF" "FSWBENCH.NCP=$SCR/FSWBENCH.NCP" > "$SCR/mkcard.txt" 2>&1
cp "$SCR/card.img.manifest" "$OUT/card.img.manifest"
cp -c "$SCR/card.img" "$SCR/boot.img"
: > "$OUT/serial.txt"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/serial.txt" \
    -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw &
pid=$!
state=timeout
for _ in $(seq 1 280); do
    sleep 2
    if grep -q "FSWBENCH.NCP' exited with rc" "$OUT/serial.txt"; then state=done; break; fi
    kill -0 "$pid" 2>/dev/null || { state=qemu-exited; break; }
done
sleep 1
kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
cstart=$(od -An -t u4 -j 454 -N 4 "$SCR/boot.img" | tr -d ' ')
MTOOLS_SKIP_CHECK=1 mcopy -o -i "$SCR/boot.img@@$((cstart * 512))" ::FSWBENCH.LOG "$OUT/FSWBENCH.LOG" || true
{
  echo "qemu $state"
  echo "kernel $(shasum -a 256 "$KERNEL" | cut -d' ' -f1)"
  echo "program $(shasum -a 256 "$SCR/FSWBENCH.NCP" | cut -d' ' -f1)"
  echo "card before $(shasum -a 256 "$SCR/card.img" | cut -d' ' -f1)"
  echo "card after  $(shasum -a 256 "$SCR/boot.img" | cut -d' ' -f1)"
  echo "C:\\FSWBENCH.LOG $(wc -c < "$OUT/FSWBENCH.LOG" 2>/dev/null || echo none) bytes"
} > "$OUT/run.txt"
rm -rf "$SCR"
cat "$OUT/run.txt"
