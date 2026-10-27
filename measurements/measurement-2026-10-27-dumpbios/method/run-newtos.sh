#!/bin/bash
# run-newtos.sh <scratch> <out-dir>
# scratch holds: base.img (+ base.img.manifest) from tools/make-sd-image.sh
# <scratch>/base.img 64, kernel8.img (rpi4 release).  The dumper is
# ../dumpers/a8b7/DUMPBIOS.NAP, the table tosdump/cases/bios-r1.tsv as
# C:\BIOS.TSV.  One APFS clone of base.img (cp -c), NEWDESK.INF
# "#Z 01 C:\DUMPBIOS.NAP@" autoruns the dumper (as
# measurements/measurement-2026-10-26-newtos-v3/method/run-newtos-v3.sh);
# QEMU raspi4b headless, -append serial=on; the clone is deleted after.
set -u
SC=$1; D=$2
M=$(cd "$(dirname "$0")" && pwd); RUN=$(dirname "$M"); REPO=$(cd "$RUN/../.." && pwd)
OFF=32256            # partition start LBA 63 * 512
mkdir -p "$D"
IMG=$SC/clone-bios.img; cp -c "$SC/base.img" "$IMG"
printf '#Z 01 C:\\DUMPBIOS.NAP@\r\n' > "$SC/NEWDESK.INF"
export MTOOLS_SKIP_CHECK=1
mcopy -o -i "$IMG@@$OFF" "$RUN/dumpers/a8b7/DUMPBIOS.NAP" ::/DUMPBIOS.NAP
mcopy -o -i "$IMG@@$OFF" "$REPO/tosdump/cases/bios-r1.tsv" ::/BIOS.TSV
mcopy -o -i "$IMG@@$OFF" "$SC/NEWDESK.INF" ::/NEWDESK.INF
: > "$D/BOOT-SERIAL.txt"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$D/BOOT-SERIAL.txt" \
   -append serial=on -kernel "$SC/kernel8.img" -drive file="$IMG",if=sd,format=raw &
pid=$!
for i in $(seq 1 240); do
  sleep 2
  /usr/bin/grep -q "desktop up" "$D/BOOT-SERIAL.txt" && break
  kill -0 $pid 2>/dev/null || break
done
sleep 3; kill $pid 2>/dev/null; sleep 1; kill -9 $pid 2>/dev/null; wait $pid 2>/dev/null
mcopy -o -i "$IMG@@$OFF" ::/BIOS.BIN "$D/BIOS.BIN"
mcopy -o -i "$IMG@@$OFF" ::/BIOS.LOG "$D/BIOS.LOG"
echo "NEWTOS: $(ls -l "$D/BIOS.BIN" 2>/dev/null | awk '{print $5}') bytes"
rm -f "$IMG"
