#!/usr/bin/env bash
# mkcard2.sh <out.img> <NEWDESK.INF> -- two FAT16 partitions C: D: of the xdprobe geometry (tosdump/probes/run-xdprobe.sh),
# both roots built in the same order: label, NEWDESK.INF, COPYME.TXT (C: "SOURCE-FROM-C-drive", D: "OLD-TARGET-ON-D-drive-longer")
# so COPYME.TXT has root slot 2 on both. Fixed time stamp. No mount.
set -euo pipefail
OUT=$1; INF=$2
PSIZE=130977; STARTS="63 131103"; STAMP=202610171200
export MTOOLSRC=/dev/null MTOOLS_NO_VFAT=1 MTOOLS_SKIP_CHECK=1 TZ=UTC
SOURCE_DATE_EPOCH=$(date -j -u -f %Y%m%d%H%M%S "${STAMP}00" +%s); export SOURCE_DATE_EPOCH
dd if=/dev/zero of="$OUT" bs=1m count=256 status=none
python3 -I - "$OUT" "$PSIZE" $STARTS <<'PY'
import struct, sys
card, size, starts = sys.argv[1], int(sys.argv[2]), [int(a) for a in sys.argv[3:]]
mbr = bytearray(512)
for i, start in enumerate(starts):
    mbr[446 + i * 16:462 + i * 16] = bytes([0, 0xFE, 0xFF, 0xFF, 6, 0xFE, 0xFF, 0xFF]) + struct.pack("<II", start, size)
mbr[510:512] = b"\x55\xaa"
with open(card, "r+b") as f: f.write(mbr)
PY
T=$(dirname "$OUT")/cmstage; mkdir -p "$T"
python3 -I -c "import sys; sys.stdout.buffer.write(b''.join(b'%07d SOURCE-FROM-C-drive 0123456789abcdef0123456789abcdef0123\n' % i for i in range(3500)))" > "$T/C.TXT"; printf 'OLD-TARGET-ON-D-drive-longer' > "$T/D.TXT"
cp -X "$INF" "$T/NEWDESK.INF"; touch -t $STAMP "$T/NEWDESK.INF" "$T/C.TXT" "$T/D.TXT"
for start in $STARTS; do
  mformat -i "$OUT@@$((start * 512))" -T "$PSIZE" -h 16 -s 63 -H "$start" -c 4 -r 32 -L 128 -N 4E543332 -v NTFS31 ::
  mcopy -m -i "$OUT@@$((start * 512))" "$T/NEWDESK.INF" ::NEWDESK.INF
done
mcopy -m -i "$OUT@@$((63 * 512))" "$T/C.TXT" ::COPYME.TXT
mcopy -m -i "$OUT@@$((131103 * 512))" "$T/D.TXT" ::COPYME.TXT
mkdir -p "$T/c" "$T/d"; cp -X "$T/NEWDESK.INF" "$T/c/"; cp -X "$T/NEWDESK.INF" "$T/d/"; cp -X "$T/C.TXT" "$T/c/COPYME.TXT"; cp -X "$T/D.TXT" "$T/d/COPYME.TXT"
cd /Volumes/Daten/Development/PiRusTOS
tools/sd-manifest.sh files "$T/c" C "$OUT"; tools/sd-manifest.sh files "$T/d" D "$OUT"
tools/sd-manifest.sh write "$OUT" 256 measurements/measurement-2026-11-06-hw08-fix-test/method/mkcard3.sh --switches n/a > /dev/null
rm -rf "$T"
