#!/bin/bash
# pi-boot.sh <kernel8.img> <out-dir> : QEMU raspi4b (serial=on) boot of a Pi test kernel on a fresh
# tools/mkcard.sh card (#R 02 only, APFS clone) until "desktop up" (max 120 s), one screendump, exception
# lines counted. The QEMU is stopped by its recorded PID. (V58; pattern of
# measurements/measurement-2026-11-07-v51/method/pi-boot.sh, REPO = the V58 worktree.)
set -u
W=/Volumes/Daten/Development/PiRusTOS/.claude/worktrees/agent-a88c9f69bef56fa4d
M=$W/measurements/measurement-2026-11-08-v58-test
S=${S:?}; K=$1; O=$2
[ ! -e "$O" ] || { echo "out exists: $O" >&2; exit 1; }
mkdir -p "$O"; O=$(cd "$O" && pwd)
SC=$(mktemp -d "$S/pb.XXXXXX"); trap 'rm -rf "$SC"' EXIT
printf '#R 02\r\n' > "$SC/NEWDESK.INF"
for try in 1 2 3; do
  (cd "$W" && tools/mkcard.sh "$SC/card.img" "$SC/NEWDESK.INF") > "$SC/mk.out" 2>&1 && [ -f "$SC/card.img" ] && break
  echo "mkcard try $try failed" >&2; sleep 20
done
[ -f "$SC/card.img" ] || { cat "$SC/mk.out" >&2; exit 1; }
cp "$SC/card.img.manifest" "$O/"; cp -c "$SC/card.img" "$SC/boot.img"
: > "$O/BOOT-SERIAL.txt"
cd "$SC"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$O/BOOT-SERIAL.txt" -append serial=on \
    -kernel "$K" -drive file="$SC/boot.img",if=sd,format=raw -qmp unix:qmp.sock,server=on,wait=off &
pid=$!
echo "qemu pid $pid" > "$O/qemu-pid.txt"
for _ in $(seq 1 60); do sleep 2; grep -q "desktop up" "$O/BOOT-SERIAL.txt" && break; done
sleep 3
python3 -I "$M/method/timed-shots.py" qmp.sock "$O" 0 > "$O/shots.log" 2>&1
kill $pid 2>/dev/null; sleep 1; kill -9 $pid 2>/dev/null; wait $pid 2>/dev/null
{ echo "kernel $(shasum -a 256 "$K" | cut -d' ' -f1)"; grep -a -c -iE "exception|panic" "$O/BOOT-SERIAL.txt" | sed 's/^/exception-panic-lines /'; grep -a "desktop up" "$O/BOOT-SERIAL.txt"; } | tee "$O/summary.txt"
