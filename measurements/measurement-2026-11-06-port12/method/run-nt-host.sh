#!/bin/bash
# PORT12 Q3a: run XGTVDI.NCP + XGTDEV.NCP (upstream tests, fake VDI) from C:\AUTO in QEMU raspi4b, read the card.
# The same upstream tests on the Mac side from the same ASSIGN.SYS in the same run dir layout; compare.
# usage: run-nt-host.sh <repo> <xgdos> <scratch> <nap-dir> <kernel8.img> <out dir>
set -euo pipefail
REPO=$1; X=$2; S=$3; B=$4; K=$5; OUT=$6
M=$REPO/measurements/measurement-2026-11-06-port12/method
mkdir -p "$S" "$OUT"
printf '#R 02\r\n' > "$S/NEWDESK.INF"
pairs=""; for f in "$X"/GEMSYS/*.SPD; do pairs="$pairs HOSTFS/C/FONTS/$(basename "$f")=$f"; done
touch "$S/keep"
MKCARD_SIZE_MIB=64 MKCARD_DIRS="HOSTFS HOSTFS/C HOSTFS/C/PRINTS" "$REPO/tools/mkcard.sh" "$S/card.img" "$S/NEWDESK.INF" \
   HOSTFS/C/ASSIGN.SYS="$M/assign-fonts.sys" AUTO/XGTVDI.NCP="$B/XGTVDI.NCP" AUTO/XGTDEV.NCP="$B/XGTDEV.NCP" $pairs > "$S/mk.out" 2>&1 || { cat "$S/mk.out"; exit 1; }
cp "$S/card.img.manifest" "$OUT/"
cp -c "$S/card.img" "$S/boot.img"
{ echo "kernel $(shasum -a 256 "$K" | cut -d' ' -f1)"; shasum -a 256 "$B"/XGT*.NCP "$M/assign-fonts.sys" | sed "s#$B/##; s#$M/##"; } > "$OUT/run.txt"
: > "$OUT/BOOT-SERIAL.txt"
cd "$S"
rm -f qmp.sock
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" -append serial=on -kernel "$K" -drive file="$S/boot.img",if=sd,format=raw -qmp unix:qmp.sock,server=on,wait=off &
pid=$!
state=timeout
for _ in $(seq 1 90); do sleep 2
  if grep -a -q "desktop up" "$OUT/BOOT-SERIAL.txt"; then state=done; break; fi
  kill -0 $pid 2>/dev/null || { state=qemu-exited; break; }
done
sleep 2; kill $pid 2>/dev/null || true; sleep 1; kill -9 $pid 2>/dev/null || true; wait $pid 2>/dev/null || true
echo "qemu $state" >> "$OUT/run.txt"
cstart=$(od -An -t u4 -j 454 -N 4 "$S/boot.img" | tr -d ' ')
I="$S/boot.img@@$((cstart * 512))"; mkdir -p "$OUT/card"
MTOOLS_SKIP_CHECK=1 mcopy -o -i "$I" ::/XGTVDI.OUT ::/XGTDEV.OUT ::/OUTSCR.PGM ::/OUTMEM.PGM ::/HOSTFS/C/TEST.GEM ::/HOSTFS/C/GEMFILE.GEM ::/HOSTFS/C/PRINTS/PRN0001.PDF ::/HOSTFS/C/PRINTS/PRN0002.PDF ::/HOSTFS/C/PRINTS/PRN0003.PDF "$OUT/card/" 2>&1 | tail -3 || true
shasum -a 256 "$S/boot.img" | cut -d' ' -f1 | sed 's/^/card after run sha256 /' >> "$OUT/run.txt"
cat "$OUT/run.txt"
