#!/usr/bin/env bash
# Fallback card build: tools/mkcard.sh hung in `diskutil` (host-wide wedge, `diskutil list` also hangs).
# Same layout idea as mkcard.sh's MKCARD_TESTDISK branch: MBR, one FAT16 partition at LBA 2048, mtools only.
set -euo pipefail
IMG=$1; INF=$2; NCP=$3; MIB=64
dd if=/dev/zero of="$IMG" bs=1m count=$MIB status=none
PSTART=2048; PCOUNT=$((MIB*2048-PSTART))
python3 -I - "$IMG" $PSTART $PCOUNT <<'PY'
import sys,struct
img,ps,pc=sys.argv[1],int(sys.argv[2]),int(sys.argv[3])
e=bytes([0x80,0xfe,0xff,0xff,0x06,0xfe,0xff,0xff])+struct.pack('<II',ps,pc)
with open(img,'r+b') as f:
    f.seek(446); f.write(e); f.seek(510); f.write(b'\x55\xaa')
PY
P="${IMG}@@$((PSTART*512))"
mformat -i "$P" -T $PCOUNT -h 16 -s 63 -H $PSTART -c 4 -N 4E544344 -v NTCARD ::
mmd -i "$P" ::AUTO
mcopy -m -i "$P" "$INF" ::NEWDESK.INF
mcopy -m -i "$P" "$NCP" ::AUTO/SCRGRAB.NCP
mdir -i "$P" -/ ::
