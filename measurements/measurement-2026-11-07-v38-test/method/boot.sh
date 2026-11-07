#!/bin/bash
# V38 acceptance: boot one kernel to the desktop and count exceptions.
#   boot.sh rpi4 <kernel8.img> <out-dir>          QEMU raspi4b, -append serial=on, C: = tools/mkcard.sh card (NEWDESK.INF "#R 02"), APFS clone
#   boot.sh qemu-virt <kernel ELF> <out-dir>      QEMU virt,gic-version=2 -cpu cortex-a72 -m 272M -device ramfb, qemu-xhci+usb-kbd+usb-mouse, -display none
# (the qemu-virt line is the one cargo xtask run --headless builds, tools/xtask/src/main.rs:925-960, with -serial file: instead of stdio
#  so that this script owns the PID). Waits for "desktop up" (limit 120 s), then 3 s more, stops QEMU by its PID.
set -u
BOARD=$1; KERNEL=$2; OUT=$3
S=${S:?set S}
[ ! -e "$OUT" ] || { echo "out exists" >&2; exit 1; }
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
SCR=$(mktemp -d "$S/boot.XXXXXX")
: > "$OUT/BOOT-SERIAL.txt"
if [ "$BOARD" = rpi4 ]; then
  cd /Volumes/Daten/Development/PiRusTOS
  printf '#R 02\r\n' > "$SCR/newdesk.inf"
  tools/mkcard.sh "$SCR/card.img" "$SCR/newdesk.inf" > "$SCR/mkcard.stdout" 2>&1 || { cat "$SCR/mkcard.stdout"; exit 1; }
  cp "$SCR/card.img.manifest" "$OUT/card.img.manifest"
  cp -c "$SCR/card.img" "$SCR/boot.img"
  qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" \
      -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw &
else
  qemu-system-aarch64 -M virt,gic-version=2 -cpu cortex-a72 -m 272M -device ramfb \
      -device qemu-xhci -device usb-kbd -device usb-mouse -display none \
      -serial file:"$OUT/BOOT-SERIAL.txt" -no-reboot -kernel "$KERNEL" &
fi
pid=$!
state=timeout
for _ in $(seq 1 240); do
  sleep 0.5
  if grep -q "desktop up" "$OUT/BOOT-SERIAL.txt"; then state=desktop-up; break; fi
  kill -0 "$pid" 2>/dev/null || { state=qemu-exited; break; }
done
sleep 3
kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
echo "state $state" > "$OUT/result.txt"
echo "exception/panic lines: $(grep -ciE 'exception|panic|abort' "$OUT/BOOT-SERIAL.txt")" >> "$OUT/result.txt"
echo "serial lines: $(wc -l < "$OUT/BOOT-SERIAL.txt")" >> "$OUT/result.txt"
echo "kernel $(shasum -a 256 "$KERNEL" | cut -d' ' -f1)" >> "$OUT/result.txt"
rm -f "$SCR/card.img" "$SCR/boot.img"
cat "$OUT/result.txt"; grep "desktop up" "$OUT/BOOT-SERIAL.txt"
