#!/usr/bin/env bash
# paging-run.sh <scratch> <out> <kernel8.img> <WFPROBE.NCP>
# The interactive wfprobe build (no WFCASE/WFKEYS) as C:\AUTO\WFPROBE.NCP: it
# runs before the AES on the text console, as from EmuCON on the Pi. Card:
# tools/mktestdisk.sh (C:, AUTO + NEWDESK.INF) + tools/mkcard.sh
# MKCARD_TESTDISK (D: behind it). Kernel: the scratch kernel with the key
# injector (apply-injector.py) and sd-fault. A QMP screendump 1 s after every
# prompt the serial line shows (the next key comes 3 s into the wait).
set -euo pipefail
SCR=$1; OUT=$2; KERNEL=$3; NCP=$4
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../../.." && pwd)
mkdir -p "$SCR" "$OUT/screendumps"
printf '#R 02\r\n' > "$SCR/newdesk.inf"
cd "$REPO"
tools/mktestdisk.sh "$SCR/test.img" NEWDESK.INF="$SCR/newdesk.inf" AUTO/WFPROBE.NCP="$NCP" > "$SCR/mktestdisk.stdout"
cp "$SCR/test.img.manifest" "$OUT/test.img.manifest"
MKCARD_TESTDISK="$SCR/test.img" tools/mkcard.sh "$SCR/card.img" "$SCR/newdesk.inf" > "$SCR/mkcard.stdout" 2>&1 || {
    cat "$SCR/mkcard.stdout" >&2; exit 1; }
cp "$SCR/card.img.manifest" "$OUT/card.img.manifest"
cp -c "$SCR/card.img" "$SCR/boot.img"
{
    echo "kernel $(shasum -a 256 "$KERNEL" | cut -d' ' -f1)"
    echo "ncp    $(shasum -a 256 "$NCP" | cut -d' ' -f1)"
    echo "card   $(shasum -a 256 "$SCR/card.img" | cut -d' ' -f1) (before the run)"
} > "$OUT/run.txt"
SER=$OUT/BOOT-SERIAL.txt; : > "$SER"
QMP=127.0.0.1:${QMP_PORT:-45491}
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$SER" -append serial=on \
    -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw -qmp tcp:"$QMP",server=on,wait=off &
pid=$!
shots=0; seen=0; state=timeout
for _ in $(seq 1 600); do
    sleep 0.5
    n=$( (grep -o -e '-- more: any key --' -e 'WFPROBE done -- any key' "$SER" || true) | wc -l | tr -d ' ')
    if [ "$n" -gt "$seen" ]; then
        seen=$n; sleep 1; shots=$((shots + 1))
        python3 -I "$HERE/qmpshot.py" "$QMP" "$OUT/screendumps/prompt-$(printf %02d $shots).png" || true
    fi
    if grep -q "desktop up" "$SER"; then state=done; break; fi
    kill -0 "$pid" 2>/dev/null || { state=qemu-exited; break; }
done
sleep 1
kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
echo "qemu   $state, $shots screendumps" >> "$OUT/run.txt"
MTOOLS_SKIP_CHECK=1 mcopy -o -i "$SCR/boot.img@@$((63 * 512))" ::/WFPROBE.LOG "$OUT/WFPROBE.LOG" ||
    echo "no WFPROBE.LOG on C:" >> "$OUT/run.txt"
echo "card   $(shasum -a 256 "$SCR/boot.img" | cut -d' ' -f1) (after the run)" >> "$OUT/run.txt"
rm -f "$SCR/boot.img" "$SCR/card.img" "$SCR/test.img"
