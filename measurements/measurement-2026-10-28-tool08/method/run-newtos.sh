#!/bin/bash
# run-newtos.sh <kernel8.img> <card.img> <out-dir> [AREA]
# One DUMPBIOS run on NewTOS, QEMU raspi4b, headless, -append serial=on.
# <card.img> comes from tools/mkcard.sh in test-disk mode (MKCARD_TESTDISK): C: = the
# test partition (the oracle's test image), D: = the program partition with DUMPBIOS.NAP
# and <AREA>.TSV; NEWDESK.INF "#Z 01 D:\DUMPBIOS.NAP@" on C: starts the dumper.
# The card is booted as it is (the caller passes an APFS clone and keeps the original).
# Writes to <out-dir>: BOOT-SERIAL.txt, <AREA>.BIN/.LOG (from D:), card.sha256 (before/after),
# testpart-before.img / testpart-after.img are NOT kept: only their sha256 and the
# comparison (compare-testpart.sh) are.
set -u
K=$1; IMG=$2; D=$3; AREA=${4:-BIOS}
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
mkdir -p "$D"
export MTOOLS_SKIP_CHECK=1 MTOOLSRC=/dev/null
le32() { od -An -t u4 -j "$2" -N 4 "$1" | tr -d ' '; }
TSTART=$(le32 "$IMG" 454); TCOUNT=$(le32 "$IMG" 458); PSTART=$(le32 "$IMG" 470)
echo "before $(shasum -a 256 "$IMG" | cut -d' ' -f1)" > "$D/card.sha256"
dd if="$IMG" of="$D/.testpart-before.img" bs=512 skip="$TSTART" count="$TCOUNT" status=none
: > "$D/BOOT-SERIAL.txt"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$D/BOOT-SERIAL.txt" \
   -append serial=on -kernel "$K" -drive file="$IMG",if=sd,format=raw &
pid=$!
for i in $(seq 1 240); do
  sleep 2
  /usr/bin/grep -q "desktop up" "$D/BOOT-SERIAL.txt" && break
  kill -0 $pid 2>/dev/null || break
done
sleep 3; kill $pid 2>/dev/null; sleep 1; kill -9 $pid 2>/dev/null; wait $pid 2>/dev/null
echo "after  $(shasum -a 256 "$IMG" | cut -d' ' -f1)" >> "$D/card.sha256"
mcopy -o -n -i "$IMG@@$((PSTART * 512))" "::/$AREA.BIN" "$D/$AREA.BIN" 2>/dev/null
mcopy -o -n -i "$IMG@@$((PSTART * 512))" "::/$AREA.LOG" "$D/$AREA.LOG" 2>/dev/null
dd if="$IMG" of="$D/.testpart-after.img" bs=512 skip="$TSTART" count="$TCOUNT" status=none
"$HERE/compare-testpart.sh" "$D/.testpart-before.img" "$D/.testpart-after.img" > "$D/testpart-compare.txt" 2>&1
echo "compare-testpart exit $?" >> "$D/testpart-compare.txt"
rm -f "$D/.testpart-before.img" "$D/.testpart-after.img"
ls -l "$D"
