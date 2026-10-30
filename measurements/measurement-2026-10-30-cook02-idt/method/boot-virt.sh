#!/bin/sh
# boot-virt.sh <log> -- COOK02 boot check on qemu-virt: release build of this tree
# through `cargo xtask run --board qemu-virt --release --headless` (programs, then
# the kernel, then QEMU virt headless; no block device, the embedded NEWDESK).
# Stops 5 s after "desktop up" or after 240 s, then kills exactly the QEMU
# processes that were not running before the start.
# (adapted from measurement-2026-10-30-xbios04/method/boot-virt.sh: no --sd card,
# QEMU stopped by pid instead of the pkill pattern that missed there.)
LOG=$1
before=$(pgrep -f qemu-system-aarch64 | tr '\n' ' ')
cargo xtask run --board qemu-virt --release --headless > "$LOG" 2>&1 < /dev/null &
pid=$!
n=0
while [ $n -lt 240 ]; do
  if grep -q "desktop up" "$LOG" 2>/dev/null; then sleep 5; break; fi
  sleep 1; n=$((n+1))
done
for q in $(pgrep -f qemu-system-aarch64); do
  case " $before " in *" $q "*) ;; *) kill "$q" 2>/dev/null ;; esac
done
sleep 1; kill $pid 2>/dev/null
echo "waited $n s"
