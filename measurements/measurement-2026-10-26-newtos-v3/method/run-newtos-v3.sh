#!/bin/bash
# run-newtos-v3.sh <scratch> <out-root>
# scratch holds: base.img (tools/make-sd-image.sh 64), kernel8.img (rpi4 release), dumpers/DUMP{VDI,AES,RAST}.NAP
# One APFS clone of base.img per table; NEWDESK.INF "#Z 01 C:\DUMPxxx.NAP@" autoruns the dumper
# (as measurements/measurement-2026-09-21-vdi-qemu); QEMU raspi4b headless 640x480, -append serial=on.
set -u
SC=$1; OUT=$2
M=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$M/../../.." && pwd 2>/dev/null || cd "$M/../../.." && pwd)
REPO=/Volumes/Daten/Development/PiRusTOS
OFF=32256
while read -r SUF NAP TSVNAME TABLE; do
  case $SUF in ''|\#*) continue;; esac
  D=$OUT/$SUF; mkdir -p "$D"
  IMG=$SC/clone-$SUF.img; cp -c "$SC/base.img" "$IMG"
  printf '#Z 01 C:\\%s@\r\n' "$NAP" > "$SC/NEWDESK.INF"
  mcopy -o -i "$IMG@@$OFF" "$SC/dumpers/$NAP" "::/$NAP"
  mcopy -o -i "$IMG@@$OFF" "$REPO/tosdump/cases/$TABLE" "::/$TSVNAME"
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
  AREA=${TSVNAME%.TSV}
  mcopy -o -i "$IMG@@$OFF" "::/$AREA.BIN" "$D/$AREA.BIN"
  mcopy -o -i "$IMG@@$OFF" "::/$AREA.LOG" "$D/$AREA.LOG"
  echo "$SUF: $(ls -l "$D/$AREA.BIN" 2>/dev/null | awk '{print $5}') bytes"
  rm -f "$IMG"
done <<'TABLES'
VDI-all   DUMPVDI.NAP  VDI.TSV    vdi-all.tsv
VDI-r2    DUMPVDI.NAP  VDI.TSV    vdi-r2.tsv
AES       DUMPAES.NAP  AES.TSV    aes-r1.tsv
AES-r2wind DUMPAES.NAP AES.TSV    aes-r2-wind.tsv
RASTER    DUMPRAST.NAP RASTER.TSV vdi-raster.tsv
TABLES
