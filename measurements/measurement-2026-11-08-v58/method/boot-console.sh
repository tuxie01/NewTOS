#!/bin/bash
# boot-console.sh <out-dir> <kernel8.img> <qemu-fb-extra or ""> : boot screen + AUTO console program
# (APITEST.NCP, blocks on Cconin) screendumps at 6/10/18/24/32/45 s, at a given framebuffer geometry.
# QEMU raspi4b serial=on on an APFS clone of a fresh tools/mkcard.sh card, stopped by its recorded PID.
# (V58; pattern of measurements/measurement-2026-11-07-v51/method/boot-console.sh, REPO = the V58 worktree.)
set -euo pipefail
REPO=/Volumes/Daten/Development/PiRusTOS/.claude/worktrees/agent-a88c9f69bef56fa4d
M=$REPO/measurements/measurement-2026-11-08-v58
cd "$REPO"
S=${S:?}; APITEST=${APITEST:?}
OUT=$1; KERNEL=$2; EXTRA=$3
[ ! -e "$OUT" ] || { echo exists >&2; exit 1; }
mkdir -p "$OUT/shots"; OUT=$(cd "$OUT" && pwd)
SCR=$(mktemp -d "$S/bc.XXXXXX"); trap 'rm -rf "$SCR"' EXIT
printf '#R 02\r\n' > "$SCR/NEWDESK.INF"
tools/mkcard.sh "$SCR/card.img" "$SCR/NEWDESK.INF" AUTO/APITEST.NCP="$APITEST" > "$SCR/mk.out" 2>&1 || { cat "$SCR/mk.out"; exit 1; }
cp "$SCR/card.img.manifest" "$OUT/"; cp -c "$SCR/card.img" "$SCR/boot.img"
echo "kernel $(shasum -a 256 "$KERNEL" | cut -d' ' -f1) APITEST.NCP $(shasum -a 256 "$APITEST" | cut -d' ' -f1) extra [$EXTRA]" > "$OUT/run.txt"
cd "$SCR"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" $EXTRA -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw -qmp unix:qmp.sock,server=on,wait=off &
pid=$!
echo "qemu pid $pid" >> "$OUT/run.txt"
python3 -I "$M/method/timed-shots.py" qmp.sock "$OUT/shots" 6 10 18 24 32 45 > "$OUT/shots.log" 2>&1 || true
kill $pid 2>/dev/null || true; sleep 1; kill -9 $pid 2>/dev/null || true; wait $pid 2>/dev/null || true
grep -a -c -i -E "exception|panic" "$OUT/BOOT-SERIAL.txt" | sed 's/^/exception-panic-lines /' >> "$OUT/run.txt" || true
cat "$OUT/run.txt"
