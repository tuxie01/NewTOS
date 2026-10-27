#!/usr/bin/env bash
# run-virt.sh -- fresh release qemu-virt kernel of the working tree (cargo xtask run --board qemu-virt --release:
# build, objcopy, QEMU virt with ramfb + xHCI keyboard/mouse, -display none -serial stdio, no C: drive), 60 s,
# serial -> <run>/serial/virt-boot-serial.log. Board footgun: touch first so the qemu-virt build is fresh.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; R="$HERE/.."; WS="$(cd "$R/../.." && pwd)"
cd "$WS"; touch kernel/src/lib.rs tos-api/src/lib.rs
cargo xtask run --board qemu-virt --release < /dev/null > "$R/serial/virt-boot-serial.log" 2>&1 &
P=$!
for _ in $(seq 1 240); do sleep 1; grep -q "desktop up" "$R/serial/virt-boot-serial.log" && break; done
sleep 10; pkill -f "qemu-system-aarch64 -M virt" ; wait $P 2>/dev/null
echo "virt: desk=$(grep -c 'desktop up' "$R/serial/virt-boot-serial.log") crash=$(grep -c '^\[crash\]' "$R/serial/virt-boot-serial.log") panic=$(grep -Ec 'PANIC|panicked|\[EXC\]' "$R/serial/virt-boot-serial.log")"
