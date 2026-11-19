#!/usr/bin/env bash
# run.sh <kernel8.img> <base card.img> <tag> "<append>" STEP... -- one headless QEMU raspi4b run on an APFS clone (cp -c)
# of the base card. Derived from measurement-2026-11-18-xbios22-test/method/run.sh: the -append is a parameter
# (serial=on [vdiplanes=32]); serial -> ../serial/<tag>.log, screendumps -> ../screendumps/<tag>-*.png (method/mdrive.py,
# STEP = 'REGEX@N=DELAY:name.png'). After the run fsck_msdos -n of the clone (via hdiutil attach of the clone only), clone sha256.
# Env: S scratch dir (agent dir/scratch), TMO per-step timeout, QDELAY wait after last step, TRACEFILES=1 copy SVCSEQ.LOG.
set -uo pipefail
K="$1"; BASE="$2"; TAG="$3"; APP="$4"; shift 4
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; S="${S:?S = scratch dir}"
OUT="$HERE/.."; mkdir -p "$OUT/serial" "$OUT/screendumps"; LOG="$OUT/serial/$TAG.log"; rm -f "$LOG"
CARD="$S/run-$TAG.img"; cp -c "$BASE" "$CARD"
{ echo "run $TAG $(date '+%Y-%m-%d %H:%M:%S %Z') append='$APP'"; shasum -a 256 "$K" | sed "s#  .*#  kernel8.img ($(basename "$K"))#"
  shasum -a 256 "$BASE" | sed "s#  .*#  base card $(basename "$BASE")#"; } > "$OUT/serial/$TAG.run.txt"
SD="$(mktemp -d "$S/q.XXXXXX")"; cd "$SD"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -append "$APP" -kernel "$K" \
  -drive file="$CARD",if=sd,format=raw -serial file:"$LOG" -qmp unix:qmp.sock,server=on,wait=off > "$SD/qemu-stdout.txt" 2>&1 &
QP=$!
SHOTS="$SD/shots"; mkdir -p "$SHOTS"
python3 "$HERE/mdrive.py" qmp.sock "$SHOTS" "$TAG" "$LOG" --timeout "${TMO:-150}" --quit-delay "${QDELAY:-1}" "$@" >> "$OUT/serial/$TAG.run.txt" 2>&1
RC=$?
for _ in $(seq 1 20); do kill -0 "$QP" 2>/dev/null || break; sleep 0.5; done
kill -9 "$QP" 2>/dev/null; wait "$QP" 2>/dev/null
cp "$SHOTS"/*.png "$OUT/screendumps/" 2>/dev/null
LBA=$(python3 -I -c 'import struct,sys; d=open(sys.argv[1],"rb").read(512); print(struct.unpack_from("<I",d,0x1C6)[0])' "$CARD")
if [ -n "${TRACEFILES:-}" ]; then for f in SVCSEQ.LOG; do mcopy -n -i "$CARD@@$((LBA*512))" "::$f" "$OUT/serial/$TAG-$f" 2>/dev/null || echo "$f: not on the card" >> "$OUT/serial/$TAG.run.txt"; done; fi
dd if="$CARD" of="$SD/fat.img" bs=512 skip=$LBA 2>/dev/null
fsck_msdos -n "$SD/fat.img" > "$OUT/serial/$TAG.fsck.txt" 2>&1; echo "fsck rc=$?" >> "$OUT/serial/$TAG.fsck.txt"
echo "clone after run $(shasum -a 256 "$CARD" | cut -c1-64) (deleted)" >> "$OUT/serial/$TAG.run.txt"
cd "$S"; rm -rf "$SD"; rm -f "$CARD"
echo "$TAG rc=$RC $(tail -1 "$OUT/serial/$TAG.fsck.txt") EXC/PANIC: $(grep -Ec '\[EXC\]|PANIC|panicked|\[crash\]' "$LOG") lines: $(grep -c . "$LOG")" | tee -a "$OUT/serial/$TAG.run.txt"
