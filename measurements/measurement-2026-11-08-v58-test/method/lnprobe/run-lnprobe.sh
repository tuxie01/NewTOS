#!/bin/bash
# run-lnprobe.sh <LNPROBE.NAP> <kernel8.img> <out-dir> : QEMU raspi4b serial=on on an APFS clone of a fresh
# tools/mkcard.sh card (#R 02, #Z autorun of the probe), until the probe exited; reads LNPROBE.LOG off C:.
# The QEMU is stopped by the PID recorded at its start.  env S = scratch.
set -u
W=/Volumes/Daten/Development/PiRusTOS/.claude/worktrees/agent-a88c9f69bef56fa4d
S=${S:?}; NAP=$1; K=$2; O=$3
[ ! -e "$O" ] || { echo "out exists" >&2; exit 1; }
mkdir -p "$O"; O=$(cd "$O" && pwd)
SC=$(mktemp -d "$S/ln.XXXXXX"); trap 'rm -rf "$SC"' EXIT
printf '#R 02\r\n#Z 01 C:\\LNPROBE.NAP@\r\n' > "$SC/NEWDESK.INF"
(cd "$W" && tools/mkcard.sh "$SC/card.img" "$SC/NEWDESK.INF" LNPROBE.NAP="$NAP") > "$SC/mk.out" 2>&1 || { cat "$SC/mk.out" >&2; exit 1; }
cp "$SC/card.img.manifest" "$O/"; cp -c "$SC/card.img" "$SC/boot.img"
: > "$O/BOOT-SERIAL.txt"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$O/BOOT-SERIAL.txt" -append serial=on \
    -kernel "$K" -drive file="$SC/boot.img",if=sd,format=raw &
pid=$!
echo "qemu pid $pid" > "$O/run.txt"
state=timeout
for _ in $(seq 1 300); do
  sleep 2
  if grep -a -q "LNPROBE.NAP' exited with rc" "$O/BOOT-SERIAL.txt"; then state=done; break; fi
  kill -0 $pid 2>/dev/null || { state=qemu-exited; break; }
done
sleep 2
kill $pid 2>/dev/null; sleep 1; kill -9 $pid 2>/dev/null; wait $pid 2>/dev/null
echo "state $state kernel $(shasum -a 256 "$K" | cut -d' ' -f1) nap $(shasum -a 256 "$NAP" | cut -d' ' -f1)" >> "$O/run.txt"
cstart=$(od -An -t u4 -j 454 -N 4 "$SC/boot.img" | tr -d ' ')
MTOOLS_SKIP_CHECK=1 mcopy -o -i "$SC/boot.img@@$((cstart * 512))" ::/LNPROBE.LOG "$O/LNPROBE.LOG" || echo "no LNPROBE.LOG" >> "$O/run.txt"
grep -a -c -iE "exception|panic" "$O/BOOT-SERIAL.txt" | sed 's/^/exception-panic-lines /' >> "$O/run.txt"
cat "$O/run.txt"; cat "$O/LNPROBE.LOG" 2>/dev/null | tail -3
