#!/bin/bash
# boot-virt.sh <tree> <out-dir> [features] : qemu-virt headless boot of <tree>'s own fresh release build,
# bounded 100 s; counts exception/panic lines and the desktop line. (Pattern: the V34 tester's
# measurements/measurement-2026-11-07-v34-test/method/boot-virt.sh.)
T=$1; O=$2; F=${3:-}
mkdir -p "$O"; cd "$T" || exit 1
FA=(); [ -n "$F" ] && FA=(--features "$F")
rm -rf target/aarch64-unknown-none-softfloat
cargo xtask build --board qemu-virt --release ${FA[@]+"${FA[@]}"} > "$O/build.log" 2>&1 || { echo build failed; exit 1; }
shasum -a 256 target/aarch64-unknown-none-softfloat/release/kernel | tee "$O/kernel.sha256"
cargo xtask run --board qemu-virt --headless --release ${FA[@]+"${FA[@]}"} > "$O/serial.txt" 2>&1 &
pid=$!; sleep 100
pkill -f "qemu-system-aarch64.*$T/target" ; sleep 1; kill $pid 2>/dev/null; wait $pid 2>/dev/null
grep -a -c -iE "exception|panic" "$O/serial.txt" | sed 's/^/exception-panic-lines /' | tee "$O/summary.txt"
grep -a -E "desktop up" "$O/serial.txt" | tee -a "$O/summary.txt"
