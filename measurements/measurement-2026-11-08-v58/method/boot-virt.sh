#!/bin/bash
# boot-virt.sh <tree> <out-dir> : qemu-virt headless boot of <tree>'s own fresh release build (V58).
# Builds with `cargo xtask build --board qemu-virt --release`, makes the flat image as `cargo xtask run`
# does (rust objcopy -O binary), and starts qemu-system-aarch64 itself with the arguments of
# `cargo xtask run --board qemu-virt --headless` (tools/xtask/src/main.rs, -M virt,gic-version=2 ...), so
# the QEMU is this script's own child: it is stopped by the PID recorded at its start, nothing else.
# Bounded to 100 s (stops 5 s after "desktop up"). Counts exception/panic lines.
set -u
T=$1; O=$2
[ ! -e "$O" ] || { echo "out exists: $O" >&2; exit 1; }
mkdir -p "$O"; O=$(cd "$O" && pwd)
cd "$T" || exit 1
rm -rf target/aarch64-unknown-none-softfloat
cargo xtask build --board qemu-virt --release > "$O/build.log" 2>&1 || { echo build failed; exit 1; }
ELF=target/aarch64-unknown-none-softfloat/release/kernel
OBJCOPY=$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/^host: //p')/bin/rust-objcopy
"$OBJCOPY" -O binary "$ELF" "$O/kernel.img"
shasum -a 256 "$ELF" "$O/kernel.img" | tee "$O/kernel.sha256"
: > "$O/serial.txt"
qemu-system-aarch64 -M virt,gic-version=2 -cpu cortex-a72 -m 272M -device ramfb \
    -device qemu-xhci -device usb-kbd -device usb-mouse -display none \
    -serial file:"$O/serial.txt" -no-reboot -kernel "$O/kernel.img" &
pid=$!
echo "qemu pid $pid" > "$O/qemu-pid.txt"
for _ in $(seq 1 100); do
  sleep 1
  kill -0 $pid 2>/dev/null || { echo "qemu exited early" >> "$O/qemu-pid.txt"; break; }
  grep -a -q "desktop up" "$O/serial.txt" && { sleep 5; break; }
done
kill $pid 2>/dev/null; sleep 1; kill -9 $pid 2>/dev/null; wait $pid 2>/dev/null
rm -f "$O/kernel.img"
{ grep -a -c -iE "exception|panic" "$O/serial.txt" | sed 's/^/exception-panic-lines /'; grep -a -E "desktop up" "$O/serial.txt"; } | tee "$O/summary.txt"
