#!/bin/bash
# PORT12 Q5: build the probe with the SDK and run it via #Z in QEMU raspi4b.
# usage: run-vec.sh <repo> <scratch> <kernel8.img> <out dir> [append]
set -euo pipefail
REPO=$1; S=$2; K=$3; OUT=$4; APPEND=${5:-serial=on}
M=$REPO/measurements/measurement-2026-11-06-port12/method
rm -rf "$S"; mkdir -p "$S" "$OUT/card"
for p in vecp vectsr veccli; do P=$(echo $p | tr a-z A-Z); "$REPO/sdk/bin/newtos-cc" -nap -lgem -O1 -I"$M" "$M/$p.c" -o "$S/$P.NAP"; done
printf '#R 02\r\n#Z 01 C:\\VECP.NAP@\r\n' > "$S/NEWDESK.INF"
"$REPO/tools/mkcard.sh" "$S/card.img" "$S/NEWDESK.INF" VECP.NAP="$S/VECP.NAP" VECTSR.NAP="$S/VECTSR.NAP" VECCLI.NAP="$S/VECCLI.NAP" > "$S/mk.out" 2>&1 || { cat "$S/mk.out"; exit 1; }
cp "$S/card.img.manifest" "$OUT/"; cp -c "$S/card.img" "$S/boot.img"
{ echo "kernel $(shasum -a 256 "$K" | cut -d' ' -f1)"; (cd "$S" && shasum -a 256 *.NAP); echo "append $APPEND"; } > "$OUT/run.txt"
: > "$OUT/BOOT-SERIAL.txt"
cd "$S"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" -append "$APPEND" -kernel "$K" -drive file="$S/boot.img",if=sd,format=raw &
pid=$!
state=timeout
for _ in $(seq 1 240); do sleep 1
  if grep -a -q "VECP.NAP' exited with rc" "$OUT/BOOT-SERIAL.txt"; then state=done; break; fi
  kill -0 $pid 2>/dev/null || { state=qemu-exited; break; }
done
sleep 2; kill $pid 2>/dev/null || true; sleep 1; kill -9 $pid 2>/dev/null || true; wait $pid 2>/dev/null || true
echo "qemu $state" >> "$OUT/run.txt"
cstart=$(od -An -t u4 -j 454 -N 4 "$S/boot.img" | tr -d ' ')
for f in VECP VECTSR VECCLI; do MTOOLS_SKIP_CHECK=1 mcopy -o -i "$S/boot.img@@$((cstart * 512))" ::/$f.LOG "$OUT/card/$f.LOG" 2>/dev/null || echo "no $f.LOG" >> "$OUT/run.txt"; done
cat "$OUT/run.txt"
