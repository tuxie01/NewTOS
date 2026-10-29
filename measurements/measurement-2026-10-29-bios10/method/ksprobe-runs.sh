#!/bin/bash
# ksprobe-runs.sh <scratch> <runs-dir> <before|after> -- BIOS10: the permanent probe
# tosdump/probes/ksprobe.c (Kbshift set arm K01-K13, key path T01-T24) through its runner
# tosdump/probes/run-ksprobe.sh. Keys: tosdump/probes/ksprobe-keys.txt (both sides).
#   before  (main tree with no product file edited yet: only the new probe files)
#           oracle tt + falcontc, and NewTOS with the injector kernel of the before tree
#   after   NewTOS: injector kernel, the same with --features bios-arg-trace (input check),
#           and the product kernel without injector (K part; part H ends "H99")
# Injector kernels: `run-ksprobe.sh kernel <tree> <out> [features]` = a scratch copy of the
# tree armed by tools/inject-arm.py launch:KSPROBE=tosdump/probes/ksprobe-keys.txt
# (method/injector-ksprobe-scratch.diff is that patch), cargo xtask build --board rpi4 --release.
set -u
S=$1; R=$2; TAG=$3
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
cd "$REPO"
export KSPROBE_SCRATCH=$S
P=tosdump/probes/run-ksprobe.sh
if [ "$TAG" = before ]; then
    $P kernel . "$S/kernel8-inj-before.img"
    $P emutos tt "$R/ksprobe-EMUTOS-tt"; echo "emutos tt exit $?"
    $P emutos falcontc "$R/ksprobe-EMUTOS-falcontc"; echo "emutos falcontc exit $?"
    $P newtos "$R/ksprobe-NEWTOS-before" "$S/kernel8-inj-before.img"; echo "newtos before exit $?"
else
    $P kernel . "$S/kernel8-inj-after.img"
    $P kernel . "$S/kernel8-inj-after-trace.img" bios-arg-trace
    cargo xtask build --board rpi4 --release >/dev/null 2>&1
    cp target/aarch64-unknown-none-softfloat/release/kernel8.img "$S/kernel8-after.img"
    $P newtos "$R/ksprobe-NEWTOS-after" "$S/kernel8-inj-after.img"; echo "after exit $?"
    $P newtos "$R/ksprobe-NEWTOS-after-trace" "$S/kernel8-inj-after-trace.img"; echo "after-trace exit $?"
    $P newtos "$R/ksprobe-NEWTOS-after-noinjector" "$S/kernel8-after.img"; echo "after-noinjector exit $?"
fi
