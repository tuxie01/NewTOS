#!/usr/bin/env bash
# run.sh <kernel8.img> <base card.img> <tag> STEP... -- one headless QEMU raspi4b run (-append serial=on) on an APFS clone
# (cp -c) of the base card; serial -> ../runs/<tag>/serial.log; marker-keyed QMP screendumps (method/mdrive.py, copied
# unchanged from measurements/measurement-2026-10-21-gh1-reg/method/; STEP = 'REGEX@N=DELAY:name.png') -> ../runs/<tag>/.
# After the run: fsck_msdos -n of the clone, C:\MCHPROBE.LOG and C:\SVCSEQ.LOG (if present) copied out, clone sha256
# recorded, clone deleted. TMO = per-step marker timeout (default 150 s), QDELAY = wait after the last step.
set -uo pipefail
K="$1"; BASE="$2"; TAG="$3"; shift 3
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; S="${S:?S = scratch dir}"
OUT="$HERE/../runs/$TAG"; mkdir -p "$OUT"; LOG="$OUT/serial.log"; rm -f "$LOG"
CARD="$S/run-$TAG.img"; cp -c "$BASE" "$CARD"
{ echo "run $TAG $(date '+%Y-%m-%d %H:%M:%S %Z')"; shasum -a 256 "$K" | sed "s#  .*#  kernel8.img ($(basename "$K"))#"
  shasum -a 256 "$BASE" | sed "s#  .*#  base card $(basename "$BASE")#"; } > "$OUT/run.txt"
SD="$(mktemp -d "$S/q.XXXXXX")"; cd "$SD"   # QMP socket relative: an absolute scratch path exceeds the 104-byte limit
qemu-system-aarch64 -M raspi4b -no-reboot -display none -append serial=on -kernel "$K" \
  -drive file="$CARD",if=sd,format=raw -serial file:"$LOG" -qmp unix:qmp.sock,server=on,wait=off > "$OUT/qemu-stdout.txt" 2>&1 &
QP=$!
python3 "$HERE/mdrive.py" qmp.sock "$OUT" "$TAG" "$LOG" --timeout "${TMO:-150}" --quit-delay "${QDELAY:-1}" "$@" >> "$OUT/run.txt" 2>&1
RC=$?
for _ in $(seq 1 20); do kill -0 "$QP" 2>/dev/null || break; sleep 0.5; done
kill -9 "$QP" 2>/dev/null; wait "$QP" 2>/dev/null
LBA=$(python3 -I -c 'import struct,sys; d=open(sys.argv[1],"rb").read(512); print(struct.unpack_from("<I",d,0x1C6)[0])' "$CARD")
DEV=$(hdiutil attach -imagekey diskimage-class=CRawDiskImage -nomount "$CARD" 2>/dev/null | head -1 | awk '{print $1}')
fsck_msdos -n "${DEV}s1" > "$OUT/fsck.txt" 2>&1; echo "fsck rc=$?" >> "$OUT/fsck.txt"
hdiutil detach "$DEV" >/dev/null 2>&1 || true
for f in MCHPROBE.LOG SVCSEQ.LOG; do mcopy -n -i "$CARD@@$((LBA*512))" "::$f" "$OUT/$f" 2>/dev/null || echo "$f: not on the card" >> "$OUT/run.txt"; done
echo "clone after run $(shasum -a 256 "$CARD" | cut -c1-64) (deleted)" >> "$OUT/run.txt"
cd "$S"; rm -rf "$SD"; rm -f "$CARD"
echo "mdrive rc=$RC $(tail -1 "$OUT/fsck.txt") EXC/PANIC: $(grep -Ec '\[EXC\]|PANIC|panicked|\[crash\]' "$LOG") lines: $(grep -c . "$LOG")" | tee -a "$OUT/run.txt"
