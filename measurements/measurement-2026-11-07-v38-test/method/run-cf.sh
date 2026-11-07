#!/bin/bash
# V38 acceptance: the 5x3 probe (method/cfprobe.c, built as C:\VDPROBE.NAP) on NewTOS in QEMU raspi4b.
#   run-cf.sh <kernel8.img> <cfprobe.NAP> <out-dir> <time-limit-s>
# C: from tools/mkcard.sh (NEWDESK.INF "#R 02", "#Z 01 C:\VDPROBE.NAP@" autorun), APFS clone per boot,
# deleted after; QEMU stopped only by the PID started here. The serial file is polled every 0.05 s; the wall
# time of "starting C:\VDPROBE.NAP" -> "exited with rc" (or the time limit) is recorded.
set -u
KERNEL=$1; NAP=$2; OUT=$3; LIMIT=$4
S=${S:?set S}
[ ! -e "$OUT" ] || { echo "out exists" >&2; exit 1; }
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
SCR=$(mktemp -d "$S/cf.XXXXXX")
cd /Volumes/Daten/Development/PiRusTOS
printf '#R 02\r\n#Z 01 C:\\VDPROBE.NAP@\r\n' > "$SCR/newdesk.inf"
tools/mkcard.sh "$SCR/card.img" "$SCR/newdesk.inf" VDPROBE.NAP="$NAP" > "$SCR/mkcard.stdout" 2>&1 || { cat "$SCR/mkcard.stdout"; exit 1; }
cp "$SCR/card.img.manifest" "$OUT/card.img.manifest"
cp -c "$SCR/card.img" "$SCR/boot.img"
: > "$OUT/BOOT-SERIAL.txt"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" \
    -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw &
pid=$!
t0=$(python3 -c 'import time;print(time.time())')
ts=""; te=""; state=timeout
end=$(python3 -c "import time;print(time.time()+$LIMIT)")
while :; do
  now=$(python3 -c 'import time;print(time.time())')
  if [ -z "$ts" ] && grep -q "starting C:.VDPROBE.NAP" "$OUT/BOOT-SERIAL.txt"; then ts=$now; fi
  if [ -z "$te" ] && grep -q "VDPROBE.NAP' exited with rc" "$OUT/BOOT-SERIAL.txt"; then te=$now; state=returned; break; fi
  kill -0 "$pid" 2>/dev/null || { state=qemu-exited; break; }
  python3 -c "import sys;sys.exit(0 if $now<$end else 1)" || break
  sleep 0.05
done
sleep 4
kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
echo "state $state" > "$OUT/timing.txt"
echo "t_start_marker_s $(python3 -c "print(($ts if '$ts' else $t0) - $t0)")" >> "$OUT/timing.txt"
[ -n "$te" ] && echo "t_exit_marker_s $(python3 -c "print($te - $t0)")" >> "$OUT/timing.txt"
[ -n "$te" ] && [ -n "$ts" ] && echo "program_wall_s $(python3 -c "print($te - $ts)")" >> "$OUT/timing.txt"
cstart=$(od -An -t u4 -j 454 -N 4 "$SCR/boot.img" | tr -d ' ')
MTOOLS_SKIP_CHECK=1 mcopy -o -i "$SCR/boot.img@@$((cstart * 512))" ::/VDPROBE.LOG "$OUT/VDPROBE.LOG" 2>/dev/null || echo "no VDPROBE.LOG" >> "$OUT/timing.txt"
echo "kernel $(shasum -a 256 "$KERNEL" | cut -d' ' -f1)  nap $(shasum -a 256 "$NAP" | cut -d' ' -f1)" >> "$OUT/timing.txt"
rm -f "$SCR/card.img" "$SCR/boot.img"
cat "$OUT/timing.txt"; cat "$OUT/VDPROBE.LOG" 2>/dev/null
