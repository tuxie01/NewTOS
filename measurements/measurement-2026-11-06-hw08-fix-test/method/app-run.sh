#!/usr/bin/env bash
# app-run.sh <out-dir> <kernel8.img> <APP.NAP> <armname e.g. FBENCH> <shots.txt> <scratch> [dest=host-file ...]
# tools/mkcard.sh card (C: = one 64-MiB FAT16 partition) with C:\<APP.NAP> and NEWDESK.INF "#R 02 / #Z 01 C:\<APP.NAP>@",
# QEMU raspi4b on a clone, QMP shots after the serial line "[inj] up:<name> armed", then the files of C: and fsck_msdos -n of the partition.
set -euo pipefail
REPO=/Volumes/Daten/Development/PiRusTOS
OUT=$1; KERNEL=$2; APP=$3; NAME=$4; SHOTS=$5; SCR=$6; shift 6
mkdir -p "$OUT/shots" "$SCR"; OUT=$(cd "$OUT" && pwd); cd "$REPO"
APPN=$(basename "$APP")
printf '#R 02\r\n#Z 01 C:\\%s@\r\n' "$APPN" > "$SCR/newdesk.inf"; cp "$SCR/newdesk.inf" "$OUT/NEWDESK.INF"
tools/mkcard.sh "$SCR/card.img" "$SCR/newdesk.inf" "$APPN=$APP" "$@" > "$SCR/mk.out" 2>&1 || { cat "$SCR/mk.out"; exit 1; }
cp "$SCR/card.img.manifest" "$OUT/card.img.manifest"; cp "$SHOTS" "$OUT/shots.txt"
cp -c "$SCR/card.img" "$SCR/boot.img"
{ echo "kernel $(shasum -a 256 "$KERNEL" | cut -d' ' -f1)"; echo "app $APPN $(shasum -a 256 "$APP" | cut -d' ' -f1)"; echo "card before $(shasum -a 256 "$SCR/card.img" | cut -d' ' -f1)"; } > "$OUT/run.txt"
: > "$OUT/BOOT-SERIAL.txt"
cd "$SCR"
rm -f qmp.sock
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" \
    -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw -qmp unix:qmp.sock,server=on,wait=off &
pid=$!
for _ in $(seq 1 100); do [ -S qmp.sock ] && break; sleep 0.1; done; sleep 2
python3 -I "$REPO/tosdump/probes/shprobe-qmpshots.py" qmp.sock "$OUT/BOOT-SERIAL.txt" "$OUT/shots.txt" "$OUT/shots" "[inj] up:$NAME armed" || echo "qmpshots failed" >> "$OUT/run.txt"
sleep 1
kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
echo "card after  $(shasum -a 256 "$SCR/boot.img" | cut -d' ' -f1)" >> "$OUT/run.txt"
export MTOOLSRC=/dev/null MTOOLS_SKIP_CHECK=1 MTOOLS_NO_VFAT=1
START=$(python3 -I -c "import struct,sys; m=open(sys.argv[1],'rb').read(512); print(struct.unpack('<I', m[446+8:446+12])[0])" "$SCR/boot.img")
SIZE=$(python3 -I -c "import struct,sys; m=open(sys.argv[1],'rb').read(512); print(struct.unpack('<I', m[446+12:446+16])[0])" "$SCR/boot.img")
echo "C: at LBA $START, $SIZE sectors" >> "$OUT/run.txt"
mdir -a -i "$SCR/boot.img@@$((START * 512))" :: > "$OUT/mdir-C.txt" 2>&1 || true
mkdir -p "$OUT/files"
for f in $(awk 'NR>3 && NF>=4 && $3 ~ /^[0-9]+$/ {print $1"."$2}' "$OUT/mdir-C.txt"); do :; done
python3 -I - "$SCR/boot.img" "$START" "$OUT/files" <<'PY'
import subprocess, sys, os
img, start, out = sys.argv[1], int(sys.argv[2]), sys.argv[3]
r = subprocess.run(["mdir", "-a", "-b", "-s", "-i", f"{img}@@{start*512}", "::"], capture_output=True, text=True, env=dict(os.environ, MTOOLSRC="/dev/null", MTOOLS_SKIP_CHECK="1", MTOOLS_NO_VFAT="1"))
for line in r.stdout.splitlines():
    p = line.strip()
    if not p or p.upper().endswith(".NAP") or "NEWDESK" in p.upper() or "SPOTLIGHT" in p.upper(): continue
    name = p.lstrip(":/").replace("/", "_")
    dst = os.path.join(out, name)
    c = subprocess.run(["mcopy", "-n", "-i", f"{img}@@{start*512}", "::" + p.lstrip(":"), dst], capture_output=True, env=dict(os.environ, MTOOLSRC="/dev/null", MTOOLS_SKIP_CHECK="1", MTOOLS_NO_VFAT="1"))
PY
dd if="$SCR/boot.img" of="$SCR/part.img" bs=512 skip="$START" count="$SIZE" 2>/dev/null
if fsck_msdos -n "$SCR/part.img" > "$OUT/fsck-C.txt" 2>&1; then echo "fsck C: exit 0" >> "$OUT/run.txt"; else echo "fsck C: exit $?" >> "$OUT/run.txt"; fi
(cd "$OUT/files" && for f in *; do [ -f "$f" ] && echo "$f $(wc -c < "$f") $(shasum -a 256 "$f" | cut -c1-16)"; done) >> "$OUT/run.txt" 2>/dev/null || true
echo "shots $(ls "$OUT/shots" | grep -c png)" >> "$OUT/run.txt"
grep -a -c -i -E "exception|panic" "$OUT/BOOT-SERIAL.txt" | sed 's/^/exception-panic-lines /' >> "$OUT/run.txt" || true
rm -f "$SCR/part.img" "$SCR/boot.img" "$SCR/card.img"
cat "$OUT/run.txt"
