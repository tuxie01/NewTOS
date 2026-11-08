#!/bin/bash
# spin.sh <T base|chg> <key a|b>: qemu-virt + usb-kbd/usb-mouse on the scratch-patched desktop kernel; QEMU killed by PID.
set -u
S=${S:?}; W=/Volumes/Daten/Development/PiRusTOS; M=$W/measurements/measurement-2026-11-08-v55-test
T=$1; K=$2; O=$M/runs/spin-$T-$K; mkdir -p "$O"; SC=$(mktemp -d "$S/sp.XXXXXX"); : > "$O/serial.txt"
( cd "$SC" && exec qemu-system-aarch64 -M virt,gic-version=2 -cpu cortex-a72 -m 272M -device ramfb \
  -device qemu-xhci -device usb-kbd -device usb-mouse -display none -serial file:"$O/serial.txt" -no-reboot \
  -kernel "$S/k/spin-$T-virt.img" -qmp unix:qmp.sock,server=on,wait=off ) &
pid=$!; echo "$(date +%H:%M:%S) spin-$T-$K pid $pid started" >> "$M/analysis/qemu-pids.txt"
(cd "$SC" && python3 -I "$M/method/virt-spin.py" qmp.sock "$O/serial.txt" "$O" $K) > "$O/virt-spin.stdout" 2>&1 || echo "virt-spin failed" >> "$O/run.txt"
kill $pid 2>/dev/null; sleep 1; kill -9 $pid 2>/dev/null; wait $pid 2>/dev/null
kill -0 $pid 2>/dev/null && echo "$(date +%H:%M:%S) spin-$T-$K pid $pid STILL ALIVE" >> "$M/analysis/qemu-pids.txt" || echo "$(date +%H:%M:%S) spin-$T-$K pid $pid gone" >> "$M/analysis/qemu-pids.txt"
grep -a -c -iE "exception|panic" "$O/serial.txt" | sed 's/^/exception-panic-lines /' >> "$O/run.txt"
rm -rf "$SC"
