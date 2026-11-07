#!/bin/bash
# V53 tester: injector-armed wkprobe kernels of base and fix (run-wkprobe.sh kernel), plus fix armed with the c1late script.
set -euo pipefail
S=${S:?}; cd /Volumes/Daten/Development/PiRusTOS
export WKPROBE_SCRATCH=$S/wk
for T in base fix; do tosdump/probes/run-wkprobe.sh kernel $S/t/$T $S/kinj-$T.img; done
W=$(mktemp -d "$S/wk/c1k.XXXXXX")
(cd "$S/t/fix" && tar --exclude ./target --exclude ./references -cf - .) | tar -xf - -C "$W"; ln -s "$PWD/references" "$W/references"
python3 -I tools/inject-arm.py "$W" "launch:WKPROBE=$PWD/measurements/measurement-2026-11-07-v53/method/wkprobe-mouse-c1late.txt" | tail -1
(cd "$W" && cargo xtask build --board rpi4 --release) > "$W.build.log" 2>&1 || { tail -30 "$W.build.log" >&2; exit 1; }
cp "$W/target/aarch64-unknown-none-softfloat/release/kernel8.img" $S/kinj-fix-c1late.img; rm -rf "$W" "$W.build.log"
shasum -a 256 $S/kinj-fix-c1late.img
