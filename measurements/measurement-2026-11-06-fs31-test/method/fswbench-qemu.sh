#!/usr/bin/env bash
# Copy of measurements/measurement-2026-11-06-fs31/method/fswbench-qemu.sh (re-pathed REPO) plus FRAG=holes:
# D: gets 20 one-cluster files, the odd ones deleted -> FSWBR.DAT starts in a hole.
# FSWBENCH (headless variant) in QEMU raspi4b: log on C:, deletes on D:, the
# pair the Pi run showed. Usage: fswbench-qemu.sh <kernel8.img> <out-dir> <scratch>
# C: = test image (tools/mktestdisk.sh) with NEWDESK.INF, D: = program partition.
set -euo pipefail
KERNEL=$1; OUT=$2; SCR=$3
REPO=/Volumes/Daten/Development/PiRusTOS
mkdir -p "$OUT" "$SCR"; OUT=$(cd "$OUT" && pwd); cd "$REPO"
FSWB_FLAGS=-DFSWB_NOKEY sdk/examples/fsbench/build-fswbench.sh "$SCR/FSWBENCH.NCP" > "$SCR/build.txt" 2>&1
printf '#R 02\r\n#Z 00 D:\\FSWBENCH.NCP@\r\n' > "$SCR/NEWDESK.INF"
tools/mktestdisk.sh "$SCR/test.img" "NEWDESK.INF=$SCR/NEWDESK.INF" > "$SCR/mktestdisk.txt" 2>&1
MKCARD_TESTDISK="$SCR/test.img" MKCARD_SIZE_MIB=512 tools/mkcard.sh "$SCR/card.img" "$SCR/NEWDESK.INF" "FSWBENCH.NCP=$SCR/FSWBENCH.NCP" > "$SCR/mkcard.txt" 2>&1
cp "$SCR/card.img.manifest" "$OUT/card.img.manifest"
if [ "${FRAG:-}" = holes ]; then
  dstart=$(od -An -t u4 -j 470 -N 4 "$SCR/card.img" | tr -d ' ')
  export MTOOLS_SKIP_CHECK=1
  head -c 8192 /dev/zero | tr '\0' 'x' > "$SCR/hole.dat"
  for i in $(seq -w 1 20); do mcopy -i "$SCR/card.img@@$((dstart * 512))" "$SCR/hole.dat" "::HOLE$i.DAT"; done
  for i in 01 03 05 07 09 11 13 15 17 19; do mdel -i "$SCR/card.img@@$((dstart * 512))" "::HOLE$i.DAT"; done
  echo "FRAG holes: D: at LBA $dstart, HOLE02..20 even kept" > "$OUT/frag.txt"
  mshowfat -i "$SCR/card.img@@$((dstart * 512))" ::HOLE20.DAT >> "$OUT/frag.txt" 2>&1 || true
  mshowfat -i "$SCR/card.img@@$((dstart * 512))" ::FSWBENCH.NCP >> "$OUT/frag.txt" 2>&1 || true
fi
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
if [ "${FRAG:-}" = holes ]; then
  dstart=$(od -An -t u4 -j 470 -N 4 "$SCR/boot.img" | tr -d ' ')
  dd if="$SCR/boot.img" of="$SCR/d.img" bs=512 skip=$dstart count=$(od -An -t u4 -j 474 -N 4 "$SCR/boot.img" | tr -d ' ') 2>/dev/null
  fsck_msdos -n "$SCR/d.img" > "$OUT/fsck-d.txt" 2>&1; echo "fsck D: exit $?" >> "$OUT/run.txt"
  cstart=$(od -An -t u4 -j 454 -N 4 "$SCR/boot.img" | tr -d ' ')
  dd if="$SCR/boot.img" of="$SCR/c.img" bs=512 skip=$cstart count=$(od -An -t u4 -j 458 -N 4 "$SCR/boot.img" | tr -d ' ') 2>/dev/null
  fsck_msdos -n "$SCR/c.img" > "$OUT/fsck-c.txt" 2>&1; echo "fsck C: exit $?" >> "$OUT/run.txt"
fi
rm -rf "$SCR"
cat "$OUT/run.txt"
