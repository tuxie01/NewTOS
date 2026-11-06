#!/bin/bash
# PORT12 Q3b: XGSCR.NAP via #Z autorun in QEMU raspi4b. Screendump when C:\XGSCR.RDY exists, card read after the exit.
# usage: run-xgscr.sh <repo> <xgdos> <scratch> <XGSCR.NAP> <kernel8.img> <out dir> [append]
set -euo pipefail
REPO=$1; X=$2; S=$3; NAP=$4; K=$5; OUT=$6; APPEND=${7:-serial=on}
M=$REPO/measurements/measurement-2026-11-06-port12/method
rm -rf "$S"; mkdir -p "$S" "$OUT/shots" "$OUT/card"
printf '#R 02\r\n#Z 01 C:\\XGSCR.NAP@\r\n' > "$S/NEWDESK.INF"
pairs=""; for f in "$X"/GEMSYS/*.SPD; do pairs="$pairs FONTS/$(basename "$f")=$f"; done
MKCARD_DIRS="PRINTS" "$REPO/tools/mkcard.sh" "$S/card.img" "$S/NEWDESK.INF" ASSIGN.SYS="$M/assign-fonts.sys" XGSCR.NAP="$NAP" $pairs > "$S/mk.out" 2>&1 || { cat "$S/mk.out"; exit 1; }
cp "$S/card.img.manifest" "$OUT/"; cp -c "$S/card.img" "$S/boot.img"
{ echo "kernel $(shasum -a 256 "$K" | cut -d' ' -f1)"; echo "nap $(shasum -a 256 "$NAP" | cut -d' ' -f1)"; echo "append $APPEND"; } > "$OUT/run.txt"
: > "$OUT/BOOT-SERIAL.txt"
cd "$S"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" -append "$APPEND" -kernel "$K" -drive file="$S/boot.img",if=sd,format=raw -qmp unix:qmp.sock,server=on,wait=off &
pid=$!
cstart=$(od -An -t u4 -j 454 -N 4 "$S/boot.img" | tr -d ' ')
img="$S/boot.img@@$((cstart * 512))"
ready=no
for _ in $(seq 1 120); do sleep 1
  if MTOOLS_SKIP_CHECK=1 mdir -i "$img" ::/XGSCR.RDY >/dev/null 2>&1; then ready=yes; break; fi
  kill -0 $pid 2>/dev/null || break
done
echo "ready $ready" >> "$OUT/run.txt"
if [ $ready = yes ]; then sleep 2; python3 -I "$M/shot.py" qmp.sock "$OUT/shots/a.ppm" "$OUT/shots/a.png"; sleep 2; python3 -I "$M/shot.py" qmp.sock "$OUT/shots/b.ppm" "$OUT/shots/b.png"; fi
state=timeout
for _ in $(seq 1 120); do sleep 1
  if grep -a -q "XGSCR.NAP' exited with rc" "$OUT/BOOT-SERIAL.txt"; then state=done; break; fi
  kill -0 $pid 2>/dev/null || { state=qemu-exited; break; }
done
sleep 2; kill $pid 2>/dev/null || true; sleep 1; kill -9 $pid 2>/dev/null || true; wait $pid 2>/dev/null || true
echo "qemu $state" >> "$OUT/run.txt"
MTOOLS_SKIP_CHECK=1 mcopy -s -o -i "$img" ::/XGSCR.LOG ::/XGSCR.STK ::/PRINTS "$OUT/card/" 2>&1 | tail -3 || true
shasum -a 256 "$S/boot.img" | cut -d' ' -f1 | sed 's/^/card after run sha256 /' >> "$OUT/run.txt"
cat "$OUT/run.txt"
