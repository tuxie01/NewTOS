#!/bin/bash
# V53: the fix tree armed with method/wkprobe-mouse-c1late.txt (tosdump/probes/wkprobe-mouse.txt with the C1
# packet moved from +4000 to +8000 ticks). Why: on the fix the W1 "left button up" packet changes nothing
# (v_opnwk already cleared MOUSE_BT, as on EmuTOS), so the probe's NewTOS wait for it runs on and swallows the
# next scripted packet -- C1's, while the workstation is still open (probe/n-fix). With C1 at +8000 the W1 wait
# ends first (its poll cap, ~3600 ticks here) and C1's packet arrives inside C1's own wait, after its v_clswk.
set -euo pipefail
cd /Volumes/Daten/Development/PiRusTOS
S=${S:?}; REPO=$PWD
W=$(mktemp -d "$S/c1k.XXXXXX")
(cd "$S/t/fix" && tar --exclude ./target --exclude ./references -cf - .) | tar -xf - -C "$W"; ln -s "$REPO/references" "$W/references"
python3 -I tools/inject-arm.py "$W" "launch:WKPROBE=$S/method/wkprobe-mouse-c1late.txt" | tail -1
(cd "$W" && cargo xtask build --board rpi4 --release) > "$W.build.log" 2>&1 || { tail -30 "$W.build.log" >&2; exit 1; }
cp "$W/target/aarch64-unknown-none-softfloat/release/kernel8.img" $S/kinj-fix-c1late.img; rm -rf "$W"
echo "kernel $(shasum -a 256 $S/kinj-fix-c1late.img | cut -d' ' -f1)"
WKPROBE_SCRATCH=$S/wk tosdump/probes/run-wkprobe.sh newtos $S/probe/n-fix-c1late $S/kinj-fix-c1late.img > $S/probe/n-fix-c1late.stdout 2>&1
tail -3 $S/probe/n-fix-c1late/run.txt
