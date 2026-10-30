#!/bin/bash
# boot-qemu-virt.sh <runs-dir> -- FS28: fresh qemu-virt release build of this tree, booted
# headless (cargo xtask run --board qemu-virt --release --headless) until "desktop up", then
# QEMU stopped. After measurements/measurement-2026-10-30-bios16/method/after-all.sh.
set -u
R=$1
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
cd "$REPO"
mkdir -p "$R/boot-qemu-virt-after"
OUT=$R/boot-qemu-virt-after/BOOT-SERIAL.txt
touch kernel/src/lib.rs
cargo xtask run --board qemu-virt --release --headless > "$OUT" 2>&1 &
pid=$!
for _ in $(seq 1 100); do sleep 3; /usr/bin/grep -aq "desktop up" "$OUT" && break; done
sleep 3
for c in $(pgrep -P $pid); do for g in $(pgrep -P "$c"); do kill "$g" 2>/dev/null; done; kill "$c" 2>/dev/null; done
kill $pid 2>/dev/null; wait $pid 2>/dev/null
/usr/bin/grep -a "desktop up\|PANIC" "$OUT"
