#!/bin/bash
# x51 extra, base and change, one QEMU at a time. S = scratch (required).
set -u
S=${S:?}; export S VDPROBE_SCRATCH=$S/vp
M=$(cd "$(dirname "$0")/.." && pwd)
mkdir -p "$VDPROBE_SCRATCH" "$M/probe-x51"
for n in base chg; do
  bash "$M/method/run-x51.sh" newtos "$M/probe-x51/$n" "$S/k/$n-rpi4-release.img" > "$M/probe-x51/$n.stdout" 2>&1
done
python3 -I "$M/../../tosdump/probes/vdprobe-compare.py" "$M/probe-x51/base" "$M/probe-x51/chg" > "$M/probe-x51/compare-base-chg.txt" 2>&1
cmp "$M/probe-x51/base/VDPROBE.BIN" "$M/probe-x51/chg/VDPROBE.BIN" > "$M/probe-x51/cmp-bin.txt" 2>&1; echo "cmp BIN exit $?" >> "$M/probe-x51/cmp-bin.txt"
diff "$M/probe-x51/base/VDPROBE.LOG" "$M/probe-x51/chg/VDPROBE.LOG" > "$M/probe-x51/log.diff"; echo "diff LOG exit $?" >> "$M/probe-x51/log.diff"
