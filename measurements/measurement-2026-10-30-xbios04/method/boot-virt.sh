#!/bin/sh
# boot-virt.sh <log> -- XBIOS04 boot check on qemu-virt: fresh release build of this
# tree through `cargo xtask run --board qemu-virt --release --headless` (builds the
# programs and the kernel for the board, then starts QEMU virt headless), stop 5 s
# after "desktop up" or after 240 s. qemu-virt has no block device: the --sd card
# (scratch, tools/mkcard.sh with NEWDESK.INF "#R 02") is not attached by xtask for
# this board; the desktop is the embedded NEWDESK.
# Note: the pkill pattern below did not match (no -drive on qemu-virt), so QEMU kept
# running after the check (reported in the hand-off).
S=$(dirname "$0"); LOG=$1
cargo xtask run --board qemu-virt --release --headless --sd "$S/boot.img" > "$LOG" 2>&1 < /dev/null &
pid=$!
n=0
while [ $n -lt 240 ]; do
  if grep -q "desktop up" "$LOG" 2>/dev/null; then sleep 5; break; fi
  sleep 1; n=$((n+1))
done
pkill -f "qemu-system-aarch64 -M virt.*$S/boot.img" 2>/dev/null
sleep 1; kill $pid 2>/dev/null
echo "waited $n s"
